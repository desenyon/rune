# Git Context Controller

**Research target:** branchable context.

**Public projects / papers:** Git-Context-Controller (GCC) — arXiv “Git Context Controller: Manage the Context of LLM-based Agents like Git”; implementations include [faugustdev/git-context-controller](https://github.com/faugustdev/git-context-controller) (v2 lean git-backed index) and earlier `.GCC/` directory designs (`theworldofagents/GCC` and related).

## Architecture (public knowledge)

Core operations: **COMMIT**, **BRANCH**, **MERGE**, **CONTEXT**.

- v1-style: `.GCC/` filesystem with `main.md` roadmap, per-branch `log.md` / `commit.md` / metadata.
- v2-style: store hash + intent + optional decision notes (~tens of tokens per entry); reconstruct full context via `git show`. Dual git-backed vs standalone markdown fallback.
- Branches may use git worktrees for isolation.
- CONTEXT retrieval levels: summary, last N, by hash, decisions-only, full.

## License

MIT as published in faugustdev/git-context-controller `dev` LICENSE (fetched 2026-08-15). Other GCC repositories may differ; confirm the specific tree. The research paper is not a software license.

## Reusable mechanisms

- Branchable context snapshots: snapshot, branch, compare, merge, archive (S023).
- Store pointers (commit hash + intent) instead of duplicating git-known diffs.
- Decision notes for things git cannot capture (rejected alternatives).
- Worktree isolation for experimental context/code (S018).

## Limitations

- A markdown `.GCC/` tree can diverge from code reality.
- Merge of *reasoning* branches is not the same as git merge; conflicts must be explicit (S096, S090).
- Silent bridges to third-party vector memory should not run without user-enabled semantic providers.

## Integration options

1. **Preferred:** Context Capsule snapshots as graph objects with parent/branch pointers in SQLite, compared via S090.
2. Optional import of a `.GCC/` or GCC `index.yaml` as documents.
3. Do not require agents to speak `/gcc` slash commands; Rune actions are semantic (S072).

## Clean-room note

Do not copy GCC shell scripts, YAML schemas, or skill files. Reimplement branchable context on capsules and git worktrees.
