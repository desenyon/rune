# Release gates

A release cannot occur while any required release gate fails.
Status values: `pass`, `fail`, `not_run`.

Last updated: 2026-08-15
Evaluator: root coordinator

## Summary

| Gate | Status |
| --- | --- |
| all required specifications complete | fail |
| all unit tests pass | pass |
| all integration tests pass | pass |
| all regression tests pass | fail |
| all security tests pass | pass |
| all migrations pass | pass |
| all compatibility tests for supported targets | fail |
| performance suite has no unresolved critical regression | fail |
| context compiler evaluation passes established thresholds | pass |
| memory freshness evaluation passes | pass |
| handoff evaluation passes | pass |
| full cross subsystem lifecycle test passes | pass |
| release binaries build successfully | fail |
| documentation matches behavior | fail |
| license review is complete | fail |
| no required feature remains represented by TODO | fail |
| no production path depends on fake data | fail |
| no known data corruption issue remains | fail |
| no known secret exposure issue remains | fail |
| no known unsafe automatic command execution remains | fail |

## Gate details

### G01 all required specifications complete

- status: fail
- evidence: S001–S100 each have exactly one state in `docs/BUILD_STATE.md`. None are `complete`. Remaining `planned`: S060, S067, S068. Several specs remain `active`.
- blocking specifications: any spec not `complete`
- notes: BUILD_STATE.md is the per-specification tracker

### G02 all unit tests pass

- status: pass
- evidence: `cargo test --workspace` succeeded on 2026-08-15 (darwin arm64)
- command: `cargo test --workspace`

### G03 all integration tests pass

- status: pass
- evidence: `crates/evals/tests/s080_lifecycle.rs` and fixture-backed index/session/worktree tests pass

### G04 all regression tests pass

- status: fail
- evidence: S081 has new regression tests for cross-file calls, HTTPS URL parse, and semantic ranking, but there is no dedicated regression corpus covering every historical bug

### G05 all security tests pass

- status: pass
- evidence: `crates/security/tests/s055_prompt_injection.rs` and permission tests pass; retrieved text is wrapped as untrusted content

### G06 all migrations pass

- status: pass
- evidence: storage migration tests cover fresh DB, checksum mismatch, and interrupted retry

### G07 compatibility tests for supported targets

- status: fail
- evidence: `docs/compatibility/` exists; CI runs on ubuntu-latest only. macOS/Linux matrix packaging is defined in `.github/workflows/release.yml` but has not produced signed artifacts on all targets

### G08 performance suite has no unresolved critical regression

- status: fail
- evidence: S066 generators exist; p95 interaction targets in S067 are not yet measured on reference hardware

### G09 context compiler evaluation thresholds

- status: pass
- evidence: `docs/benchmarks/compiler_s063.json` — evidence_recall 1.0; `rune eval` / `cargo test -p rune-evals`

### G10 memory freshness evaluation

- status: pass
- evidence: `docs/benchmarks/memory_s065.json` — staleness, inference rejection, and conflict preservation pass

### G11 handoff evaluation

- status: pass
- evidence: `docs/benchmarks/handoff_s064.json` — structured handoff complete vs transcript baseline

### G12 full cross subsystem lifecycle test

- status: pass
- evidence: `s080_full_cross_subsystem_lifecycle` in `crates/evals/tests/s080_lifecycle.rs`

### G13 release binaries build successfully

- status: fail
- evidence: `scripts/package.sh` and release workflow exist; multi-arch artifacts have not been published from a tag build

### G14 documentation matches behavior

- status: fail
- evidence: architecture docs lag some new CLI surfaces (`rune eval`, `rune package`, `rune crash`, `rune update`, `rune impact`)

### G15 license review is complete

- status: fail
- evidence: Apache-2.0 LICENSE/NOTICE exist; ureq/rustls addition needs NOTICE refresh for new crates

### G16 no required feature remains represented by TODO

- status: fail
- evidence: production paths are real, but remaining planned specs (S060 offline certification, S067 p95 targets, S068 recovery matrix) and active specs (S023, S032, S041–S049, S066, S070, S075, S082, S092) are unfinished

### G17 no production path depends on fake data

- status: fail
- evidence: mocks are test-only; semantic search uses an honest local hashing embedder, not a neural model. Gate stays fail until every planned spec is implemented

### G18 no known data corruption issue remains

- status: fail
- evidence: integrity checks and repair exist; S068 recovery is partial (cache rebuild, missing provider isolation) not a full crash-recovery matrix

### G19 no known secret exposure issue remains

- status: fail
- evidence: export and crash bundles redact; not every provider payload has been audited end-to-end

### G20 no known unsafe automatic command execution remains

- status: fail
- evidence: agent runtime refuses auto-execute under default policy; remaining planned runtime paths keep the gate fail until S019 is `complete`

## Release rule

The project is not releasable. Repeat verification after each specification moves to `complete`.
