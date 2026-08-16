# Context7

**Research target:** current dependency documentation.

**Public project:** [upstash/context7](https://github.com/upstash/context7). Version-specific library documentation retrieval for LLMs via MCP, CLI (`ctx7`), and SDKs.

## Architecture (public knowledge)

- Resolve a library to a Context7 library ID (for example `/vercel/next.js`).
- Query docs for that ID with a natural-language question; return current snippets.
- MCP tools and CLI; OAuth/API key for hosted index.
- Skills that teach agents to fetch docs instead of training-cutoff knowledge.

## License

MIT as commonly published for the MCP/CLI repositories. The **hosted documentation index** is a network service, not fully local OSS data. Confirm LICENSE and Terms for the service.

## Reusable mechanisms

- ExternalDocument nodes: library, version, source, retrieval time, section, content, relevance (S030).
- Version-aware cache; invalidate when lockfile versions change (S031).
- Warn when mixing incompatible versions.
- Provider adapter: query/inspect only; no pretend write APIs.

## Limitations

- Network-dependent; must fail clearly offline (S060).
- Retrieved docs are untrusted content (prompt injection tests in S055).
- Hosted index licensing/ToS may restrict redistribution of snippets in exports (S076, S088).

## Integration options

1. **Preferred:** `rune-docs-context` provider wrapping Context7 (or equivalents) with provenance and version stamps.
2. Disabled by default until the user enables network documentation.
3. Do not bake Context7 API keys into Rune.

## Clean-room note

Do not copy MCP server source. Implement a documentation provider against public HTTP/MCP contracts if the user enables it. Cache locally under content hashes.
