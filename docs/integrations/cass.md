# CASS

**Research target:** cross-agent session discovery and search.

**Public project:** [Dicklesworthstone/coding_agent_session_search](https://github.com/Dicklesworthstone/coding_agent_session_search) (`cass`). Unified CLI/TUI that indexes local coding-agent session histories across many providers.

## Architecture (public knowledge)

- Per-provider connectors (Claude Code JSONL, Codex rollouts, Cursor SQLite, Aider markdown, Gemini, OpenCode, Copilot, and others). Canonical inventory via `cass capabilities --json`.
- Normalized model: Conversation → Message → Snippet.
- Full-text and optional semantic search; robot/JSON mode for agents (`--robot` / `--json`). Interactive TUI is a separate surface (bare `cass` launches TUI).
- Filters by agent, recency, workspace.
- Multi-machine sync and HTML export are advertised; treat as optional and confirm before depending on them.

## License

Commonly published as **MIT with an OpenAI/Anthropic rider**. GitHub may label the repo license as “Other”. The rider restricts use by OpenAI, Anthropic, affiliates, and parties acting for them, and restricts providing the software to those parties.

**Do not copy CASS source.** The rider is incompatible with treating the code as ordinary MIT. Independent clean-room implementation of the *idea* is required.

## Reusable mechanisms

- Adapter registry with machine-readable capability inventory (DEC-008, S009).
- Normalize heterogeneous session files into one model while preserving raw paths.
- Agent-oriented JSON CLI (stdout data, stderr diagnostics).
- Search across providers for “has anyone solved this already?”

## Limitations

- Connector formats change as vendors change session storage; each adapter needs tests and honest capability flags.
- Cursor/VS Code databases may contain secrets; redaction is mandatory (S053).
- Semantic refinement is optional; lexical fallback must be explained (S089).

## Integration options

1. **Preferred:** `rune-sessions` adapters implemented independently, storing canonical Session/Turn nodes.
2. Optional subprocess provider that calls a user-installed `cass --robot` if present — still treat output as untrusted content.
3. Never vendor CASS source or link it as a library.

## Clean-room note

Do not copy connectors, TUI, or robot-mode code. Reimplement session discovery from public file-format knowledge and Rune tests. Respect that copying is especially inappropriate given the published rider.
