# Rate Mirrors

**former Rate Arch Mirrors (changed in v0.4.0)** - [previous README](https://github.com/westandskif/rate-mirrors/blob/98f6417ff30b5148ab80f742c8eb729b78ca20c1/README.md)

![Tag Badge](https://img.shields.io/github/tag/westandskif/rate-mirrors.svg)

This is a tool, which tests mirror speed for:

- Arch Linux
- Manjaro
- RebornOS
- Artix Linux
- CachyOS
- any http/https mirrors via stdin.

It uses info about submarine cables and internet exchanges to jump between
countries and find fast mirrors. And it's fast enough to run it before each
system update (**~30 seconds with default options**).

## Installation

- [ArchLinux AUR](https://aur.archlinux.org/packages/rate-mirrors-bin/): `yay -S rate-mirrors-bin` - pre-built binary with statically linked `musl`
- [ArchLinux AUR](https://aur.archlinux.org/packages/rate-mirrors/): `yay -S rate-mirrors` - build binary from sources, linking `glibc` dynamically
- [Github releases](https://github.com/westandskif/rate-mirrors/releases): pre-built binary with statically linked musl

or build manually:

```
cargo build --release --locked
```

## Usage

- format is: `rate-mirrors {base options} sub-command {sub-command options}`
- run `rate-mirrors help` to see base options, which go before sub-command
- run `rate-mirrors arch --help` to see e.g. `arch` sub-command options, which go after sub-command
- it doesn't need root, but if you wish just pass `--allow-root` option.

### There are 5 sub-commands:

1. `rate-mirrors arch` -- fetches Arch Linux mirrors, skips outdated/syncing
   ones and tests them.

   To backup `/etc/pacman.d/mirrorlist` file and update it with the rated mirrors run the command below:

   ```
   export TMPFILE="$(mktemp)"; \
       sudo true; \
       rate-mirrors --save=$TMPFILE arch --max-delay=21600 \
         && sudo mv /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist-backup \
         && sudo mv $TMPFILE /etc/pacman.d/mirrorlist
   ```

   Or just put the output to `/etc/pacman.d/mirrorlist` yourself.

2. `rate-mirrors manjaro` -- fetches Manjaro mirrors, skips outdated ones and tests them

   See _rate-mirrors arch_ example above for more info.

3. `rate-mirrors rebornos` -- fetches RebornOS mirrors and tests them

   See _rate-mirrors arch_ example above for more info.

4. `rate-mirrors artix` -- fetches Artix Linux mirrors and tests them

   See _rate-mirrors arch_ example above for more info.

5. `rate-mirrors cachyos` -- fetches CachyOS mirrors and tests them

   See _rate-mirrors arch_ example above for more info.

6. `rate-mirrors stdin` -- takes mirrors from stdin

   Each string should comply with one of two supported formats:

    - tab-separated url and country (either name or country code)
    - tab-separated country and url -- just in case :)
    - url

   e.g. we have a file with mirrors and we'd like to test it & format output for
Arch:

   ```
   cat mirrors_by_country.txt | \
       rate-mirrors --concurrency=40 stdin \
          --path-to-test="community/os/x86_64/community.files" \
          --path-to-return='$repo/os/$arch' --comment-prefix="# "
   ```


## Algorithm

The tool uses the following info:

- continents to naively assume countries of the same continent are directly linked
- number of internet exchanges per country and distances to weight country connections; thanks to [github.com/telegeography/www.internetexchangemap.com](https://github.com/telegeography/www.internetexchangemap.com)
- submarine cable connections, thanks to [github.com/telegeography/www.submarinecablemap.com](https://github.com/telegeography/www.submarinecablemap.com)

### e.g. steps for arch:

1. fetch mirrors from [Arch Linux - Mirror status](https://archlinux.org/mirrors/status/) as [json](https://archlinux.org/mirrors/status/json/)
2. skip ones, which haven’t completed syncing (`--completion=1` option)
3. skip ones with delays-since-the-last-sync longer than 1 day (`--max-delay` option)
4. sort mirrors by “Arch Linux - Mirror Status” [score](https://archlinux.org/mirrors/status/) - the lower the better (`--sort-mirrors-by=score_asc` option)
5. take the next country to explore (or `--entry-country` option, `US` by default -- no need to change)
6. find neighbor countries `--country-neighbors-per-country=3`, using multiple strategies:

   - major internet hubs first ( _first two jumps only_ )
   - closest by distance first ( _every jump_ )

7. take `--country-test-mirrors-per-country=2` mirrors per country, selected at step **6**, test speed and find 2 mirrors: 1 fastest and 1 with shortest connection time
8. take countries of mirrors from step **7** and go to step **5**
9. after ``--max-jumps=7`` jumps are done, take top M mirrors by speed (`--top-mirrors-number-to-retest=5`), test them with no concurrency, sort by speed and prepend to the resulting list


## Example of everyday use on Arch Linux:

```
alias ua-drop-caches='sudo paccache -rk3; yay -Sc --aur --noconfirm'
alias ua-update-all='export TMPFILE="$(mktemp)"; \
    sudo true; \
    rate-mirrors --save=$TMPFILE arch --max-delay=21600 \
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

### Output example:

Here is an example of running the arch mode from Belarus (_output truncated_):

```
# STARTED AT: 2021-06-23 21:44:49.694758291 +03:00
# ARGS: rate-mirrors arch
# FETCHED MIRRORS: 845
# MIRRORS LEFT AFTER FILTERING: 471
# JUMP #1
# EXPLORING US
# VISITED US
#     + NEIGHBOR ID (by HubsFirst)
#     + NEIGHBOR UK (by HubsFirst)
#     + NEIGHBOR CN (by HubsFirst)
#     + NEIGHBOR DE (by DistanceFirst)
#     + NEIGHBOR CA (by DistanceFirst)
#     + NEIGHBOR FR (by DistanceFirst)
# [US] SpeedTestResult { speed: 561.60 KB/s; elapsed: 1.087839405s; connection_time: 381.733267ms}
# [US] SpeedTestResult { speed: 1.32 MB/s; elapsed: 1.157819036s; connection_time: 276.577298ms}
# [UK] SpeedTestResult { speed: 2.65 MB/s; elapsed: 1.224126693s; connection_time: 261.353312ms}
# [UK] SpeedTestResult { speed: 5.48 MB/s; elapsed: 1.327602061s; connection_time: 94.138156ms}
# [ID] SpeedTestResult { speed: 179.68 KB/s; elapsed: 745.86963ms; connection_time: 670.100893ms}
# [DE] SpeedTestResult { speed: 1.98 MB/s; elapsed: 1.392069748s; connection_time: 106.878831ms}
# [DE] SpeedTestResult { speed: 1.76 MB/s; elapsed: 1.26812617s; connection_time: 223.358858ms}
# [CA] SpeedTestResult { speed: 1.04 MB/s; elapsed: 962.017229ms; connection_time: 517.499369ms}
# [CA] SpeedTestResult { speed: 1.67 MB/s; elapsed: 1.20327262s; connection_time: 296.884889ms}
# [FR] SpeedTestResult { speed: 2.50 MB/s; elapsed: 1.154907178s; connection_time: 341.534506ms}
# [FR] SpeedTestResult { speed: 2.25 MB/s; elapsed: 1.374341411s; connection_time: 118.696039ms}
#     TOP NEIGHBOR - CONNECTION TIME: UK - 94.138156ms
#     TOP NEIGHBOR - SPEED: UK - 5.48 MB/s
#
# JUMP #2
# EXPLORING UK
#     + NEIGHBOR NL (by HubsFirst)
#     + NEIGHBOR NO (by HubsFirst)
#     + NEIGHBOR AU (by HubsFirst)
#     + NEIGHBOR CZ (by DistanceFirst)
#     + NEIGHBOR CH (by DistanceFirst)
#     + NEIGHBOR SE (by DistanceFirst)
# ...
# JUMP #7
# ...

# RE-TESTING TOP MIRRORS
# [EE] SpeedTestResult { speed: 4.92 MB/s; elapsed: 1.320800025s; connection_time: 178.606272ms}
# [UK] SpeedTestResult { speed: 6.95 MB/s; elapsed: 791.741365ms; connection_time: 709.525808ms}
# [FI] SpeedTestResult { speed: 8.94 MB/s; elapsed: 1.376742398s; connection_time: 123.530685ms}
# [TR] SpeedTestResult { speed: 5.57 MB/s; elapsed: 1.243114863s; connection_time: 257.162963ms}
# [EE] SpeedTestResult { speed: 9.68 MB/s; elapsed: 1.41574022s; connection_time: 84.436077ms}
# ==== RESULTS (top re-tested) ====
#   1. [EE] SpeedTestResult { speed: 9.68 MB/s; elapsed: 1.41574022s; connection_time: 84.436077ms} -> http://mirror.cspacehostings.com/archlinux/
#   2. [FI] SpeedTestResult { speed: 8.94 MB/s; elapsed: 1.376742398s; connection_time: 123.530685ms} -> http://arch.mirror.far.fi/
#   3. [UK] SpeedTestResult { speed: 6.95 MB/s; elapsed: 791.741365ms; connection_time: 709.525808ms} -> http://archlinux.uk.mirror.allworldit.com/archlinux/
#   4. [TR] SpeedTestResult { speed: 5.57 MB/s; elapsed: 1.243114863s; connection_time: 257.162963ms} -> http://mirror.veriteknik.net.tr/archlinux/
#   5. [EE] SpeedTestResult { speed: 4.92 MB/s; elapsed: 1.320800025s; connection_time: 178.606272ms} -> https://mirror.cspacehostings.com/archlinux/
#   6. [BY] SpeedTestResult { speed: 3.66 MB/s; elapsed: 1.455269308s; connection_time: 44.256906ms} -> http://ftp.byfly.by/pub/archlinux/
#   7. [SE] SpeedTestResult { speed: 3.40 MB/s; elapsed: 1.270306507s; connection_time: 230.675741ms} -> https://mirror.osbeck.com/archlinux/
#   8. [BY] SpeedTestResult { speed: 3.40 MB/s; elapsed: 1.467606681s; connection_time: 32.547398ms} -> http://mirror.datacenter.by/pub/archlinux/
# ...
# FINISHED AT: 2021-06-23 21:45:15.555642390 +03:00
Server = http://mirror.cspacehostings.com/archlinux/$repo/os/$arch
Server = http://arch.mirror.far.fi/$repo/os/$arch
Server = http://archlinux.uk.mirror.allworldit.com/archlinux/$repo/os/$arch
Server = http://mirror.veriteknik.net.tr/archlinux/$repo/os/$arch
Server = https://mirror.cspacehostings.com/archlinux/$repo/os/$arch
Server = http://ftp.byfly.by/pub/archlinux/$repo/os/$arch
Server = https://mirror.osbeck.com/archlinux/$repo/os/$arch
Server = http://mirror.datacenter.by/pub/archlinux/$repo/os/$arch
```

## License

The tool is made available under the following
[Creative Commons License: Attribution-NonCommercial-ShareAlike 3.0 Unported (CC BY-NC-SA 3.0)](https://creativecommons.org/licenses/by-nc-sa/3.0/).
