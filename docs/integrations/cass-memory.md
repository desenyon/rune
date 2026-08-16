# CASS Memory

**Research target:** procedural memory extraction.

**Public project:** [Dicklesworthstone/cass_memory_system](https://github.com/Dicklesworthstone/cass_memory_system) (`cm`). Builds persistent cross-agent procedural memory on top of CASS session search.

## Architecture (public knowledge)

Three-layer model:

| Layer | Role |
| --- | --- |
| Episodic | Raw sessions via `cass` search |
| Working | Structured session summaries / diary |
| Procedural | Distilled playbook rules with helpful/harmful counters |

- Onboarding (`cm onboard`) uses the user’s already-running coding agent to mine history rather than a separate paid API by default.
- Playbook bullets with source tracing, tombstones for deprecation, search pointers back into CASS queries.
- Documented emphasis on deterministic curation (delta updates) to avoid “context collapse” from rewriting the whole playbook with an LLM.
- Validation of candidate rules against historical sessions before acceptance.

## License

Commonly published as **MIT with an OpenAI/Anthropic rider** (same family as CASS). GitHub may show “Other”.

**Do not copy CASS Memory source.** Independent implementation only.

## Reusable mechanisms

- Candidate → verified memory pipeline; agent-mined rules start untrusted (S012).
- Helpful/harmful feedback on retrieved memories.
- Supersession / tombstone rather than silent rewrite (S013, S096).
- Compact playbook entries that point at evidence instead of inlining transcripts.
- Cross-agent availability of a pattern learned in one provider.

## Limitations

- Depends on CASS (or equivalent) for episodic search.
- Automatic promotion of playbook bullets would violate Rune’s rule that agent guesses are never verified.
- Rider makes source reuse inappropriate even if the idea is useful.

## Integration options

1. **Preferred:** `rune-memory` extraction + freshness, with human verification gates.
2. Optional import of a user playbook file as `candidate` memories with provenance.
3. Do not ship `cm` as a hidden backend.

## Clean-room note

Do not copy TypeScript/Bun sources, skill files, or MCP tool lists. Reimplement procedural memory on Rune’s canonical Memory nodes.
