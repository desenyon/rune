#!/usr/bin/env bash
set -euo pipefail
# Build release binaries for the current host. Cross targets are documented in
# docs/compatibility/ and require the matching rustup targets to be installed.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="${HOME}/.cargo/bin:${PATH}"
cd "$ROOT"
cargo build --release -p rune
mkdir -p dist
cp -f target/release/rune dist/rune
if command -v rustup >/dev/null; then
  echo "host binary written to dist/rune"
fi
