# Architecture decisions

This file records architecture decisions that affect more than one subsystem.
Product name is Rune. CLI and crate prefixes use `rune`.

---

## DEC-001 Workspace layout and crate names

- decision identifier: DEC-001
- date: 2026-08-15
- problem: AGENTS.md lists both `apps/contextos/` and crate directories, while requiring the product name Rune everywhere.
- options considered:
  - Keep `contextos` binary and crate names, rebrand only in UI copy
  - Rename the application, CLI, and crate prefixes to `rune` while keeping crate directory names from the specification
  - Collapse crates to reduce file count
- decision: Use crate directories from AGENTS.md. Package names are `rune-*`. The application binary and CLI verb is `rune`. `apps/rune` replaces `apps/contextos`. Example commands in AGENTS.md (`contextos index`) map to `rune index`.
- reason: The name override is mandatory. Directory names preserve the specified architecture boundaries. Collapsing crates would violate the instruction that boundaries may not be removed merely to reduce file count.
- tradeoffs: More crates and longer compile graphs. Clearer ownership and independent testability.
- affected components: all crates, CLI, packaging, documentation
- migration implications: none; greenfield repository

---

## DEC-002 Canonical graph in SQLite with typed JSON payloads

- decision identifier: DEC-002
- date: 2026-08-15
- problem: The system must store many node kinds and remain extensible without schema redesign.
- options considered:
  - Table-per-node-kind
  - Property graph in a dedicated graph database
  - SQLite nodes/edges tables with JSON payloads plus extracted indexes and FTS5
- decision: SQLite is the system of record. `nodes`, `edges`, and `provenance` are the canonical tables. Kind-specific fields live in JSON payloads. Common query fields are indexed. Full text lives in FTS5. Large immutable artifacts use a Blake3 content-addressed blob store.
- reason: AGENTS.md requires SQLite unless benchmarks prove otherwise. JSON payloads keep unknown future node types from forcing a redesign. Provenance is first-class, not an afterthought column.
- tradeoffs: JSON queries are less rigid than typed columns. Integrity depends on application-level validation and graph checks.
- affected components: storage, graph, memory, sessions, tasks, specs, search, context compiler
- migration implications: every schema change requires a numbered migration; never rebuild by destroying user data

---

## DEC-003 Identity, hashing, and time

- decision identifier: DEC-003
- date: 2026-08-15
- problem: Objects need stable identifiers independent of mutable line numbers and filesystem paths.
- options considered:
  - UUID v4
  - UUID v7
  - ULID
  - content hash as primary key
- decision: UUID v7 is the object identifier. Blake3 content hashes identify file bytes and derived artifacts. File identity is a node whose payload records repo-relative path plus a stable `file_key` that survives renames when Git rename detection or watcher events can prove continuity. Line numbers are locators, not identities.
- reason: UUID v7 is time-ordered, which helps locality and debugging. Content hashes make caches and blobs deterministic. Separating identity from path/line numbers is required by S003.
- tradeoffs: Joining current path to identity requires an index. UUID strings are larger than integers.
- affected components: core, storage, index, git intelligence, cache
- migration implications: identifier format is part of the on-disk contract

---

## DEC-004 Synchronous SQLite with WAL, async at the edges

- decision identifier: DEC-004
- date: 2026-08-15
- problem: The application is Tokio-based while rusqlite is blocking.
- options considered:
  - sqlx sqlite
  - rusqlite on a dedicated writer plus `spawn_blocking` for reads
  - fully synchronous process with a thin async runtime only for subprocesses
- decision: rusqlite with bundled SQLite, WAL mode, busy timeout, and a connection manager. Async crates call storage through `spawn_blocking`. One writer connection at a time; multiple readers.
- reason: rusqlite gives direct FTS5, backup, and function control. Bundled SQLite makes installs reproducible. sqlx compile-time macros are a poor fit for a library with user-local database paths.
- tradeoffs: blocking calls must never run on the UI/async worker thread without `spawn_blocking`.
- affected components: storage, app, all persistence users
- migration implications: none

---

## DEC-005 Structural features work without semantic providers

- decision identifier: DEC-005
- date: 2026-08-15
- problem: Semantic embeddings and language models may be unavailable, disabled, or offline.
- options considered:
  - Require a default remote model
  - Ship a bundled local model
  - Make semantic retrieval optional; keep structural, git, memory, task, and specification features fully usable
- decision: Semantic mode is pluggable and may be disabled. Core navigation, indexing, search (exact/fuzzy/FTS/structural/graph/temporal), memory, tasks, specs, and context compilation of non-semantic evidence must work offline.
- reason: S059 and S060. The Context OS is the system of record; models are providers.
- tradeoffs: Semantic ranking quality depends on user-configured providers. Default experience is still useful.
- affected components: semantic, search, context compiler, providers, configuration
- migration implications: stored embeddings are provider-scoped and must not be mixed across incompatible models

---

## DEC-006 Domain crates must not depend on Ratatui

- decision identifier: DEC-006
- date: 2026-08-15
- problem: Terminal rendering should not own application state.
- options considered:
  - Put state in the UI crate
  - Keep domain crates renderer-free; UI and motion consume snapshots
- decision: `rune-core`, graph, retrieval, memory, indexing, sessions, tasks, compiler, and providers have no Ratatui/Crossterm dependency. `rune-ui` and `rune-motion` render snapshots. `rune-terminal` owns capability detection and backend I/O. `rune-app` is the orchestrator.
- reason: Required by AGENTS.md section 3. Enables CLI, tests, and future non-TUI surfaces.
- tradeoffs: extra snapshot types. Prevents accidental UI coupling.
- affected components: app, ui, terminal, motion, all domain crates
- migration implications: none

