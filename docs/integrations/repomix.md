# Repomix

**Research target:** portable context packaging.

**Public project:** [yamadashy/repomix](https://github.com/yamadashy/repomix). Packs a repository into AI-oriented single-file formats (XML, Markdown, JSON, plain text), with gitignore awareness, optional Tree-sitter compression, and secret scanning (Secretlint).

## Architecture (public knowledge)

- Walk the repo respecting ignore rules.
- Emit a packed artifact plus structure/report.
- `--compress` keeps signatures/structure while dropping bodies where configured.
- MCP server for pack/remote-pack style tools.
- Security checks intended to reduce secret leakage in packs.

## License

MIT as published (Copyright 2024 Kazuki Yamada; LICENSE fetched 2026-08-15). Confirm at the version inspected.

## Reusable mechanisms

- Portable packs with a manifest (S032): repository, task, review, bug, handoff, architecture, custom.
- Inspect included content before export.
- Default: do not include secrets (S076, S054).
- Ignore rules aligned with git and extra denylists.

## Limitations

- A single packed file is a lossy export, not the live graph.
- Remote clone-and-pack is a network feature and a supply-chain risk.
- Compression that drops bodies can remove the evidence the compiler needs; packing is not a substitute for S024.

## Integration options

1. **Preferred:** `rune` export of context packs from capsules and graph selections, with manifests.
2. Optional Repomix CLI provider if installed.
3. Redact using Rune’s secret detector before any pack leaves the machine.

## Clean-room note

Do not copy Repomix TypeScript packing code or XML templates. Reimplement pack generation from Rune’s blob/graph export APIs.
