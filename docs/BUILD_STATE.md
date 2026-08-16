# Build state

Product: Rune
Last updated: 2026-08-15
Coordinator: root coordinator

Every required specification has exactly one state: `planned`, `active`, `blocked`, `verification`, or `complete`.

## Summary

| Spec | Title | Status | Owner | Dependencies |
| --- | --- | --- | --- | --- |
| S001 | Workspace discovery | verification | rune-index | S056, S057, S052 |
| S002 | Terminal capability engine | verification | rune-terminal | none |
| S003 | Structural code index | verification | rune-index | S001, S056, S058 |
| S004 | Semantic repository graph | verification | rune-semantic | S003, S059 |
| S005 | Multimodal knowledge graph | verification | rune-index | S003, S056 |
| S006 | Search engine | verification | rune-search | S003, S005, S008, S011, S056 |
| S007 | Symbol intelligence | verification | rune-graph | S003, S008, S016, S015, S009, S011 |
| S008 | Git temporal intelligence | verification | rune-git-intelligence | S001, S056 |
| S009 | Agent session ingestion | verification | rune-sessions | S056, S033 |
| S010 | Session intelligence | verification | rune-sessions | S009 |
| S011 | Persistent memory system | verification | rune-memory | S056, S054 |
| S012 | Memory extraction | verification | rune-memory | S011, S010, S008, S015 |
| S013 | Memory freshness engine | verification | rune-memory | S011, S003, S008 |
| S014 | Historical reasoning graph | verification | rune-history | S008, S010, S011, S016 |
| S015 | Specification system | verification | rune-specs | S056 |
| S016 | Task dependency graph | verification | rune-tasks | S015, S056 |
| S017 | Parallelization analysis | verification | rune-tasks | S016, S003 |
| S018 | Worktree orchestration | verification | rune-worktrees | S016, S008 |
| S019 | Agent runtime | verification | rune-agent-runtime | S018, S054, S024 |
| S020 | Agent event normalization | verification | rune-agent-runtime | S019 |
| S021 | Cross agent handoff | verification | rune-handoff | S019, S010, S016 |
| S022 | Handoff compiler | verification | rune-handoff | S021, S024 |
| S023 | Branchable context | active | rune-context-compiler | S024, S090 |
| S024 | Context compiler | verification | rune-context-compiler | S006, S007, S011, S013, S015, S016, S008, S030 |
| S025 | Retrieval scoring | verification | rune-context-compiler | S024 |
| S026 | Token budget allocator | verification | rune-context-compiler | S024 |
| S027 | Context deduplication | verification | rune-context-compiler | S024 |
| S028 | Adaptive compression | verification | rune-compression | S020, S024 |
| S029 | Agent communication policy | verification | rune-agent-runtime | S019 |
| S030 | External documentation context | verification | rune-docs-context | S033, S056 |
| S031 | Documentation freshness | verification | rune-docs-context | S030, S001 |
| S032 | Portable context packs | active | rune-context-compiler | S024, S054, S076 |
| S033 | OSS provider framework | verification | rune-providers | S054 |
| S034 | Required OSS research targets | verification | docs/integrations | none |
| S035 | Command line tool adapters | verification | rune-tools | S033 |
| S036 | MCP interoperability | verification | rune-mcp | S033, S054 |
| S037 | Plugin system | verification | rune-plugins | S033, S054 |
| S038 | Universal command palette | verification | rune-ui | S006, S039, S072 |
| S039 | TUI design system | verification | rune-ui | S002 |
| S040 | Motion engine | verification | rune-motion | S039, S073 |
| S041 | Shared element transitions | active | rune-motion | S040 |
| S042 | Adaptive rendering | active | rune-ui | S002, S040 |
| S043 | Graph explorer | active | rune-ui | S039, S100 |
| S044 | Context inspector | active | rune-ui | S024, S092, S039 |
| S045 | Memory timeline | verification | rune-memory | S011, S013, S039 |
| S046 | Session explorer | active | rune-ui | S009, S010, S039 |
| S047 | Agent cockpit | active | rune-ui | S019, S020, S039 |
| S048 | Task graph view | active | rune-ui | S016, S039 |
| S049 | Specification coverage view | active | rune-ui | S015, S003, S051, S039 |
| S050 | Diff intelligence | verification | rune-index | S003, S008, S013, S016, S015 |
| S051 | Test intelligence | active | rune-index | S003 |
| S052 | Process awareness | verification | rune-index | S001 |
| S053 | Environment awareness | verification | rune-security | S054 |
| S054 | Security model | verification | rune-security | none |
| S055 | Prompt injection resistance | verification | rune-security | S054, S024, S011, S036 |
| S056 | Storage | verification | rune-storage | none |
| S057 | Database migrations | verification | rune-storage | S056 |
| S058 | Content addressed cache | verification | rune-storage | S056 |
| S059 | Semantic provider abstraction | verification | rune-semantic | S033 |
| S060 | Offline operation | planned | unassigned | S003, S008, S011, S016, S056 |
| S061 | Observability | verification | rune-telemetry | none |
| S062 | Evaluation framework | verification | rune-evals | S056 |
| S063 | Context compiler evaluation | verification | rune-evals | S062, S024 |
| S064 | Handoff evaluation | verification | rune-evals | S062, S022 |
| S065 | Memory evaluation | verification | rune-evals | S062, S012, S013 |
| S066 | Performance suite | active | tests/performance | S079 |
| S067 | Responsiveness targets | planned | unassigned | S066, S042, S038 |
| S068 | Failure recovery | planned | unassigned | S056, S057, S019 |
| S069 | Compatibility matrix | verification | docs/compatibility | S002, S009 |
| S070 | Configuration | active | rune-core | none |
| S071 | Themes | verification | rune-ui | S039, S070 |
| S072 | Keybinding system | verification | rune-ui | S070 |
| S073 | Accessibility | verification | rune-ui | S039, S040 |
| S074 | Onboarding | verification | rune-cli | S001, S002, S035, S009 |
| S075 | Import | active | rune-sessions | S009, S015, S016, S011, S008 |
| S076 | Export | verification | rune-app | S032, S054 |
| S077 | CLI surface | verification | rune-cli | S001, S006, S078 |
| S078 | Doctor command | verification | rune-cli | S056, S002, S033, S009 |
| S079 | Integration test repositories | verification | tests/fixtures/repositories | none |
| S080 | Cross subsystem integration tests | verification | rune-evals | S012, S013, S016, S019, S021, S024, S049 |
| S081 | Regression discipline | verification | tests | S079 |
| S082 | Documentation | active | docs/architecture | none |
| S083 | Architecture diagrams | verification | docs/architecture | S082 |
| S084 | Licensing review | verification | root coordinator | none |
| S085 | Packaging | verification | apps/rune | S077 |
| S086 | Updates | verification | rune-app | S057, S085 |
| S087 | Crash reporting | verification | rune-app | S061, S054 |
| S088 | Data ownership | verification | rune-providers | S054 |
| S089 | Deterministic state inspection | verification | rune-context-compiler | S024, S013, S016, S006, S017 |
| S090 | Context difference engine | verification | rune-context-compiler | S024 |
| S091 | Agent knowledge comparison | verification | rune-context-compiler | S009, S024 |
| S092 | Context provenance visualization | active | rune-ui | S024, S044 |
| S093 | Context pinning | verification | rune-context-compiler | S024 |
| S094 | Context exclusion | verification | rune-context-compiler | S024 |
| S095 | Human decisions | verification | rune-memory | S011 |
| S096 | Conflict resolution | verification | rune-memory | S011, S095 |
| S097 | Entity resolution | verification | rune-memory | S003, S005, S011 |
| S098 | Background indexing | verification | rune-index | S003, S099 |
| S099 | File watching | verification | rune-index | S001, S003 |
| S100 | Graph integrity | verification | rune-storage | S056 |

