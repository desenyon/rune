# Evaluation result schema (S062–S066)

Evaluation JSON files belong under `docs/benchmarks/` once runs exist. This schema is the contract. No result files are claimed here.

Store reference hardware with every result.

## File naming

```text
docs/benchmarks/<suite>-<YYYYMMDD>T<HHmmss>Z-<host_slug>.json
```

Examples of `<suite>`: `symbol_retrieval`, `structural_path`, `semantic_retrieval`, `historical_reasoning`, `memory_retrieval`, `memory_freshness`, `task_retrieval`, `specification_coverage`, `handoff_fidelity`, `context_compilation`, `compression_quality`, `agent_compatibility`, `performance`.

## JSON Schema (draft 2020-12)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://rune.dev/schemas/eval-result.json",
  "title": "Rune evaluation result",
  "type": "object",
  "required": [
    "schema_version",
    "suite",
    "spec",
    "git_commit",
    "recorded_at",
    "hardware",
    "status",
    "metrics"
  ],
  "properties": {
    "schema_version": { "type": "integer", "minimum": 1 },
    "suite": { "type": "string" },
    "spec": { "type": "string", "pattern": "^S[0-9]{3}$" },
    "git_commit": { "type": "string", "minLength": 7 },
    "rune_version": { "type": "string" },
    "recorded_at": { "type": "string", "format": "date-time" },
    "hardware": {
      "type": "object",
      "required": ["cpu", "os", "arch", "memory_bytes"],
      "properties": {
        "cpu": { "type": "string" },
        "os": { "type": "string" },
        "arch": { "type": "string" },
        "memory_bytes": { "type": "integer", "minimum": 0 },
        "notes": { "type": "string" }
      },
      "additionalProperties": false
    },
    "fixture": { "type": "string" },
    "status": {
      "type": "string",
      "enum": ["pass", "fail", "regressed", "not_run"]
    },
    "thresholds": {
      "type": "object",
      "additionalProperties": { "type": "number" }
    },
    "metrics": {
      "type": "object",
      "additionalProperties": { "type": "number" }
    },
    "context_compiler": {
      "type": "object",
      "properties": {
        "evidence_recall": { "type": "number" },
        "irrelevant_context_rate": { "type": "number" },
        "stale_context_rate": { "type": "number" },
        "contradiction_rate": { "type": "number" },
        "duplicate_rate": { "type": "number" },
        "token_cost": { "type": "number" },
        "latency_ms": { "type": "number" }
      },
      "additionalProperties": false
    },
    "performance": {
      "type": "object",
      "properties": {
        "tier": {
          "type": "string",
          "enum": ["small", "medium", "large", "very_large"]
        },
        "startup_ms_p95": { "type": "number" },
        "initial_index_ms_p95": { "type": "number" },
        "incremental_index_ms_p95": { "type": "number" },
        "search_latency_ms_p95": { "type": "number" },
        "graph_query_ms_p95": { "type": "number" },
        "memory_query_ms_p95": { "type": "number" },
        "session_search_ms_p95": { "type": "number" },
        "context_compile_ms_p95": { "type": "number" },
        "render_ms_p95": { "type": "number" },
        "database_bytes": { "type": "integer" },
        "peak_memory_bytes": { "type": "integer" }
      },
      "additionalProperties": false
    },
    "cases": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "status"],
        "properties": {
          "id": { "type": "string" },
          "status": { "type": "string", "enum": ["pass", "fail", "skip"] },
          "detail": { "type": "string" }
        },
        "additionalProperties": false
      }
    }
  },
  "additionalProperties": false
}
```

## Metric notes (S063, S067)

A Context Compiler change that lowers `token_cost` while materially reducing `evidence_recall` is a regression.

Interaction targets (aim, not yet measured):

- command palette input response under 50 ms at p95
- cached structural navigation under 100 ms at p95
- ordinary graph query under 200 ms at p95
