use std::{
    env,
    fmt::Write as _,
    fs,
    io::{self, Read, Write},
    path::PathBuf,
    process,
    str::FromStr,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

// https://searchfox.org/mozilla-central/source/netwerk/cache2/CacheIndex.h
struct CacheIndexHeader {
    version: u32,
    time_stamp: u32,
    is_dirty: u32,
}

impl CacheIndexHeader {
    fn parse<R: Read>(mut r: R) -> io::Result<Self> {
        let version = r.read_u32::<BigEndian>()?;
        let time_stamp = r.read_u32::<BigEndian>()?;
        let is_dirty = r.read_u32::<BigEndian>()?;

        Ok(Self {
            version,
            time_stamp,
            is_dirty,
        })
    }

    fn write<W: Write>(&self, mut w: W) -> io::Result<()> {
        w.write_u32::<BigEndian>(self.version)?;
        w.write_u32::<BigEndian>(self.time_stamp)?;
        w.write_u32::<BigEndian>(self.is_dirty)?;

        Ok(())
    }
}

struct CacheIndexRecord {
    hash: [u8; 20],
    frecency: u32,
    origin_attrs_hash: u64,
    on_start_time: u16,
    on_stop_time: u16,
    content_type: u8,
    base_domain_access_count: u16,
    file_size: u32,
    is_reserved: bool,
    has_cached_alt_data: bool,
    is_pinned: bool,
    is_fresh: bool,
    is_dirty: bool,
    is_removed: bool,
    is_anonymous: bool,
    is_initialized: bool,
}

impl CacheIndexRecord {
    fn parse<R: Read>(mut r: R) -> io::Result<Self> {
        let mut hash = [0; 20];
        r.read_exact(&mut hash)?;
        let frecency = r.read_u32::<BigEndian>()?;
        let origin_attrs_hash = r.read_u64::<BigEndian>()?;
        let on_start_time = r.read_u16::<BigEndian>()?;
        let on_stop_time = r.read_u16::<BigEndian>()?;
        let content_type = r.read_u8()?;
        let base_domain_access_count = r.read_u16::<BigEndian>()?;
        let flags = r.read_u32::<BigEndian>()?;
        let file_size = flags & 0x00FF_FFFF;
        let is_reserved = (flags >> 24) != 0;
        let has_cached_alt_data = (flags >> 25) != 0;
        let is_pinned = (flags >> 26) != 0;
        let is_fresh = (flags >> 27) != 0;
        let is_dirty = (flags >> 28) != 0;
        let is_removed = (flags >> 29) != 0;
        let is_anonymous = (flags >> 30) != 0;
        let is_initialized = (flags >> 31) != 0;

        Ok(Self {
            hash,
            frecency,
            origin_attrs_hash,
            on_start_time,
            on_stop_time,
            content_type,
            base_domain_access_count,
            file_size,
            is_reserved,
            has_cached_alt_data,
            is_pinned,
            is_fresh,
            is_dirty,
            is_removed,
            is_anonymous,
            is_initialized,
        })
    }

    fn write<W: Write>(&self, mut w: W) -> io::Result<()> {
        w.write_all(&self.hash)?;
        w.write_u32::<BigEndian>(self.frecency)?;
        w.write_u64::<BigEndian>(self.origin_attrs_hash)?;
        w.write_u16::<BigEndian>(self.on_start_time)?;
        w.write_u16::<BigEndian>(self.on_stop_time)?;
        w.write_u8(self.content_type)?;
        w.write_u16::<BigEndian>(self.base_domain_access_count)?;
        let mut flags = self.file_size;
        flags |= (self.is_reserved as u32) << 24;
        flags |= (self.has_cached_alt_data as u32) << 25;
        flags |= (self.is_pinned as u32) << 26;
        flags |= (self.is_fresh as u32) << 27;
        flags |= (self.is_dirty as u32) << 28;
        flags |= (self.is_removed as u32) << 29;
        flags |= (self.is_anonymous as u32) << 30;
        flags |= (self.is_initialized as u32) << 31;
        w.write_u32::<BigEndian>(flags)?;

        Ok(())
    }
}

fn get_path(name: &str) -> Option<PathBuf> {
    let mut buf = PathBuf::from_str(name).ok()?;
    buf.push("cache2");

    Some(buf)
}

fn get_size(path: &mut PathBuf) -> Option<u64> {
    let mut size = 0;

    path.push("entries");
    for x in fs::read_dir(&path).unwrap() {
        let x = x.ok()?;
        assert!(!x.file_type().ok()?.is_dir());
        size += x.metadata().ok()?.len();
    }
    path.pop();

    Some(size)
}

fn main() -> io::Result<()> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!("Usage: ./ff_cache <profile_name> <target size of profile cache in kilobytes>");
        process::exit(3);
    }

    let mut profile_path =
        get_path(&args[1]).expect("Could not get path to profile, does it exist?");

    let target_size = args[2]
        .parse::<u64>()
        .expect("Could not parse target size!");

    let mut cache_size =
        get_size(&mut profile_path).expect("Could not calculate current cache size") / 1024;

    if cache_size <= target_size {
        // Nothing to do
        return Ok(());
    }

    profile_path.push("index");
    let mut file = fs::File::open(&profile_path)?;
    profile_path.pop();

    let header = CacheIndexHeader::parse(&mut file)?;

    if header.is_dirty != 0 {
        eprintln!("Cache is dirty, please close all firefox instances");
        process::exit(1);
    }

    if header.version != 8 {
        eprintln!("Unsupported version: {}", header.version);
        process::exit(2);
    }

    let mut records = Vec::new();
    while let Ok(record) = CacheIndexRecord::parse(&mut file) {
        records.push(record);
    }

    records.sort_unstable_by_key(|x| x.frecency);
    profile_path.push("entries");

    while cache_size > target_size {
        let removed = records.pop().unwrap();
        let mut file_name = String::new();
        for x in &removed.hash[..] {
            write!(&mut file_name, "{:02X}", x).expect("wtf");
        }
        profile_path.push(&file_name);
        println!("{}", profile_path.as_path().to_str().unwrap());
        fs::remove_file(&profile_path).expect("could not remove cache entry");
        profile_path.pop();

        cache_size -= removed.file_size as u64;
    }
    profile_path.pop();

    profile_path.push("index");
    file = fs::File::create(&profile_path)?;

    header.write(&mut file)?;

    for record in records {
        record.write(&mut file)?;
    }

    Ok(())
}