Highest-leverage unblocked work: S023 branchable context merge UX; S032 portable packs; S041 shared-element motion; S066/S067 measured performance on large fixtures; remaining TUI inspector depth. Semantic search, Context7 HTTPS, S080 lifecycle, S063–S065 evals, cross-file calls, multimodal documents, diff impact, and multi-arch packaging scripts are in verification.

## Protocol notes

- owner: crate or agent currently responsible
- tests: names of unit/integration/regression coverage
- known failures: open defects
- benchmark status: `not_run` | `pass` | `fail` | `regressed`
- documentation status: `missing` | `partial` | `matches`
- integration status: `none` | `partial` | `verified`

---

## S001 Workspace discovery

- specification: S001
- owner: rune-index
- dependencies: S056, S057, S052
- current status: verification
- implemented components: git/monorepo/nested/worktree detection, languages, package managers, incremental hash skip, coding-agent and spec directory markers
- remaining components: CLI `rune index` wiring; process association is in S052
- tests: cargo_workspace_monorepo_detected, nested_repo_detected, incremental_workspace_scan_skips_unchanged_files
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S002 Terminal capability engine

- specification: S002
- owner: rune-terminal
- dependencies: none
- current status: verification
- implemented components: capability detection (true color, unicode, mouse, hyperlinks, synchronized output, Kitty/Sixel/iTerm graphics, cells/pixels), renderer levels Basic through Graphics, graceful degradation
- remaining components: dated per-terminal certification runs (S069)
- tests: rune-terminal unit tests for env-based probes
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S003 Structural code index

- specification: S003
- owner: rune-index
- dependencies: S001, S056, S058
- current status: verification
- implemented components: Tree-sitter (and fallback) symbol parse, stable file_key, incremental skip, persist to graph, intra-file and cross-file `Calls`/`References` via unique name plus import-path disambiguation
- remaining components: full import-graph resolution for ambiguous names; symbol-level git blame
- tests: rust_function_symbols_indexed, rust_function_and_test_are_indexed, file_key_is_stable_across_content_hash_changes, cross_file_calls_are_resolved
- known failures: ambiguous same-name callees without a unique import match stay pending
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S004 Semantic repository graph

- specification: S004
- owner: rune-semantic
- dependencies: S003, S059
- current status: verification
- implemented components: SemanticComponent linked to structural nodes, fingerprint invalidation, Disabled mode stores manual summaries without inventing vectors
- remaining components: automatic summary generation when an LLM provider is configured; UI
- tests: disabled_provider_still_allows_manual_component_summary, fingerprint_change_invalidates_component, disabled_embed_does_not_invent_vectors
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S005 Multimodal knowledge graph

- specification: S005
- owner: rune-index
- dependencies: S003, S056
- current status: verification
- implemented components: markdown headings, toml tables, json/yaml keys indexed as Document + DocumentationSection with Documents/Contains edges; structure preserved rather than flattened chunks
- remaining components: PDF, image metadata, issue/PR content, generated docs, database schemas
- tests: markdown_is_indexed_as_document_sections
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S006 Search engine

- specification: S006
- owner: rune-search
- dependencies: S003, S005, S008, S011, S056
- current status: verification
- implemented components: exact, fuzzy (Nucleo), FTS5, structural, graph, temporal, hybrid, semantic (hashed 3-gram embedder); intent router including why/similar/meaning; forced `--mode`; CLI `rune search`
- remaining components: neural/remote embeddings via S059 ProcessBackend; p95 latency measurement (S067)
- tests: search_router_picks_graph_for_path_queries, router_picks_structural_for_fn_prefix, forced_mode_overrides_router, exact_and_fuzzy_find_function, graph_search_returns_path_nodes, search_finds_indexed_function, semantic mode routing tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S007 Symbol intelligence

