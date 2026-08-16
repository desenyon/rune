use crate::error::{Result, WorktreeError};
use std::path::Path;
use std::process::Command;

pub fn git_output(repo: &Path, args: &[&str]) -> Result<String> {
    let git = which::which("git").map_err(|err| WorktreeError::GitMissing(err.to_string()))?;
    let output = Command::new(git)
        .args(["-c", "commit.gpgsign=false"])
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(WorktreeError::git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn git_status_ok(repo: &Path, args: &[&str]) -> Result<()> {
    git_output(repo, args).map(|_| ())
}

pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
        || git_output(path, &["rev-parse", "--is-inside-work-tree"])
            .map(|s| s.trim() == "true")
            .unwrap_or(false)
}
