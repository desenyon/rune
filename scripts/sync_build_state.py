#!/usr/bin/env python3
"""Rewrite BUILD_STATE detailed entries for recently implemented specs and
rebuild a unique S001–S100 summary table from the detailed sections."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PATH = ROOT / "docs" / "BUILD_STATE.md"

UPDATES = {
    "S003": """## S003 Structural code index

- specification: S003
- owner: rune-index
- dependencies: S001, S056, S058
- current status: verification
- implemented components: Tree-sitter (and fallback) symbol parse, stable file_key, incremental skip, persist to graph, intra-file and cross-file `Calls`/`References` via unique name plus import-path disambiguation
- remaining components: full import-graph resolution for ambiguous names; symbol-level git blame
- tests: rust_function_symbols_indexed, rust_function_and_test_are_indexed, file_key_is_stable_across_content_hash_changes, cross_file_calls_are_resolved
- known failures: ambiguous same-name callees without a unique import match stay pending
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S005": """## S005 Multimodal knowledge graph

- specification: S005
- owner: rune-index
- dependencies: S003, S056
- current status: verification
- implemented components: markdown headings, toml tables, json/yaml keys indexed as Document + DocumentationSection with Documents/Contains edges; structure preserved rather than flattened chunks
- remaining components: PDF, image metadata, issue/PR content, generated docs, database schemas
- tests: markdown_is_indexed_as_document_sections
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S006": """## S006 Search engine

- specification: S006
- owner: rune-search
- dependencies: S003, S005, S008, S011, S056
- current status: verification
- implemented components: exact, fuzzy (Nucleo), FTS5, structural, graph, temporal, hybrid, semantic (hashed 3-gram embedder); intent router including why/similar/meaning; forced `--mode`; CLI `rune search`
- remaining components: neural/remote embeddings via S059 ProcessBackend; p95 latency measurement (S067)
- tests: search_router_picks_graph_for_path_queries, router_picks_structural_for_fn_prefix, forced_mode_overrides_router, exact_and_fuzzy_find_function, graph_search_returns_path_nodes, search_finds_indexed_function, semantic mode routing tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S008": """## S008 Git temporal intelligence

- specification: S008
- owner: rune-git-intelligence
- dependencies: S001, S056
- current status: verification
- implemented components: commit/branch/tag/worktree indexing via porcelain; commit nodes after commit; rename parse; Author nodes with created_by
- remaining components: symbol-level change derivation; historical question compiler wiring
- tests: git_commit_nodes_created_after_a_commit, parses_commit_log_and_renames
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S017": """## S017 Parallelization analysis

- specification: S017
- owner: rune-tasks
- dependencies: S016, S003
- current status: verification
- implemented components: overlapping files/symbols refuse conflict-free claims; confidence and explanation; no claim without evidence
- remaining components: schema/migration/generated-artifact overlap
- tests: parallelization_refuses_conflict_free_claim_when_same_file_listed, parallelization_refuses_without_resource_evidence, s080_full_cross_subsystem_lifecycle
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S024": """## S024 Context compiler

- specification: S024
- owner: rune-context-compiler
- dependencies: S006, S007, S011, S013, S015, S016, S008, S030
- current status: verification
- implemented components: compile pipeline with budget, pin, exclude, capsule diff; CLI retrievers use MemoryStore/TaskStore/SpecStore so stale memory is omitted from guidance
- remaining components: neural semantic retriever; inspector UI event loop
- tests: compiler::tests::{compiler_respects_budget, pinned_item_is_included, excluded_item_is_omitted, capsule_diff_shows_added_and_removed}; compiler_s063
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial
""",
    "S030": """## S030 External documentation context

- specification: S030
- owner: rune-docs-context
- dependencies: S033, S056
- current status: verification
- implemented components: Context7 provider adapter; HTTPS via ureq/rustls (port 443) plus HTTP TCP; versioned objects; Query fails clearly when network/policy denied; `data_leaving_machine` disclosure
- remaining components: live fetch against the public Context7 API is not certified in CI; version pinning UX
- tests: docs_context unit tests for https/http parse and network-policy denied
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S050": """## S050 Diff intelligence

- specification: S050
- owner: rune-index
- dependencies: S003, S008, S013, S016, S015
- current status: verification
- implemented components: `impact_for_files` returns changed symbols, dependents, tests, tasks, specs, potentially stale memories, related decisions; CLI `rune impact`
- remaining components: automatic review-capsule compilation from impact; Git hunk-level symbol mapping
- tests: s080_full_cross_subsystem_lifecycle uses impact_for_files
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S059": """## S059 Semantic provider abstraction

