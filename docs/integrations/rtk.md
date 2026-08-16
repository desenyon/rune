# RTK

**Research target:** tool output normalization and compression.

**Public project:** commonly [rtk-ai/rtk](https://github.com/rtk-ai/rtk) (“Rust Token Killer”) — CLI proxy that filters and compresses command outputs (for example `git status`) before they reach the model. Related write-ups describe deterministic parsers rather than LLM summarization.

## Architecture (public knowledge)

- Agent runs a command; RTK intercepts or prefixes the command.
- Tool-specific compressors emit a compact structured view with the same operational signal.
- Goal: reduce tokens on verbose CLI output without dropping errors.
- Independent benchmarks (for example JetBrains public tests of similar tools) have reported much smaller savings than marketing claims on real agentic coding tasks. Treat advertised percentages as unverified.

## License

Apache-2.0 as commonly published for rtk-ai/rtk. Confirm LICENSE at the version inspected. Wrapper repos may be MIT or other licenses.

## Reusable mechanisms

- Adaptive compression representations: raw, structured, summary, errors, diff, changes_since_previous (S028).
- Do not blindly compress every command; preserve enough for correct reasoning.
- Compression reversible when raw output is stored locally (blob store).
- Prefer structured git/tool output (`--porcelain`, JSON) before regex scraping (S035).

## Limitations

- Over-compression hides the one line that explains a failure.
- A global shell proxy is a security and compatibility hazard; Rune should compress inside the event pipeline it owns.
- Marketing token-savings numbers are not release gates.

## Integration options

1. **Preferred:** `rune-compression` on normalized agent events, with inspectable before/after in the Context Inspector.
2. Optional RTK provider if the binary is installed — still keep raw blobs.
3. Do not rewrite the user’s shell globally.

## Clean-room note

Do not copy RTK parsers. Implement compressors from documented tool machine formats (git porcelain, cargo JSON, etc.).
