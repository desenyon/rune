# OSS research targets (S034)

These notes inspect public architecture and commonly published licenses. They identify reusable **mechanisms**, not source to copy.

**Clean-room rule:** do not copy source without license compatibility review (S084). Prefer independent reimplementation of concepts. Conceptual inspiration does not require source copying. When uncertain, implement the mechanism independently.

Licenses below are as commonly published on public repositories as of 2026-08-15. Confirm the LICENSE file at the version you inspect before any reuse.

| Target | File | Commonly published license |
| --- | --- | --- |
| Graft | [graft.md](graft.md) | MIT (NanoNets/Graft) |
| Codebase Memory MCP | [codebase-memory-mcp.md](codebase-memory-mcp.md) | MIT |
| Serena | [serena.md](serena.md) | MIT |
| Probe | [probe.md](probe.md) | Apache-2.0 |
| Graphify | [graphify.md](graphify.md) | Apache-2.0 |
| Rekal | [rekal.md](rekal.md) | Apache-2.0 |
| CASS | [cass.md](cass.md) | MIT + OpenAI/Anthropic rider |
| CASS Memory | [cass-memory.md](cass-memory.md) | MIT + OpenAI/Anthropic rider |
| Catchup | [catchup.md](catchup.md) | MIT |
| Git Context Controller | [git-context-controller.md](git-context-controller.md) | MIT |
| Beads | [beads.md](beads.md) | MIT |
| OpenSpec | [openspec.md](openspec.md) | MIT |
| Spec Kit | [spec-kit.md](spec-kit.md) | MIT |
| Context7 | [context7.md](context7.md) | MIT |
| RTK | [rtk.md](rtk.md) | Apache-2.0 |
| Caveman | [caveman.md](caveman.md) | Split MIT / BSL-1.1 |
| Repomix | [repomix.md](repomix.md) | MIT |
| Herdr | [herdr.md](herdr.md) | Apache-2.0 |

CASS and CASS Memory riders restrict OpenAI and Anthropic entities. Rune must not copy that source. Independent implementation of the *ideas* (cross-agent session search, procedural playbooks) is the specified path.
