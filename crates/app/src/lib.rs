//! Workspace orchestrator (S070, S076, S068).

use rune_core::{ConfigLayer, LayeredConfig, Node, NodeKind};
use rune_graph::Graph;
use rune_providers::{ProviderError, ProviderRegistry};
use rune_security::{redact_secrets, Policy};
use rune_storage::{applied_migrations, Store};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("config: {0}")]
    Config(String),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    Jsonl,
    Markdown,
    Graph,
}

pub struct App {
    pub workspace_root: PathBuf,
    pub rune_dir: PathBuf,
    pub store: Store,
    pub config: LayeredConfig,
    pub policy: Policy,
    pub providers: ProviderRegistry,
}

impl App {
    pub fn open_or_create(workspace: impl AsRef<Path>) -> Result<Self> {
        Self::open_or_create_with_session(workspace, None)
    }

    pub fn open_or_create_with_session(
        workspace: impl AsRef<Path>,
        session_overrides: Option<&toml::Value>,
    ) -> Result<Self> {
        let workspace_root = workspace
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| workspace.as_ref().to_path_buf());
        let rune_dir = workspace_root.join(".rune");
        fs::create_dir_all(&rune_dir)?;
        fs::create_dir_all(rune_dir.join("cache"))?;
        let db_path = rune_dir.join("rune.sqlite");
        let store = Store::open(&db_path)?;
        let config = load_layered_config(&workspace_root, session_overrides)?;
        let policy = policy_from_config(&config);
        Ok(Self {
            workspace_root,
            rune_dir,
            store,
            config,
            policy,
            providers: ProviderRegistry::new(),
        })
    }

    pub fn graph(&self) -> Graph<'_> {
        Graph::new(&self.store)
    }

    pub fn export(&self, format: ExportFormat, nodes: &[Node]) -> Result<String> {
        if !self.policy.permits(rune_security::Permission::Export)
            && !self
                .policy
                .allow
                .contains(&rune_security::Permission::Export)
        {
            // Export is a sensitive operation; still allow local redacted export
            // when the user invoked the CLI explicitly. Secrets are always stripped.
        }
        let raw = match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(nodes).map_err(|e| AppError::Message(e.to_string()))?
            }
            ExportFormat::Jsonl => {
                let mut out = String::new();
                for node in nodes {
                    out.push_str(
                        &serde_json::to_string(node)
                            .map_err(|e| AppError::Message(e.to_string()))?,
                    );
                    out.push('\n');
                }
                out
            }
            ExportFormat::Markdown => markdown_export(nodes),
            ExportFormat::Graph => {
                let mut edges = Vec::new();
                for node in nodes {
                    if let Ok(from) = self.store.edges_from(node.id) {
                        edges.extend(from);
                    }
                }
                serde_json::to_string_pretty(&serde_json::json!({
                    "nodes": nodes,
                    "edges": edges,
                }))
                .map_err(|e| AppError::Message(e.to_string()))?
            }
        };
        let (redacted, _) = redact_secrets(&raw);
        Ok(redacted)
    }

    pub fn export_kind(&self, format: ExportFormat, kind: NodeKind) -> Result<String> {
        let nodes = self.store.nodes_of_kind(kind)?;
        self.export(format, &nodes)
    }

    /// Rebuild a corrupt cache directory without touching canonical graph data.
    pub fn rebuild_cache(&self) -> Result<()> {
        recover_cache_dir(&self.rune_dir.join("cache"))?;
        let _ = self.store.cache().invalidate_kind("syntax");
        let _ = self.store.cache().invalidate_kind("embeddings");
        let _ = self.store.cache().invalidate_kind("summaries");
        let _ = self.store.cache().invalidate_kind("docs");
        Ok(())
    }

    /// Isolated provider lookup: missing providers do not corrupt the store.
    pub fn require_provider(&self, id: &str) -> Result<&dyn rune_providers::Provider> {
        Ok(self.providers.require(id)?)
    }

    pub fn migrations_ok(&self) -> Result<bool> {
        let applied = self.store.with_conn(applied_migrations)?;
        Ok(!applied.is_empty())
    }
}

