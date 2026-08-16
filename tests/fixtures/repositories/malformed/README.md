This directory contains a file whose bytes are not valid UTF-8.

- `not_utf8.bin` is written by `bootstrap.sh` (Latin-1 / invalid UTF-8).
- Indexers must not crash; they should record a diagnostic and skip or replace
  invalid sequences (S068, S079).
