# Spec Kit

**Research target:** structured specification workflows.

**Public project:** [github/spec-kit](https://github.com/github/spec-kit). Toolkit for spec-driven development with phased artifacts (specify, plan, tasks, implement) and an extension catalog.

## Architecture (public knowledge)

- Feature folders with spec, plan, and tasks documents as a lab-notebook for changes.
- Phase-oriented workflow (heavier than OpenSpec’s free iteration, per public comparisons).
- Agent slash commands / skills per supported coding agent.
- Community extensions (including MCP orchestrators) are optional and separately licensed.

## License

MIT as commonly published on github/spec-kit. Confirm LICENSE. Extensions have their own licenses.

## Reusable mechanisms

- Explicit workflow stages that produce inspectable artifacts.
- Plan and task breakdown linked back to a specification.
- Extension points for tools (Rune’s plugin/provider model is the analog).

## Limitations

- Rigid phase gates can fight Rune’s continuous graph loop (work produces evidence continuously).
- Python/CLI setup cost is irrelevant to Rune’s Rust workspace except as an import source.
- Generated Markdown volume can drown retrieval unless compiled with budgets.

## Integration options

1. **Preferred:** map Spec Kit artifacts into Specification, Requirement, and Task nodes on import (S075).
2. Do not require users to run Spec Kit to use Rune specs.
3. Treat extension MCPs as untrusted providers.

## Clean-room note

Do not copy Spec Kit templates, prompt files, or Python tooling. Reimplement structured workflows as graph state machines if needed.