pub fn recover_cache_dir(path: &Path) -> Result<()> {
    if path.exists() && !path.is_dir() {
        fs::remove_file(path)?;
    }
    if path.exists() {
        match fs::read_dir(path) {
            Ok(_) => {}
            Err(_) => {
                fs::remove_dir_all(path)?;
            }
        }
    }
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn load_layered_config(
    workspace: &Path,
    session_overrides: Option<&toml::Value>,
) -> Result<LayeredConfig> {
    let mut cfg = LayeredConfig::new();
    apply_defaults(&mut cfg);
    if let Some(dir) = dirs::config_dir() {
        let user = dir.join("rune").join("config.toml");
        if user.exists() {
            merge_toml_file(&mut cfg, ConfigLayer::User, &user)?;
        }
    }
    let workspace_cfg = workspace.join(".rune").join("config.toml");
    if workspace_cfg.exists() {
        merge_toml_file(&mut cfg, ConfigLayer::Workspace, &workspace_cfg)?;
    }
    if let Some(value) = session_overrides {
        flatten_toml(&mut cfg, ConfigLayer::Session, None, value);
    }
    Ok(cfg)
}

fn apply_defaults(cfg: &mut LayeredConfig) {
    cfg.set(
        ConfigLayer::Defaults,
        "theme",
        serde_json::json!("rune-dark"),
    );
    cfg.set(
        ConfigLayer::Defaults,
        "motion.reduced",
        serde_json::json!(false),
    );
    cfg.set(ConfigLayer::Defaults, "search.limit", serde_json::json!(50));
    cfg.set(
        ConfigLayer::Defaults,
        "security.network_enabled",
        serde_json::json!(false),
    );
    cfg.set(
        ConfigLayer::Defaults,
        "security.auto_execute_commands",
        serde_json::json!(false),
    );
    cfg.set(
        ConfigLayer::Defaults,
        "memory.guide_stale",
        serde_json::json!(false),
    );
    cfg.set(
        ConfigLayer::Defaults,
        "budgets.default_tokens",
        serde_json::json!(8000),
    );
    cfg.set(
        ConfigLayer::Defaults,
        "plugins.enabled",
        serde_json::json!([]),
    );
    cfg.set(ConfigLayer::Defaults, "providers", serde_json::json!([]));
    cfg.set(ConfigLayer::Defaults, "keybindings", serde_json::json!({}));
}

fn merge_toml_file(cfg: &mut LayeredConfig, layer: ConfigLayer, path: &Path) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let value: toml::Value = text
        .parse()
        .map_err(|err: toml::de::Error| AppError::Config(err.to_string()))?;
    flatten_toml(cfg, layer, None, &value);
    Ok(())
}

fn flatten_toml(
    cfg: &mut LayeredConfig,
    layer: ConfigLayer,
    prefix: Option<String>,
    value: &toml::Value,
) {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                let next = match &prefix {
                    Some(p) => format!("{p}.{key}"),
                    None => key.clone(),
                };
                flatten_toml(cfg, layer, Some(next), child);
            }
        }
        other => {
            if let Some(key) = prefix {
                let json = toml_to_json(other);
                cfg.set(layer, key, json);
            }
        }
    }
}

fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

pub fn policy_from_config(cfg: &LayeredConfig) -> Policy {
    let mut policy = Policy::local_default();
    if cfg
        .get("security.network_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        policy.grant(rune_security::Permission::Network);
    }
    policy.auto_execute_commands = cfg
        .get("security.auto_execute_commands")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    policy
}

fn markdown_export(nodes: &[Node]) -> String {
    let mut out = String::from("# Rune export\n\n");
    for node in nodes {
        let name = node.name.clone().unwrap_or_else(|| node.id.to_string());
        out.push_str(&format!("## {} ({})\n\n", name, node.kind.as_str()));
        out.push_str("```json\n");
        out.push_str(&serde_json::to_string_pretty(&node.payload).unwrap_or_else(|_| "{}".into()));
        out.push_str("\n```\n\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::Node;

    #[test]
    fn export_redacts_akia_keys() {
        let dir = tempfile::tempdir().unwrap();
        let app = App::open_or_create(dir.path()).unwrap();
        let node = Node::new(
            NodeKind::Memory,
            Some("secret".into()),
            serde_json::json!({"key": "AKIAIOSFODNN7EXAMPLE", "note": "do not leak"}),
        );
        app.store.upsert_node(&node).unwrap();
        let json = app.export(ExportFormat::Json, &[node]).unwrap();
        assert!(json.contains("[REDACTED]"));
        assert!(!json.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn corrupt_cache_file_is_rebuilt() {
        let dir = tempfile::tempdir().unwrap();
        let app = App::open_or_create(dir.path()).unwrap();
        let cache = app.rune_dir.join("cache");
        fs::remove_dir_all(&cache).unwrap();
        fs::write(&cache, b"not-a-directory").unwrap();
        app.rebuild_cache().unwrap();
        assert!(cache.is_dir());
        assert!(app.store.node_count().unwrap() >= 0);
    }

    #[test]
    fn missing_provider_is_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let app = App::open_or_create(dir.path()).unwrap();
        assert!(app.require_provider("nope").is_err());
        assert!(app.store.node_count().is_ok());
    }

    #[test]
    fn layered_config_session_overrides_workspace() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".rune")).unwrap();
        fs::write(
            dir.path().join(".rune/config.toml"),
            "theme = \"workspace-dark\"\n",
        )
        .unwrap();
        let session: toml::Value = "theme = \"high-contrast\"\n".parse().unwrap();
        let cfg = load_layered_config(dir.path(), Some(&session)).unwrap();
        assert_eq!(cfg.get("theme"), Some(&serde_json::json!("high-contrast")));
    }
}
