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
#   ./install.sh --all        # full CachyOS path: system install → smoke → pacman -Syu →
#                             # reinstall (in case package overwrote us) → smoke → rank mirrors
#   ./install.sh --all --noconfirm
#   ./install.sh --all --exclude-countries=RU,CN
#   ./install.sh --rank-only  # only re-rank Arch + CachyOS mirrors (needs installed binary)
#
# Env:
#   PREFIX, DESTDIR, CARGO, CARGO_FLAGS
#   EXCLUDE_COUNTRIES / RATE_MIRRORS_EXCLUDE_COUNTRIES  (default for --all: RU)
#   ENTRY_COUNTRY / RATE_MIRRORS_ENTRY_COUNTRY          (optional; wrapper may geoip)
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
DO_ALL=0
RANK_ONLY=0
NOCONFIRM=0
# Prefer explicit CLI; then EXCLUDE_COUNTRIES; then RATE_MIRRORS_*; empty until --all sets default
EXCLUDE_COUNTRIES="${EXCLUDE_COUNTRIES:-${RATE_MIRRORS_EXCLUDE_COUNTRIES:-}}"
ENTRY_COUNTRY="${ENTRY_COUNTRY:-${RATE_MIRRORS_ENTRY_COUNTRY:-}}"
EXCLUDE_SET_BY_CLI=0

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

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

