# Performance suite generators (S066)

Do **not** commit huge generated codebases. Generate them locally or in CI, then discard.

## Tiers

| Tier | Approx. files | Approx. LOC | Intent |
| --- | --- | --- | --- |
| small | 100 | ~5k | Fast unit-adjacent benches |
| medium | 1_000 | ~50k | Interactive latency |
| large | 10_000 | ~500k | Index and search pressure |
| very_large | 100_000 | ~5M | Stress; overnight / dedicated hardware |

Exact counts are produced by `gen.rs`. Measure on reference hardware and store JSON under `docs/benchmarks/` using [schema.md](../../docs/benchmarks/schema.md).

## Metrics (required)

startup, initial indexing, incremental indexing, search latency, graph query latency, memory query latency, session search latency, context compilation, render latency, database size, peak memory.

## Generator

`tests/performance/gen.rs` is a standalone program (not a workspace member yet). Compile later with:

```bash
rustc tests/performance/gen.rs -o /tmp/rune-perf-gen
/tmp/rune-perf-gen --tier medium --out /tmp/rune-perf-medium
```

Or add a `rune-evals` binary that embeds the same generator. Output directories must stay outside git (for example `/tmp` or a local `target/perf-fixtures/` ignored path).

## Reference hardware

Record CPU, OS, arch, and memory_bytes in every result file. Do not compare p95 numbers across unlabeled machines.
