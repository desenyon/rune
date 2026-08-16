# Rune

```text
  ┌─────────────────────────────────────────┐
  │   ᚱ  RUNE                               │
  │   local-first context os                │
  └─────────────────────────────────────────┘
```

Local-first Context OS for AI-assisted software development.

Rune is a terminal-native control plane shared by coding agents, developer tools, repositories, tasks, specifications, historical sessions, code intelligence, external documentation, and persistent memory.

A developer can move between Claude Code, Codex, Cursor, OpenCode, Gemini CLI, Aider, and future coding agents without rebuilding repository understanding from scratch.

## One-shot install

```bash
curl -fsSL https://raw.githubusercontent.com/desenyon/rune/main/scripts/install.sh | bash
```

The installer:

1. Installs a Rust toolchain if `cargo` is missing
2. Clones this repository into `~/.local/src/rune`
3. Builds the `rune` binary
4. Adds the binary directory to your PATH (`~/.zprofile`, `~/.zshrc`, `~/.bashrc`, `~/.profile`)
5. Runs `rune doctor` and `rune onboard` in the current directory
6. Opens the TUI when the shell is interactive

No account. Nothing leaves the machine unless you later enable a network provider.

```bash
# optional overrides
RUNE_PREFIX=$HOME/.local RUNE_WORKSPACE=$PWD bash scripts/install.sh
```

## Product rule

Everything is an object. Every object can have relationships. Every object can have actions. Every object can contribute context.

External tools are implementation providers. Rune is the system of record.

## CLI

```text
rune                 # TUI (default)
rune index
rune search "auth"
rune graph
rune memory
rune sessions import
rune tasks
rune specs
rune handoff compile --from claude --to codex "continue the task"
rune context compile "fix the race"
rune agents
rune doctor
rune onboard
rune export --format json
```

## TUI

Keyboard-first. Views, not chrome.

```text
1 Home     2 Graph     3 Memory     4 Sessions
5 Tasks    6 Specs     7 Context    8 Agents
ctrl+p command palette      tab next view      q quit
```

The palette searches commands and indexed objects together. Status never relies on color alone.

## Local-first

Project data belongs to the user. Core navigation, structural indexing, Git intelligence, tasks, specifications, stored memories, stored sessions, graph exploration, and cached context work offline. Network-dependent features fail clearly.

## Development

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace
cargo run -p rune -- onboard
cargo run -p rune --
```

Domain crates do not depend on Ratatui. The TUI is a renderer over snapshots.

See:

- [Architecture](docs/architecture/README.md)
- [Specifications S001–S100](docs/specifications/README.md)
- [Build state](docs/BUILD_STATE.md)
- [Decisions](docs/DECISIONS.md)
- [Release gates](docs/RELEASE_GATES.md)

## Status

The product is under active construction. Many specifications are implemented and in verification. **Release gates have not passed.** Do not treat this as a finished 1.0.

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
