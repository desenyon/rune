# Herdr

**Research target:** agent runtime and worktree / workspace orchestration.

**Public project:** [herdrdev/herdr](https://github.com/herdrdev/herdr). Terminal-native background server that owns persistent panes/workspaces for coding agents. Agents keep running when the client detaches. Apache-2.0 Rust binary.

## Architecture (public knowledge)

- Server owns terminals; client attaches locally or over SSH.
- Detects many agent CLIs and marks panes working / blocked / idle.
- CLI and local socket API: spawn panes, send prompts, wait for blocked state, subscribe to events.
- Does not wrap or replace agent CLIs; it owns the terminal underneath them.
- Plugin marketplace exists; treat third-party plugins as untrusted.
- Windows support has been described as beta; confirm current matrix.

## License

Apache License 2.0 as commonly published. Confirm LICENSE. Default update/phone-home behavior (if any) must be verified; Rune must not inherit undisclosed network calls (S088).

## Reusable mechanisms

- Persistent agent processes with reattach (S019).
- Semantic agent states for a cockpit (S047).
- Socket/CLI API as the same surface humans and agents use.
- Isolation: Rune should still assign worktrees and permissions; a multiplexer is not a security boundary by itself.

## Limitations

- Screen-scraping agent state is brittle; prefer native session identity when adapters provide it.
- Sitting between the user and every terminal is a large trust ask; Rune can orchestrate agents it spawned without replacing the user’s multiplexer.
- Plugin registries without review are a security concern (S037, S054).

## Integration options

1. **Preferred:** `rune-agent-runtime` spawns and supervises agent subprocesses; `rune-worktrees` isolates files.
2. Optional Herdr provider: list/attach sessions if the user runs Herdr, capability-declared.
3. Do not require Herdr for Rune’s own agent cockpit.

## Clean-room note

Do not copy Herdr source, keybindings, or agent-detection manifests. Reimplement process supervision and status in Rune. Detection of installed agent binaries can use `which` and documented session paths, independently maintained.
