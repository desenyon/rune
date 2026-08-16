use crate::error::{IndexError, Result};
use crate::languages::{
    apply_file_name_tests, apply_marker_file, detect_monorepo_kind, git_dir_at, inspect_marker_contents,
    is_agent_config, is_docs_dir, is_specs_dir, is_storm_dir, language_from_path, parse_worktree_porcelain,
    LanguageCensus, MonorepoKind, WorktreeListing,
};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceDiscovery {
    pub root: PathBuf,
    pub is_git: bool,
    pub nested_repos: Vec<PathBuf>,
    pub worktrees: Vec<WorktreeListing>,
    pub monorepo: Option<MonorepoKind>,
    pub languages: Vec<String>,
    pub package_managers: Vec<String>,
    pub build_systems: Vec<String>,
    pub test_frameworks: Vec<String>,
    pub agent_configs: Vec<String>,
    pub docs_dirs: Vec<PathBuf>,
    pub spec_dirs: Vec<PathBuf>,
    pub rune_state: Option<PathBuf>,
    pub files: Vec<DiscoveredFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub relative: String,
    pub content_hash: String,
    pub bytes: u64,
    pub language: Option<String>,
}

pub fn discover(root: &Path) -> Result<WorkspaceDiscovery> {
    if !root.is_dir() {
        return Err(IndexError::NotADirectory(root.to_path_buf()));
    }
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let is_git = git_dir_at(&root);
    let mut nested_repos = Vec::new();
    let mut census = LanguageCensus::default();
    let mut agent_configs = Vec::new();
    let mut docs_dirs = Vec::new();
    let mut spec_dirs = Vec::new();
    let mut files = Vec::new();
    let rune_state = root.join(".rune").exists().then(|| root.join(".rune"));
    let monorepo = detect_monorepo_kind(&root);

    let walker = WalkBuilder::new(&root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .follow_links(false)
        .filter_entry(|entry| {
            let name = entry.file_name();
            if name == ".git" {
                return false;
            }
            !is_storm_dir(name)
        })
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                tracing::warn!(error = %err, "skipping unreadable walk entry");
                continue;
            }
        };
        let path = entry.path();
        if path == root {
            continue;
        }
        if entry.file_type().is_some_and(|t| t.is_dir()) {
            if git_dir_at(path) && path != root {
                nested_repos.push(path.to_path_buf());
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if is_docs_dir(name) {
                    docs_dirs.push(path.to_path_buf());
                }
                if is_specs_dir(name) {
                    spec_dirs.push(path.to_path_buf());
                }
            }
            if let Some(config) = is_agent_config(path) {
                push_unique(&mut agent_configs, config.to_string());
            }
            continue;
        }
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if let Some(config) = is_agent_config(path) {
            push_unique(&mut agent_configs, config.to_string());
        }
        apply_marker_file(path, &mut census);
        apply_file_name_tests(path, &mut census);
        inspect_marker_contents(path, &mut census);
        if let Some(lang) = language_from_path(path) {
            if lang != crate::languages::SourceLanguage::Other {
                census.languages.insert(lang.as_str().to_string());
            }
        }
        let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let hash = hash_file(path)?;
        files.push(DiscoveredFile {
            relative: crate::languages::relative_posix(&root, path),
            path: path.to_path_buf(),
            content_hash: hash,
            bytes,
            language: language_from_path(path).map(|l| l.as_str().to_string()),
        });
    }

    nested_repos.sort();
    nested_repos.dedup();
    docs_dirs.sort();
    docs_dirs.dedup();
    spec_dirs.sort();
    spec_dirs.dedup();
    agent_configs.sort();
    agent_configs.dedup();
    files.sort_by(|a, b| a.relative.cmp(&b.relative));

    let worktrees = if is_git {
        list_worktrees(&root).unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(WorkspaceDiscovery {
        root,
        is_git,
        nested_repos,
        worktrees,
        monorepo,
        languages: census.languages.into_iter().collect(),
        package_managers: census.package_managers.into_iter().collect(),
        build_systems: census.build_systems.into_iter().collect(),
        test_frameworks: census.test_frameworks.into_iter().collect(),
        agent_configs,
        docs_dirs,
        spec_dirs,
        rune_state,
        files,
    })
}

pub fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(rune_core::ContentHash::hash(&bytes).to_hex())
}

pub fn list_worktrees(root: &Path) -> Result<Vec<WorktreeListing>> {
    let git = which::which("git").map_err(|err| IndexError::git(err.to_string()))?;
    let output = Command::new(git)
        .args(["-C", &root.to_string_lossy(), "worktree", "list", "--porcelain"])
        .output()?;
    if !output.status.success() {
        return Err(IndexError::git(String::from_utf8_lossy(&output.stderr).trim().to_string()));
    }
    Ok(parse_worktree_porcelain(&String::from_utf8_lossy(&output.stdout)))
}

fn push_unique(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|item| item == &value) {
        items.push(value);
    }
}

#[cfg(test)]
mod tests {
    use crate::languages::parse_worktree_porcelain;

    #[test]
    fn parses_porcelain_worktrees() {
        let raw = "\
worktree /repo
HEAD abcdef
branch refs/heads/main

worktree /repo-feat
HEAD 123456
detached
";
        let listed = parse_worktree_porcelain(raw);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].branch.as_deref(), Some("main"));
        assert!(listed[1].detached);
    }
}
