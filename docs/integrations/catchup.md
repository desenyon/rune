# Catchup

**Research target:** cross-agent handoff.

**Public project:** [wilbeibi/catchup](https://github.com/wilbeibi/catchup). Local-first CLI that turns agent session history into handoff-ready Markdown and can `fork` work into the same or another agent.

## Architecture (public knowledge)

- Read local sessions from Claude Code, Codex, OpenCode, Pi Agent, and related tools (some forks list Cursor/Cline/Kimi/Antigravity).
- Produce a cleaned Markdown context: conversation-focused; commonly strips tool calls, command output, and reasoning traces.
- `catchup fork` continues with the same agent (native resume when possible) or `--into` another agent (transcript seed, not native state).
- Search/list sessions by keyword and recency.
- Read-only except `fork`. Does not merge histories from two agents.

## License

MIT as published in the project LICENSE file fetched 2026-08-15 (Copyright 2026 wilbeibi). Confirm at the version inspected. Related forks may differ; pin the canonical repo.

## Reusable mechanisms

- Handoff as an explicit object, not “paste the whole chat” (S021).
- Same-agent resume vs cross-agent seed are different capabilities — declare them (DEC-008).
- Human-readable package the user can inspect before transfer (S022).
- Conversation-only compact mode as one compiler mode, not the only mode (Rune also needs files, tests, failures).

## Limitations

- Stripping tool output discards failed commands and test evidence that Rune’s compiler should keep when relevant.
- One agent at a time; no lineage graph.
- Markdown is a transport, not the system of record.

## Integration options

1. **Preferred:** `rune-handoff` compiler producing a graph object plus optional Markdown/JSON export.
2. Optional Catchup CLI provider if installed.
3. Do not treat Catchup’s stripped transcript as equivalent to a full Context Capsule.

## Clean-room note

Do not copy Catchup source or prompt templates used to seed target agents. Reimplement handoff compilation on Rune retrieval.
