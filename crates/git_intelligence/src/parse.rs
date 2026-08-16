use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitRecord {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_at: String,
    pub parents: Vec<String>,
    pub subject: String,
    pub files: Vec<FileChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileChange {
    pub status: String,
    pub path: String,
    pub previous_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefRecord {
    pub sha: String,
    pub kind: RefKind,
    pub full_name: String,
    pub short_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    Branch,
    Tag,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorktreeRecord {
    pub path: PathBuf,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkingTreeStatus {
    pub oid: Option<String>,
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub entries: Vec<StatusEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusEntry {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub untracked: bool,
}

pub fn parse_log(output: &str) -> Vec<CommitRecord> {
    let mut commits = Vec::new();
    let mut current: Option<CommitRecord> = None;
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("COMMIT\u{1f}") {
            if let Some(commit) = current.take() {
                commits.push(commit);
            }
            let parts: Vec<&str> = rest.split('\u{1f}').collect();
            if parts.len() < 6 {
                continue;
            }
            current = Some(CommitRecord {
                sha: parts[0].to_string(),
                author_name: parts[1].to_string(),
                author_email: parts[2].to_string(),
                authored_at: parts[3].to_string(),
                parents: if parts[4].is_empty() {
                    Vec::new()
                } else {
                    parts[4].split(' ').map(ToOwned::to_owned).collect()
                },
                subject: parts[5].to_string(),
                files: Vec::new(),
            });
        } else if !line.is_empty() {
            if let Some(commit) = current.as_mut() {
                if let Some(change) = parse_name_status(line) {
                    commit.files.push(change);
                }
            }
        }
    }
    if let Some(commit) = current {
        commits.push(commit);
    }
    commits
}

fn parse_name_status(line: &str) -> Option<FileChange> {
    let mut parts = line.split('\t');
    let status = parts.next()?.to_string();
    let first = parts.next()?.to_string();
    let second = parts.next().map(ToOwned::to_owned);
    if let Some(new_path) = second {
        Some(FileChange {
            status,
            path: new_path,
            previous_path: Some(first),
        })
    } else {
        Some(FileChange {
            status,
            path: first,
            previous_path: None,
        })
    }
}

pub fn parse_refs(output: &str) -> Vec<RefRecord> {
    let mut out = Vec::new();
    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\u{1}').collect();
        if parts.len() < 4 {
            continue;
        }
        let kind = if parts[2].starts_with("refs/tags/") {
            RefKind::Tag
        } else {
            RefKind::Branch
        };
        out.push(RefRecord {
            sha: parts[0].to_string(),
            kind,
            full_name: parts[2].to_string(),
            short_name: parts[3].to_string(),
        });
    }
    out
}

pub fn parse_worktrees(output: &str) -> Vec<WorktreeRecord> {
    let mut out = Vec::new();
    let mut current: Option<WorktreeRecord> = None;
    for line in output.lines() {
        if line.is_empty() {
            if let Some(item) = current.take() {
                out.push(item);
            }
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(item) = current.take() {
                out.push(item);
            }
            current = Some(WorktreeRecord {
                path: PathBuf::from(path),
                head: None,
                branch: None,
                bare: false,
                detached: false,
            });
        } else if let Some(item) = current.as_mut() {
            if let Some(head) = line.strip_prefix("HEAD ") {
                item.head = Some(head.to_string());
            } else if let Some(branch) = line.strip_prefix("branch ") {
                item.branch = Some(branch.trim_start_matches("refs/heads/").to_string());
            } else if line == "bare" {
                item.bare = true;
            } else if line == "detached" {
                item.detached = true;
            }
        }
    }
    if let Some(item) = current {
        out.push(item);
    }
    out
}

pub fn parse_status_v2(output: &str) -> WorkingTreeStatus {
    let mut status = WorkingTreeStatus::default();
    for line in output.lines() {
        if let Some(oid) = line.strip_prefix("# branch.oid ") {
            status.oid = Some(oid.to_string());
        } else if let Some(head) = line.strip_prefix("# branch.head ") {
            status.head = Some(head.to_string());
        } else if let Some(upstream) = line.strip_prefix("# branch.upstream ") {
            status.upstream = Some(upstream.to_string());
        } else if let Some(rest) = line.strip_prefix("? ") {
            status.entries.push(StatusEntry {
                path: rest.to_string(),
                status: "?".into(),
                staged: false,
                untracked: true,
            });
        } else if let Some(rest) = line.strip_prefix("1 ") {
            let mut bits = rest.split_whitespace();
            let xy = bits.next().unwrap_or(".");
            let path = bits.next_back().unwrap_or("").to_string();
            status.entries.push(StatusEntry {
                path,
                status: xy.to_string(),
                staged: !xy.starts_with('.'),
                untracked: false,
            });
        } else if let Some(rest) = line.strip_prefix("2 ") {
            let path = rest.split_whitespace().next_back().unwrap_or("").to_string();
            status.entries.push(StatusEntry {
                path,
                status: "R".into(),
                staged: true,
                untracked: false,
            });
        }
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commit_log_and_renames() {
        let raw = "\
COMMIT\u{1f}abc\u{1f}Ada\u{1f}ada@x\u{1f}2024-01-01T00:00:00Z\u{1f}\u{1f}init
A\tREADME.md
COMMIT\u{1f}def\u{1f}Ada\u{1f}ada@x\u{1f}2024-01-02T00:00:00Z\u{1f}abc\u{1f}rename
R100\told.rs\tnew.rs
";
        let commits = parse_log(raw);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[1].files[0].path, "new.rs");
        assert_eq!(commits[1].files[0].previous_path.as_deref(), Some("old.rs"));
    }
}