- specification: S007
- owner: rune-graph
- dependencies: S003, S008, S016, S015, S009, S011
- current status: verification
- implemented components: definition node lookup, callers/callees/references via graph neighbors; TUI graph explorer lists neighbors
- remaining components: implementations/tests/commits/tasks/specs/sessions/memories/failures as a single inspector surface
- tests: callers_and_callees_follow_call_edges, cross_file_calls_are_resolved
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S008 Git temporal intelligence

- specification: S008
- owner: rune-git-intelligence
- dependencies: S001, S056
- current status: verification
- implemented components: commit/branch/tag/worktree indexing via porcelain; commit nodes after commit; rename parse; Author nodes with created_by
- remaining components: symbol-level change derivation; historical question compiler wiring
- tests: git_commit_nodes_created_after_a_commit, parses_commit_log_and_renames
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S009 Agent session ingestion

- specification: S009
- owner: rune-sessions
- dependencies: S056, S033
- current status: verification
- implemented components: Claude Code, Codex, Cursor, OpenCode, Gemini CLI, Aider file adapters; discovery/import/inspect/query only; raw transcript in payload and blob store
- remaining components: continuation, context injection, streaming, command invocation, and handoff are intentionally undeclared
- tests: adapters_do_not_declare_continuation_if_not_implemented, missing_session_dir_yields_empty_list_not_error
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S010 Session intelligence

- specification: S010
- owner: rune-sessions
- dependencies: S009
- current status: verification
- implemented components: deterministic heuristic extraction of goal/attempts/failures/files with source turn ids; Validity::Candidate only
- remaining components: LLM-assisted extraction remains optional and must not auto-verify
- tests: extraction_from_fixture_creates_attempt_and_failure_linked_to_turns
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S011 Persistent memory system

- specification: S011
- owner: rune-memory
- dependencies: S056, S054
- current status: verification
- implemented components: categories, lifecycle validity, agent-guidance vs historical retrieval, inspectable records
- remaining components: TUI timeline wiring (S045)
- tests: stale_memory_not_in_guidance_retrieval, agent_inference_stored_as_candidate_not_verified
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S012 Memory extraction

- specification: S012
- owner: rune-memory
- dependencies: S011, S010, S008, S015
- current status: verification
- implemented components: extractors for sessions, human statements, commits, specifications; `persist` auto-ingests agent inferences as Candidate and human preferences; ObservedFacts from transcripts are not auto-verified
- remaining components: extract from Git commit messages during `rune index`; TUI confirmation of inferred human ObservedFacts
- tests: agent_inference_stored_as_candidate_not_verified
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S013 Memory freshness engine

- specification: S013
- owner: rune-memory
- dependencies: S011, S003, S008
- current status: verification
- implemented components: hash-change staleness, inspectable FreshnessReason; `rune index` applies freshness to changed files
- remaining components: automatic run after Git indexer for commit-linked memories
- tests: freshness_marks_memory_stale_when_related_file_hash_changes, reindex_marks_related_memory_stale
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S014 Historical reasoning graph

- specification: S014
- owner: rune-history
- dependencies: S008, S010, S011, S016
- current status: verification
- implemented components: discussed_in/decided_in/attempted_in/failed_in/changed_by helpers, why_path, failed_approaches_for
- remaining components: does not invent Git/task/spec links; those nodes must already exist
- tests: why_path_and_failed_approaches
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S015 Specification system

- specification: S015
- owner: rune-specs
- dependencies: S056
- current status: verification
- implemented components: specification and requirement nodes, coverage of implementing evidence, CLI `rune specs`
- remaining components: import from OpenSpec/markdown trees; coverage TUI (S049)
- tests: requirement_with_no_evidence_listed_as_uncovered
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S016 Task dependency graph

- specification: S016
- owner: rune-tasks
- dependencies: S015, S056
- current status: verification
- implemented components: tasks, blockers, actionable set, cycle detection, CLI `rune tasks`
- remaining components: assignment/worktree fields in live TUI (S048)
- tests: task_cycle_detected, actionable_requires_complete_dependencies
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S017 Parallelization analysis

- specification: S017
- owner: rune-tasks
- dependencies: S016, S003
- current status: verification
- implemented components: overlapping files/symbols refuse conflict-free claims; confidence and explanation; no claim without evidence
- remaining components: schema/migration/generated-artifact overlap
- tests: parallelization_refuses_conflict_free_claim_when_same_file_listed, parallelization_refuses_without_resource_evidence, s080_full_cross_subsystem_lifecycle
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S018 Worktree orchestration

- specification: S018
- owner: rune-worktrees
- dependencies: S016, S008
- current status: verification
- implemented components: create/list/inspect, stale detection, delete requires confirm: true
- remaining components: task/agent assignment UI; abandoned cleanup policy UX
- tests: worktree_delete_without_confirm_fails, create_list_and_stale_detection
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S019 Agent runtime

- specification: S019
- owner: rune-agent-runtime
- dependencies: S018, S054, S024
- current status: verification
- implemented components: execution records, policy-gated local subprocess, default no auto-execute and no network
- remaining components: remote agent extension; cockpit UI; token/cost from real providers when available
- tests: refuses_execute_when_policy_denies_process_execute, default_policy_does_not_auto_execute_commands
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S020 Agent event normalization

- specification: S020
- owner: rune-agent-runtime
- dependencies: S019
- current status: verification
- implemented components: stdout heuristics into thinking/search/read/write/command/test/error/warning/decision/question/result/handoff/completion; raw preserved
- remaining components: provider-specific streaming parsers
- tests: machine_policy_matches_expected_style
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S021 Cross agent handoff

