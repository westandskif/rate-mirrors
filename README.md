# Rate Arch Mirrors

![Tag Badge](https://img.shields.io/github/tag/westandskif/rate-arch-mirrors.svg)

This is a tool, which fetches mirrors, skips outdated/syncing Arch Linux mirrors, then uses info about submarine cables and internet exchanges to jump between countries and find fast mirrors. And it's fast enough to run it before each update:

Here is an example of the use (_output truncated; running from Belarus_):
```
~ ❯❯❯ rate-arch-mirrors --max-delay=43200
# JUMP #1
# EXPLORING US
# VISITED US
#     + NEIGHBOR GB (by HubsFirst)
#     + NEIGHBOR ID (by HubsFirst)
#     + NEIGHBOR CN (by HubsFirst)
#     + NEIGHBOR DE (by DistanceFirst)
#     + NEIGHBOR CA (by DistanceFirst)
#     + NEIGHBOR FR (by DistanceFirst)
# [GB] SpeedTestResult { speed: 1.06 MB/s; elapsed: 538.340192ms; connection_time: 881.71627ms}
# [US] SpeedTestResult { speed: 460.57 KB/s; elapsed: 177.295735ms; connection_time: 1.055605287s}
# [ID] SpeedTestResult { speed: 150.09 KB/s; elapsed: 531.388261ms; connection_time: 963.771889ms}
# [ID] SpeedTestResult { speed: 204.86 KB/s; elapsed: 669.180228ms; connection_time: 551.295751ms}
# [US] SpeedTestResult { speed: 397.77 KB/s; elapsed: 875.305259ms; connection_time: 535.300839ms}
# [GB] SpeedTestResult { speed: 2.48 MB/s; elapsed: 869.932998ms; connection_time: 628.702664ms}
# [DE] SpeedTestResult { speed: 1.22 MB/s; elapsed: 1.182854363s; connection_time: 302.571795ms}
# [DE] SpeedTestResult { speed: 6.97 MB/s; elapsed: 1.304992036s; connection_time: 195.399714ms}
# [FR] SpeedTestResult { speed: 313.91 KB/s; elapsed: 782.076292ms; connection_time: 696.641193ms}
# [FR] SpeedTestResult { speed: 473.08 KB/s; elapsed: 1.215924352s; connection_time: 251.996648ms}
#     TOP NEIGHBOR - CONNECTION TIME: DE - 195.399714ms
#     TOP NEIGHBOR - SPEED: DE - 6.97 MB/s

# JUMP #2
# EXPLORING DE
# ...
#     TOP NEIGHBOR - CONNECTION TIME: CZ - 129.078569ms
#     TOP NEIGHBOR - SPEED: SE - 7.05 MB/s

# JUMP #3
# EXPLORING CZ
#     + NEIGHBOR PL (by DistanceFirst)
#     + NEIGHBOR HU (by DistanceFirst)
#     + NEIGHBOR RO (by DistanceFirst)
# EXPLORING SE
#     + NEIGHBOR FI (by DistanceFirst)
#     + NEIGHBOR RU (by DistanceFirst)
#     + NEIGHBOR BG (by DistanceFirst)
# ...

# JUMP #7
# ...

# RE-TESTING TOP MIRRORS
# [SE] SpeedTestResult { speed: 6.78 MB/s; elapsed: 1.359863331s; connection_time: 139.824ms}
# [DE] SpeedTestResult { speed: 7.95 MB/s; elapsed: 1.421512862s; connection_time: 77.633338ms}
# [BE] SpeedTestResult { speed: 5.79 MB/s; elapsed: 1.371025574s; connection_time: 128.863947ms}
# [IT] SpeedTestResult { speed: 1.13 MB/s; elapsed: 841.363582ms; connection_time: 636.626088ms}
# [BY] SpeedTestResult { speed: 8.38 MB/s; elapsed: 1.458407988s; connection_time: 42.413635ms}
# === RESULTS (top re-tested) ====
#   1. [BY] SpeedTestResult { speed: 8.38 MB/s; elapsed: 1.458407988s; connection_time: 42.413635ms} -> http://ftp.byfly.by/pub/archlinux/
#   2. [DE] SpeedTestResult { speed: 7.95 MB/s; elapsed: 1.421512862s; connection_time: 77.633338ms} -> http://mirror.f4st.host/archlinux/
#   3. [SE] SpeedTestResult { speed: 6.78 MB/s; elapsed: 1.359863331s; connection_time: 139.824ms} -> https://mirror.osbeck.com/archlinux/
#   4. [BE] SpeedTestResult { speed: 5.79 MB/s; elapsed: 1.371025574s; connection_time: 128.863947ms} -> http://mirror.tiguinet.net/arch/
#   5. [IT] SpeedTestResult { speed: 1.13 MB/s; elapsed: 841.363582ms; connection_time: 636.626088ms} -> https://archmirror.it/
Server = http://ftp.byfly.by/pub/archlinux/$repo/os/$arch
Server = http://mirror.f4st.host/archlinux/$repo/os/$arch
Server = https://mirror.osbeck.com/archlinux/$repo/os/$arch
Server = http://mirror.tiguinet.net/arch/$repo/os/$arch
Server = https://archmirror.it/$repo/os/$arch
Server = http://mirror.ihost.md/archlinux/$repo/os/$arch
# ...
```
Full output - https://gist.github.com/westandskif/b6abdcb00e8471a1bcd7eb93650f9fc7. 

The output format is acceptable to be put in `/etc/pacman.d/mirrorlist`, see an example of how it may be done below.

The tool uses the following info:

- continents to naively assume countries of the same continent are directly linked
- number of internet exchanges per country and distances to weight country connections; thanks to [github.com/telegeography/www.internetexchangemap.com](https://github.com/telegeography/www.internetexchangemap.com)
- submarine cable connections, thanks to [github.com/telegeography/www.submarinecablemap.com](https://github.com/telegeography/www.submarinecablemap.com)

## Installation

Available on:

* [AUR](https://aur.archlinux.org/packages/rate-arch-mirrors-bin/):  `yay -S rate-arch-mirrors-bin` - pre-built binary with statically linked `musl`
* [AUR](https://aur.archlinux.org/packages/rate-arch-mirrors/):  `yay -S rate-arch-mirrors` - build binary from sources, linking `glibc` dynamically

or build manually:

```
cargo build --release --locked
```

## Algorithm

1. take the next country to explore (or `--entry-country` option, `US` by default)
2. find neighbor countries `--country-neighbors-per-country`, using multiple strategies, at the moment 2:

- major internet hubs first ( _first two jumps only_ )
- closest by distance first ( _every jump_ )

3. take Arch mirrors of countries, selected at step 2, test speed and take 2 mirrors: 1 fastest and 1 with shortest connection time
4. take countries of mirrors from step 3 and go to step 1
5. after N jumps are done, take top M mirrors by speed, test them with no concurrency, sort by speed and prepend to the resulting list

## Example of everyday use

Simple one:
```
export TMPFILE="$(mktemp)"; \
    sudo true; \
	rate-arch-mirrors --max-delay=21600 --save=$TMPFILE \
	  && sudo mv /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist-backup \
	  && sudo mv $TMPFILE /etc/pacman.d/mirrorlist
```

Extended one:
```
alias ua-drop-caches='sudo paccache -rk3; yay -Sc --aur --noconfirm'
alias ua-update-all='export TMPFILE="$(mktemp)"; \
	sudo true; \
	rate-arch-mirrors --max-delay=21600 --save=$TMPFILE \
	  && sudo mv /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist-backup \
	  && sudo mv $TMPFILE /etc/pacman.d/mirrorlist \
	  && ua-drop-caches \
	  && yay -Syyu --noconfirm'
```

Few notes:
- the tool won't work with root permissions because it doesn't need them
- `ua-` prefix means "user alias"
- `paccache` from `pacman-contrib` package
- `yay` is an AUR helper
- `sudo true` forces password prompt in the very beginning

To persist aliases, add them to `~/.zshrc` or `~/.bashrc` (based on the shell you use)

Once done, just launch a new terminal and run:
```
ua-update-all
```

## License

The tool is made available under the following
[Creative Commons License: Attribution-NonCommercial-ShareAlike 3.0 Unported (CC BY-NC-SA 3.0)](https://creativecommons.org/licenses/by-nc-sa/3.0/).
