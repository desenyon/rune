//! Workspace discovery, structural indexing, watching, and process awareness.

pub mod background;
pub mod discovery;
pub mod documents;
pub mod error;
pub mod impact;
pub mod indexer;
pub mod languages;
pub mod persist;
pub mod process;
pub mod structural;
pub mod watch;

pub use background::{BackgroundIndexer, IndexJob, IndexQueue};
pub use discovery::{discover, DiscoveredFile, WorkspaceDiscovery};
pub use error::{IndexError, Result};
pub use documents::{parse_document, ParsedDocument, ParsedSection};
pub use impact::{impact_for_files, DiffImpact};
pub use indexer::{IndexedChange, Indexer, TestRun, WorkspaceScanReport};
pub use languages::{file_key, MonorepoKind, SourceLanguage, WorktreeListing};
pub use process::{discover_processes, ProcessInfo};
pub use structural::{parse_source, ParsedFile, ParsedSymbol};
pub use watch::{coalesce_events, classify_event, WatchConfig, WorkspaceWatcher};

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::NodeKind;
    use rune_storage::Store;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args([
                "-c",
                "user.name=Rune Test",
                "-c",
                "user.email=rune@test.local",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "Rune Test")
            .env("GIT_AUTHOR_EMAIL", "rune@test.local")
            .env("GIT_COMMITTER_NAME", "Rune Test")
            .env("GIT_COMMITTER_EMAIL", "rune@test.local")
            .status()
            .expect("git must be installed");
        assert!(status.success(), "git {args:?} failed");
    }

    fn init_repo() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        git(&path, &["init", "--quiet"]);
        fs::write(path.join("README.md"), "hello\n").unwrap();
        git(&path, &["add", "README.md"]);
        git(&path, &["commit", "-m", "init"]);
        (dir, path)
    }

    #[test]
    fn incremental_workspace_scan_skips_unchanged_files() {
        let (_tmp, root) = init_repo();
        fs::write(root.join("hello.rs"), "pub fn greet() {}\n").unwrap();
        let store = Store::open_in_memory().unwrap();
        let indexer = Indexer::new(store, &root).unwrap();
        let first = indexer.scan_workspace().unwrap();
        assert!(first.files_indexed >= 1);
        assert_eq!(first.files_skipped_unchanged, 0);
        let second = indexer.scan_workspace().unwrap();
        assert!(second.files_skipped_unchanged >= 1);
        assert_eq!(second.files_indexed, 0);
        fs::write(root.join("hello.rs"), "pub fn greet() {}\npub fn other() {}\n").unwrap();
        let third = indexer.scan_workspace().unwrap();
        assert_eq!(third.files_indexed, 1);
        assert!(third.files_seen > third.files_indexed);
    }

    #[test]
    fn rust_function_symbols_indexed() {
        let (_tmp, root) = init_repo();
        fs::write(
            root.join("lib.rs"),
            "pub fn greet(name: &str) -> String { format!(\"hi {name}\") }\n",
        )
        .unwrap();
        let store = Store::open_in_memory().unwrap();
        let indexer = Indexer::new(store, &root).unwrap();
        indexer.scan_workspace().unwrap();
        let functions = indexer.store().nodes_of_kind(NodeKind::Function).unwrap();
        assert!(
            functions.iter().any(|n| n.name.as_deref() == Some("greet")),
            "expected greet function, got {:?}",
            functions.iter().map(|n| n.name.clone()).collect::<Vec<_>>()
        );
        let greet = functions
            .iter()
            .find(|n| n.name.as_deref() == Some("greet"))
            .unwrap();
        assert_eq!(greet.payload["file_key"], serde_json::json!(file_key(&root, &root.join("lib.rs"))));
        assert!(greet.payload.get("start_line").and_then(|v| v.as_u64()).unwrap() >= 1);
    }

    #[test]
    fn nested_repo_detected() {
        let (_tmp, root) = init_repo();
        let nested = root.join("vendor").join("inner");
        fs::create_dir_all(&nested).unwrap();
        git(&nested, &["init", "--quiet"]);
        fs::write(nested.join("file.txt"), "nested\n").unwrap();
        git(&nested, &["add", "file.txt"]);
        git(&nested, &["commit", "-m", "nested"]);
        let store = Store::open_in_memory().unwrap();
        let indexer = Indexer::new(store, &root).unwrap();
        let report = indexer.scan_workspace().unwrap();
        assert!(
            report.nested_repos.iter().any(|p| p.ends_with("inner")),
            "nested repos: {:?}",
            report.nested_repos
        );
    }

    #[test]
    fn cargo_workspace_monorepo_detected() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/core\"]\n",
        )
        .unwrap();
        let discovered = discover(dir.path()).unwrap();
        assert_eq!(discovered.monorepo, Some(MonorepoKind::CargoWorkspace));
        assert!(discovered.package_managers.iter().any(|m| m == "cargo"));
    }

    #[test]
    fn cross_file_calls_are_resolved() {
        let (_tmp, root) = init_repo();
        fs::write(root.join("a.rs"), "pub fn greet() {}\n").unwrap();
        fs::write(
            root.join("b.rs"),
            "pub fn hello() { greet(); }\n",
        )
        .unwrap();
        let store = Store::open_in_memory().unwrap();
        let indexer = Indexer::new(store, &root).unwrap();
        indexer.scan_workspace().unwrap();
        let functions = indexer.store().nodes_of_kind(NodeKind::Function).unwrap();
        let greet = functions
            .iter()
            .find(|n| n.name.as_deref() == Some("greet"))
            .unwrap();
        let hello = functions
            .iter()
            .find(|n| n.name.as_deref() == Some("hello"))
            .unwrap();
        let edge = indexer
            .store()
            .find_edge(hello.id, greet.id, rune_core::EdgeKind::Calls)
            .unwrap();
        assert!(edge.is_some(), "expected hello -> greet call edge");
    }

    #[test]
    fn markdown_is_indexed_as_document_sections() {
        let (_tmp, root) = init_repo();
        fs::write(
            root.join("ADR.md"),
            "# Decision\n\nUse SQLite.\n\n## Consequences\n\nLocal first.\n",
        )
        .unwrap();
        let store = Store::open_in_memory().unwrap();
        let indexer = Indexer::new(store, &root).unwrap();
        indexer.scan_workspace().unwrap();
        let docs = indexer.store().nodes_of_kind(NodeKind::Document).unwrap();
        assert!(!docs.is_empty());
        let sections = indexer
            .store()
            .nodes_of_kind(NodeKind::DocumentationSection)
            .unwrap();
        assert!(sections.iter().any(|n| n.name.as_deref() == Some("Decision")));
    }
}