- specification: S021
- owner: rune-handoff
- dependencies: S019, S010, S016
- current status: verification
- implemented components: handoff graph objects, lineage session A → handoff → session B, CLI compile/list
- remaining components: live agent transfer via runtime (S019)
- tests: handoff lineage session A → handoff → session B (rune-handoff)
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S022 Handoff compiler

- specification: S022
- owner: rune-handoff
- dependencies: S021, S024
- current status: verification
- implemented components: mutable HandoffPackage; modes full/balanced/compact/custom; uses compiler retrievers
- remaining components: inspect/edit UI before transfer
- tests: rune-handoff unit tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S023 Branchable context

- specification: S023
- owner: rune-context-compiler
- dependencies: S024, S090
- current status: active
- implemented components: ContextController snapshot/branch/compare/merge/archive types
- remaining components: CLI commands and inspect UI
- tests: compiler unit tests for controller types
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: none

## S024 Context compiler

- specification: S024
- owner: rune-context-compiler
- dependencies: S006, S007, S011, S013, S015, S016, S008, S030
- current status: verification
- implemented components: compile pipeline with budget, pin, exclude, capsule diff; CLI retrievers use MemoryStore/TaskStore/SpecStore so stale memory is omitted from guidance
- remaining components: neural semantic retriever; inspector UI event loop
- tests: compiler::tests::{compiler_respects_budget, pinned_item_is_included, excluded_item_is_omitted, capsule_diff_shows_added_and_removed}; compiler_s063
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial

## S025 Retrieval scoring

- specification: S025
- owner: rune-context-compiler
- dependencies: S024
- current status: verification
- implemented components: multi-signal rank_candidates with configurable RankingWeights
- remaining components: evaluation-tuned defaults (S063)
- tests: compiler_respects_budget, pinned_item_is_included
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S026 Token budget allocator

- specification: S026
- owner: rune-context-compiler
- dependencies: S024
- current status: verification
- implemented components: category allocation adapted by task type; used token reporting
- remaining components: measured token accuracy vs real providers
- tests: compiler_respects_budget
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S027 Context deduplication

- specification: S027
- owner: rune-context-compiler
- dependencies: S024
- current status: verification
- implemented components: candidate dedup with provenance retained on kept item
- remaining components: semantic-summary vs raw-content collapse
- tests: compiler unit tests (duplicates_removed)
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S028 Adaptive compression

- specification: S028
- owner: rune-compression
- dependencies: S020, S024
- current status: verification
- implemented components: raw/structured/summary/errors/diff/changes_since_previous; reversible via blob raw_hash
- remaining components: wire into agent event stream (S020)
- tests: tests::{small_payloads_stay_raw, error_output_uses_errors_representation, roundtrip_via_blob_store}
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S029 Agent communication policy

- specification: S029
- owner: rune-agent-runtime
- dependencies: S019
- current status: verification
- implemented components: full/concise/minimal/machine formatters
- remaining components: TUI rendering of machine events
- tests: machine_policy_matches_expected_style
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S030 External documentation context

- specification: S030
- owner: rune-docs-context
- dependencies: S033, S056
- current status: verification
- implemented components: Context7 provider adapter; HTTPS via ureq/rustls (port 443) plus HTTP TCP; versioned objects; Query fails clearly when network/policy denied; `data_leaving_machine` disclosure
- remaining components: live fetch against the public Context7 API is not certified in CI; version pinning UX
- tests: docs_context unit tests for https/http parse and network-policy denied
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S031 Documentation freshness

- specification: S031
- owner: rune-docs-context
- dependencies: S030, S001
- current status: verification
- implemented components: version mismatch warning and cache invalidation
- remaining components: automatic refresh when lockfile versions change
- tests: docs_context freshness tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S032 Portable context packs

- specification: S032
- owner: rune-context-compiler
- dependencies: S024, S054, S076
- current status: active
- implemented components: ContextPack types and manifest
- remaining components: inspect-before-export UI; pack CLI
- tests: compiler pack types
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: none

## S033 OSS provider framework

- specification: S033
- owner: rune-providers
- dependencies: S054
- current status: verification
- implemented components: Provider trait, identity, capabilities, optional data_leaving_machine disclosure
- remaining components: async query/execute/stream/inspect/export/import as a uniform runtime, not only per-adapter methods
- tests: provider registry and capability tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S034 Required OSS research targets

- specification: S034
- owner: docs/integrations
- dependencies: none
- current status: verification
- implemented components: 18 research notes plus index covering architecture, license, mechanisms, limitations, integration options, and clean-room rules; CASS rider called out as do-not-copy
- remaining components: confirm licenses against pinned upstream versions before any code reuse; keep notes current as integrations land
- tests: crates/core/tests/docs_contracts.rs::s034_research_notes_cover_required_targets_and_sections
- known failures: none recorded
- benchmark status: not_run
- documentation status: matches
- integration status: partial

## S035 Command line tool adapters

- specification: S035
- owner: rune-tools
- dependencies: S033
- current status: verification
- implemented components: git, gh, rg, fd, bat, jq, curl, docker, kubectl, ssh, cargo, npm, pnpm, bun, uv, python, go, brew, hyperfine; which() first; JSON/porcelain when available; missing tool is Unavailable
- remaining components: live invoke tests per installed binary; gh unknown commands are not table-scraped
- tests: catalog_covers_required_tools, missing_jq_returns_structured_unavailable_error, git_status_injects_porcelain_not_human_output
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S036 MCP interoperability

- specification: S036
- owner: rune-mcp
- dependencies: S033, S054
- current status: verification
- implemented components: .mcp.json/Claude/Cursor discovery, stdio JSON-RPC initialize/tools/list/tools/call gated by McpTool+ProcessExecute; results UntrustedContent never verified memory; SSE URLs discovered then rejected as unsupported
- remaining components: live mock-server roundtrip; SSE transport
- tests: tool_result_is_untrusted_and_not_verified_memory, spawn_requires_policy, discover_missing_configs_is_empty, discover_mcp_json
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S037 Plugin system

