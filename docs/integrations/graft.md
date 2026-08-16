# Graft

**Research target:** semantic component descriptions (AGENTS.md §111).

**Primary public project:** [NanoNets/Graft](https://github.com/nanonets/graft) — indexes a repository into linked markdown nodes with summaries, crux excerpts, and typed links.

**Adjacent project (different product):** [flyingrobots/graft](https://github.com/flyingrobots/graft) is a context governor for structurally correct reads (Tree-sitter outlines, read policy). It is not the semantic-node target. Apache-2.0 as commonly published.

## Architecture (public knowledge)

NanoNets Graft builds understanding once and writes it into the repo as markdown nodes (one node per system, API, or concept) plus a symbol wiring graph.

- Tier 1: Tree-sitter pass — functions, classes, call edges; deterministic; no model required.
- Optional `--deep` pass: one-line summary and crux excerpt per symbol, cached by body hash.
- Node contents commonly include: summary, crux (the few lines that carry the logic), source files with content hashes for staleness, typed links (`depends_on`, `part_of`, `uses`, `implements`, `produces`) as wikilinks, and human notes below generated sections.
- CLI surfaces such as `graft grep` / `graft map` / `graft viz` for orientation.

## License

MIT as commonly published on NanoNets/Graft. Confirm LICENSE at the version inspected. flyingrobots/graft is commonly Apache-2.0.

## Reusable mechanisms

- Semantic node shape: purpose, responsibilities, important behavior, dependencies, dependents, constraints, risk, related tests — stored as graph objects, not only markdown files.
- Content-hash invalidation of summaries when source bodies change.
- Typed links between components.
- Structural pass that works without a language model (aligns with DEC-005).

## Limitations

- Markdown-in-repo graphs can drift from the canonical database unless Rune owns the system of record.
- Deep summaries require a model; they must be provenance-tagged as derived.
- A folder of markdown is not a substitute for symbol indexing (S004).

## Integration options

1. **Preferred:** clean-room semantic nodes in the canonical graph (`rune-semantic`), invalidated by structural fingerprints.
2. Optional import of an existing Graft markdown tree as `Document` nodes with provenance `external`, never as verified memory.
3. Do not shell out to Graft as a hidden production dependency.

## Clean-room note

Do not copy Graft source, generated markdown templates, or wiring JSON schemas. Reimplement the *idea* of concise semantic component cards linked to structural truth.