- specification: S059
- owner: rune-semantic
- dependencies: S033
- current status: verification
- implemented components: LocalEmbed/RemoteEmbed/LocalLlm/RemoteLlm/Disabled; Disabled declares no Embed/Complete; LocalEmbed without a process backend uses the hashed n-gram embedder (not a neural model)
- remaining components: wire ProcessBackend to a real local/remote embedder; do not mix embeddings across providers
- tests: disabled_embed_does_not_invent_vectors, disabled_provider_still_allows_manual_component_summary, hash_embed unit tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S062": """## S062 Evaluation framework

- specification: S062
- owner: rune-evals
- dependencies: S056
- current status: verification
- implemented components: named evals, `rune eval`, optional `RUNE_WRITE_BENCHMARKS=1` writes `docs/benchmarks/*.json`
- remaining components: larger evidence corpora; CI job that fails on metric regression
- tests: rune-evals unit tests; cargo test -p rune-evals
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial
""",
    "S063": """## S063 Context compiler evaluation

- specification: S063
- owner: rune-evals
- dependencies: S062, S024
- current status: verification
- implemented components: evidence recall, stale/contradiction/duplicate rates, token cost, latency; `docs/benchmarks/compiler_s063.json`
- remaining components: larger question set with known supporting evidence beyond the synthetic store
- tests: compiler_s063
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial
""",
    "S064": """## S064 Handoff evaluation

- specification: S064
- owner: rune-evals
- dependencies: S062, S022
- current status: verification
- implemented components: structured handoff vs raw transcript completeness; `docs/benchmarks/handoff_s064.json`
- remaining components: live two-agent transfer measurement
- tests: handoff_s064, handoff_completeness
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial
""",
    "S065": """## S065 Memory evaluation

- specification: S065
- owner: rune-evals
- dependencies: S062, S012, S013
- current status: verification
- implemented components: extraction, inference rejection, staleness, conflict preservation; `docs/benchmarks/memory_s065.json`
- remaining components: scope isolation corpus at repository scale
- tests: memory_s065, memory_staleness
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: partial
""",
    "S077": """## S077 CLI surface

- specification: S077
- owner: rune-cli
- dependencies: S001, S006, S078
- current status: verification
- implemented components: `rune index|search|graph|memory|sessions|tasks|specs|handoff|context|agents|doctor|onboard|tui|export|completions|eval|impact|package|crash|update`
- remaining components: machine-readable schema stability; sessions ingest command
- tests: index_workspace_creates_function_nodes, search_finds_indexed_function, doctor_returns_ok_on_memory_db, guidance_retriever_omits_stale_memory
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S080": """## S080 Cross subsystem integration tests

- specification: S080
- owner: rune-evals
- dependencies: S012, S013, S016, S019, S021, S024, S049
- current status: verification
- implemented components: `s080_full_cross_subsystem_lifecycle` — index, cross-file calls, session memory, freshness after edit, spec/task, compile with pins, worktree edit, test run, commit+ChangedBy, handoff transfer, coverage link
- remaining components: live agent subprocess in the chain; S049 TUI coverage view
- tests: crates/evals/tests/s080_lifecycle.rs
- known failures: none recorded
- benchmark status: pass
- documentation status: partial
- integration status: verified
""",
    "S081": """## S081 Regression discipline

- specification: S081
- owner: tests
- dependencies: S079
- current status: verification
- implemented components: regression tests for cross-file calls, HTTPS URL parse, semantic ranking, S080 lifecycle
- remaining components: dedicated corpus covering every historical bug; policy that every future fix adds a test
- tests: cross_file_calls_are_resolved, docs_context HTTPS parse, search semantic routing, s080_full_cross_subsystem_lifecycle
- known failures: corpus is not exhaustive
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S085": """## S085 Packaging

- specification: S085
- owner: apps/rune
- dependencies: S077
- current status: verification
- implemented components: `scripts/package.sh` for host or rustc target; `rune package`; GitHub Actions matrix for macOS aarch64/x86_64 and Linux x86_64/arm64; sha256 sidecar
- remaining components: published tag artifacts; Windows target; signed releases; shell completion install in the package tarball
- tests: package.sh exists and is executable; CLI `package` subcommand
- known failures: multi-arch artifacts have not been published from a tag build
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S086": """## S086 Updates

- specification: S086
- owner: rune-app
- dependencies: S057, S085
- current status: verification
- implemented components: `rune update` / `App::check_update` discovers versions without replacing the running binary; `refuse_replace_running_binary`
- remaining components: network latest-version fetch; post-update migration prompt
- tests: app unit coverage around refuse_replace_running_binary and crash_bundle
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S087": """## S087 Crash reporting

