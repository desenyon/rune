use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Go,
    Ruby,
    Java,
    Kotlin,
    Markdown,
    Json,
    Toml,
    Yaml,
    Other,
}

impl SourceLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::Go => "go",
            Self::Ruby => "ruby",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::Markdown => "markdown",
            Self::Json => "json",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Other => "other",
        }
    }

    pub fn is_indexable(self) -> bool {
        matches!(
            self,
            Self::Rust | Self::Python | Self::JavaScript | Self::TypeScript | Self::Tsx | Self::Go
        )
    }
}

pub fn language_from_path(path: &Path) -> Option<SourceLanguage> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if name == "makefile" || name == "cmakelists.txt" {
        return Some(SourceLanguage::Other);
    }
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" => SourceLanguage::Rust,
        "py" | "pyi" => SourceLanguage::Python,
        "js" | "jsx" | "mjs" | "cjs" => SourceLanguage::JavaScript,
        "ts" => SourceLanguage::TypeScript,
        "tsx" => SourceLanguage::Tsx,
        "go" => SourceLanguage::Go,
        "rb" => SourceLanguage::Ruby,
        "java" => SourceLanguage::Java,
        "kt" | "kts" => SourceLanguage::Kotlin,
        "md" => SourceLanguage::Markdown,
        "json" => SourceLanguage::Json,
        "toml" => SourceLanguage::Toml,
        "yml" | "yaml" => SourceLanguage::Yaml,
        _ => SourceLanguage::Other,
    })
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LanguageCensus {
    pub languages: BTreeSet<String>,
    pub package_managers: BTreeSet<String>,
    pub build_systems: BTreeSet<String>,
    pub test_frameworks: BTreeSet<String>,
}

pub fn apply_marker_file(path: &Path, census: &mut LanguageCensus) {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "cargo.toml" => {
            census.languages.insert("rust".into());
            census.package_managers.insert("cargo".into());
            census.build_systems.insert("cargo".into());
            census.test_frameworks.insert("cargo_test".into());
        }
        "cargo.lock" => {
            census.package_managers.insert("cargo".into());
        }
        "package.json" => {
            census.languages.insert("javascript".into());
            census.package_managers.insert("npm".into());
            census.build_systems.insert("npm".into());
        }
        "package-lock.json" => {
            census.package_managers.insert("npm".into());
        }
        "yarn.lock" => {
            census.package_managers.insert("yarn".into());
        }
        "pnpm-lock.yaml" | "pnpm-workspace.yaml" => {
            census.package_managers.insert("pnpm".into());
        }
        "bun.lock" | "bun.lockb" => {
            census.package_managers.insert("bun".into());
        }
        "go.mod" | "go.sum" | "go.work" => {
            census.languages.insert("go".into());
            census.package_managers.insert("go_modules".into());
            census.build_systems.insert("go".into());
            census.test_frameworks.insert("go_test".into());
        }
        "pyproject.toml" => {
            census.languages.insert("python".into());
            census.package_managers.insert("pip".into());
        }
        "requirements.txt" => {
            census.languages.insert("python".into());
            census.package_managers.insert("pip".into());
        }
        "poetry.lock" => {
            census.package_managers.insert("poetry".into());
        }
        "uv.lock" => {
            census.package_managers.insert("uv".into());
        }
        "pipfile" | "pipfile.lock" => {
            census.package_managers.insert("pipenv".into());
        }
        "gemfile" | "gemfile.lock" => {
            census.languages.insert("ruby".into());
            census.package_managers.insert("bundler".into());
        }
        "pom.xml" => {
            census.languages.insert("java".into());
            census.package_managers.insert("maven".into());
            census.build_systems.insert("maven".into());
        }
        "build.gradle" | "build.gradle.kts" | "settings.gradle" | "settings.gradle.kts" => {
            census.package_managers.insert("gradle".into());
            census.build_systems.insert("gradle".into());
        }
        "makefile" | "gnumakefile" => {
            census.build_systems.insert("make".into());
        }
        "cmakelists.txt" => {
            census.build_systems.insert("cmake".into());
        }
        "meson.build" => {
            census.build_systems.insert("meson".into());
        }
        "workspace" | "workspace.bazel" | "module.bazel" | "build" | "build.bazel" => {
            census.build_systems.insert("bazel".into());
        }
        "lerna.json" | "nx.json" => {}
        "tsconfig.json" => {
            census.languages.insert("typescript".into());
        }
        _ => {}
    }
}

pub fn apply_file_name_tests(path: &Path, census: &mut LanguageCensus) {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    if name.ends_with("_test.go") {
        census.test_frameworks.insert("go_test".into());
    }
    if name.starts_with("test_") && name.ends_with(".py") || name.ends_with("_test.py") {
        census.test_frameworks.insert("pytest".into());
    }
    if name.ends_with(".test.ts")
        || name.ends_with(".test.js")
        || name.ends_with(".spec.ts")
        || name.ends_with(".spec.js")
    {
        census.test_frameworks.insert("jest".into());
    }
}

