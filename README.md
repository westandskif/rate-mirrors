# Rate Mirrors

![Tag Badge](https://img.shields.io/github/tag/westandskif/rate-mirrors.svg)
[![License: CC BY-NC-SA 3.0](https://img.shields.io/badge/License-CC%20BY--NC--SA%203.0-lightgrey.svg)](https://creativecommons.org/licenses/by-nc-sa/3.0/)

A fast mirror ranking tool that finds the best mirrors for your Linux distribution. It uses submarine cable and internet exchange data to intelligently hop between countries and discover fast mirrors in ~30 seconds.

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Supported Distributions](#supported-distributions)
- [Usage](#usage)
- [Common Options](#common-options)
- [Algorithm](#algorithm)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)
- [Exit Codes](#exit-codes)
- [License](#license)

## Quick Start

```bash
# Arch Linux
rate-mirrors arch | sudo tee /etc/pacman.d/mirrorlist

# Manjaro
rate-mirrors manjaro | sudo tee /etc/pacman.d/mirrorlist

# With backup
export TMPFILE="$(mktemp)"; \
    rate-mirrors --save=$TMPFILE arch --max-delay=21600 \
    && sudo mv /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist-backup \
    && sudo mv $TMPFILE /etc/pacman.d/mirrorlist

# See all options
rate-mirrors --help
```

## Installation

| Platform | Command | Notes |
|----------|---------|-------|
| Arch Linux (binary) | `yay -S rate-mirrors-bin` | Pre-built, musl-linked |
| Arch Linux (source) | `yay -S rate-mirrors` | Built from source, glibc |
| OpenBSD | `pkg_add rate-mirrors` | From ports |
| GitHub Releases | [Download](https://github.com/westandskif/rate-mirrors/releases) | Pre-built binaries |
| From source | `cargo build --release --locked` | Requires Rust toolchain |

## Supported Distributions

### Arch-based

| Command | Distribution | Notes |
|---------|-------------|-------|
| `rate-mirrors arch` | Arch Linux | Skips outdated/syncing mirrors |
| `rate-mirrors archarm` | Arch Linux ARM | |
| `rate-mirrors arcolinux` | ArcoLinux | |
| `rate-mirrors artix` | Artix Linux | |
| `rate-mirrors blackarch` | BlackArch Linux | |
| `rate-mirrors cachyos` | CachyOS | |
| `rate-mirrors chaotic-aur` | Chaotic-AUR | Arch Linux repository |
| `rate-mirrors archlinuxcn` | Arch Linux CN | Chinese community repo |
| `rate-mirrors endeavouros` | EndeavourOS | Skips outdated mirrors |
| `rate-mirrors manjaro` | Manjaro | Skips outdated mirrors |
| `rate-mirrors rebornos` | RebornOS | |

### Other

| Command | Distribution |
|---------|-------------|
| `rate-mirrors openbsd` | OpenBSD |
| `rate-mirrors stdin` | Custom mirrors (see [Advanced Usage](#advanced-usage-stdin)) |

## Usage

```
rate-mirrors [OPTIONS] <SUBCOMMAND> [SUBCOMMAND-OPTIONS]
```

- Run `rate-mirrors --help` to see base options
- Run `rate-mirrors <subcommand> --help` to see subcommand-specific options
- The tool doesn't need root; use `--allow-root` if you must run as root

## Common Options

| Option | Description | Default |
|--------|-------------|---------|
| `--save=FILE` | Save output to file instead of stdout | - |
| `--concurrency=N` | Number of simultaneous speed tests | 16 |
| `--max-jumps=N` | Maximum country hops | 7 |
| `--entry-country=CC` | Starting country code | US |
| `--exclude-countries=CC,CC` | Exclude countries (comma-separated codes) | - |
| `--protocol=PROTO` | Test only specified protocol (http/https) | - |
| `--max-mirrors-to-output=N` | Maximum mirrors to output | - |
| `--disable-comments` | Disable printing comments | false |
| `--allow-root` | Allow running as root | false |

### Subcommand Options (arch example)

| Option | Description | Default |
|--------|-------------|---------|
| `--completion=N` | Minimum sync completion (0.0-1.0) | 1.0 |
| `--sort-mirrors-by=MODE` | Sort by: score_asc, score_desc, delay_asc, delay_desc | score_asc |

## Algorithm

The tool uses:
- Submarine cable connections data
- Internet exchange locations and counts per country
- Continental groupings for geographic proximity

### How it works (arch example):

1. Fetch mirrors from [Arch Linux Mirror Status](https://archlinux.org/mirrors/status/json/)
2. Filter out incomplete or outdated mirrors
3. Sort by mirror score
4. Starting from entry country, find neighbor countries using:
   - Major internet hubs (first two jumps)
   - Geographic proximity (every jump)
5. Test mirrors from each country, track fastest and lowest-latency
6. Jump to countries of best mirrors, repeat
7. After max jumps, re-test top mirrors sequentially and output final ranking

**Data attribution:** Submarine cable and IX data from [TeleGeography](https://www2.telegeography.com).

## Examples

### Everyday use on Arch Linux

```bash
alias ua-drop-caches='sudo paccache -rk3; yay -Sc --aur --noconfirm'
alias ua-update-all='export TMPFILE="$(mktemp)"; \
    sudo true; \
    rate-mirrors --save=$TMPFILE arch --max-delay=21600 \
      && sudo mv /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist-backup \
      && sudo mv $TMPFILE /etc/pacman.d/mirrorlist \
      && ua-drop-caches \
      && yay -Syyu --noconfirm'
```

Notes:
- `sudo true` prompts for password at the start
- `paccache` is from `pacman-contrib`
- `yay` is an AUR helper

Add to `~/.bashrc` or `~/.zshrc`, then run `ua-update-all`.

### Output example

```
# STARTED AT: 2025-08-31 14:04:21.217766 +03:00
# ARGS: rate-mirrors arch
# FETCHED MIRRORS: 1147
# MIRRORS LEFT AFTER FILTERING: 730
# JUMP #1
# EXPLORING US
#     + NEIGHBOR UK (by HubsFirst)
#     + NEIGHBOR DE (by DistanceFirst)
# [DE] SpeedTestResult { speed: 29.0 MB/s; elapsed: 1.00s; connection_time: 79ms }
# ...
# RE-TESTING TOP MIRRORS
# [SE] SpeedTestResult { speed: 64.6 MB/s; elapsed: 745ms; connection_time: 122ms }
# [BY] SpeedTestResult { speed: 61.2 MB/s; elapsed: 786ms; connection_time: 16ms }
# ==== RESULTS (top re-tested) ====
#   1. [SE] 64.6 MB/s -> https://mirror.osbeck.com/archlinux/
#   2. [BY] 61.2 MB/s -> http://mirror.datacenter.by/pub/archlinux/
#   3. [LT] 54.1 MB/s -> http://mirrors.atviras.lt/archlinux/
# FINISHED AT: 2025-08-31 14:04:40.296066 +03:00
Server = https://mirror.osbeck.com/archlinux/$repo/os/$arch
Server = http://mirror.datacenter.by/pub/archlinux/$repo/os/$arch
Server = http://mirrors.atviras.lt/archlinux/$repo/os/$arch
```

### Advanced Usage: stdin

For custom mirror lists or unsupported distributions:

```bash
# Input format (tab-separated):
# URL
# COUNTRY<tab>URL
# URL<tab>COUNTRY

cat mirrors.txt | rate-mirrors --concurrency=40 stdin \
    --path-to-test="extra/os/x86_64/extra.files" \
    --path-to-return='$repo/os/$arch' \
    --comment-prefix="# " \
    --output-prefix="Server = "
```

Example `mirrors.txt`:
```
https://mirror-a.example.org/repo/
US	https://mirror-b.example.org/repo/
https://mirror-c.example.org/repo/	DE
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (network failure, invalid arguments, etc.) |

## License

[Creative Commons Attribution-NonCommercial-ShareAlike 3.0 Unported (CC BY-NC-SA 3.0)](https://creativecommons.org/licenses/by-nc-sa/3.0/)

---

<sub>Previously known as "Rate Arch Mirrors" (changed in v0.4.0). [View old README](https://github.com/westandskif/rate-mirrors/blob/98f6417ff30b5148ab80f742c8eb729b78ca20c1/README.md)</sub>
