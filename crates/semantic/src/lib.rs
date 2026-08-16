//! Semantic repository understanding with a pluggable, optionally disabled provider.

use async_trait::async_trait;
use rune_core::{Fingerprint, Node, NodeId, NodeKind, Validity};
use rune_providers::{
    deny_if_missing, Capability, Provider, ProviderError, ProviderIdentity, ProviderKind,
    ProviderRequest, ProviderResponse, Result as ProviderResult,
};
use rune_security::{Permission, Policy};
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum SemanticError {
    #[error("semantic provider is disabled")]
    Disabled,
    #[error("semantic provider `{0}` is unavailable: {1}")]
    Unavailable(String, String),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SemanticError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticMode {
    LocalEmbed,
    RemoteEmbed,
    LocalLlm,
    RemoteLlm,
    Disabled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticComponent {
    pub structural_node_id: NodeId,
    pub purpose: String,
    pub responsibilities: Vec<String>,
    pub important_behavior: Vec<String>,
    pub dependencies: Vec<NodeId>,
    pub dependents: Vec<NodeId>,
    pub constraints: Vec<String>,
    pub risk_areas: Vec<String>,
    pub related_tests: Vec<NodeId>,
    pub related_decisions: Vec<NodeId>,
    pub historical_changes: Vec<String>,
    pub supporting_fingerprint: Fingerprint,
    pub validity: Validity,
}

impl SemanticComponent {
    pub fn manual(
        structural_node_id: NodeId,
        purpose: impl Into<String>,
        fingerprint: Fingerprint,
    ) -> Self {
        Self {
            structural_node_id,
            purpose: purpose.into(),
            responsibilities: Vec::new(),
            important_behavior: Vec::new(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            constraints: Vec::new(),
            risk_areas: Vec::new(),
            related_tests: Vec::new(),
            related_decisions: Vec::new(),
            historical_changes: Vec::new(),
            supporting_fingerprint: fingerprint,
            validity: Validity::Candidate,
        }
    }

    pub fn stale_if_fingerprint_changed(&mut self, current: &Fingerprint) -> bool {
        if self.supporting_fingerprint.hash != current.hash {
            self.validity = Validity::Stale;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcessBackend {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl ProcessBackend {
    pub async fn run_json(&self, payload: &serde_json::Value) -> Result<serde_json::Value> {
        if !self.program.exists() {
            return Err(SemanticError::Unavailable(
                self.program.display().to_string(),
                "program not found".into(),
            ));
        }
        let mut child = Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(payload.to_string().as_bytes()).await?;
        }
        let output = child.wait_with_output().await?;
        if !output.status.success() {
            return Err(SemanticError::Unavailable(
                self.program.display().to_string(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }
}

#[derive(Clone, Debug)]
pub struct SemanticEngine {
    pub mode: SemanticMode,
    pub embedder: Option<ProcessBackend>,
    pub completer: Option<ProcessBackend>,
}

impl Default for SemanticEngine {
    fn default() -> Self {
        Self::disabled()
    }
}

impl SemanticEngine {
    pub fn disabled() -> Self {
        Self {
            mode: SemanticMode::Disabled,
            embedder: None,
            completer: None,
        }
    }

    pub fn capabilities(&self) -> BTreeSet<Capability> {
        let mut caps = BTreeSet::new();
        match self.mode {
            SemanticMode::Disabled => {}
            SemanticMode::LocalEmbed => {
                caps.insert(Capability::Embed);
            }
            SemanticMode::RemoteEmbed => {
                if self.embedder.is_some() {
                    caps.insert(Capability::Embed);
                }
            }
            SemanticMode::LocalLlm | SemanticMode::RemoteLlm => {
                if self.completer.is_some() {
                    caps.insert(Capability::Complete);
                }
            }
        }
        caps
    }

    pub async fn embed(&self, texts: &[String], policy: &Policy) -> Result<Vec<Vec<f32>>> {
        if self.mode == SemanticMode::Disabled {
            return Err(SemanticError::Disabled);
        }
        if self.embedder.is_none() && matches!(self.mode, SemanticMode::LocalEmbed) {
            return Ok(texts.iter().map(|t| hash_embed(t)).collect());
        }
        if self.embedder.is_none() {
            return Err(SemanticError::Disabled);
        }
        if !policy.permits(Permission::ProcessExecute) {
            return Err(SemanticError::Provider(ProviderError::Permission(
                Permission::ProcessExecute,
            )));
        }
        if matches!(self.mode, SemanticMode::RemoteEmbed) && !policy.permits(Permission::Network) {
            return Err(SemanticError::Provider(ProviderError::Permission(
                Permission::Network,
            )));
        }
        let backend = self.embedder.as_ref().unwrap();
        let response = backend
            .run_json(&serde_json::json!({"texts": texts}))
            .await?;
        let embeddings = response
            .get("embeddings")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                SemanticError::Unavailable("embed".into(), "response missing embeddings".into())
            })?;
        embeddings
            .iter()
            .map(|row| {
                row.as_array()
                    .ok_or_else(|| {
                        SemanticError::Unavailable("embed".into(), "invalid vector".into())
                    })
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(|v| v.as_f64().map(|n| n as f32))
                            .collect()
                    })
            })
            .collect()
    }

    pub async fn complete(&self, prompt: &str, policy: &Policy) -> Result<String> {
        if self.mode == SemanticMode::Disabled || self.completer.is_none() {
            return Err(SemanticError::Disabled);
        }
        if !policy.permits(Permission::ProcessExecute) {
            return Err(SemanticError::Provider(ProviderError::Permission(
                Permission::ProcessExecute,
            )));
        }
        if matches!(self.mode, SemanticMode::RemoteLlm) && !policy.permits(Permission::Network) {
            return Err(SemanticError::Provider(ProviderError::Permission(
                Permission::Network,
            )));
        }
        let backend = self.completer.as_ref().unwrap();
        let response = backend
            .run_json(&serde_json::json!({"prompt": prompt}))
            .await?;
        Ok(response
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }
}

/// Local hashing embedder used when no process backend is configured.
fn hash_embed(text: &str) -> Vec<f32> {
    const DIM: usize = 256;
    let mut vec = vec![0f32; DIM];
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    if bytes.len() < 3 {
        for b in bytes {
            vec[(*b as usize) % DIM] += 1.0;
        }
    } else {
        for window in bytes.windows(3) {
            let mut h = 0x811c9dc5u32;
            for b in window {
                h ^= u32::from(*b);
                h = h.wrapping_mul(0x01000193);
            }
            vec[(h as usize) % DIM] += 1.0;
        }
    }
    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vec {
            *v /= norm;
        }
    }
    vec
}

#[async_trait]
impl Provider for SemanticEngine {
    fn identity(&self) -> ProviderIdentity {
        ProviderIdentity {
            id: "semantic".into(),
            name: "Semantic provider".into(),
            version: None,
            kind: ProviderKind::Semantic,
        }
    }

    fn capabilities(&self) -> BTreeSet<Capability> {
        SemanticEngine::capabilities(self)
    }

    fn required_permissions(&self) -> BTreeSet<Permission> {
        let mut set = BTreeSet::new();
        if self.embedder.is_some() || self.completer.is_some() {
            set.insert(Permission::ProcessExecute);
        }
        if matches!(
            self.mode,
            SemanticMode::RemoteEmbed | SemanticMode::RemoteLlm
        ) {
            set.insert(Permission::Network);
        }
        set
    }

    async fn invoke(
        &self,
        request: ProviderRequest,
        policy: &Policy,
    ) -> ProviderResult<ProviderResponse> {
        deny_if_missing(self, &request.capability)?;
        match request.capability {
            Capability::Embed => {
                let texts = request
                    .payload
                    .get("texts")
                    .and_then(|v| v.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let vectors = self
                    .embed(&texts, policy)
                    .await
                    .map_err(|err| ProviderError::Message(err.to_string()))?;
                Ok(ProviderResponse {
                    capability: Capability::Embed,
                    payload: serde_json::json!({ "embeddings": vectors }),
                    raw: None,
                })
            }
            Capability::Complete => {
                let prompt = request
                    .payload
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let text = self
                    .complete(prompt, policy)
                    .await
                    .map_err(|err| ProviderError::Message(err.to_string()))?;
                Ok(ProviderResponse {
                    capability: Capability::Complete,
                    payload: serde_json::json!({ "text": text }),
                    raw: Some(text),
                })
            }
            other => Err(ProviderError::Unsupported(
                format!("{other:?}").to_lowercase(),
                "semantic".into(),
            )),
        }
    }
}

pub struct SemanticStore<'a> {
    store: &'a Store,
}

impl<'a> SemanticStore<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn put(&self, component: &SemanticComponent) -> Result<NodeId> {
        if let Some(mut node) = self.node_for_structural(component.structural_node_id)? {
            node.payload = serde_json::to_value(component)?;
            node.validity = component.validity;
            node.name = Some(component.purpose.clone());
            node.touch();
            self.store.upsert_node(&node)?;
            return Ok(node.id);
        }
        let mut node = Node::new(
            NodeKind::Unknown("semantic_component".into()),
            Some(component.purpose.clone()),
            serde_json::to_value(component)?,
        );
        node.validity = component.validity;
        self.store.upsert_node(&node)?;
        Ok(node.id)
    }

    fn node_for_structural(&self, id: NodeId) -> Result<Option<Node>> {
        for node in self
            .store
            .nodes_of_kind(NodeKind::Unknown("semantic_component".into()))?
        {
            if let Ok(component) = serde_json::from_value::<SemanticComponent>(node.payload.clone())
            {
                if component.structural_node_id == id {
                    return Ok(Some(node));
                }
            }
        }
        Ok(None)
    }

    pub fn for_structural_node(&self, id: NodeId) -> Result<Vec<SemanticComponent>> {
        let mut found = Vec::new();
        for node in self
            .store
            .nodes_of_kind(NodeKind::Unknown("semantic_component".into()))?
        {
            if let Ok(component) = serde_json::from_value::<SemanticComponent>(node.payload.clone())
            {
                if component.structural_node_id == id {
                    found.push(component);
                }
            }
        }
        Ok(found)
    }

    pub fn invalidate_if_changed(
        &self,
        structural_id: NodeId,
        current: &Fingerprint,
    ) -> Result<usize> {
        let mut count = 0;
        for mut component in self.for_structural_node(structural_id)? {
            if component.stale_if_fingerprint_changed(current) {
                count += 1;
                let _ = self.put(&component)?;
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::{Fingerprint, Node};
    use rune_providers::Capability;
    use rune_security::Policy;
    use rune_storage::Store;

    #[test]
    fn disabled_provider_still_allows_manual_component_summary() {
        let engine = SemanticEngine::disabled();
        assert_eq!(engine.mode, SemanticMode::Disabled);
        assert!(!engine.capabilities().contains(&Capability::Embed));
        assert!(!engine.capabilities().contains(&Capability::Complete));
        let store = Store::open_in_memory().unwrap();
        let file = Node::new(
            NodeKind::File,
            Some("auth.rs".into()),
            serde_json::json!({"path": "src/auth.rs"}),
        );
        store.upsert_node(&file).unwrap();
        let fingerprint = Fingerprint::of("file", &[b"fn auth() {}"]);
        let component =
            SemanticComponent::manual(file.id, "Authentication entrypoint", fingerprint);
        let semantic = SemanticStore::new(&store);
        let id = semantic.put(&component).unwrap();
        let loaded = store.get_node(id).unwrap();
        assert_eq!(loaded.kind, NodeKind::Unknown("semantic_component".into()));
        let fetched = semantic.for_structural_node(file.id).unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].purpose, "Authentication entrypoint");
        assert_eq!(fetched[0].validity, Validity::Candidate);
    }

    #[test]
    fn fingerprint_change_invalidates_component() {
        let store = Store::open_in_memory().unwrap();
        let file_id = NodeId::generate();
        let original = Fingerprint::of("file", &[b"old"]);
        let mut component = SemanticComponent::manual(file_id, "parser", original);
        let changed = Fingerprint::of("file", &[b"new"]);
        assert!(component.stale_if_fingerprint_changed(&changed));
        assert_eq!(component.validity, Validity::Stale);
        let semantic = SemanticStore::new(&store);
        semantic.put(&component).unwrap();
        assert_eq!(
            semantic.invalidate_if_changed(file_id, &changed).unwrap(),
            1
        );
        let fetched = semantic.for_structural_node(file_id).unwrap();
        assert_eq!(fetched[0].validity, Validity::Stale);
    }

    #[tokio::test]
    async fn local_embed_without_process_uses_hashing() {
        let engine = SemanticEngine {
            mode: SemanticMode::LocalEmbed,
            embedder: None,
            completer: None,
        };
        assert!(engine.capabilities().contains(&Capability::Embed));
        let vectors = engine
            .embed(&["authentication tokens".into()], &Policy::local_default())
            .await
            .unwrap();
        assert_eq!(vectors.len(), 1);
        assert_eq!(vectors[0].len(), 256);
    }

    #[tokio::test]
    async fn disabled_embed_does_not_invent_vectors() {
        let engine = SemanticEngine::disabled();
        let err = engine
            .embed(&["hello".into()], &Policy::local_default())
            .await
            .unwrap_err();
        assert!(matches!(err, SemanticError::Disabled));
    }
}