- specification: S037
- owner: rune-plugins
- dependencies: S033, S054
- current status: verification
- implemented components: manifest contributions and permissions; load only from configured dir; invalid plugin fails without dropping others; default no process access
- remaining components: actual contribution loading (search sources, renderers) beyond manifest validation
- tests: plugin_without_permissions_cannot_execute_process, invalid_plugin_fails_clearly_without_corrupting_others, default_policy_cannot_load_plugins
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S038 Universal command palette

- specification: S038
- owner: rune-ui
- dependencies: S006, S039, S072
- current status: verification
- implemented components: palette actions plus SearchEngine hits; nucleo ordering
- remaining components: contextual actions per object kind; all snapshot views
- tests: rune-ui palette tests; CLI TUI uses SearchEngine
- known failures: live TUI mounts palette plus status, not every snapshot view
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S039 TUI design system

- specification: S039
- owner: rune-ui
- dependencies: S002
- current status: verification
- implemented components: semantic tokens, spacing, typography, high-contrast theme
- remaining components: apply tokens to every remaining view in the live event loop
- tests: rune-ui theme/key tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S040 Motion engine

- specification: S040
- owner: rune-motion
- dependencies: S039, S073
- current status: verification
- implemented components: shared effects, reduced motion instant path, frame budgets, shared-element interpolation, buffer diff hint
- remaining components: TUI widget integration (S039 still open)
- tests: tests::{reduced_motion_zeroes_duration, ordinary_and_high_fidelity_budgets, shared_element_reaches_new_rect, buffer_diff_skips_identical_frames}
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S041 Shared element transitions

- specification: S041
- owner: rune-motion
- dependencies: S040
- current status: active
- implemented components: rectangle/style interpolation, reduced-motion instant path
- remaining components: live shared-element from palette hit to inspector header in the TUI event loop
- tests: shared_element_reaches_new_rect, reduced_motion_zeroes_duration
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S042 Adaptive rendering

- specification: S042
- owner: rune-ui
- dependencies: S002, S040
- current status: active
- implemented components: live TUI event loop, buffer-diff skip, motion frame budgets
- remaining components: measured 30/60 fps on reference terminals; synchronized updates where supported
- tests: buffer_diff_skips_identical_frames; rune-ui render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S043 Graph explorer

- specification: S043
- owner: rune-ui
- dependencies: S039, S100
- current status: active
- implemented components: TUI graph view with focus, neighbor expand/collapse, kind filter, path list
- remaining components: pan/zoom abstraction, subgraph compare, provenance panel, small-screen layout polish
- tests: rune-ui graph explorer tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S044 Context inspector

- specification: S044
- owner: rune-ui
- dependencies: S024, S092, S039
- current status: active
- implemented components: inspector snapshot with budget, included objects, reasons
- remaining components: pin/remove before invocation in the live event loop; excluded-candidate list
- tests: rune-ui inspector render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S045 Memory timeline

- specification: S045
- owner: rune-memory
- dependencies: S011, S013, S039
- current status: verification
- implemented components: MemoryTimeline events for created/verified/stale/contradicted/superseded/archived with evidence counts
- remaining components: Ratatui lifecycle view (S039)
- tests: covered via freshness and conflict persistence of timeline fields
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S046 Session explorer

- specification: S046
- owner: rune-ui
- dependencies: S009, S010, S039
- current status: active
- implemented components: unified session list with provider/project filters in the TUI
- remaining components: resume/fork/handoff/compare actions from the explorer
- tests: rune-ui session explorer render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S047 Agent cockpit

- specification: S047
- owner: rune-ui
- dependencies: S019, S020, S039
- current status: active
- implemented components: agent cards with provider, task, status, recent events
- remaining components: live event stream navigation; token/cost when providers emit them
- tests: rune-ui cockpit render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S048 Task graph view

- specification: S048
- owner: rune-ui
- dependencies: S016, S039
- current status: active
- implemented components: ready/active/blocked/failed/review/complete rendering and blocker text
- remaining components: assignment to agents from the view
- tests: rune-ui task graph render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S049 Specification coverage view

- specification: S049
- owner: rune-ui
- dependencies: S015, S003, S051, S039
- current status: active
- implemented components: requirement-to-evidence list and uncovered detection in the TUI
- remaining components: jump to implementing symbols/tests/commits from the view
- tests: rune-ui spec coverage render tests; S080 coverage link
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S050 Diff intelligence

- specification: S050
- owner: rune-index
- dependencies: S003, S008, S013, S016, S015
- current status: verification
- implemented components: `impact_for_files` returns changed symbols, dependents, tests, tasks, specs, potentially stale memories, related decisions; CLI `rune impact`
- remaining components: automatic review-capsule compilation from impact; Git hunk-level symbol mapping
- tests: s080_full_cross_subsystem_lifecycle uses impact_for_files
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S051 Test intelligence

- specification: S051
- owner: rune-index
- dependencies: S003
- current status: active
- implemented components: test function detection during structural parse; TestRun payload type
- remaining components: test run history, flakiness, compiler debugging weight
- tests: rust_function_and_test_are_indexed
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S052 Process awareness

- specification: S052
- owner: rune-index
- dependencies: S001
- current status: verification
- implemented components: ps/lsof/stat parsers associating cwd; no terminate API
- remaining components: live sampling on all OS; graph persistence of Process nodes from CLI
- tests: lsof_and_ps_parsers_associate_cwd, linux_stat_ppid_parses
- known failures: lsof/ps spawn failure no longer aborts `rune index` (empty process list)
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S053 Environment awareness

