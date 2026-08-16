//! Isolated Git worktree orchestration for agent tasks.

pub mod error;
pub mod git;
pub mod manager;

pub use error::{Result, WorktreeError};
pub use manager::{
    CreateWorktree, StaleCriteria, StaleWorktree, WorkingState, WorktreeInfo, WorktreeManager,
};

#[cfg(test)]
mod tests {
    use super::*;
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
    fn worktree_delete_without_confirm_fails() {
        let store = Store::open_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        git(tmp.path(), &["init", "--quiet"]);
        fs::write(tmp.path().join("README.md"), "x\n").unwrap();
        git(tmp.path(), &["add", "README.md"]);
        git(tmp.path(), &["commit", "-m", "init"]);
        let manager = WorktreeManager::new(store, tmp.path()).unwrap();
        let err = manager
            .remove(Path::new("/tmp/does-not-matter"), false, false)
            .unwrap_err();
        assert!(matches!(err, WorktreeError::DeleteRequiresConfirm { .. }));
    }

    #[test]
    fn create_list_and_stale_detection() {
        let store = Store::open_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        git(tmp.path(), &["init", "--quiet"]);
        fs::write(tmp.path().join("README.md"), "x\n").unwrap();
        git(tmp.path(), &["add", "README.md"]);
        git(tmp.path(), &["commit", "-m", "init"]);
        let manager = WorktreeManager::new(store, tmp.path()).unwrap();
        let wt_path = tmp.path().join("wt-task");
        let info = manager
            .create(CreateWorktree {
                path: wt_path.clone(),
                branch: "task/demo".into(),
                create_branch: true,
                base_commit: None,
                task: Some("TASK-1".into()),
                agent: Some("codex".into()),
            })
            .unwrap();
        assert_eq!(info.task.as_deref(), Some("TASK-1"));
        let listed = manager.list().unwrap();
        assert!(listed.iter().any(|w| w.path == wt_path || w.path.ends_with("wt-task")));
        let stale = manager
            .detect_stale(&StaleCriteria {
                idle_for: std::time::Duration::from_secs(0),
                no_new_commits: true,
            })
            .unwrap();
        assert!(stale.iter().any(|s| s.info.task.as_deref() == Some("TASK-1")));
        manager.remove(&wt_path, true, true).unwrap();
        assert!(!wt_path.exists());
    }
}
