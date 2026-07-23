#!/usr/bin/env bash
# install.sh — build and install rate-mirrors so wrappers (e.g. cachyos-rate-mirrors) pick up this tree.
#
# Usage:
#   ./install.sh              # release build → /usr/local/bin/rate-mirrors (sudo if needed)
#   ./install.sh --system     # same, but → /usr/bin/rate-mirrors (what pacman ships; recommended on CachyOS)
#   ./install.sh --prefix DIR # install to DIR/bin/rate-mirrors
#   ./install.sh --user       # install to ~/.local/bin (no sudo; may not be on sudo's PATH)
#   ./install.sh --build-only # cargo build --release only
#   ./install.sh --smoke      # after install, verify CachyOS country parsing is live
#
# Env:
#   PREFIX, DESTDIR, CARGO, CARGO_FLAGS
# END HELP

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

PREFIX="${PREFIX:-/usr/local}"
DESTDIR="${DESTDIR:-}"
CARGO="${CARGO:-cargo}"
CARGO_FLAGS="${CARGO_FLAGS:---release --locked}"
BUILD_ONLY=0
SMOKE=0
SYSTEM=0
USER_INSTALL=0

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

info() {
    printf '==> %s\n' "$*"
}

warn() {
    printf 'warning: %s\n' "$*" >&2
}

usage() {
    sed -n '2,/^# END HELP/p' "$0" | sed '/^# END HELP/d; s/^# \{0,1\}//'
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage ;;
        --build-only) BUILD_ONLY=1 ;;
        --smoke) SMOKE=1 ;;
        --system)
            SYSTEM=1
            PREFIX=/usr
            ;;
        --user)
            USER_INSTALL=1
            PREFIX="${HOME}/.local"
            ;;
        --prefix)
            [[ $# -ge 2 ]] || die "--prefix requires a directory"
            PREFIX="$2"
            shift
            ;;
        --prefix=*)
            PREFIX="${1#*=}"
            ;;
        *)
            die "unknown option: $1 (try --help)"
            ;;
    esac
    shift
done

if [[ "$SYSTEM" -eq 1 && "$USER_INSTALL" -eq 1 ]]; then
    die "use only one of --system or --user"
fi

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

need_cmd "$CARGO"

BIN_SRC="$ROOT/target/release/rate_mirrors"
BIN_NAME="rate-mirrors"
INSTALL_DIR="${DESTDIR}${PREFIX}/bin"
BIN_DST="${INSTALL_DIR}/${BIN_NAME}"

info "Building rate-mirrors (${CARGO_FLAGS})..."
# shellcheck disable=SC2086
"$CARGO" build ${CARGO_FLAGS}

[[ -x "$BIN_SRC" ]] || die "build finished but binary missing: $BIN_SRC"

if [[ "$BUILD_ONLY" -eq 1 ]]; then
    info "Build only: $BIN_SRC"
    exit 0
fi

run_install() {
    local mode="$1" # root | user
    if [[ "$mode" == root ]]; then
        if [[ "$(id -u)" -eq 0 ]]; then
            "$@"
        else
            need_cmd sudo
            sudo "$@"
        fi
    else
        "$@"
    fi
}

install_as_root=1
if [[ "$USER_INSTALL" -eq 1 ]]; then
    install_as_root=0
elif [[ -w "$INSTALL_DIR" ]] || { [[ ! -e "$INSTALL_DIR" ]] && [[ -w "$(dirname "$INSTALL_DIR")" ]]; }; then
    # writable without sudo (e.g. custom prefix)
    if [[ "$(id -u)" -ne 0 ]] && [[ "$PREFIX" == /usr || "$PREFIX" == /usr/local ]]; then
        install_as_root=1
    else
        install_as_root=0
    fi
fi

if [[ "$install_as_root" -eq 1 ]]; then
    run_priv() { run_install root "$@"; }
else
    run_priv() { "$@"; }
fi

info "Install directory: $INSTALL_DIR"
run_priv mkdir -p "$INSTALL_DIR"

