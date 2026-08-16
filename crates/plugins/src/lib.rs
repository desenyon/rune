//! Permission-declared plugins. Default: no unrestricted filesystem or process access.

use rune_providers::ProviderKind;
use rune_security::{Permission, Policy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("permission denied: {0:?}")]
    Denied(Permission),
    #[error("invalid plugin at `{path}`: {message}")]
    Invalid { path: String, message: String },
    #[error("plugin `{0}` is not allowed to execute processes")]
    ProcessForbidden(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, PluginError>;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginContributions {
    #[serde(default)]
    pub providers: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub search_sources: Vec<String>,
    #[serde(default)]
    pub node_types: Vec<String>,
    #[serde(default)]
    pub edge_types: Vec<String>,
    #[serde(default)]
    pub renderers: Vec<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub session_adapters: Vec<String>,
    #[serde(default)]
    pub documentation_adapters: Vec<String>,
    #[serde(default)]
    pub agent_adapters: Vec<String>,
    #[serde(default)]
    pub exporters: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub contributions: PluginContributions,
    #[serde(default)]
    pub permissions: BTreeSet<Permission>,
}

#[derive(Clone, Debug)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub directory: PathBuf,
}

#[derive(Debug, Default)]
pub struct LoadReport {
    pub loaded: Vec<LoadedPlugin>,
    pub failures: Vec<PluginError>,
}

impl PluginManifest {
    pub fn validate(&self, path: &Path) -> Result<()> {
        if self.id.is_empty() {
            return Err(invalid(path, "plugin id is required"));
        }
        if self
            .id
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        {
            return Err(invalid(
                path,
                "plugin id must be alphanumeric, hyphen, or underscore",
            ));
        }
        if self.id.contains("..") || self.id.contains('/') || self.id.contains('\\') {
            return Err(invalid(path, "plugin id must not contain path components"));
        }
        Ok(())
    }

    pub fn may_execute_process(&self) -> bool {
        self.permissions.contains(&Permission::ProcessExecute)
    }
}

fn invalid(path: &Path, message: impl Into<String>) -> PluginError {
    PluginError::Invalid {
        path: path.display().to_string(),
        message: message.into(),
    }
}

pub fn load_dir(plugin_dir: &Path, policy: &Policy) -> Result<LoadReport> {
    if !policy.permits(Permission::PluginLoad) {
        return Err(PluginError::Denied(Permission::PluginLoad));
    }
    let mut report = LoadReport::default();
    if !plugin_dir.exists() {
        return Ok(report);
    }
    let root = fs::canonicalize(plugin_dir)?;
    for entry in fs::read_dir(&root)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                report.failures.push(err.into());
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            match load_plugin_dir(&root, &path) {
                Ok(plugin) => report.loaded.push(plugin),
                Err(err) => report.failures.push(err),
            }
        } else if matches!(
            path.file_name().and_then(|n| n.to_str()),
            Some("plugin.json" | "manifest.json")
        ) {
            match load_manifest_file(&root, &path) {
                Ok(plugin) => report.loaded.push(plugin),
                Err(err) => report.failures.push(err),
            }
        }
    }
    Ok(report)
}

fn load_plugin_dir(root: &Path, dir: &Path) -> Result<LoadedPlugin> {
    let canonical = fs::canonicalize(dir)?;
    if !canonical.starts_with(root) {
        return Err(invalid(
            dir,
            "plugin directory escapes configured plugin root",
        ));
    }
    let manifest_path = if canonical.join("plugin.json").is_file() {
        canonical.join("plugin.json")
    } else if canonical.join("manifest.json").is_file() {
        canonical.join("manifest.json")
    } else {
        return Err(invalid(&canonical, "missing plugin.json or manifest.json"));
    };
    load_manifest_file(root, &manifest_path)
}

fn load_manifest_file(root: &Path, path: &Path) -> Result<LoadedPlugin> {
    let canonical = fs::canonicalize(path)?;
    if !canonical.starts_with(root) {
        return Err(invalid(
            path,
            "manifest path escapes configured plugin root",
        ));
    }
    let text = fs::read_to_string(&canonical)?;
    let manifest: PluginManifest =
        serde_json::from_str(&text).map_err(|err| invalid(&canonical, err.to_string()))?;
    manifest.validate(&canonical)?;
    Ok(LoadedPlugin {
        directory: canonical.parent().unwrap_or(root).to_path_buf(),
        manifest,
    })
}

impl LoadedPlugin {
    pub fn kind(&self) -> ProviderKind {
        ProviderKind::Plugin
    }

    pub fn execute_process(
        &self,
        policy: &Policy,
        program: &Path,
        args: &[&str],
    ) -> Result<std::process::Output> {
        if !self.manifest.may_execute_process() {
            return Err(PluginError::ProcessForbidden(self.manifest.id.clone()));
        }
        if !policy.permits(Permission::ProcessExecute) {
            return Err(PluginError::Denied(Permission::ProcessExecute));
        }
        let output = std::process::Command::new(program)
            .args(args)
            .current_dir(&self.directory)
            .output()?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn allow_plugin_load() -> Policy {
        let mut policy = Policy::local_default();
        policy.grant(Permission::PluginLoad);
        policy
    }

    #[test]
    fn plugin_without_permissions_cannot_execute_process() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("sample");
        fs::create_dir(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"id":"sample","contributions":{"commands":["hello"]},"permissions":[]}"#,
        )
        .unwrap();
        let report = load_dir(dir.path(), &allow_plugin_load()).unwrap();
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        assert_eq!(report.loaded.len(), 1);
        let plugin = &report.loaded[0];
        assert!(!plugin.manifest.may_execute_process());
        let err = plugin
            .execute_process(&allow_plugin_load(), Path::new("/bin/echo"), &["hi"])
            .unwrap_err();
        assert!(matches!(err, PluginError::ProcessForbidden(id) if id == "sample"));
    }

    #[test]
    fn invalid_plugin_fails_clearly_without_corrupting_others() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good");
        let bad = dir.path().join("bad");
        fs::create_dir(&good).unwrap();
        fs::create_dir(&bad).unwrap();
        fs::write(
            good.join("plugin.json"),
            r#"{"id":"good","permissions":[]}"#,
        )
        .unwrap();
        fs::write(bad.join("plugin.json"), "{not json").unwrap();
        let report = load_dir(dir.path(), &allow_plugin_load()).unwrap();
        assert_eq!(report.loaded.len(), 1);
        assert_eq!(report.loaded[0].manifest.id, "good");
        assert_eq!(report.failures.len(), 1);
    }

    #[test]
    fn default_policy_cannot_load_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_dir(dir.path(), &Policy::local_default()).unwrap_err();
        assert!(matches!(err, PluginError::Denied(Permission::PluginLoad)));
    }
}
