# OpenSpec

**Research target:** persistent intent and requirements.

**Public project:** [Fission-AI/OpenSpec](https://github.com/Fission-AI/openspec). Spec-driven development: specifications and change proposals live in the repo (`openspec/specs`, `openspec/changes`) and are consumed by coding agents.

## Architecture (public knowledge)

- Lightweight SDD: agree on specs before code.
- Repo-shaped artifacts rather than a hosted service; no API key required for the core CLI.
- Change proposals with review/apply workflow.
- Optional community MCP servers (separate packages, not necessarily core).
- Positioned as lighter and more iterative than Spec Kit’s phase gates.

## License

MIT as commonly published. Confirm LICENSE. Optional MCP dashboards are separate packages with their own licenses.

## Reusable mechanisms

- Specifications as first-class graph objects with addressable requirements (S015).
- Problem / current / desired behavior / constraints / acceptance criteria fields.
- Change proposals linked to specs and later to implementing tasks and commits.
- Coverage: requirements without implementation evidence (S049).

## Limitations

- Markdown specs without graph identity make requirement-level coverage hard.
- Optional MCP servers must not be assumed to be the core CLI.
- Importing OpenSpec files must retain provenance and not auto-verify memories.

## Integration options

1. **Preferred:** `rune-specs` with Requirement nodes and `implements_spec` / `satisfies_requirement` edges.
2. Import `openspec/` trees as Specification/Requirement nodes.
3. Export Rune specs to Markdown for agents that only read files.

## Clean-room note

Do not copy OpenSpec CLI source or prompt packs. Reimplement structured spec objects and coverage independently. Markdown field names may be mapped, not cloned as a copyrighted template pack.
