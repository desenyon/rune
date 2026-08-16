# Graphify

**Research target:** multimodal knowledge linking.

**Public project:** [Graphify-Labs/graphify](https://github.com/Graphify-Labs/graphify) (also associated with safishamsi/graphify). Parses code, docs, schemas, and media into a queryable knowledge graph. Code maps are local Tree-sitter; docs/PDFs/images may use a model.

## Architecture (public knowledge)

- Deterministic AST extraction for code; semantic pass for non-code when a model is available.
- Edges tagged EXTRACTED vs INFERRED (honesty about provenance).
- Query / path / explain CLI and slash-command skill for many coding agents.
- Token budget on query results.
- Not a vector index as the primary model: traverse a graph.
- Assistant installers write hooks or AGENTS.md so tools prefer the graph over grep.

## License

Apache License 2.0 as commonly published. Confirm LICENSE at the version inspected.

## Reusable mechanisms

- Multimodal nodes that preserve document structure (S005): markdown, schemas, config, images with metadata, generated docs.
- Explicit EXTRACTED vs INFERRED on every edge (aligns with Rune provenance and DEC-007).
- Path and explain APIs with a token budget.
- Community detection / clustering as an optional analysis, not as identity.

## Limitations

- Model-backed media extraction is network- and cost-sensitive; must fail clearly offline.
- Hooking every agent to “query the graph first” is a policy Rune should own, not an instruction file dumped into user repos by default.
- GitHub star counts are not evidence of correctness.

## Integration options

1. **Preferred:** multimodal indexer in `rune-docs-context` + canonical graph, with provenance tags.
2. Optional import of a Graphify `graph.json` export as untrusted external documents.
3. Do not copy skill.md instruction text into Rune’s privileged prompts.

## Clean-room note

Do not copy Graphify source, skills, or installer hooks. Reimplement multimodal linking and EXTRACTED/INFERRED labeling independently.
