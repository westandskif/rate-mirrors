# Rate Mirrors

**former Rate Arch Mirrors (changed in v0.4.0)** - [previous README](https://github.com/westandskif/rate-mirrors/blob/98f6417ff30b5148ab80f742c8eb729b78ca20c1/README.md)

![Tag Badge](https://img.shields.io/github/tag/westandskif/rate-mirrors.svg)

This is a tool, which tests mirror speed for:

- Arch Linux
    * including Chaotic-AUR
    * including Arch Linux CN
- Arch Linux ARM
- ArcoLinux
- Artix Linux
- BlackArch Linux
- CachyOS
- EndeavourOS
- Manjaro
- OpenBSD
- RebornOS
- any http/https mirrors via stdin.



It uses info about submarine cables and internet exchanges (**kudos to [TeleGeography](https://www2.telegeography.com) for data**) to jump between
countries and find fast mirrors. And it's fast enough to run it before each
system update (_~30 seconds with default options_).

## Installation

- [ArchLinux AUR](https://aur.archlinux.org/packages/rate-mirrors-bin/): `yay -S rate-mirrors-bin` - pre-built binary with statically linked `musl`
- [ArchLinux AUR](https://aur.archlinux.org/packages/rate-mirrors/): `yay -S rate-mirrors` - build binary from sources, linking `glibc` dynamically
- [Github releases](https://github.com/westandskif/rate-mirrors/releases): pre-built binary with statically linked musl
- [OpenBSD ports](https://github.com/openbsd/ports/tree/master/net/rate-mirrors): `pkg_add rate-mirrors`

or build manually:

```
cargo build --release --locked
```

## Usage

- format is: `rate-mirrors {base options} subcommand {subcommand options}`
- run `rate-mirrors help` to see base options, which go before subcommand
- it doesn't need root, but if you wish just pass `--allow-root` option.

### Here are supported subcommands:

Each subcommand has its own options, so run `rate-mirrors arch --help` to see
`arch` specific options, which should go after arch sub-command.


1. `rate-mirrors arch` — fetches Arch Linux mirrors, skips outdated/syncing
   ones and tests them.

   To backup `/etc/pacman.d/mirrorlist` file and update it with the rated mirrors run the command below:

   ```
   export TMPFILE="$(mktemp)"; \
       sudo true; \
       rate-mirrors --save=$TMPFILE arch --max-delay=43200 \
         && sudo mv /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist-backup \
         && sudo mv $TMPFILE /etc/pacman.d/mirrorlist
   ```

   Or if you don't need a backup: `rate-mirrors arch | sudo tee /etc/pacman.d/mirrorlist`.

1. `rate-mirrors chaotic-aur` - fetches Arch Linux Chaotic-AUR mirrors and tests them

1. `rate-mirrors archlinuxcn` - fetches Arch Linux CN mirrors and tests them

1. `rate-mirrors archarm` — fetches Arch Linux ARM mirrors and tests them

1. `rate-mirrors arcolinux` — fetches RebornOS mirrors and tests them

1. `rate-mirrors artix` — fetches Artix Linux mirrors and tests them

1. `rate-mirrors blackarch` - fetches BlackArch mirrors and tests them

1. `rate-mirrors cachyos` — fetches CachyOS mirrors and tests them

1. `rate-mirrors endeavouros` — fetches/reads EndeavourOS mirrors, skips outdated ones and tests them

1. `rate-mirrors manjaro` — fetches Manjaro mirrors, skips outdated ones and tests them

1. `rate-mirrors openbsd` — fetches OpenBSD mirrors and tests them

1. `rate-mirrors rebornos` — fetches RebornOS mirrors and tests them

1. `rate-mirrors stdin` — takes mirrors from stdin

   Each string should comply with one of two supported formats:

    - tab-separated url and country (either name or country code)
    - tab-separated country and url — just in case :)
    - url

   Urls should be what `--path-to-test` and `--path-to-return` are joined to.

   e.g. we have a file with mirrors (countries are required for country-hopping):
   ```
   https://mirror-a.mirrors.org/best-linux-distro/
   US\thttps://mirror-b.mirrors.org/best-linux-distro/
   https://mirror-c.mirrors.org/best-linux-distro/\tDE
   https://mirror-d.mirrors.org/best-linux-distro/\tAustria
   ```
   and we'd like to test it & format output for Arch:

   ```
   cat mirrors_by_country.txt | \
       rate-mirrors --concurrency=40 stdin \
          --path-to-test="extra/os/x86_64/extra.files" \
          --path-to-return='$repo/os/$arch' \
          --comment-prefix="# " \
          --output-prefix="Server = "
   ```


## Algorithm

The tool uses the following info:

- submarine cable connections
- number of internet exchanges per country and distances to weight country connections
- continents to naively assume countries of the same continent are directly linked

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
# STARTED AT: 2025-08-31 14:04:21.217766 +03:00
# ARGS: rate-mirrors arch
# FETCHED MIRRORS: 1147
# MIRRORS LEFT AFTER FILTERING: 730
# JUMP #1
# EXPLORING US
# VISITED US
#     + NEIGHBOR UK (by HubsFirst)
#     + NEIGHBOR ID (by HubsFirst)
#     + NEIGHBOR FR (by HubsFirst)
#     + NEIGHBOR CA (by DistanceFirst)
#     + NEIGHBOR DE (by DistanceFirst)
#     + NEIGHBOR NL (by DistanceFirst)
# TOO FEW BYTES LOADED http://ams.nl.mirrors.bjg.at/arch/
# [FR] FAILED TO CONNECT TO https://mirror.trap.moe/archlinux/extra/os/x86_64/extra.files
# [DE] SpeedTestResult { speed: 29.0 MB/s; elapsed: 1.00s; connection_time: 79ms }
# [DE] SpeedTestResult { speed: 8.8 MB/s; elapsed: 999ms; connection_time: 114ms }
# TOO FEW BYTES LOADED http://mirror.cyberbits.eu/archlinux/
# [UK] SpeedTestResult { speed: 5.7 MB/s; elapsed: 950ms; connection_time: 122ms }
# [NL] SpeedTestResult { speed: 3.7 MB/s; elapsed: 985ms; connection_time: 236ms }
# [UK] SpeedTestResult { speed: 6.3 MB/s; elapsed: 991ms; connection_time: 275ms }
# [US] SpeedTestResult { speed: 324.9 KB/s; elapsed: 998ms; connection_time: 288ms }
# [ID] SpeedTestResult { speed: 315.6 KB/s; elapsed: 996ms; connection_time: 465ms }
# [CA] SpeedTestResult { speed: 334.0 KB/s; elapsed: 998ms; connection_time: 670ms }
# [US] SpeedTestResult { speed: 448.7 KB/s; elapsed: 912ms; connection_time: 670ms }
# [CA] SpeedTestResult { speed: 953.6 KB/s; elapsed: 996ms; connection_time: 1.21s }
# [ID] SpeedTestResult { speed: 1.2 MB/s; elapsed: 946ms; connection_time: 1.27s }
#     TOP NEIGHBOR - CONNECTION TIME: DE - 79ms
#     TOP NEIGHBOR - SPEED: DE - 29.0 MB/s
#
# JUMP #2
# EXPLORING DE
#     + NEIGHBOR CN (by HubsFirst)
#     + NEIGHBOR JP (by HubsFirst)
#     + NEIGHBOR IN (by HubsFirst)
#     + NEIGHBOR CH (by DistanceFirst)
#     + NEIGHBOR CZ (by DistanceFirst)
#     + NEIGHBOR DK (by DistanceFirst)
# ...
# JUMP #7
# ...

# RE-TESTING TOP MIRRORS
# [SE] SpeedTestResult { speed: 64.6 MB/s; elapsed: 745ms; connection_time: 122ms }
# [BY] SpeedTestResult { speed: 61.2 MB/s; elapsed: 786ms; connection_time: 16ms }
# [CH] SpeedTestResult { speed: 38.0 MB/s; elapsed: 990ms; connection_time: 119ms }
# [DE] SpeedTestResult { speed: 22.0 MB/s; elapsed: 1.00s; connection_time: 73ms }
# [LT] SpeedTestResult { speed: 54.1 MB/s; elapsed: 889ms; connection_time: 70ms }
# ==== RESULTS (top re-tested) ====
#   1. [SE] SpeedTestResult { speed: 64.6 MB/s; elapsed: 745ms; connection_time: 122ms } -> https://mirror.osbeck.com/archlinux/
#   2. [BY] SpeedTestResult { speed: 61.2 MB/s; elapsed: 786ms; connection_time: 16ms } -> http://mirror.datacenter.by/pub/archlinux/
#   3. [LT] SpeedTestResult { speed: 54.1 MB/s; elapsed: 889ms; connection_time: 70ms } -> http://mirrors.atviras.lt/archlinux/
#   4. [CH] SpeedTestResult { speed: 38.0 MB/s; elapsed: 990ms; connection_time: 119ms } -> http://ch.mirrors.cicku.me/archlinux/
#   5. [DE] SpeedTestResult { speed: 22.0 MB/s; elapsed: 1.00s; connection_time: 73ms } -> http://mirror.moson.org/arch/
#   6. [FI] SpeedTestResult { speed: 23.4 MB/s; elapsed: 1.00s; connection_time: 92ms } -> http://cdnmirror.com/archlinux/
#   7. [RS] SpeedTestResult { speed: 20.0 MB/s; elapsed: 1.00s; connection_time: 101ms } -> http://mirror.pmf.kg.ac.rs/archlinux/
#   8. [BY] SpeedTestResult { speed: 19.2 MB/s; elapsed: 995ms; connection_time: 56ms } -> http://ftp.byfly.by/pub/archlinux/
# ...
# FINISHED AT: 2025-08-31 14:04:40.296066 +03:00
Server = https://mirror.osbeck.com/archlinux/$repo/os/$arch
Server = http://mirror.datacenter.by/pub/archlinux/$repo/os/$arch
Server = http://mirrors.atviras.lt/archlinux/$repo/os/$arch
Server = http://ch.mirrors.cicku.me/archlinux/$repo/os/$arch
Server = http://mirror.moson.org/arch/$repo/os/$arch
Server = http://cdnmirror.com/archlinux/$repo/os/$arch
Server = http://mirror.pmf.kg.ac.rs/archlinux/$repo/os/$arch
Server = http://ftp.byfly.by/pub/archlinux/$repo/os/$arch
```

## License

The tool is made available under the following
[Creative Commons License: Attribution-NonCommercial-ShareAlike 3.0 Unported (CC BY-NC-SA 3.0)](https://creativecommons.org/licenses/by-nc-sa/3.0/).
