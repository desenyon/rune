# Probe

**Research target:** fast structural and semantic search concepts.

**Public project:** [probelabs/probe](https://github.com/probelabs/probe) (also associated historically with buger/probe). Code search engine with CLI, MCP tools, and an optional Probe Agent.

## Architecture (public knowledge)

- `search`: Elasticsearch-style queries over code.
- `query`: AST / Tree-sitter structural pattern matching.
- `extract`: extract code blocks by line or symbol.
- `symbols`: list symbols in a file.
- Surfaces: raw MCP tools, a higher-level Probe Agent MCP, CLI, Node SDK.
- Intended to stop AI editors from reading entire files to answer structural questions.

## License

Apache License 2.0 as commonly published on probelabs/probe. Older npm wrappers have been published under other identifiers (for example ISC on a Node wrapper); confirm the repository LICENSE for the version inspected. Do not mix wrapper licenses with the core.

## Reusable mechanisms

- Search router with explicit modes: exact, fuzzy, FTS, structural, semantic (S006).
- AST query distinct from regex grep.
- Extract-by-symbol to bound tokens.
- Machine-readable CLI for agents.

## Limitations

- Semantic search quality depends on optional model configuration.
- A search engine without a persistent canonical graph cannot answer task/memory/session questions.
- Agent-mode MCP that piggybacks on another product’s auth is not a Rune runtime.

## Integration options

1. **Preferred:** Nucleo (interactive fuzzy) + SQLite FTS5 + structural graph queries inside `rune-search`.
2. Optional Probe CLI/MCP provider when installed (`which probe`), capability-declared.
3. Do not embed Probe’s agent or auth bridging.

## Clean-room note

Do not copy Probe query parsers or MCP tool implementations. Reimplement search modes on Rune indexes. Elasticsearch-like query syntax, if offered, must be independently specified.
