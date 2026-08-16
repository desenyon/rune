#!/usr/bin/env bash
# Rune one-shot installer.
#
#   curl -fsSL https://raw.githubusercontent.com/desenyon/rune/main/scripts/install.sh | bash
#
# Installs a Rust toolchain if needed, builds `rune`, puts it on PATH,
# runs doctor + onboarding, and opens the TUI when a terminal is attached.

set -euo pipefail

REPO_URL="${RUNE_REPO_URL:-https://github.com/desenyon/rune.git}"
REPO_REF="${RUNE_REF:-main}"
PREFIX="${RUNE_PREFIX:-$HOME/.local}"
BIN_DIR="${RUNE_BIN_DIR:-$PREFIX/bin}"
SRC_DIR="${RUNE_SRC_DIR:-$HOME/.local/src/rune}"
WORKSPACE="${RUNE_WORKSPACE:-$(pwd)}"

log() { printf '\n  ᚱ  %s\n' "$*"; }
die() { printf 'rune install: %s\n' "$*" >&2; exit 1; }

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

ensure_path_file() {
  local file="$1"
  local line="$2"
  if [ -f "$file" ] && grep -Fqs "$line" "$file"; then
    return 0
  fi
  mkdir -p "$(dirname "$file")"
  touch "$file"
  printf '\n# Rune\n%s\n' "$line" >> "$file"
}

printf '\n'
printf '  ┌─────────────────────────────────────────┐\n'
printf '  │   ᚱ  RUNE                               │\n'
printf '  │   local-first context os                │\n'
printf '  └─────────────────────────────────────────┘\n'

if ! need_cmd git; then
  die "git is required"
fi

if ! need_cmd curl; then
  die "curl is required"
fi

if ! need_cmd rustc || ! need_cmd cargo; then
  log "installing Rust via rustup"
  if ! need_cmd rustup; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  fi
  # shellcheck disable=SC1091
  if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
  fi
  export PATH="$HOME/.cargo/bin:$PATH"
fi

need_cmd cargo || die "cargo is not available after rustup install"

log "fetching source → $SRC_DIR"
mkdir -p "$(dirname "$SRC_DIR")"
if [ -d "$SRC_DIR/.git" ]; then
  git -C "$SRC_DIR" fetch --depth 1 origin "$REPO_REF"
  git -C "$SRC_DIR" checkout -q FETCH_HEAD
else
  rm -rf "$SRC_DIR"
  git clone --depth 1 --branch "$REPO_REF" "$REPO_URL" "$SRC_DIR" \
    || git clone --depth 1 "$REPO_URL" "$SRC_DIR"
fi

log "building rune (release)"
cargo install --path "$SRC_DIR/apps/rune" --locked --root "$PREFIX" --force \
  || cargo install --path "$SRC_DIR/apps/rune" --root "$PREFIX" --force

mkdir -p "$BIN_DIR"
if [ -x "$PREFIX/bin/rune" ]; then
  :
elif [ -x "$HOME/.cargo/bin/rune" ]; then
  BIN_DIR="$HOME/.cargo/bin"
fi

need_cmd rune || [ -x "$BIN_DIR/rune" ] || die "rune binary was not installed"

PATH_LINE="export PATH=\"$BIN_DIR:\$PATH\""
ensure_path_file "$HOME/.zprofile" "$PATH_LINE"
ensure_path_file "$HOME/.zshrc" "$PATH_LINE"
ensure_path_file "$HOME/.bashrc" "$PATH_LINE"
ensure_path_file "$HOME/.profile" "$PATH_LINE"
export PATH="$BIN_DIR:$PATH"

RUNE_BIN="$BIN_DIR/rune"
if [ ! -x "$RUNE_BIN" ]; then
  RUNE_BIN="$(command -v rune)"
fi

log "doctor"
"$RUNE_BIN" doctor --path "$WORKSPACE" --format text || true

log "onboarding (no account required)"
"$RUNE_BIN" onboard --path "$WORKSPACE" --format text || true

cat <<EOF

  Rune is installed.

    binary   $RUNE_BIN
    source   $SRC_DIR
    PATH     $BIN_DIR  (added to ~/.zprofile, ~/.zshrc, ~/.bashrc, ~/.profile)

  Open a new shell, or:

    export PATH="$BIN_DIR:\$PATH"

    rune                 # TUI
    rune index           # index this repository
    rune search auth     # search the graph
    rune doctor          # diagnostics

EOF

if [ -t 1 ] && [ -t 0 ]; then
  log "starting TUI"
  exec "$RUNE_BIN" --path "$WORKSPACE"
fi
