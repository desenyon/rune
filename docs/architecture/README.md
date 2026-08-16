# Architecture

These documents describe the **specified** architecture of Rune. They match `AGENTS.md`, `docs/DECISIONS.md`, and the types that already exist in early crates. They do not claim that the full pipeline is implemented.

| Document | Specs |
| --- | --- |
| [Runtime](runtime.md) | S001, S019, S060, S061, S068, S077 |
| [Graph](graph.md) | S003–S005, S007, S014, S043, S100 |
| [Context compiler](context-compiler.md) | S024–S028, S044, S089–S094 |
| [Memory lifecycle](memory-lifecycle.md) | S011–S013, S045, S065, S095–S096 |
| [Session ingestion](session-ingestion.md) | S009–S010, S046, S075 |
| [Handoff](handoff.md) | S021–S022, S064, S091 |
| [Agent runtime](agent-runtime.md) | S018–S020, S029, S047 |
| [Providers](providers.md) | S033–S037, S059 |
| [Storage](storage.md) | S056–S058, S057, S100 |
| [TUI rendering](tui-rendering.md) | S002, S038–S049, S071–S073 |
| [Configuration](configuration.md) | S070 |

Crate names use the `rune-*` prefix. Directory names follow `crates/<name>` from AGENTS.md. The CLI is `rune`.
