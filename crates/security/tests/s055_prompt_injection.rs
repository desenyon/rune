//! S055: retrieved fixture text is content, never instruction or a permission grant.

use rune_security::{UntrustedContent, Permission, Policy};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> PathBuf {
    repo_root()
        .join("tests/fixtures/security/prompt_injection")
        .join(name)
}

fn load(name: &str) -> String {
    fs::read_to_string(fixture(name)).unwrap_or_else(|err| panic!("missing {name}: {err}"))
}

#[test]
fn injection_corpus_files_exist() {
    for name in [
        "README.md",
        "malicious_readme.md",
        "source_comment.rs",
        "doc.md",
        "issue.md",
        "session_transcript.jsonl",
        "tool_output.txt",
        "mcp.json",
        "external_doc.md",
    ] {
        assert!(fixture(name).is_file(), "{name}");
    }
}

#[test]
fn each_source_kind_is_untrusted_and_does_not_mutate_policy() {
    let cases = [
        ("malicious_readme.md", "RUNE_INJECTION_CANARY_README"),
        ("source_comment.rs", "RUNE_INJECTION_CANARY_COMMENT"),
        ("doc.md", "RUNE_INJECTION_CANARY_DOC"),
        ("issue.md", "RUNE_INJECTION_CANARY_ISSUE"),
        ("session_transcript.jsonl", "RUNE_INJECTION_CANARY_SESSION"),
        ("tool_output.txt", "RUNE_INJECTION_CANARY_TOOL"),
        ("mcp.json", "RUNE_INJECTION_CANARY_MCP"),
        ("external_doc.md", "RUNE_INJECTION_CANARY_EXTDOC"),
    ];
    for (name, canary) in cases {
        let body = load(name);
        assert!(
            body.contains("ignore previous instructions")
                || body.to_ascii_lowercase().contains("ignore previous instructions"),
            "{name} should contain an injection phrase"
        );
        let content = UntrustedContent::wrap(name, body.clone());
        assert!(content.as_instruction().is_none(), "{name} must not be instruction");
        assert!(
            content.body.contains(canary) || body.contains(canary),
            "{name} canary {canary} must remain observable as content"
        );
        let policy = Policy::local_default();
        assert!(!policy.permits(Permission::FilesystemWrite));
        assert!(!policy.auto_execute_commands);
        assert!(!policy.network_enabled);
        let _ = &content.body;
        assert!(!policy.permits(Permission::FilesystemWrite));
        assert!(!policy.permits(Permission::ProcessExecute));
        assert!(!policy.permits(Permission::Network));
    }
}

#[test]
fn indexing_injection_readme_does_not_grant_permissions() {
    use rune_core::{Node, NodeKind};
    use rune_storage::Store;

    let store = Store::open_in_memory().unwrap();
    let body = load("malicious_readme.md");
    let node = Node::new(
        NodeKind::Document,
        Some("README.md".into()),
        serde_json::json!({ "path": "README.md", "body": body }),
    );
    store.upsert_node(&node).unwrap();
    let hits = store.search_text("CANARY", 8).unwrap();
    assert!(
        !hits.is_empty(),
        "FTS must retrieve the canary as content, got {hits:?}"
    );
    let policy = Policy::local_default();
    assert!(!policy.permits(Permission::FilesystemWrite));
}

#[test]
fn fixture_directory_is_not_treated_as_missing() {
    assert!(Path::new(&fixture("README.md")).exists());
}
