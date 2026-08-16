# Prompt injection fixtures (S055)

These files are **attack corpora for tests**. They are not operator instructions.

Rune must treat retrieved text as data. A README, comment, document, issue, session transcript, tool output, MCP payload, or external doc that says “ignore previous instructions” must never:

- change agent permissions
- auto-execute commands
- promote itself to verified memory
- disable security policy
- grant network, plugin, or worktree rights

Tests should index and retrieve these files, then assert that policy and memory extraction ignore the imperative content.

Each fixture contains a distinctive canary string so tests can prove the text was retrieved as content (`RUNE_INJECTION_CANARY_*`).