- specification: S087
- owner: rune-app
- dependencies: S061, S054
- current status: verification
- implemented components: optional local crash bundle with version, platform, terminal, sanitized stack, provider disclosure, database diagnostics; `rune crash`; secrets redacted
- remaining components: automatic capture from panics in the TUI; log export beyond the bundle note
- tests: crash_bundle_redacts_secrets
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S002": """## S002 Terminal capability engine

- specification: S002
- owner: rune-terminal
- dependencies: none
- current status: verification
- implemented components: capability detection (true color, unicode, mouse, hyperlinks, synchronized output, Kitty/Sixel/iTerm graphics, cells/pixels), renderer levels Basic through Graphics, graceful degradation
- remaining components: dated per-terminal certification runs (S069)
- tests: rune-terminal unit tests for env-based probes
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S007": """## S007 Symbol intelligence

- specification: S007
- owner: rune-graph
- dependencies: S003, S008, S016, S015, S009, S011
- current status: verification
- implemented components: definition node lookup, callers/callees/references via graph neighbors; TUI graph explorer lists neighbors
- remaining components: implementations/tests/commits/tasks/specs/sessions/memories/failures as a single inspector surface
- tests: callers_and_callees_follow_call_edges, cross_file_calls_are_resolved
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S033": """## S033 OSS provider framework

- specification: S033
- owner: rune-providers
- dependencies: S054
- current status: verification
- implemented components: Provider trait, identity, capabilities, optional data_leaving_machine disclosure
- remaining components: async query/execute/stream/inspect/export/import as a uniform runtime, not only per-adapter methods
- tests: provider registry and capability tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S041": """## S041 Shared element transitions

- specification: S041
- owner: rune-motion
- dependencies: S040
- current status: active
- implemented components: rectangle/style interpolation, reduced-motion instant path
- remaining components: live shared-element from palette hit to inspector header in the TUI event loop
- tests: shared_element_reaches_new_rect, reduced_motion_zeroes_duration
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S042": """## S042 Adaptive rendering

- specification: S042
- owner: rune-ui
- dependencies: S002, S040
- current status: active
- implemented components: live TUI event loop, buffer-diff skip, motion frame budgets
- remaining components: measured 30/60 fps on reference terminals; synchronized updates where supported
- tests: buffer_diff_skips_identical_frames; rune-ui render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S043": """## S043 Graph explorer

- specification: S043
- owner: rune-ui
- dependencies: S039, S100
- current status: active
- implemented components: TUI graph view with focus, neighbor expand/collapse, kind filter, path list
- remaining components: pan/zoom abstraction, subgraph compare, provenance panel, small-screen layout polish
- tests: rune-ui graph explorer tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S044": """## S044 Context inspector

- specification: S044
- owner: rune-ui
- dependencies: S024, S092, S039
- current status: active
- implemented components: inspector snapshot with budget, included objects, reasons
- remaining components: pin/remove before invocation in the live event loop; excluded-candidate list
- tests: rune-ui inspector render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S046": """## S046 Session explorer

- specification: S046
- owner: rune-ui
- dependencies: S009, S010, S039
- current status: active
- implemented components: unified session list with provider/project filters in the TUI
- remaining components: resume/fork/handoff/compare actions from the explorer
- tests: rune-ui session explorer render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S047": """## S047 Agent cockpit

- specification: S047
- owner: rune-ui
- dependencies: S019, S020, S039
- current status: active
- implemented components: agent cards with provider, task, status, recent events
- remaining components: live event stream navigation; token/cost when providers emit them
- tests: rune-ui cockpit render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S048": """## S048 Task graph view

- specification: S048
- owner: rune-ui
- dependencies: S016, S039
- current status: active
- implemented components: ready/active/blocked/failed/review/complete rendering and blocker text
- remaining components: assignment to agents from the view
- tests: rune-ui task graph render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S049": """## S049 Specification coverage view

- specification: S049
- owner: rune-ui
- dependencies: S015, S003, S051, S039
- current status: active
- implemented components: requirement-to-evidence list and uncovered detection in the TUI
- remaining components: jump to implementing symbols/tests/commits from the view
- tests: rune-ui spec coverage render tests; S080 coverage link
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S053": """## S053 Environment awareness

- specification: S053
- owner: rune-security
- dependencies: S054
- current status: verification
- implemented components: nonsecret project facts; secret detection and redaction before persist/export/crash bundles
- remaining components: richer environment census in onboard
- tests: export_redacts_akia_keys, crash_bundle_redacts_secrets
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S054": """## S054 Security model