- specification: S053
- owner: rune-security
- dependencies: S054
- current status: verification
- implemented components: nonsecret project facts; secret detection and redaction before persist/export/crash bundles
- remaining components: richer environment census in onboard
- tests: export_redacts_akia_keys, crash_bundle_redacts_secrets
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S054 Security model

- specification: S054
- owner: rune-security
- dependencies: none
- current status: verification
- implemented components: untrusted content wrapping, permission policy, filesystem/process/network/plugin/MCP/agent boundaries
- remaining components: end-to-end audit of every provider payload
- tests: rune-security policy tests; S055 corpus
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S055 Prompt injection resistance

- specification: S055
- owner: rune-security
- dependencies: S054, S024, S011, S036
- current status: verification
- implemented components: prompt-injection corpus under tests/fixtures/security/prompt_injection; UntrustedContent never becomes instruction; FTS retrieval of canary does not grant permissions
- remaining components: compiler/memory/MCP path tests that extracted memories stay candidate; full S080 chain
- tests: crates/security/tests/s055_prompt_injection.rs
- known failures: none recorded
- benchmark status: not_run
- documentation status: matches
- integration status: partial

## S056 Storage

- specification: S056
- owner: rune-storage
- dependencies: none
- current status: verification
- implemented components: SQLite nodes/edges/provenance/FTS5/settings; content-addressed blobs
- remaining components: measured size on very large repositories (S066)
- tests: storage unit and migration tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S057 Database migrations

- specification: S057
- owner: rune-storage
- dependencies: S056
- current status: verification
- implemented components: numbered migrations, fresh DB, checksum mismatch, interrupted retry; no silent destroy
- remaining components: rollback strategy where practical
- tests: storage migration tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S058 Content addressed cache

- specification: S058
- owner: rune-storage
- dependencies: S056
- current status: verification
- implemented components: Blake3 blob/cache keys; fingerprint invalidation
- remaining components: cache of embeddings/layouts at repository scale
- tests: storage cache tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S059 Semantic provider abstraction

- specification: S059
- owner: rune-semantic
- dependencies: S033
- current status: verification
- implemented components: LocalEmbed/RemoteEmbed/LocalLlm/RemoteLlm/Disabled; Disabled declares no Embed/Complete; LocalEmbed without a process backend uses the hashed n-gram embedder (not a neural model)
- remaining components: wire ProcessBackend to a real local/remote embedder; do not mix embeddings across providers
- tests: disabled_embed_does_not_invent_vectors, disabled_provider_still_allows_manual_component_summary, hash_embed unit tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S060 Offline operation

- specification: S060
- owner: unassigned
- dependencies: S003, S008, S011, S016, S056
- current status: planned
- implemented components: none
- remaining components: core offline paths, clear network failures
- tests: none
- known failures: none recorded
- benchmark status: not_run
- documentation status: missing
- integration status: none

## S061 Observability

- specification: S061
- owner: rune-telemetry
- dependencies: none
- current status: verification
- implemented components: structured tracing, secret-safe log export
- remaining components: richer operation metrics (index duration, render duration) in exported debug logs
- tests: exported_logs_redact_secrets
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S062 Evaluation framework

- specification: S062
- owner: rune-evals
- dependencies: S056
- current status: verification
- implemented components: named evals, `rune eval`, optional `RUNE_WRITE_BENCHMARKS=1` writes `docs/benchmarks/*.json`
- remaining components: larger evidence corpora; CI job that fails on metric regression
- tests: rune-evals unit tests; cargo test -p rune-evals
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial

## S063 Context compiler evaluation

- specification: S063
- owner: rune-evals
- dependencies: S062, S024
- current status: verification
- implemented components: evidence recall, stale/contradiction/duplicate rates, token cost, latency; `docs/benchmarks/compiler_s063.json`
- remaining components: larger question set with known supporting evidence beyond the synthetic store
- tests: compiler_s063
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial

## S064 Handoff evaluation

- specification: S064
- owner: rune-evals
- dependencies: S062, S022
- current status: verification
- implemented components: structured handoff vs raw transcript completeness; `docs/benchmarks/handoff_s064.json`
- remaining components: live two-agent transfer measurement
- tests: handoff_s064, handoff_completeness
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial

## S065 Memory evaluation

- specification: S065
- owner: rune-evals
- dependencies: S062, S012, S013
- current status: verification
- implemented components: extraction, inference rejection, staleness, conflict preservation; `docs/benchmarks/memory_s065.json`
- remaining components: scope isolation corpus at repository scale
- tests: memory_s065, memory_staleness
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial

## S066 Performance suite

- specification: S066
- owner: tests/performance
- dependencies: S079
- current status: active
- implemented components: repository generator and README for small/medium/large/very large tiers
- remaining components: measured runs on reference hardware; stored results in docs/benchmarks
- tests: tests/performance/gen.rs
- known failures: p95 targets not measured
- benchmark status: not_run
- documentation status: partial
- integration status: none

## S067 Responsiveness targets

- specification: S067
- owner: unassigned
- dependencies: S066, S042, S038
- current status: planned
- implemented components: none
- remaining components: p95 measurements and optimization if missed
- tests: none
- known failures: none recorded
- benchmark status: not_run
- documentation status: missing
- integration status: none

## S068 Failure recovery

- specification: S068
- owner: unassigned
- dependencies: S056, S057, S019
- current status: planned
- implemented components: none
- remaining components: listed failure modes, isolation of integration faults
- tests: none
- known failures: none recorded
- benchmark status: not_run
- documentation status: missing
- integration status: none

## S069 Compatibility matrix

