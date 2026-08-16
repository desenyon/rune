# Specifications S001–S100

Index of required specifications with crate mapping. Status lives in [`docs/BUILD_STATE.md`](../BUILD_STATE.md) and is **not** complete. Crate directories follow `crates/<name>`; package names are `rune-*` (DEC-001). Crates marked *planned* are not workspace members yet.

The CLI is `rune`.

| Spec | Title | Primary crate(s) |
| --- | --- | --- |
| S001 | Workspace discovery | `rune-app` (planned), `rune-git-intelligence` (planned), `rune-core` |
| S002 | Terminal capability engine | `rune-terminal` |
| S003 | Structural code index | `rune-index` (planned) |
| S004 | Semantic repository graph | `rune-semantic` (planned), `rune-graph` |
| S005 | Multimodal knowledge graph | `rune-docs-context` (planned), `rune-graph` |
| S006 | Search engine | `rune-search` (planned) |
| S007 | Symbol intelligence | `rune-index` (planned), `rune-graph`, `rune-ui` (planned) |
| S008 | Git temporal intelligence | `rune-git-intelligence` (planned) |
| S009 | Agent session ingestion | `rune-sessions` (planned), `rune-providers` |
| S010 | Session intelligence | `rune-sessions` (planned) |
| S011 | Persistent memory system | `rune-memory` (planned) |
| S012 | Memory extraction | `rune-memory` (planned) |
| S013 | Memory freshness engine | `rune-memory` (planned), `rune-index` (planned) |
| S014 | Historical reasoning graph | `rune-history` (planned) |
| S015 | Specification system | `rune-specs` (planned) |
| S016 | Task dependency graph | `rune-tasks` (planned) |
| S017 | Parallelization analysis | `rune-tasks` (planned), `rune-graph` |
| S018 | Worktree orchestration | `rune-worktrees` (planned) |
| S019 | Agent runtime | `rune-agent-runtime` (planned) |
| S020 | Agent event normalization | `rune-agent-runtime` (planned) |
| S021 | Cross agent handoff | `rune-handoff` (planned) |
| S022 | Handoff compiler | `rune-handoff` (planned), `rune-context-compiler` (planned) |
| S023 | Branchable context | `rune-context-compiler` (planned), `rune-history` (planned) |
| S024 | Context compiler | `rune-context-compiler` (planned) |
| S025 | Retrieval scoring | `rune-context-compiler` (planned) |
| S026 | Token budget allocator | `rune-context-compiler` (planned) |
| S027 | Context deduplication | `rune-context-compiler` (planned) |
| S028 | Adaptive compression | `rune-compression` (planned) |
| S029 | Agent communication policy | `rune-agent-runtime` (planned), `rune-ui` (planned) |
| S030 | External documentation context | `rune-docs-context` (planned) |
| S031 | Documentation freshness | `rune-docs-context` (planned) |
| S032 | Portable context packs | `rune-cli` (planned), `rune-context-compiler` (planned) |
| S033 | OSS provider framework | `rune-providers` |
| S034 | Required OSS research targets | `docs/integrations/` (documentation) |
| S035 | Command line tool adapters | `rune-tools` (planned) |
| S036 | MCP interoperability | `rune-mcp` (planned) |
| S037 | Plugin system | `rune-plugins` (planned) |
| S038 | Universal command palette | `rune-ui` (planned), `rune-search` (planned) |
| S039 | TUI design system | `rune-ui` (planned) |
| S040 | Motion engine | `rune-motion` (planned) |
| S041 | Shared element transitions | `rune-motion` (planned) |
| S042 | Adaptive rendering | `rune-ui` (planned), `rune-terminal` |
| S043 | Graph explorer | `rune-ui` (planned), `rune-graph` |
| S044 | Context inspector | `rune-ui` (planned), `rune-context-compiler` (planned) |
| S045 | Memory timeline | `rune-ui` (planned), `rune-memory` (planned) |
| S046 | Session explorer | `rune-ui` (planned), `rune-sessions` (planned) |
| S047 | Agent cockpit | `rune-ui` (planned), `rune-agent-runtime` (planned) |
| S048 | Task graph view | `rune-ui` (planned), `rune-tasks` (planned) |
| S049 | Specification coverage view | `rune-ui` (planned), `rune-specs` (planned) |
| S050 | Diff intelligence | `rune-index` (planned), `rune-git-intelligence` (planned) |
| S051 | Test intelligence | `rune-index` (planned) |
| S052 | Process awareness | `rune-app` (planned), `rune-tools` (planned) |
| S053 | Environment awareness | `rune-security` |
| S054 | Security model | `rune-security` |
| S055 | Prompt injection resistance | `rune-security`, `tests/fixtures/security/` |
| S056 | Storage | `rune-storage` |
| S057 | Database migrations | `rune-storage` |
| S058 | Content addressed cache | `rune-storage` |
| S059 | Semantic provider abstraction | `rune-semantic` (planned), `rune-providers` |
| S060 | Offline operation | `rune-app` (planned), `rune-providers` |
| S061 | Observability | `rune-telemetry` |
| S062 | Evaluation framework | `rune-evals` (planned) |
| S063 | Context compiler evaluation | `rune-evals` (planned) |
| S064 | Handoff evaluation | `rune-evals` (planned) |
| S065 | Memory evaluation | `rune-evals` (planned) |
| S066 | Performance suite | `rune-evals` (planned), `tests/performance/` |
| S067 | Responsiveness targets | `rune-ui` (planned), `rune-evals` (planned) |
| S068 | Failure recovery | `rune-storage`, `rune-app` (planned) |
| S069 | Compatibility matrix | `docs/compatibility/` |
| S070 | Configuration | `rune-core` (types), loaders planned |
| S071 | Themes | `rune-ui` (planned) |
| S072 | Keybinding system | `rune-ui` (planned) |
| S073 | Accessibility | `rune-ui` (planned), `rune-motion` (planned) |
| S074 | Onboarding | `rune-app` (planned), `rune-cli` (planned) |
| S075 | Import | `rune-cli` (planned), `rune-sessions` (planned) |
| S076 | Export | `rune-cli` (planned) |
| S077 | CLI surface | `rune-cli` (planned), `apps/rune` (planned) |
| S078 | Doctor command | `rune-cli` (planned) |
| S079 | Integration test repositories | `tests/fixtures/repositories/` |
| S080 | Cross subsystem integration tests | `tests/` (planned) |
| S081 | Regression discipline | `tests/` (planned) |
| S082 | Documentation | `docs/` |
| S083 | Architecture diagrams | `docs/architecture/` |
| S084 | Licensing review | `LICENSE`, `NOTICE`, `docs/integrations/` |
| S085 | Packaging | planned (release engineering) |
| S086 | Updates | planned |
| S087 | Crash reporting | `rune-telemetry` |
| S088 | Data ownership | `rune-security` |
| S089 | Deterministic state inspection | `rune-context-compiler` (planned), `rune-search` (planned) |
| S090 | Context difference engine | `rune-context-compiler` (planned) |
| S091 | Agent knowledge comparison | `rune-sessions` (planned), `rune-context-compiler` (planned) |
| S092 | Context provenance visualization | `rune-ui` (planned) |
| S093 | Context pinning | `rune-context-compiler` (planned) |
| S094 | Context exclusion | `rune-context-compiler` (planned) |
| S095 | Human decisions | `rune-memory` (planned) |
| S096 | Conflict resolution | `rune-memory` (planned), `rune-graph` |
| S097 | Entity resolution | `rune-graph`, `rune-semantic` (planned) |
| S098 | Background indexing | `rune-index` (planned) |
| S099 | File watching | `rune-index` (planned) |
| S100 | Graph integrity | `rune-storage`, `rune-graph` |

## Workspace members today

`rune-core`, `rune-storage`, `rune-graph`, `rune-security`, `rune-telemetry`, `rune-providers`, `rune-terminal`.
