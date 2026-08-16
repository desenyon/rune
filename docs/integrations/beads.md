# Beads

**Research target:** dependency-aware tasks.

**Public project:** [steveyegge/beads](https://github.com/steveyegge/beads) / [gastownhall/beads](https://github.com/gastownhall/beads) (`bd`). Git-friendly, agent-oriented issue tracker with a dependency graph and a ready-work queue.

## Architecture (public knowledge)

- Issues with typed dependencies: `blocks`, parent-child, `discovered-from`, related.
- `bd ready` surfaces work with no blockers; claim/close cycle.
- Hash-based IDs to reduce collisions across agents and branches.
- JSON output on CLI for agents.
- Storage has evolved (git-backed files historically; Dolt-powered SQL with cell-level merge is widely documented). Confirm current storage before any interoperability work.
- Optional MCP / editor plugins.

## License

MIT as commonly published. Confirm LICENSE at the version inspected.

## Reusable mechanisms

- Persistent task graph with blockers and ready-work computation (S016).
- `discovered-from` as a first-class edge when an agent finds new work mid-task.
- Cycle detection and actionable-set calculation.
- Hash/UUID identifiers that survive branch merges (Rune already chose UUID v7).
- Machine-readable CLI (`rune tasks` specified).

## Limitations

- Dolt (or any extra database engine) is not Rune’s store; SQLite is unless benchmarks say otherwise.
- Beads is a task tracker, not a context compiler or symbol index.
- Auto-sync git hooks must not be installed without user approval.

## Integration options

1. **Preferred:** `rune-tasks` dependency graph in the canonical store.
2. Optional import/export of Beads JSON as Task nodes with provenance.
3. Do not embed Dolt.

## Clean-room note

Do not copy Beads Go source or schema. Reimplement dependency-aware tasks on Rune nodes/edges.
