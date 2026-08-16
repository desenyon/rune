# Serena

**Research target:** symbol-centered navigation and operations.

**Public project:** [oraios/serena](https://github.com/oraios/serena) — MCP toolkit that exposes IDE-like symbol tools (find symbol, references, overview, replace symbol body) backed by language servers or an optional JetBrains plugin.

## Architecture (public knowledge)

- MCP server in front of LSP (and optionally JetBrains).
- Retrieval tools: find symbol by name path, file outline, referencing symbols, implementations/declarations where the backend supports them.
- Editing tools: replace symbol body, insert before/after symbol — token-efficient compared with line-number patches.
- Language coverage depends on installed language servers.
- Optional paid JetBrains backend for richer type hierarchy and dependency search.

## License

MIT as commonly published for the core (Oraios AI). Confirm LICENSE. The JetBrains plugin backend is a separate, paid product and must not be treated as OSS.

## Reusable mechanisms

- Symbol as the primary navigation unit (S007): definition, references, callers, callees, implementations, tests.
- Scoped edits by symbol identity rather than brittle line patches.
- Capability matrix per backend: hide operations the language server cannot provide (DEC-008).

## Limitations

- LSP quality varies by language and project setup.
- Serena is an MCP sidecar, not a persistent project graph with memory, tasks, or sessions.
- Editing tools are out of scope for Rune’s default local-first inspector until the agent runtime owns write policy.

## Integration options

1. **Preferred:** symbol intelligence in `rune-index` / TUI, querying the canonical graph.
2. Optional Serena MCP provider for users who already run it; results ingested with provenance `mcp`, never as verified memory.
3. Do not spawn language servers unless the user enables that provider.

## Clean-room note

Do not copy Serena Python/MCP source or tool implementations. Reimplement symbol-centered workflows against Tree-sitter (and later optional LSP) inside Rune.
