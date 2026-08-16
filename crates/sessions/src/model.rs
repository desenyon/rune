use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Roots used for best-effort local session discovery. Missing directories are empty, not errors.
#[derive(Clone, Debug)]
pub struct DiscoveryContext {
    pub home: PathBuf,
    pub workspace: Option<PathBuf>,
    pub extra_roots: Vec<PathBuf>,
}

impl DiscoveryContext {
    pub fn from_env() -> Self {
        Self {
            home: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            workspace: std::env::current_dir().ok(),
            extra_roots: Vec::new(),
        }
    }

    pub fn isolated(home: impl Into<PathBuf>, workspace: Option<PathBuf>) -> Self {
        Self {
            home: home.into(),
            workspace,
            extra_roots: Vec::new(),
        }
    }

    pub fn home_join(&self, parts: &[&str]) -> PathBuf {
        let mut path = self.home.clone();
        for part in parts {
            path.push(part);
        }
        path
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveredSession {
    pub provider: String,
    pub external_id: String,
    pub path: PathBuf,
    pub title: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedTurn {
    pub external_id: String,
    pub role: String,
    pub text: String,
    pub raw: serde_json::Value,
    pub timestamp: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedSession {
    pub provider: String,
    pub external_id: String,
    pub source_path: PathBuf,
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub raw: String,
    pub turns: Vec<NormalizedTurn>,
}

impl NormalizedSession {
    pub fn empty(provider: impl Into<String>, path: &Path) -> Self {
        Self {
            provider: provider.into(),
            external_id: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("session")
                .to_string(),
            source_path: path.to_path_buf(),
            title: None,
            cwd: None,
            raw: String::new(),
            turns: Vec::new(),
        }
    }
}
