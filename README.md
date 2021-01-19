# rate-arch-mirrors

This is a tool to filter out out-of-date Arch Linux mirrors and rate them (and also a rust-learning project):

```
rate-arch-mirrors --max-delay=43200
```

It outputs the results in a format acceptable for `/etc/pacman.d/mirrorlist`.

It uses the following info:

- continents to naively assume countries of the same continent are directly linked
- number of internet exchanges per country to weight country connections; thanks to [github.com/telegeography/www.internetexchangemap.com](https://github.com/telegeography/www.internetexchangemap.com)
- submarine cable connections, thanks to [github.com/telegeography/www.submarinecablemap.com](https://github.com/telegeography/www.submarinecablemap.com)

## Installation

Available on [AUR](https://aur.archlinux.org/packages/rate-arch-mirrors/): e.g. `yay -S rate-arch-mirrors`

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

## Example of everyday use
Few notes:
- `ua-` prefix means "user alias"
- `paccache` from `pacman-contrib` package
- `yay` is an AUR helper
- `sudo true` forces password prompt in the very beginning


```
alias ua-drop-caches='sudo paccache -rk3; yay -Sc --aur --noconfirm'
alias ua-update-all='export TMPFILE="$(mktemp)"; \
	sudo true; \
	rate-arch-mirrors --max-delay=21600 | tee -a $TMPFILE \
	  && sudo mv /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist-backup \
	  && sudo mv $TMPFILE /etc/pacman.d/mirrorlist \
	  && ua-drop-caches \
	  && yay -Syyu --noconfirm'
```
To persist aliases, add them to `~/.zshrc` or `~/.bashrc` (based on the shell you use)

## License

The tool is made available under the following
[Creative Commons License: Attribution-NonCommercial-ShareAlike 3.0 Unported (CC BY-NC-SA 3.0)](https://creativecommons.org/licenses/by-nc-sa/3.0/).
