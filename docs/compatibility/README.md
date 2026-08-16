# Compatibility matrix (S069)

Status values for this matrix: **implemented** or **planned**.

Nothing in this matrix is a tested release. Foundational crates exist (`rune-terminal` capability types, `rune-providers` capability enums) but OS builds, terminal labs, and agent adapters are **planned**. Do not treat a row as certified until `docs/compatibility/` gains dated run evidence.

Last updated: 2026-08-15.

## Operating systems

| Platform | Status | Notes |
| --- | --- | --- |
| macOS arm64 | planned | Packaging script and CI matrix exist (`scripts/package.sh`, `.github/workflows/release.yml`). No dated certification run. |
| macOS x86_64 | planned | Same packaging matrix. No dated certification run. |
| Linux x86_64 | planned | Same packaging matrix. No dated certification run. |
| Linux arm64 | planned | Same packaging matrix. No dated certification run. |
| Windows | planned | Specified where support is reliable. Agent runtime and watchers need extra work (`notify` uses macOS kqueue in workspace deps today). |

## Terminals

Renderer must degrade by capability level (S002). Missing graphics must not make the TUI unusable.

| Terminal | Status | Expected capability notes (specified, untested) |
| --- | --- | --- |
| Ghostty | planned | True color, Kitty graphics family likely. |
| Kitty | planned | Kitty graphics, true color, hyperlinks. |
| WezTerm | planned | True color; Sixel/Kitty depending on config. |
| iTerm2 | planned | iTerm inline images; true color. |
| Alacritty | planned | True color; no native image protocol in typical builds. |
| Apple Terminal | planned | Limited color/graphics; must remain usable at Standard/Basic. |
| Common SSH terminals | planned | Often Basic/Standard; synchronized output and graphics frequently absent. |

Detection lives in `rune-terminal` as types and env-based probes. Per-terminal certification runs are not implemented.

## Agent adapters (S009)

Adapters declare capabilities. Unsupported operations must fail clearly.

| Adapter | Status | Discovery | Import | Continuation | Context injection | Command invocation | Handoff | Streaming events |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Claude Code | planned | planned | planned | planned | planned | planned | planned | planned |
| Codex | planned | planned | planned | planned | planned | planned | planned | planned |
| Cursor | planned | planned | planned | planned | planned | planned | planned | planned |
| OpenCode | planned | planned | planned | planned | planned | planned | planned | planned |
| Gemini CLI | planned | planned | planned | planned | planned | planned | planned | planned |
| Aider | planned | planned | planned | planned | planned | planned | planned | planned |
| Plugin-provided agents | planned | planned | planned | planned | planned | planned | planned | planned |

When an adapter is implemented, replace **planned** cells with **implemented** only for capabilities that exist and are tested. Do not mark continuation implemented if the provider cannot resume native sessions.

## How to update

After a real run, add a dated file under `docs/compatibility/runs/` (not created yet) with OS, terminal, `$TERM`, capability JSON from `rune-terminal`, adapter, and pass/fail. Then edit this matrix.
