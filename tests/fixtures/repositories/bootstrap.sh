#!/usr/bin/env bash
# Initialize Git repositories in each language fixture (S079).
# Idempotent. Does not generate large corpora.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"

init_repo() {
  local dir="$1"
  mkdir -p "$dir"
  local template
  template="$(mktemp -d "${TMPDIR:-/tmp}/rune-git-template.XXXXXX")"
  git -C "$dir" init -q --template="$template"
  rmdir "$template"
  git -C "$dir" add -A
  if git -C "$dir" diff --cached --quiet; then
    return 0
  fi
  git -C "$dir" \
    -c user.email="fixture@rune.test" \
    -c user.name="Rune Fixture" \
    commit -q -m "fixture: initial commit"
}

# malformed bytes (not valid UTF-8)
python3 - "$ROOT" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
path = root / "malformed" / "not_utf8.bin"
path.write_bytes(b"not utf8: \xff\xfe cafe\xe9\n")
print(f"wrote {path}")
PY

# Keep a latin-1 named sidecar description in ASCII README only.

REPOS=(
  rust_lib
  python_app
  typescript_app
  go_mod
  mixed_monorepo
  unicode_paths
  malformed
)

for name in "${REPOS[@]}"; do
  init_repo "$ROOT/$name"
  echo "initialized git repo: $name"
done
