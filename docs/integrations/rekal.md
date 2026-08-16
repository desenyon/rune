# Rekal

**Research target:** Git-anchored historical reasoning.

**Public project:** [rekal-dev/rekal-cli](https://github.com/rekal-dev/rekal-cli). Git-anchored intent/conversation ledger. Checkpoints AI session context at commit time; search reconstructs why code looks the way it does.

## Architecture (public knowledge)

- `rekal init` installs git hooks and local `.rekal/` state.
- Post-commit hook snapshots the active AI session into an append-only local database (turns, tool calls, files touched) **without storing code diffs** — git already has the code.
- Push/pull via an orphan git branch so teams share intent history without cluttering primary history.
- Hybrid search (lexical + semantic, as documented by the project) returning scored snippets with turn indexes.
- Query windows: human-only turns (cheap), session window, full dump (rare).
- Public paper: git-bound memory, commit–session links as ground truth, routed answer assembly rather than dumping episodes.

## License

Apache License 2.0 as published in the project LICENSE file fetched 2026-08-15. Confirm at the version inspected.

## Reusable mechanisms

- Bind sessions, decisions, and attempts to commit SHAs (S008, S014).
- Reconstruct “why” from commit–session links rather than grep of raw transcripts.
- Keep code in git; keep intent slim and reconstructible.
- Route historical questions: structural map vs gated episodes vs decision synthesis.
- Do not inject ungated episode dumps (the paper reports that degrades answers).

## Limitations

- Capture depends on hooks and on the agent session being discoverable at commit time.
- Orphan-branch distribution is one option; Rune’s system of record is SQLite, with Git as temporal backbone.
- Semantic search in Rekal is their stack; Rune’s semantic mode is pluggable and optional.

## Integration options

1. **Preferred:** `rune-history` + `rune-git-intelligence` linking Session/Decision/Attempt nodes to Commit nodes.
2. Optional import of a Rekal ledger as provenance-tagged history.
3. Do not install Rekal hooks into user repos as a side effect of running Rune.

## Clean-room note

Do not copy Rekal CLI source, wire format, or hooks. Reimplement Git-anchored reasoning on Rune’s graph.