- specification: S054
- owner: rune-security
- dependencies: none
- current status: verification
- implemented components: untrusted content wrapping, permission policy, filesystem/process/network/plugin/MCP/agent boundaries
- remaining components: end-to-end audit of every provider payload
- tests: rune-security policy tests; S055 corpus
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S056": """## S056 Storage

- specification: S056
- owner: rune-storage
- dependencies: none
- current status: verification
- implemented components: SQLite nodes/edges/provenance/FTS5/settings; content-addressed blobs
- remaining components: measured size on very large repositories (S066)
- tests: storage unit and migration tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S057": """## S057 Database migrations

- specification: S057
- owner: rune-storage
- dependencies: S056
- current status: verification
- implemented components: numbered migrations, fresh DB, checksum mismatch, interrupted retry; no silent destroy
- remaining components: rollback strategy where practical
- tests: storage migration tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S058": """## S058 Content addressed cache

- specification: S058
- owner: rune-storage
- dependencies: S056
- current status: verification
- implemented components: Blake3 blob/cache keys; fingerprint invalidation
- remaining components: cache of embeddings/layouts at repository scale
- tests: storage cache tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S061": """## S061 Observability

- specification: S061
- owner: rune-telemetry
- dependencies: none
- current status: verification
- implemented components: structured tracing, secret-safe log export
- remaining components: richer operation metrics (index duration, render duration) in exported debug logs
- tests: exported_logs_redact_secrets
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S066": """## S066 Performance suite

- specification: S066
- owner: tests/performance
- dependencies: S079
- current status: active
- implemented components: repository generator and README for small/medium/large/very large tiers
- remaining components: measured runs on reference hardware; stored results in docs/benchmarks
- tests: tests/performance/gen.rs
- known failures: p95 targets not measured
- benchmark status: not_run
- documentation status: partial
- integration status: none
""",
    "S070": """## S070 Configuration

- specification: S070
- owner: rune-core
- dependencies: none
- current status: active
- implemented components: layered defaults/user/workspace/session configuration types
- remaining components: load all layers from disk in the CLI for every key in AGENTS.md S070
- tests: rune-core config tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S071": """## S071 Themes

- specification: S071
- owner: rune-ui
- dependencies: S039, S070
- current status: verification
- implemented components: semantic token themes including high-contrast; ember default
- remaining components: user themes without recompile loaded from config files
- tests: rune-ui theme tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S072": """## S072 Keybinding system

- specification: S072
- owner: rune-ui
- dependencies: S070
- current status: verification
- implemented components: semantic actions, configurable bindings, conflict detection
- remaining components: user keymap file in the layered config
- tests: rune-ui keybinding tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S073": """## S073 Accessibility

- specification: S073
- owner: rune-ui
- dependencies: S039, S040
- current status: verification
- implemented components: reduced motion, high contrast, keyboard-only navigation, non-color status labels
- remaining components: screen-size adaptation polish
- tests: rune-ui accessibility/theme tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S074": """## S074 Onboarding

- specification: S074
- owner: rune-cli
- dependencies: S001, S002, S035, S009
- current status: verification
- implemented components: `rune onboard` environment inspection; no account creation
- remaining components: first-run TUI wizard
- tests: onboard detection tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S076": """## S076 Export

- specification: S076
- owner: rune-app
- dependencies: S032, S054
- current status: verification
- implemented components: JSON/JSONL/Markdown export with secrets redacted by default
- remaining components: graph format and context-pack export (S032)
- tests: export_redacts_akia_keys
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S089": """## S089 Deterministic state inspection

- specification: S089
- owner: rune-context-compiler
- dependencies: S024, S013, S016, S006, S017
- current status: verification
- implemented components: explain_why for capsule objects; freshness reasons; task blocked reasons; parallelization explanations
- remaining components: search ranking evidence in the TUI
- tests: compiler inspect tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S090": """## S090 Context difference engine

- specification: S090
- owner: rune-context-compiler
- dependencies: S024
- current status: verification
- implemented components: compare_capsules added/removed/changed objects and token allocation
- remaining components: TUI diff view
- tests: capsule_diff_shows_added_and_removed
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S091": """## S091 Agent knowledge comparison

- specification: S091
- owner: rune-context-compiler
- dependencies: S009, S024
- current status: verification
- implemented components: compare_knowledge over observable supplied context and sessions only
- remaining components: TUI comparison view
- tests: compiler knowledge comparison tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S092": """## S092 Context provenance visualization

