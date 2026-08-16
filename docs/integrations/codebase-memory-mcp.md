# Codebase Memory MCP

**Research target:** structural relationship graph.

**Public project:** [DeusData/codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp) — MCP server that indexes a codebase into a persistent knowledge graph (functions, classes, call chains, routes, cross-service links). Local processing; no embedded LLM required for the graph itself.

## Architecture (public knowledge)

- Tree-sitter syntactic pass across a large language set.
- Optional “Hybrid LSP” type-resolution layer for selected languages, embedded rather than spawning language servers.
- Graph query tools (search, path trace, architecture, snippets, coverage).
- Incremental watcher; optional compressed graph snapshot for teammates.
- Optional on-device embeddings for semantic search (project documents nomic-embed-code in-binary).
- MCP client (the coding agent) is the natural-language layer; the server serves the graph.

## License

MIT as commonly published. Confirm LICENSE at the version inspected.

## Reusable mechanisms

- Graph-first answers to structural questions instead of file-by-file grep.
- Call/import/route edges with confidence.
- Incremental reindex on watch events.
- Coverage checks so agents do not cite unindexed files as complete.
- Change-impact mapping from a git diff to affected symbols.

## Limitations

- Hybrid type resolution is a large independent engineering effort; Rune should not claim IDE-grade resolution until tests prove it.
- Shipping embeddings inside the binary has size, license, and update implications.
- MCP is a transport, not the system of record. Rune’s SQLite graph is.

## Integration options

1. **Preferred:** implement structural indexing in `rune-index` + `rune-graph` (Tree-sitter, incremental, SQLite).
2. Optional MCP *client* that can query an installed codebase-memory-mcp instance as a provider with declared capabilities.
3. Do not vendor the C/native binary or copy its parsers.

## Clean-room note

Do not copy source, grammar bundles, or tool schemas verbatim. Reimplement graph query concepts against Rune’s canonical model. MCP JSON-RPC shapes may follow the public MCP specification, not this project’s private tool list.
