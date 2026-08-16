//! Git-anchored temporal intelligence for Rune.

pub mod error;
pub mod git;
pub mod indexer;
pub mod parse;

pub use error::{GitIntelError, Result};
pub use indexer::{GitIndexReport, GitIndexer};
pub use parse::{CommitRecord, FileChange, RefRecord, StatusEntry, WorkingTreeStatus, WorktreeRecord};

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
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn git_commit_nodes_created_after_a_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        git(root, &["init", "--quiet"]);
        fs::write(root.join("hello.rs"), "fn main() {}\n").unwrap();
        git(root, &["add", "hello.rs"]);
        git(root, &["commit", "-m", "add hello"]);
        let store = Store::open_in_memory().unwrap();
        let indexer = GitIndexer::new(store, root).unwrap();
        let report = indexer.index().unwrap();
        assert!(report.commits_indexed >= 1);
        let commits = indexer.store().nodes_of_kind(NodeKind::Commit).unwrap();
        assert!(!commits.is_empty());
        assert!(commits.iter().any(|c| c.payload["subject"] == "add hello"));
        let touching = indexer.commits_touching_path("hello.rs").unwrap();
        assert_eq!(touching.len(), 1);
        let blame = indexer.blame_lite("hello.rs", Some("fn main")).unwrap();
        assert_eq!(blame.len(), 1);
        let branches = indexer.store().nodes_of_kind(NodeKind::Branch).unwrap();
        assert!(!branches.is_empty());
        let authors = indexer.store().nodes_of_kind(NodeKind::Author).unwrap();
        assert!(!authors.is_empty());
        let commit_id = commits[0].id;
        let created = indexer
            .store()
            .edges_from_kind(commit_id, rune_core::EdgeKind::CreatedBy)
            .unwrap();
        assert_eq!(created.len(), 1);
    }
}