- specification: S092
- owner: rune-ui
- dependencies: S024, S044
- current status: active
- implemented components: inspector fields for source, reason, freshness, token cost
- remaining components: retrieval-path visualization beyond the reason string
- tests: rune-ui inspector render tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S093": """## S093 Context pinning

- specification: S093
- owner: rune-context-compiler
- dependencies: S024
- current status: verification
- implemented components: PinSet survives ranking; compile includes pins
- remaining components: stale/contradicted pin warnings in the TUI
- tests: pinned_item_is_included; S080 pins
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S094": """## S094 Context exclusion

- specification: S094
- owner: rune-context-compiler
- dependencies: S024
- current status: verification
- implemented components: scoped exclusions; excluded items omitted
- remaining components: do not persist session exclusions as user preferences (enforced in types; TUI still needed)
- tests: excluded_item_is_omitted
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S100": """## S100 Graph integrity

- specification: S100
- owner: rune-storage
- dependencies: S056
- current status: verification
- implemented components: dangling edges, missing objects, duplicate identities, orphaned session refs, invalid task deps, invalid provenance, migration inconsistency; repair actions on findings
- remaining components: automatic repair CLI beyond doctor
- tests: storage integrity tests
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
    "S088": """## S088 Data ownership

- specification: S088
- owner: rune-providers
- dependencies: S054
- current status: verification
- implemented components: `Provider::data_leaving_machine` default None; Context7 returns an HTTPS disclosure; crash bundles include provider disclosure strings; local mode does not upload
- remaining components: per-command prompt before first network use in the TUI
- tests: docs_context network policy denied; crash_bundle_redacts_secrets
- known failures: none recorded
- benchmark status: not_run
- documentation status: partial
- integration status: partial
""",
}

HEADER = """# Build state

Product: Rune
Last updated: 2026-08-15
Coordinator: root coordinator

Every required specification has exactly one state: `planned`, `active`, `blocked`, `verification`, or `complete`.

## Summary

"""

NOTE = """
Highest-leverage unblocked work: S023 branchable context merge UX; S032 portable packs; S041 shared-element motion; S066/S067 measured performance on large fixtures; remaining TUI inspector depth. Semantic search, Context7 HTTPS, S080 lifecycle, S063–S065 evals, cross-file calls, multimodal documents, diff impact, and multi-arch packaging scripts are in verification.

## Protocol notes

- owner: crate or agent currently responsible
- tests: names of unit/integration/regression coverage
- known failures: open defects
- benchmark status: `not_run` | `pass` | `fail` | `regressed`
- documentation status: `missing` | `partial` | `matches`
- integration status: `none` | `partial` | `verified`

---
"""


def split_sections(text: str) -> dict[str, str]:
    parts = re.split(r"(?=^## S\d{3} )", text, flags=re.M)
    sections: dict[str, str] = {}
    for part in parts:
        match = re.match(r"^## (S\d{3}) ", part)
        if not match:
            continue
        spec = match.group(1)
        sections[spec] = part if part.endswith("\n") else part + "\n"
    return sections


def field(section: str, name: str) -> str:
    match = re.search(rf"^- {name}: (.+)$", section, re.M)
    if not match:
        raise SystemExit(f"missing {name} in section:\n{section[:120]}")
    return match.group(1).strip()


def title(section: str) -> str:
    match = re.match(r"^## S\d{3} (.+)$", section, re.M)
    if not match:
        raise SystemExit("missing title")
    return match.group(1).strip()


def main() -> None:
    text = PATH.read_text()
    sections = split_sections(text)
    missing = [spec for spec in UPDATES if spec not in sections]
    if missing:
        raise SystemExit(f"missing sections: {missing}")
    sections.update(UPDATES)

    expected = [f"S{i:03d}" for i in range(1, 101)]
    absent = [spec for spec in expected if spec not in sections]
    extra = sorted(set(sections) - set(expected))
    if absent or extra:
        raise SystemExit(f"spec set mismatch absent={absent} extra={extra}")

    rows = [
        "| Spec | Title | Status | Owner | Dependencies |",
        "| --- | --- | --- | --- | --- |",
    ]
    for spec in expected:
        section = sections[spec]
        rows.append(
            f"| {spec} | {title(section)} | {field(section, 'current status')} | {field(section, 'owner')} | {field(section, 'dependencies')} |"
        )

    body = "\n".join(sections[spec].rstrip() + "\n" for spec in expected)
    PATH.write_text(HEADER + "\n".join(rows) + "\n" + NOTE + "\n" + body)


if __name__ == "__main__":
    main()
