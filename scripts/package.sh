#!/usr/bin/env bash
# Build a distributable `rune` binary for one target (S085).
#
#   scripts/package.sh
#   scripts/package.sh aarch64-apple-darwin
#   scripts/package.sh x86_64-unknown-linux-gnu
#
# Does not replace a running binary. Output lands in dist/.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

HOST="$(rustc -vV | awk '/^host:/{print $2}')"
TARGET="${1:-$HOST}"
OUT_DIR="${RUNE_DIST:-$ROOT/dist}"
mkdir -p "$OUT_DIR"

log() { printf '  ᚱ  %s\n' "$*"; }

log "packaging rune for $TARGET"

BIN_NAME="rune"
if [[ "$TARGET" == *windows* ]]; then
  BIN_NAME="rune.exe"
fi

if [[ "$TARGET" == "$HOST" ]]; then
  cargo build --release -p rune --locked 2>/dev/null \
    || cargo build --release -p rune
  SRC="$ROOT/target/release/$BIN_NAME"
else
  rustup target add "$TARGET" >/dev/null 2>&1 || true
  cargo build --release -p rune --target "$TARGET" --locked 2>/dev/null \
    || cargo build --release -p rune --target "$TARGET"
  SRC="$ROOT/target/$TARGET/release/$BIN_NAME"
fi

if [[ ! -x "$SRC" && ! -f "$SRC" ]]; then
  printf 'package: missing binary at %s\n' "$SRC" >&2
  exit 1
fi

DEST="$OUT_DIR/rune-$TARGET"
if [[ "$BIN_NAME" == *.exe ]]; then
  DEST="$DEST.exe"
fi

if [[ -x "$DEST" ]]; then
  running="$(ps -o args= -p $$ || true)"
  if [[ "$running" == *"$DEST"* ]]; then
    printf 'package: refusing to overwrite a running binary at %s\n' "$DEST" >&2
    exit 1
  fi
fi

cp "$SRC" "$DEST"
chmod +x "$DEST" 2>/dev/null || true

cat > "$OUT_DIR/rune-$TARGET.sha256" <<EOF
$(shasum -a 256 "$DEST" | awk '{print $1}')  $(basename "$DEST")
EOF

log "wrote $DEST"
ls -lh "$DEST"
