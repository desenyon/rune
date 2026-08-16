# Caveman

**Research target:** minimal communication policy.

**Public project:** [JuliusBrussee/caveman](https://github.com/JuliusBrussee/caveman). Skill/policy that makes agents speak in terse fragments to reduce output tokens. Later versions add input compression, proxies, and a split-licensed engine.

## Architecture (public knowledge)

- Presentation skill: drop filler, keep code/errors/commands verbatim.
- Modes of terseness (lite/full/ultra and others).
- Optional input compressors for large instruction files.
- Independent tests (JetBrains, 2026) found output-token savings on real coding tasks much smaller than chat-style marketing figures, because tool output and code dominate. Style change did not clearly harm task success in that study.

## License

**Split license as commonly published:**

- MIT: skill, many SDKs, CLI surfaces, contracts, graders (confirm current map).
- BSL-1.1: engine/proxy/MCP/`shrink` and related runtime modules; source-available, not OSI Open Source until the change date (documented conversion toward Apache-2.0). Third-party hosted/embedded use may need a commercial license.

Confirm the per-directory license map before any near-reuse. **Do not copy BSL engine code into Rune.**

## Reusable mechanisms

- Communication policy modes: full, concise, minimal, machine (S029).
- Policy controls presentation, not reasoning quality.
- Machine mode: structured events (`SEARCH`, `READ`, `EDIT`, `TEST`, `COMPLETE`) for the TUI to render.
- Preserve errors and code exactly.

## Limitations

- Terse chat is not context compression of tool output (that is S028).
- Auto-install scripts that rewrite user agent configs must not run without approval.
- BSL components are not Apache-2.0-compatible until they convert.

## Integration options

1. **Preferred:** implement S029 inside Rune’s agent runtime and UI rendering.
2. Do not vendor Caveman proxy/engine.
3. Optional: user-selected presentation theme inspired by the idea, independently written.

## Clean-room note

Do not copy skill text, rewriter, or engine. Independently specify Rune’s four presentation modes and event grammar.
