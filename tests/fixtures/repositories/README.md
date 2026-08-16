# Integration test repositories (S079)

Small, committed fixtures for indexer, search, and language detection tests. They are not performance corpora.

| Directory | Purpose |
| --- | --- |
| `rust_lib/` | Cargo library with functions and unit tests |
| `python_app/` | `pyproject.toml` module and pytest-style tests |
| `typescript_app/` | `package.json` and TypeScript entry |
| `go_mod/` | `go.mod` and `main.go` |
| `mixed_monorepo/` | Rust crate plus Python package |
| `unicode_paths/` | Source file whose path contains Unicode |
| `malformed/` | Invalid UTF-8 / Latin-1 bytes |

Run `./bootstrap.sh` from this directory to initialize a Git repository in each fixture (idempotent). Tests that need Git history should call the script or assume it has been run in CI.

Do **not** commit huge generated trees. See `tests/performance/README.md` for scale tiers.