---

## DEC-007 Data is not instruction

- decision identifier: DEC-007
- date: 2026-08-15
- problem: Repository content, sessions, MCP output, and documentation may contain prompt-injection attempts.
- options considered:
  - Trust retrieved text as operator instruction
  - Treat all retrieved text as untrusted content with explicit provenance and permission boundaries
- decision: Retrieved text never changes agent permissions, plugin grants, or tool policy. Provenance tags every fact. Memory extracted from agents starts as `candidate`, never `verified`. Human decisions outrank agent inferences. Secrets are redacted before persistence.
- reason: S054, S055, S011, S012, S095, S088.
- tradeoffs: more friction before automation. Safer default.
- affected components: security, memory, mcp, plugins, agent runtime, context compiler
- migration implications: existing candidate memories must not be auto-promoted

---

## DEC-008 Provider capability declarations

- decision identifier: DEC-008
- date: 2026-08-15
- problem: Coding agents and OSS tools expose uneven operations.
- options considered:
  - Uniform facade that pretends every agent can resume, inject context, and stream
  - Capability-declared providers; unsupported operations fail clearly
- decision: Every provider implements `Provider` and returns an explicit capability set. UI and runtime hide or disable unsupported actions. Missing tools produce structured errors, not silent fallbacks.
- reason: S009, S033, S035. Claiming capabilities that do not exist is forbidden.
- tradeoffs: more branching in UI. Honest behavior.
- affected components: providers, sessions, agent runtime, tools, mcp, plugins, UI
- migration implications: capability changes are additive where possible

---

## DEC-009 Author is a first-class node kind

- decision identifier: DEC-009
- date: 2026-08-15
- problem: S008 requires indexing authors. Git ingestion stored them as `NodeKind::Unknown("author")`.
- options considered:
  - Keep authors as payload fields on commits only
  - Use `Unknown("author")` until a schema migration
  - Add `NodeKind::Author` as a canonical kind
- decision: `NodeKind::Author` is canonical. Commits link to authors with `created_by`. Identity is email when present, otherwise name.
- reason: Authors are specified graph objects, not an unforeseen extension. Unknown is reserved for future kinds.
- tradeoffs: AGENTS.md's minimum kind list did not name Author; this is an additive extension allowed by the extensible model.
- affected components: core, git intelligence, search, history
- migration implications: nodes previously stored as kind `author` via Unknown already serialize as the string `author` and load as `Author`

---

## DEC-010 Hashed n-gram embeddings for default semantic search

- decision identifier: DEC-010
- date: 2026-08-15
- problem: S006 requires a semantic search mode and S059 forbids binding the architecture to one embedding model. A neural model is not bundled.
- options considered:
  - Leave semantic mode unimplemented until a remote/local model is configured
  - Ship a bundled ONNX/GGUF embedder
  - Provide an honest local hashed character n-gram embedder as the default LocalEmbed path
- decision: Default semantic search uses a 256-dimension hashed 3-gram + token embedder in `rune-search`. `rune-semantic` LocalEmbed without a process backend uses the same hashing. Disabled still errors on embed. Neural/remote providers remain pluggable and must not mix vectors across models.
- reason: Structural features must work offline (DEC-005). Semantic mode must exist without pretending a neural model is present. Hashing is inspectable and deterministic.
- tradeoffs: Ranking quality is lexical-ish, not neural. Queries such as "why" can still route to semantic mode and rank related tokens.
- affected components: search, semantic, CLI `--mode semantic`, context compiler retrieval
- migration implications: stored neural embeddings must be provider-scoped; hashed vectors are not interchangeable with model vectors

---

## DEC-011 Context7 HTTPS via ureq

- decision identifier: DEC-011
- date: 2026-08-15
- problem: The Context7 default endpoint is HTTPS. A raw TCP HTTP/1.0 client cannot speak TLS, so live fetches failed unless an http:// URL was configured.
- options considered:
  - Keep HTTP-only TCP and document HTTPS as unsupported
  - Add rustls plus a hand-rolled TLS client
  - Use ureq 2.x with rustls for HTTPS and keep TCP for http://
- decision: HTTPS uses ureq with rustls. HTTP keeps the existing TCP client. Network policy still gates fetches. The provider declares `data_leaving_machine` (S088).
- reason: S030 requires current external documentation retrieval. Silent HTTP-only behavior against an HTTPS URL is a hidden fallback.
- tradeoffs: New TLS crates in Cargo.lock. Live Context7 is still not certified in CI.
- affected components: docs_context, providers, packaging size
- migration implications: none for stored docs; cached HTTP responses remain valid

---

## DEC-012 Cross-file call edges by unique name

- decision identifier: DEC-012
- date: 2026-08-15
- problem: Call edges were resolved only inside a file. Cross-file `greet()` in `b.rs` did not link to `greet` in `a.rs`.
- options considered:
  - Full import-graph and type-aware resolution
  - Leave calls intra-file until a language server is embedded
  - Resolve pending calls after each scan: unique other-file name, else import-path unique match
- decision: Indexer stores unresolved `pending_calls` on the file record, then `resolve_cross_file_calls` after scan/`index_path`. Unique callee names across other files become `Calls` + `References`. Ambiguous names resolve only when an import path uniquely matches. Remaining pendings stay on the record.
- reason: S003 requires call relationships when derivable. Name uniqueness is derivable without a type checker. Ambiguous matches must not invent edges.
- tradeoffs: False negatives for overloaded names; possible false positives if two files export the same unique name and the caller meant a different unindexed target.
- affected components: index persist, indexer, graph, search structural mode, S080 lifecycle
- migration implications: file records gain `pending_calls` with `#[serde(default)]` so older records load empty