if [[ -e "$BIN_DST" || -L "$BIN_DST" ]]; then
    backup="${BIN_DST}.bak.$(date +%Y%m%d%H%M%S)"
    info "Backing up existing binary → $backup"
    run_priv cp -a "$BIN_DST" "$backup"
fi

info "Installing $BIN_SRC → $BIN_DST"
run_priv install -m755 "$BIN_SRC" "$BIN_DST"

# Help CachyOS/pacman users: if they installed to /usr/local but sudo only
# sees /usr/bin, warn and offer the usual fix path.
resolved="$(command -v "$BIN_NAME" 2>/dev/null || true)"
if [[ -n "$resolved" ]]; then
    info "Shell resolves: $resolved"
    if [[ "$(readlink -f "$resolved" 2>/dev/null || echo "$resolved")" != "$(readlink -f "$BIN_DST" 2>/dev/null || echo "$BIN_DST")" ]]; then
        warn "PATH prefers a different rate-mirrors than the one just installed."
        warn "  installed: $BIN_DST"
        warn "  first in PATH: $resolved"
        if [[ "$PREFIX" != /usr ]]; then
            warn "For cachyos-rate-mirrors (runs as root), re-run: ./install.sh --system"
        fi
    fi
fi

if command -v sudo >/dev/null 2>&1 && [[ "$(id -u)" -ne 0 ]]; then
    sudo_path="$(sudo sh -c 'command -v rate-mirrors' 2>/dev/null || true)"
    if [[ -n "$sudo_path" ]]; then
        info "sudo resolves: $sudo_path"
        if [[ "$sudo_path" != "$BIN_DST" && "$SYSTEM" -eq 0 ]]; then
            warn "Root's PATH does not prefer $BIN_DST."
            warn "cachyos-rate-mirrors will keep using: $sudo_path"
            warn "Fix: ./install.sh --system"
        fi
    fi
fi

info "Installed version:"
if [[ -x "$BIN_DST" ]]; then
    "$BIN_DST" --version 2>/dev/null || true
    # clap style: first line of help / config
    "$BIN_DST" 2>&1 | head -1 || true
fi

if [[ "$SMOKE" -eq 1 ]]; then
    info "Smoke test: CachyOS country filter (exclude RU)..."
    # Short: only need to see COUNTRY FILTER / neighbor hop, not full ranking.
    # Use a high concurrency timeout; network required.
    out="$("$BIN_DST" --entry-country=US --exclude-countries=RU --protocol=https \
        --max-mirrors-to-output=1 cachyos 2>&1 || true)"
    if printf '%s\n' "$out" | grep -q 'COUNTRY FILTER:'; then
        printf '%s\n' "$out" | grep -E 'COUNTRY FILTER:|EXPLORING |VISITED |NEIGHBOR |BLANK ITERATION|VERSION:'
        info "Smoke OK — country metadata is being applied."
    elif printf '%s\n' "$out" | grep -q 'BLANK ITERATION'; then
        printf '%s\n' "$out" | grep -E 'VERSION:|EXPLORING |VISITED |BLANK|UNLABELED' || true
        die "Smoke FAILED — still geography-blind (old binary or install path wrong)."
    else
        printf '%s\n' "$out" | head -30
        warn "Could not confirm COUNTRY FILTER (network/error?). Binary installed at $BIN_DST"
    fi
fi

cat <<EOF

Done.

  Binary: $BIN_DST

CachyOS users (recommended):
  ./install.sh --system --smoke
  sudo RATE_MIRRORS_EXCLUDE_COUNTRIES=RU cachyos-rate-mirrors

Or rank CachyOS only with this build:
  $BIN_DST --entry-country=US --exclude-countries=RU --protocol=https --allow-root cachyos \\
    | sudo tee /etc/pacman.d/cachyos-mirrorlist

Note: pacman -Syu may overwrite /usr/bin/rate-mirrors if you used --system.
      Re-run ./install.sh --system after package updates until the fix is upstream.

EOF
