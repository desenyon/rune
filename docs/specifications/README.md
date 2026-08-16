# Specifications S001–S100

Index of required specifications with crate mapping. Status lives in [`docs/BUILD_STATE.md`](../BUILD_STATE.md) and is **not** complete. Crate directories follow `crates/<name>`; package names are `rune-*` (DEC-001). Implementation status lives in BUILD_STATE, not in this crate map.

The CLI is `rune`.

| Spec | Title | Primary crate(s) |
| --- | --- | --- |
| S001 | Workspace discovery | `rune-app`, `rune-git-intelligence`, `rune-core` |
| S002 | Terminal capability engine | `rune-terminal` |
| S003 | Structural code index | `rune-index` |
| S004 | Semantic repository graph | `rune-semantic`, `rune-graph` |
| S005 | Multimodal knowledge graph | `rune-index`, `rune-graph` |
| S006 | Search engine | `rune-search` |
| S007 | Symbol intelligence | `rune-index`, `rune-graph`, `rune-ui` |
| S008 | Git temporal intelligence | `rune-git-intelligence` |
| S009 | Agent session ingestion | `rune-sessions`, `rune-providers` |
| S010 | Session intelligence | `rune-sessions` |
| S011 | Persistent memory system | `rune-memory` |
| S012 | Memory extraction | `rune-memory` |
| S013 | Memory freshness engine | `rune-memory`, `rune-index` |
| S014 | Historical reasoning graph | `rune-history` |
| S015 | Specification system | `rune-specs` |
| S016 | Task dependency graph | `rune-tasks` |
| S017 | Parallelization analysis | `rune-tasks`, `rune-graph` |
| S018 | Worktree orchestration | `rune-worktrees` |
| S019 | Agent runtime | `rune-agent-runtime` |
| S020 | Agent event normalization | `rune-agent-runtime` |
| S021 | Cross agent handoff | `rune-handoff` |
| S022 | Handoff compiler | `rune-handoff`, `rune-context-compiler` |
| S023 | Branchable context | `rune-context-compiler`, `rune-history` |
| S024 | Context compiler | `rune-context-compiler` |
| S025 | Retrieval scoring | `rune-context-compiler` |
| S026 | Token budget allocator | `rune-context-compiler` |
| S027 | Context deduplication | `rune-context-compiler` |
| S028 | Adaptive compression | `rune-compression` |
| S029 | Agent communication policy | `rune-agent-runtime`, `rune-ui` |
| S030 | External documentation context | `rune-docs-context` |
| S031 | Documentation freshness | `rune-docs-context` |
| S032 | Portable context packs | `rune-cli`, `rune-context-compiler` |
| S033 | OSS provider framework | `rune-providers` |
| S034 | Required OSS research targets | `docs/integrations/` (documentation) |
| S035 | Command line tool adapters | `rune-tools` |
| S036 | MCP interoperability | `rune-mcp` |
| S037 | Plugin system | `rune-plugins` |
| S038 | Universal command palette | `rune-ui`, `rune-search` |
| S039 | TUI design system | `rune-ui` |
| S040 | Motion engine | `rune-motion` |
| S041 | Shared element transitions | `rune-motion` |
| S042 | Adaptive rendering | `rune-ui`, `rune-terminal` |
| S043 | Graph explorer | `rune-ui`, `rune-graph` |
| S044 | Context inspector | `rune-ui`, `rune-context-compiler` |
| S045 | Memory timeline | `rune-ui`, `rune-memory` |
| S046 | Session explorer | `rune-ui`, `rune-sessions` |
| S047 | Agent cockpit | `rune-ui`, `rune-agent-runtime` |
| S048 | Task graph view | `rune-ui`, `rune-tasks` |
| S049 | Specification coverage view | `rune-ui`, `rune-specs` |
| S050 | Diff intelligence | `rune-index`, `rune-git-intelligence` |
| S051 | Test intelligence | `rune-index` |
| S052 | Process awareness | `rune-app`, `rune-tools` |
| S053 | Environment awareness | `rune-security` |
| S054 | Security model | `rune-security` |
| S055 | Prompt injection resistance | `rune-security`, `tests/fixtures/security/` |
| S056 | Storage | `rune-storage` |
| S057 | Database migrations | `rune-storage` |
| S058 | Content addressed cache | `rune-storage` |
| S059 | Semantic provider abstraction | `rune-semantic`, `rune-providers` |
| S060 | Offline operation | `rune-app`, `rune-providers` |
| S061 | Observability | `rune-telemetry` |
| S062 | Evaluation framework | `rune-evals` |
| S063 | Context compiler evaluation | `rune-evals` |
| S064 | Handoff evaluation | `rune-evals` |
| S065 | Memory evaluation | `rune-evals` |
| S066 | Performance suite | `rune-evals`, `tests/performance/` |
| S067 | Responsiveness targets | `rune-ui`, `rune-evals` |
| S068 | Failure recovery | `rune-storage`, `rune-app` |
| S069 | Compatibility matrix | `docs/compatibility/` |
| S070 | Configuration | `rune-core` (layered types) |
| S071 | Themes | `rune-ui` |
| S072 | Keybinding system | `rune-ui` |
| S073 | Accessibility | `rune-ui`, `rune-motion` |
| S074 | Onboarding | `rune-app`, `rune-cli` |
| S075 | Import | `rune-cli`, `rune-sessions` |
| S076 | Export | `rune-cli` |
| S077 | CLI surface | `rune-cli`, `apps/rune` |
| S078 | Doctor command | `rune-cli` |
| S079 | Integration test repositories | `tests/fixtures/repositories/` |
| S080 | Cross subsystem integration tests | `rune-evals` |
| S081 | Regression discipline | `tests/`, crate unit tests |
| S082 | Documentation | `docs/` |
| S083 | Architecture diagrams | `docs/architecture/` |
| S084 | Licensing review | `LICENSE`, `NOTICE`, `docs/integrations/` |
| S085 | Packaging | `scripts/package.sh`, `.github/workflows/release.yml`, `apps/rune` |
| S086 | Updates | `rune-app`, `rune` CLI |
| S087 | Crash reporting | `rune-app`, `rune-telemetry` |
| S088 | Data ownership | `rune-security` |
| S089 | Deterministic state inspection | `rune-context-compiler`, `rune-search` |
| S090 | Context difference engine | `rune-context-compiler` |
| S091 | Agent knowledge comparison | `rune-sessions`, `rune-context-compiler` |
| S092 | Context provenance visualization | `rune-ui` |
| S093 | Context pinning | `rune-context-compiler` |
| S094 | Context exclusion | `rune-context-compiler` |
| S095 | Human decisions | `rune-memory` |
| S096 | Conflict resolution | `rune-memory`, `rune-graph` |
| S097 | Entity resolution | `rune-graph`, `rune-semantic` |
| S098 | Background indexing | `rune-index` |
| S099 | File watching | `rune-index` |
| S100 | Graph integrity | `rune-storage`, `rune-graph` |

## Workspace members today

All `rune-*` crates listed in the root `Cargo.toml` workspace, plus `apps/rune`.
