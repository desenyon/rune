//! Contract tests for documentation artifacts produced for S034, S069, S079, S083.

use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn s034_research_notes_cover_required_targets_and_sections() {
    let dir = root().join("docs/integrations");
    let targets = [
        "graft.md",
        "codebase-memory-mcp.md",
        "serena.md",
        "probe.md",
        "graphify.md",
        "rekal.md",
        "cass.md",
        "cass-memory.md",
        "catchup.md",
        "git-context-controller.md",
        "beads.md",
        "openspec.md",
        "spec-kit.md",
        "context7.md",
        "rtk.md",
        "caveman.md",
        "repomix.md",
        "herdr.md",
    ];
    let required = [
        "## Architecture",
        "## License",
        "## Reusable mechanisms",
        "## Limitations",
        "## Integration options",
        "## Clean-room",
    ];
    for file in targets {
        let path = dir.join(file);
        let body = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        for heading in required {
            assert!(
                body.contains(heading),
                "{} missing section starting {heading}",
                path.display()
            );
        }
    }
}

#[test]
fn s079_language_fixtures_exist() {
    let repos = root().join("tests/fixtures/repositories");
    let required = [
        "rust_lib/src/lib.rs",
        "python_app/src/python_app/__init__.py",
        "typescript_app/src/index.ts",
        "go_mod/main.go",
        "mixed_monorepo/crates/mixed_core/src/lib.rs",
        "mixed_monorepo/packages/py_util/py_util.py",
        "unicode_paths/こんにちは.rs",
        "malformed/not_utf8.bin",
        "bootstrap.sh",
    ];
    for rel in required {
        let path = repos.join(rel);
        assert!(path.is_file(), "missing fixture {}", path.display());
    }
    let bytes = fs::read(repos.join("malformed/not_utf8.bin")).unwrap();
    assert!(std::str::from_utf8(&bytes).is_err(), "malformed fixture must not be utf-8");
}

#[test]
fn s083_architecture_diagrams_use_mermaid() {
    let dir = root().join("docs/architecture");
    let files = [
        "runtime.md",
        "graph.md",
        "context-compiler.md",
        "memory-lifecycle.md",
        "session-ingestion.md",
        "handoff.md",
        "agent-runtime.md",
        "providers.md",
        "storage.md",
        "tui-rendering.md",
    ];
    for file in files {
        let body = fs::read_to_string(dir.join(file)).unwrap();
        assert!(
            body.contains("```mermaid"),
            "{file} must contain a mermaid diagram"
        );
    }
}

#[test]
fn s069_compatibility_matrix_exists() {
    let body = fs::read_to_string(root().join("docs/compatibility/README.md")).unwrap();
    for needle in ["Ghostty", "Kitty", "WezTerm", "iTerm", "Alacritty", "macOS", "Linux", "Windows"] {
        assert!(body.contains(needle), "compatibility matrix missing {needle}");
    }
}