sudo_run() {
    if [[ "$(id -u)" -eq 0 ]]; then
        "$@"
    else
        need_cmd sudo
        sudo "$@"
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage ;;
        --build-only) BUILD_ONLY=1 ;;
        --smoke) SMOKE=1 ;;
        --all) DO_ALL=1 ;;
        --rank-only) RANK_ONLY=1 ;;
        --noconfirm) NOCONFIRM=1 ;;
        --system)
            SYSTEM=1
            PREFIX=/usr
            ;;
        --user)
            USER_INSTALL=1
            PREFIX="${HOME}/.local"
            ;;
        --exclude-countries)
            [[ $# -ge 2 ]] || die "--exclude-countries requires a value (e.g. RU or RU,CN)"
            EXCLUDE_COUNTRIES="$2"
            EXCLUDE_SET_BY_CLI=1
            shift
            ;;
        --exclude-countries=*)
            EXCLUDE_COUNTRIES="${1#*=}"
            EXCLUDE_SET_BY_CLI=1
            ;;
        --entry-country)
            [[ $# -ge 2 ]] || die "--entry-country requires a 2-letter code"
            ENTRY_COUNTRY="$2"
            shift
            ;;
        --entry-country=*)
            ENTRY_COUNTRY="${1#*=}"
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

# --all: one-shot CachyOS setup
if [[ "$DO_ALL" -eq 1 ]]; then
    SYSTEM=1
    PREFIX=/usr
    SMOKE=1
    USER_INSTALL=0
    # Default exclude RU for the full path unless user overrode via env/CLI
    if [[ "$EXCLUDE_SET_BY_CLI" -eq 0 && -z "${RATE_MIRRORS_EXCLUDE_COUNTRIES:-}" && -z "${EXCLUDE_COUNTRIES:-}" ]]; then
        EXCLUDE_COUNTRIES=RU
    fi
    # If env was set but EXCLUDE_COUNTRIES still empty string intentionally, leave it
    if [[ -z "$EXCLUDE_COUNTRIES" && -n "${RATE_MIRRORS_EXCLUDE_COUNTRIES:-}" ]]; then
        EXCLUDE_COUNTRIES="$RATE_MIRRORS_EXCLUDE_COUNTRIES"
    fi
fi

if [[ "$SYSTEM" -eq 1 && "$USER_INSTALL" -eq 1 ]]; then
    die "use only one of --system or --user"
fi

if [[ "$RANK_ONLY" -eq 1 && "$DO_ALL" -eq 1 ]]; then
    die "use only one of --all or --rank-only"
fi

BIN_SRC="$ROOT/target/release/rate_mirrors"
BIN_NAME="rate-mirrors"
INSTALL_DIR="${DESTDIR}${PREFIX}/bin"
BIN_DST="${INSTALL_DIR}/${BIN_NAME}"

setup_priv() {
    install_as_root=1
    if [[ "$USER_INSTALL" -eq 1 ]]; then
        install_as_root=0
    elif [[ -w "$INSTALL_DIR" ]] || { [[ ! -e "$INSTALL_DIR" ]] && [[ -w "$(dirname "$INSTALL_DIR")" ]]; }; then
        if [[ "$(id -u)" -ne 0 ]] && [[ "$PREFIX" == /usr || "$PREFIX" == /usr/local ]]; then
            install_as_root=1
        else
            install_as_root=0
        fi
    fi

    if [[ "$install_as_root" -eq 1 ]]; then
        run_priv() {
            if [[ "$(id -u)" -eq 0 ]]; then
                "$@"
            else
                need_cmd sudo
                sudo "$@"
            fi
        }
    else
        run_priv() { "$@"; }
    fi
}

build_binary() {
    need_cmd "$CARGO"
    info "Building rate-mirrors (${CARGO_FLAGS})..."
    # shellcheck disable=SC2086
    "$CARGO" build ${CARGO_FLAGS}
    [[ -x "$BIN_SRC" ]] || die "build finished but binary missing: $BIN_SRC"
}

install_binary() {
    setup_priv
    info "Install directory: $INSTALL_DIR"
    run_priv mkdir -p "$INSTALL_DIR"

    if [[ -e "$BIN_DST" || -L "$BIN_DST" ]]; then
        local backup
        backup="${BIN_DST}.bak.$(date +%Y%m%d%H%M%S)"
        info "Backing up existing binary → $backup"
        run_priv cp -a "$BIN_DST" "$backup"
    fi

    info "Installing $BIN_SRC → $BIN_DST"
    run_priv install -m755 "$BIN_SRC" "$BIN_DST"

    local resolved sudo_path
    resolved="$(command -v "$BIN_NAME" 2>/dev/null || true)"
    if [[ -n "$resolved" ]]; then
        info "Shell resolves: $resolved"
        if [[ "$(readlink -f "$resolved" 2>/dev/null || echo "$resolved")" != "$(readlink -f "$BIN_DST" 2>/dev/null || echo "$BIN_DST")" ]]; then
            warn "PATH prefers a different rate-mirrors than the one just installed."
            warn "  installed: $BIN_DST"
            warn "  first in PATH: $resolved"
            if [[ "$PREFIX" != /usr ]]; then
                warn "For cachyos-rate-mirrors (runs as root), use: ./install.sh --system"
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
                warn "Fix: ./install.sh --system   or   ./install.sh --all"
            fi
        fi
    fi

    info "Installed: $BIN_DST"
    if [[ -x "$BIN_DST" ]]; then
        "$BIN_DST" --version 2>/dev/null || true
        "$BIN_DST" 2>&1 | head -1 || true
    fi
}

run_smoke() {
    local label="${1:-}"
    [[ -x "$BIN_DST" ]] || die "binary not installed: $BIN_DST"
    info "Smoke test${label:+ ($label)}: CachyOS country filter (exclude ${EXCLUDE_COUNTRIES:-RU})..."
    local exclude="${EXCLUDE_COUNTRIES:-RU}"
    local entry_args=()
    [[ -n "$ENTRY_COUNTRY" ]] && entry_args=(--entry-country="$ENTRY_COUNTRY")
    [[ ${#entry_args[@]} -eq 0 ]] && entry_args=(--entry-country=US)

    local out
    out="$("$BIN_DST" "${entry_args[@]}" --exclude-countries="$exclude" --protocol=https \
        --max-mirrors-to-output=1 cachyos 2>&1 || true)"
    if printf '%s\n' "$out" | grep -q 'COUNTRY FILTER:'; then
        printf '%s\n' "$out" | grep -E 'COUNTRY FILTER:|EXPLORING |VISITED |NEIGHBOR |BLANK ITERATION|VERSION:' || true
        info "Smoke OK — country metadata is being applied."
        return 0
    elif printf '%s\n' "$out" | grep -q 'BLANK ITERATION'; then
        printf '%s\n' "$out" | grep -E 'VERSION:|EXPLORING |VISITED |BLANK|UNLABELED' || true
        die "Smoke FAILED — still geography-blind (old binary or install path wrong)."
    else
        printf '%s\n' "$out" | head -30
        warn "Could not confirm COUNTRY FILTER (network/error?). Binary at $BIN_DST"
        return 0
    fi
}

run_pacman_syu() {
    need_cmd pacman
    info "Updating system packages (pacman -Syu)..."
    warn "This may reinstall distro rate-mirrors and overwrite /usr/bin/rate-mirrors — we reinstall our build after."
    local args=(-Syu)
    [[ "$NOCONFIRM" -eq 1 ]] && args+=(--noconfirm)
    sudo_run pacman "${args[@]}"
    info "pacman -Syu finished."
}

run_rank_mirrors() {
    local exclude="${EXCLUDE_COUNTRIES:-}"
    local export_env=()
    [[ -n "$exclude" ]] && export_env+=(RATE_MIRRORS_EXCLUDE_COUNTRIES="$exclude")
    [[ -n "$ENTRY_COUNTRY" ]] && export_env+=(RATE_MIRRORS_ENTRY_COUNTRY="$ENTRY_COUNTRY")

    if command -v cachyos-rate-mirrors >/dev/null 2>&1; then
        info "Ranking Arch + CachyOS mirrors via cachyos-rate-mirrors..."
        if [[ ${#export_env[@]} -gt 0 ]]; then
            info "Env: ${export_env[*]}"
            sudo_run env "${export_env[@]}" cachyos-rate-mirrors
        else
            sudo_run cachyos-rate-mirrors
        fi
        return 0
    fi

    # Fallback: write lists with the binary we installed
    local bin="$BIN_DST"
    [[ -x "$bin" ]] || bin="$(command -v rate-mirrors)" || die "rate-mirrors not found"
    info "cachyos-rate-mirrors not found; ranking with $bin directly..."

    local tmp arch_list cachy_list
    tmp="$(mktemp -d)"
    arch_list="$tmp/mirrorlist"
    cachy_list="$tmp/cachyos-mirrorlist"
    # shellcheck disable=SC2064
    trap "rm -rf '$tmp'" RETURN

    local common=(--protocol=https --allow-root --disable-comments-in-file)
    [[ -n "$exclude" ]] && common+=(--exclude-countries="$exclude")
    [[ -n "$ENTRY_COUNTRY" ]] && common+=(--entry-country="$ENTRY_COUNTRY")

    info "Ranking Arch → /etc/pacman.d/mirrorlist"
    sudo_run env RATE_MIRRORS_ALLOW_ROOT=true "$bin" --save="$arch_list" "${common[@]}" arch
    sudo_run install -m644 --backup=simple --suffix="-backup" "$arch_list" /etc/pacman.d/mirrorlist

    info "Ranking CachyOS → /etc/pacman.d/cachyos-mirrorlist"
    sudo_run env RATE_MIRRORS_ALLOW_ROOT=true "$bin" --save="$cachy_list" "${common[@]}" cachyos
    sudo_run install -m644 --backup=simple --suffix="-backup" "$cachy_list" /etc/pacman.d/cachyos-mirrorlist

    if [[ -f /etc/pacman.d/cachyos-v3-mirrorlist ]]; then
        sudo_run cp -a /etc/pacman.d/cachyos-mirrorlist /etc/pacman.d/cachyos-v3-mirrorlist
        sudo_run sed -i 's|/$arch/|/$arch_v3/|g' /etc/pacman.d/cachyos-v3-mirrorlist
    fi
    if [[ -f /etc/pacman.d/cachyos-v4-mirrorlist ]]; then
        sudo_run cp -a /etc/pacman.d/cachyos-mirrorlist /etc/pacman.d/cachyos-v4-mirrorlist
        sudo_run sed -i 's|/$arch/|/$arch_v4/|g' /etc/pacman.d/cachyos-v4-mirrorlist
    fi
    info "Mirrorlists updated."
}

# --- main flows ---

if [[ "$RANK_ONLY" -eq 1 ]]; then
    # Prefer the path we would install to; fall back to PATH
    if [[ ! -x "$BIN_DST" ]]; then
        BIN_DST="$(command -v rate-mirrors 2>/dev/null || true)"
    fi
    [[ -n "${BIN_DST:-}" && -x "$BIN_DST" ]] || die "rate-mirrors not found; run ./install.sh --system first"
    run_rank_mirrors
    exit 0
fi

build_binary

if [[ "$BUILD_ONLY" -eq 1 ]]; then
    info "Build only: $BIN_SRC"
    exit 0
fi

install_binary

if [[ "$SMOKE" -eq 1 && "$DO_ALL" -eq 0 ]]; then
    run_smoke
fi

if [[ "$DO_ALL" -eq 1 ]]; then
    run_smoke "pre-update"

    run_pacman_syu

    info "Reinstalling our rate-mirrors after pacman (package may have overwritten it)..."
    # Rebuild only if binary disappeared; usually just re-copy
    [[ -x "$BIN_SRC" ]] || build_binary
    install_binary
    run_smoke "post-update"

    run_rank_mirrors

    cat <<EOF

All done.

  Binary:     $BIN_DST
  Excluded:   ${EXCLUDE_COUNTRIES:-"(none)"}
  Ranked via: $(command -v cachyos-rate-mirrors 2>/dev/null || echo "$BIN_DST")

If pacman upgrades rate-mirrors again later:
  ./install.sh --system --smoke
  # or full path again:
  ./install.sh --all

EOF
    exit 0
fi

cat <<EOF

Done.

  Binary: $BIN_DST

One-shot CachyOS (install → smoke → pacman -Syu → reinstall → rank):
  ./install.sh --all
  ./install.sh --all --noconfirm
  ./install.sh --all --exclude-countries=RU,CN

System install only (recommended for cachyos-rate-mirrors):
  ./install.sh --system --smoke

Rank only (after install):
  ./install.sh --rank-only --exclude-countries=RU

Note: pacman -Syu may overwrite /usr/bin/rate-mirrors until the fix is upstream.
      --all reinstalls our build after the update for that reason.

EOF