- specification: S069
- owner: docs/compatibility
- dependencies: S002, S009
- current status: verification
- implemented components: OS/terminal/agent adapter matrix with honest planned status; rune-terminal capability probes exist
- remaining components: dated certification runs per terminal and OS; adapter rows remain planned until S009 lands
- tests: crates/core/tests/docs_contracts.rs::s069_compatibility_matrix_exists
- known failures: none recorded
- benchmark status: not_run
- documentation status: matches
- integration status: partial

## S070 Configuration

- specification: S070
- owner: rune-core
- dependencies: none
- current status: active
- implemented components: layered defaults/user/workspace/session configuration types
- remaining components: load all layers from disk in the CLI for every key in AGENTS.md S070
- tests: rune-core config tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S071 Themes

- specification: S071
- owner: rune-ui
- dependencies: S039, S070
- current status: verification
- implemented components: semantic token themes including high-contrast; ember default
- remaining components: user themes without recompile loaded from config files
- tests: rune-ui theme tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S072 Keybinding system

- specification: S072
- owner: rune-ui
- dependencies: S070
- current status: verification
- implemented components: semantic actions, configurable bindings, conflict detection
- remaining components: user keymap file in the layered config
- tests: rune-ui keybinding tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S073 Accessibility

- specification: S073
- owner: rune-ui
- dependencies: S039, S040
- current status: verification
- implemented components: reduced motion, high contrast, keyboard-only navigation, non-color status labels
- remaining components: screen-size adaptation polish
- tests: rune-ui accessibility/theme tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S074 Onboarding

- specification: S074
- owner: rune-cli
- dependencies: S001, S002, S035, S009
- current status: verification
- implemented components: `rune onboard` environment inspection; no account creation
- remaining components: first-run TUI wizard
- tests: onboard detection tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S075 Import

- specification: S075
- owner: rune-sessions
- dependencies: S009, S015, S016, S011, S008
- current status: active
- implemented components: session import with AgentSession provenance, raw blobs, `rune sessions import`, candidate memory extraction
- remaining components: spec/task/git/MCP config imports
- tests: extraction_from_fixture_creates_attempt_and_failure_linked_to_turns, session_import_extracts_candidate_agent_memory_not_guidance
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S076 Export

- specification: S076
- owner: rune-app
- dependencies: S032, S054
- current status: verification
- implemented components: JSON/JSONL/Markdown export with secrets redacted by default
- remaining components: graph format and context-pack export (S032)
- tests: export_redacts_akia_keys
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S077 CLI surface

- specification: S077
- owner: rune-cli
- dependencies: S001, S006, S078
- current status: verification
- implemented components: `rune index|search|graph|memory|sessions|tasks|specs|handoff|context|agents|doctor|onboard|tui|export|completions|eval|impact|package|crash|update`
- remaining components: machine-readable schema stability; sessions ingest command
- tests: index_workspace_creates_function_nodes, search_finds_indexed_function, doctor_returns_ok_on_memory_db, guidance_retriever_omits_stale_memory
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S078 Doctor command

- specification: S078
- owner: rune-cli
- dependencies: S056, S002, S033, S009
- current status: verification
- implemented components: database, migrations, terminal, providers, git, agents diagnostics with repair text
- remaining components: MCP/plugin/semantic provider repair paths
- tests: doctor_returns_ok_on_memory_db
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S079 Integration test repositories

- specification: S079
- owner: tests/fixtures/repositories
- dependencies: none
- current status: verification
- implemented components: rust, python, typescript, go, mixed monorepo, unicode path, malformed bytes fixtures; bootstrap.sh for nested git
- remaining components: large generated corpus remains a generator not a committed tree; index/search integration tests must consume these fixtures
- tests: crates/core/tests/docs_contracts.rs::s079_language_fixtures_exist
- known failures: none recorded
- benchmark status: not_run
- documentation status: matches
- integration status: partial

## S080 Cross subsystem integration tests

- specification: S080
- owner: rune-evals
- dependencies: S012, S013, S016, S019, S021, S024, S049
- current status: verification
- implemented components: `s080_full_cross_subsystem_lifecycle` — index, cross-file calls, session memory, freshness after edit, spec/task, compile with pins, worktree edit, test run, commit+ChangedBy, handoff transfer, coverage link
- remaining components: live agent subprocess in the chain; S049 TUI coverage view
- tests: crates/evals/tests/s080_lifecycle.rs
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: verified

## S081 Regression discipline

- specification: S081
- owner: tests
- dependencies: S079
- current status: verification
- implemented components: regression tests for cross-file calls, HTTPS URL parse, semantic ranking, S080 lifecycle
- remaining components: dedicated corpus covering every historical bug; policy that every future fix adds a test
- tests: cross_file_calls_are_resolved, docs_context HTTPS parse, search semantic routing, s080_full_cross_subsystem_lifecycle
- known failures: corpus is not exhaustive
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S082 Documentation

- specification: S082
- owner: docs/architecture
- dependencies: none
- current status: active
- implemented components: README, architecture set, specifications index, configuration notes, integrations
- remaining components: contributing/testing/benchmarking docs must match implemented behavior as crates land; many subsystem docs still describe specified architecture rather than shipped CLI
- tests: crates/core/tests/docs_contracts.rs
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S083 Architecture diagrams

- specification: S083
- owner: docs/architecture
- dependencies: S082
- current status: verification
- implemented components: mermaid diagrams for runtime, graph, compiler, memory, sessions, handoff, agent runtime, providers, storage, TUI
- remaining components: update diagrams when architecture changes; configuration.md has mermaid too
- tests: crates/core/tests/docs_contracts.rs::s083_architecture_diagrams_use_mermaid
- known failures: none recorded
- benchmark status: not_run
- documentation status: matches
- integration status: partial

## S084 Licensing review

