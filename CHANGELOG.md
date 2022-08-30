# 0.11.1 (2022-08-30)

 - fixed `archarm` output

# 0.11.0 (2022-08-15)

- added Archlinux ARM support - [#30](https://github.com/westandskif/rate-mirrors/pull/30)
- added `--concurrency-for-unlabeled` option (default: 40) to speed up cases
  when country hopping yields too few results and we have to fall back to
  testing the remainder. The remainder may contain tens or hundreds of mirrors,
  so it makes sense to have separate default concurrency for unlabeled mirrors
  (ones without country). This should not worsen results too much because
  there's still `--top-mirrors-number-to-retest` (default: 5) with `1`
  concurrency.

# 0.10.0 (2022-05-25)

- added `--disable-comments` flag to suppress comment printing

# 0.9.3 (2022-05-17)

- fixed stdin `--path-to-return` bug, submitted by [arthurflor23](https://github.com/westandskif/rate-mirrors/issues/28)

# 0.9.2 (2022-03-20)

- code refactoring by [Anexen](https://github.com/Anexen)
- left aside debian/ubuntu mirrors support for now

# 0.9.1 (2022-03-14)

- fixed formatting of SpeedTestResult Display

# 0.9.0 (2022-01-06)

- added initial EndeavourOS support

# 0.8.0 (2022-01-05)

- added `--output-prefix` option to `rate-mirrors stdin` subcommand, e.g. to
  append `Server = ` to resulting lines

# 0.7.0 (2021-12-31)

- added CachyOS support - [#21](https://github.com/westandskif/rate-mirrors/pull/21)

# 0.6.3 (2021-12-29)

- commented out never read field - [#19](https://github.com/westandskif/rate-mirrors/issues/19)

# 0.6.2 (2021-12-29)

- fixed comments: store_asc -> score_asc - [#18](https://github.com/westandskif/rate-mirrors/issues/18)

# 0.6.1 (2021-12-21)

- fixed `--protocol` option when used in `--protocol http` form - [#17](https://github.com/westandskif/rate-mirrors/pull/17)

# 0.6.0 (2021-11-21)

- added ArtixLinux support
- added clear error messages

# 0.5.1 (2021-09-22)

- fixed dead code warning - [#9](https://github.com/westandskif/rate-mirrors/issues/9)
- brought package version up to date with the tag - [#10](https://github.com/westandskif/rate-mirrors/issues/10)

# 0.5.0 (2021-07-15)

- Added RebornOS support

# 0.4.0 (2021-06-24) -- BREAKING CHANGES

- **! BREAKING CHANGE !** now the tool is named "rate mirrors"
- **! BREAKING CHANGE !** now there are three subcommands for three different modes:

  - arch
  - manjaro
  - stdin

  See the readme for details.

  Configuration options are also split into common ones (which go before
  subcommand) and mode-specific ones (which go after)

- Added `--allow-root` option to run as root

# 0.3.0 (2021-05-20)

- Added `--save` option to write output to file
- Now it fails when run as root

# 0.2.1 (2021-02-22)

- Enabled _vendored_ feature for `openssl-sys` crate to allow for musl builds

# 0.2.0 (2021-02-21)

- Added `--sort-mirrors-by` option to control how mirrors are initially sorted
  within the country, `score_asc` by default. The full list of options is:
  _score_asc, score_desc, delay_asc, delay_desc, random_

- Added `--protocol` option to control acceptable protocols `https`, `http`. Both
  both are enabled by default. Rsync not supported.

# 0.1.0 (2021-01-17)

Initial.
