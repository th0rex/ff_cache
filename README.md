Prune the firefox cache from your command line!

Disclaimer: This seems to be working for me, but isn't really tested.

# Installation

`cargo install --git https://github.com/th0rex/ff_cache`

# Usage

Make sure `~/.cargo/bin` is in your path.

`ff_cache /home/user/.cache/mozilla/firefox/profile_name target_cache_size_in_kb`

# Why would I need this?
I used this in systemd script that moves my firefox cache and profile to a
zram device, so firefox doesn't use the disk.
To ensure that my ram usage doesn't explode I prune the firefox cache to 300mb
on every shutdown in the same systemd script.

My scripts and systemd service for that can be found [here](https://github.com/th0rex/dotfiles/blob/master/systemd/zram.service), (here)[https://github.com/th0rex/dotfiles/blob/master/bin/ff_cache) and [here](https://github.com/th0rex/dotfiles/blob/master/bin/ff_uncache).

# Credits
The datastructures for the cache are taken from the [firefox source](https://searchfox.org/mozilla-central/source/netwerk/cache2/CacheIndex.h).
