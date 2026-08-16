# Release gates

A release cannot occur while any required release gate fails.
Status values: `pass`, `fail`, `not_run`.

Last updated: 2026-08-15
Evaluator: root coordinator

## Summary

| Gate | Status |
| --- | --- |
| all required specifications complete | fail |
| all unit tests pass | fail |
| all integration tests pass | fail |
| all regression tests pass | fail |
| all security tests pass | fail |
| all migrations pass | fail |
| all compatibility tests pass for supported targets | fail |
| performance suite has no unresolved critical regression | fail |
| context compiler evaluation passes established thresholds | fail |
| memory freshness evaluation passes | fail |
| handoff evaluation passes | fail |
| full cross subsystem lifecycle test passes | fail |
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
- evidence: repository is greenfield; S001–S100 are not implemented
- blocking specifications: S001–S100
- notes: BUILD_STATE.md is the per-specification tracker

### G02 all unit tests pass

- status: fail
- evidence: test suite not yet established
- command: `cargo test --workspace`

### G03 all integration tests pass

- status: fail
- evidence: `tests/` fixtures and cross-crate journeys are not implemented

### G04 all regression tests pass

- status: fail
- evidence: no regression corpus yet (S081)

### G05 all security tests pass

- status: fail
- evidence: prompt-injection corpus (S055) and permission tests (S054) are missing

### G06 all migrations pass

- status: fail
- evidence: storage crate and migration tests not implemented

### G07 compatibility tests for supported targets

- status: fail
- evidence: `docs/compatibility/` matrix not populated; no OS/terminal/agent runs

### G08 performance suite has no unresolved critical regression

- status: fail
- evidence: S066/S067 benchmarks not implemented

### G09 context compiler evaluation thresholds

- status: fail
- evidence: S063 evaluation harness not implemented

### G10 memory freshness evaluation

- status: fail
- evidence: S065 evaluation harness not implemented

### G11 handoff evaluation

- status: fail
- evidence: S064 evaluation harness not implemented

### G12 full cross subsystem lifecycle test

- status: fail
- evidence: S080 chain not implemented

### G13 release binaries build successfully

- status: fail
- evidence: packaging (S085) not implemented; workspace not compiling yet

### G14 documentation matches behavior

- status: fail
- evidence: architecture and subsystem docs are incomplete relative to S082/S083

### G15 license review is complete

- status: fail
- evidence: third-party notices and integration license records (S034, S084) are incomplete

### G16 no required feature remains represented by TODO

- status: fail
- evidence: implementation has not started

### G17 no production path depends on fake data

- status: fail
- evidence: no production paths exist yet; gate stays fail until implementations are real and tests-only mocks are isolated

### G18 no known data corruption issue remains

- status: fail
- evidence: integrity repair (S100) and crash recovery (S068) not implemented

### G19 no known secret exposure issue remains

- status: fail
- evidence: secret detection/redaction (S053) not implemented

### G20 no known unsafe automatic command execution remains

- status: fail
- evidence: agent runtime permission policy (S019, S054, S116) not implemented

## Release rule

The project is not releasable. Repeat verification after each specification moves to `complete`.