pub fn inspect_marker_contents(path: &Path, census: &mut LanguageCensus) {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let Ok(contents) = std::fs::read_to_string(path) else {
        return;
    };
    match name {
        "package.json" => {
            if contents.contains("\"jest\"") {
                census.test_frameworks.insert("jest".into());
            }
            if contents.contains("\"vitest\"") {
                census.test_frameworks.insert("vitest".into());
            }
            if contents.contains("\"mocha\"") {
                census.test_frameworks.insert("mocha".into());
            }
            if contents.contains("\"workspaces\"") {
                census.build_systems.insert("npm_workspaces".into());
            }
        }
        "pyproject.toml" | "requirements.txt" => {
            if contents.contains("pytest") {
                census.test_frameworks.insert("pytest".into());
            }
        }
        "Cargo.toml" => {
            if contents.contains("[workspace]") {
                census.build_systems.insert("cargo_workspace".into());
            }
        }
        _ => {}
    }
}

pub fn is_docs_dir(name: &str) -> bool {
    matches!(name, "docs" | "documentation" | "doc")
}

pub fn is_specs_dir(name: &str) -> bool {
    matches!(name, "spec" | "specs" | "specifications")
}

pub fn is_agent_config(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    match name {
        ".claude" => Some(".claude"),
        ".cursor" => Some(".cursor"),
        ".aider" | ".aider.conf.yml" | ".aiderignore" => Some(".aider"),
        ".codex" => Some(".codex"),
        "GEMINI.md" => Some("GEMINI.md"),
        "AGENTS.md" => Some("AGENTS.md"),
        ".opencode" => Some(".opencode"),
        _ => None,
    }
}

pub fn is_storm_dir(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some("target" | "node_modules" | ".git" | "dist" | "build" | ".next" | "__pycache__" | ".venv" | "coverage")
    )
}

pub fn path_has_storm_component(root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return path.components().any(|c| is_storm_dir(c.as_os_str()));
    };
    rel.components().any(|c| is_storm_dir(c.as_os_str()))
}

pub fn relative_posix(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn file_key(root: &Path, path: &Path) -> String {
    rune_core::ContentHash::hash(format!("file:{}", relative_posix(root, path)).as_bytes()).to_hex()
}

pub fn looks_like_temp_write(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.ends_with('~')
        || name.ends_with(".swp")
        || name.ends_with(".swo")
        || name.ends_with(".tmp")
        || name.ends_with(".temp")
        || name.starts_with(".#")
        || name == ".DS_Store"
}

pub fn detect_monorepo_kind(root: &Path) -> Option<MonorepoKind> {
    let cargo = root.join("Cargo.toml");
    if cargo.is_file() {
        if let Ok(text) = std::fs::read_to_string(&cargo) {
            if text.contains("[workspace]") {
                return Some(MonorepoKind::CargoWorkspace);
            }
        }
    }
    let pkg = root.join("package.json");
    if pkg.is_file() {
        if let Ok(text) = std::fs::read_to_string(&pkg) {
            if text.contains("\"workspaces\"") {
                if root.join("pnpm-workspace.yaml").is_file() {
                    return Some(MonorepoKind::PnpmWorkspace);
                }
                if root.join("yarn.lock").is_file() {
                    return Some(MonorepoKind::YarnWorkspace);
                }
                return Some(MonorepoKind::NpmWorkspace);
            }
        }
    }
    if root.join("pnpm-workspace.yaml").is_file() {
        return Some(MonorepoKind::PnpmWorkspace);
    }
    if root.join("go.work").is_file() {
        return Some(MonorepoKind::GoWork);
    }
    if root.join("lerna.json").is_file() {
        return Some(MonorepoKind::Lerna);
    }
    if root.join("nx.json").is_file() {
        return Some(MonorepoKind::Nx);
    }
    if root.join("WORKSPACE").is_file()
        || root.join("WORKSPACE.bazel").is_file()
        || root.join("MODULE.bazel").is_file()
    {
        return Some(MonorepoKind::Bazel);
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonorepoKind {
    CargoWorkspace,
    NpmWorkspace,
    PnpmWorkspace,
    YarnWorkspace,
    GoWork,
    Lerna,
    Nx,
    Bazel,
}

impl MonorepoKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CargoWorkspace => "cargo_workspace",
            Self::NpmWorkspace => "npm_workspace",
            Self::PnpmWorkspace => "pnpm_workspace",
            Self::YarnWorkspace => "yarn_workspace",
            Self::GoWork => "go_work",
            Self::Lerna => "lerna",
            Self::Nx => "nx",
            Self::Bazel => "bazel",
        }
    }
}

pub fn git_dir_at(path: &Path) -> bool {
    path.join(".git").exists()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorktreeListing {
    pub path: PathBuf,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
}

pub fn parse_worktree_porcelain(output: &str) -> Vec<WorktreeListing> {
    let mut out = Vec::new();
    let mut current: Option<WorktreeListing> = None;
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
            current = Some(WorktreeListing {
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