- specification: S084
- owner: root coordinator
- dependencies: none
- current status: verification
- implemented components: Apache-2.0 LICENSE, NOTICE, per-target license notes in docs/integrations, clean-room rule for CASS rider and Caveman BSL
- remaining components: Cargo.lock third-party notice generation at release; ureq/rustls recorded in NOTICE; re-verify licenses at copied-code time (none copied)
- tests: crates/core/tests/docs_contracts.rs::s034_research_notes_cover_required_targets_and_sections
- known failures: none recorded
- benchmark status: not_run
- documentation status: matches
- integration status: partial

## S085 Packaging

- specification: S085
- owner: apps/rune
- dependencies: S077
- current status: verification
- implemented components: `scripts/package.sh` for host or rustc target; `rune package`; GitHub Actions matrix for macOS aarch64/x86_64 and Linux x86_64/arm64; sha256 sidecar
- remaining components: published tag artifacts; Windows target; signed releases; shell completion install in the package tarball
- tests: package.sh exists and is executable; CLI `package` subcommand
- known failures: multi-arch artifacts have not been published from a tag build
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S086 Updates

- specification: S086
- owner: rune-app
- dependencies: S057, S085
- current status: verification
- implemented components: `rune update` / `App::check_update` discovers versions without replacing the running binary; `refuse_replace_running_binary`
- remaining components: network latest-version fetch; post-update migration prompt
- tests: app unit coverage around refuse_replace_running_binary and crash_bundle
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S087 Crash reporting

- specification: S087
- owner: rune-app
- dependencies: S061, S054
- current status: verification
- implemented components: optional local crash bundle with version, platform, terminal, sanitized stack, provider disclosure, database diagnostics; `rune crash`; secrets redacted
- remaining components: automatic capture from panics in the TUI; log export beyond the bundle note
- tests: crash_bundle_redacts_secrets
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S088 Data ownership

- specification: S088
- owner: rune-providers
- dependencies: S054
- current status: verification
- implemented components: `Provider::data_leaving_machine` default None; Context7 returns an HTTPS disclosure; crash bundles include provider disclosure strings; local mode does not upload
- remaining components: per-command prompt before first network use in the TUI
- tests: docs_context network policy denied; crash_bundle_redacts_secrets
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S089 Deterministic state inspection

- specification: S089
- owner: rune-context-compiler
- dependencies: S024, S013, S016, S006, S017
- current status: verification
- implemented components: explain_why for capsule objects; freshness reasons; task blocked reasons; parallelization explanations
- remaining components: search ranking evidence in the TUI
- tests: compiler inspect tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S090 Context difference engine

- specification: S090
- owner: rune-context-compiler
- dependencies: S024
- current status: verification
- implemented components: compare_capsules added/removed/changed objects and token allocation
- remaining components: TUI diff view
- tests: capsule_diff_shows_added_and_removed
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S091 Agent knowledge comparison

- specification: S091
- owner: rune-context-compiler
- dependencies: S009, S024
- current status: verification
- implemented components: compare_knowledge over observable supplied context and sessions only
- remaining components: TUI comparison view
- tests: compiler knowledge comparison tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S092 Context provenance visualization

- specification: S092
- owner: rune-ui
- dependencies: S024, S044
- current status: active
- implemented components: inspector fields for source, reason, freshness, token cost
- remaining components: retrieval-path visualization beyond the reason string
- tests: rune-ui inspector render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S093 Context pinning

- specification: S093
- owner: rune-context-compiler
- dependencies: S024
- current status: verification
- implemented components: PinSet survives ranking; compile includes pins
- remaining components: stale/contradicted pin warnings in the TUI
- tests: pinned_item_is_included; S080 pins
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S094 Context exclusion

- specification: S094
- owner: rune-context-compiler
- dependencies: S024
- current status: verification
- implemented components: scoped exclusions; excluded items omitted
- remaining components: do not persist session exclusions as user preferences (enforced in types; TUI still needed)
- tests: excluded_item_is_omitted
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S095 Human decisions

- specification: S095
- owner: rune-memory
- dependencies: S011
- current status: verification
- implemented components: record_human_decision with Human authority; agents cannot auto-verify inferences
- remaining components: TUI decision inspector
- tests: conflicting_memories_both_preserved; human authority ranking
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S096 Conflict resolution

- specification: S096
- owner: rune-memory
- dependencies: S011, S095
- current status: verification
- implemented components: both claims preserved, contradicts edges, refuse overwrite
- remaining components: surface conflicts in context compiler warnings consistently
- tests: conflicting_memories_both_preserved
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S097 Entity resolution

- specification: S097
- owner: rune-memory
- dependencies: S003, S005, S011
- current status: verification
- implemented components: reversible symbol merges with snapshot restore
- remaining components: cross-kind linking (documents/tasks/sessions) beyond symbols
- tests: entity_merge_is_reversible
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S098 Background indexing

- specification: S098
- owner: rune-index
- dependencies: S003, S099
- current status: verification
- implemented components: IndexQueue with recently-touched priority and pause flag
- remaining components: CPU backoff while typing; app-level scheduler
- tests: recently_touched_sorts_first, pause_prevents_pop
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S099 File watching

- specification: S099
- owner: rune-index
- dependencies: S001, S003
- current status: verification
- implemented components: notify watcher config, ignore target/node_modules/.git/dist, rename-both as atomic save, coalesce
- remaining components: generated-file storm rate limiter in the app loop
- tests: ignores_target_and_node_modules, rename_both_is_atomic_save
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial

## S100 Graph integrity

- specification: S100
- owner: rune-storage
- dependencies: S056
- current status: verification
- implemented components: dangling edges, missing objects, duplicate identities, orphaned session refs, invalid task deps, invalid provenance, migration inconsistency; repair actions on findings
- remaining components: automatic repair CLI beyond doctor
- tests: storage integrity tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
