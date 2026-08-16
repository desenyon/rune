//! External tools are implementation providers. Rune is the system of record.
//!
//! Providers declare capabilities. Unsupported operations fail clearly and
//! never pretend to exist.

use async_trait::async_trait;
use rune_security::{Permission, Policy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("capability `{0}` is not supported by provider `{1}`")]
    Unsupported(String, String),
    #[error("provider `{0}` is unavailable: {1}")]
    Unavailable(String, String),
    #[error("permission denied: {0:?}")]
    Permission(Permission),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, ProviderError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderIdentity {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub kind: ProviderKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    CodingAgent,
    DeveloperTool,
    Documentation,
    Semantic,
    Search,
    SessionAdapter,
    Mcp,
    Plugin,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Query,
    Execute,
    Stream,
    Inspect,
    Export,
    Import,
    SessionDiscovery,
    SessionImport,
    SessionContinuation,
    ContextInjection,
    CommandInvocation,
    Handoff,
    StreamingEvents,
    Embed,
    Complete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub capability: Capability,
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub capability: Capability,
    pub payload: serde_json::Value,
    pub raw: Option<String>,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn identity(&self) -> ProviderIdentity;
    fn capabilities(&self) -> BTreeSet<Capability>;
    fn required_permissions(&self) -> BTreeSet<Permission>;

    fn supports(&self, capability: &Capability) -> bool {
        self.capabilities().contains(capability)
    }

    async fn invoke(
        &self,
        request: ProviderRequest,
        policy: &Policy,
    ) -> Result<ProviderResponse>;
}

pub fn deny_if_missing(
    provider: &dyn Provider,
    capability: &Capability,
) -> Result<()> {
    if provider.supports(capability) {
        Ok(())
    } else {
        Err(ProviderError::Unsupported(
            format!("{capability:?}").to_lowercase(),
            provider.identity().id,
        ))
    }
}

pub fn deny_if_unauthorized(policy: &Policy, permission: Permission) -> Result<()> {
    if policy.permits(permission.clone()) {
        Ok(())
    } else {
        Err(ProviderError::Permission(permission))
    }
}

/// Registry that never silently substitutes a missing provider.
#[derive(Default)]
pub struct ProviderRegistry {
    providers: Vec<Box<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Box<dyn Provider>) {
        self.providers.push(provider);
    }

    pub fn get(&self, id: &str) -> Option<&dyn Provider> {
        self.providers
            .iter()
            .find(|provider| provider.identity().id == id)
            .map(|provider| provider.as_ref())
    }

    pub fn of_kind(&self, kind: ProviderKind) -> Vec<&dyn Provider> {
        self.providers
            .iter()
            .filter(|provider| provider.identity().kind == kind)
            .map(|provider| provider.as_ref())
            .collect()
    }

    pub fn require(&self, id: &str) -> Result<&dyn Provider> {
        self.get(id)
            .ok_or_else(|| ProviderError::Unavailable(id.to_string(), "not registered".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct ProbeProvider;

    #[async_trait]
    impl Provider for ProbeProvider {
        fn identity(&self) -> ProviderIdentity {
            ProviderIdentity {
                id: "probe".into(),
                name: "Probe".into(),
                version: Some("test".into()),
                kind: ProviderKind::Search,
            }
        }

        fn capabilities(&self) -> BTreeSet<Capability> {
            BTreeSet::from([Capability::Query])
        }

        fn required_permissions(&self) -> BTreeSet<Permission> {
            BTreeSet::from([Permission::FilesystemRead])
        }

        async fn invoke(
            &self,
            request: ProviderRequest,
            policy: &Policy,
        ) -> Result<ProviderResponse> {
            deny_if_missing(self, &request.capability)?;
            deny_if_unauthorized(policy, Permission::FilesystemRead)?;
            Ok(ProviderResponse {
                capability: request.capability,
                payload: serde_json::json!({"hits": []}),
                raw: None,
            })
        }
    }

    #[tokio::test]
    async fn unsupported_capability_fails_clearly() {
        let provider = ProbeProvider;
        let err = deny_if_missing(&provider, &Capability::Handoff).unwrap_err();
        assert!(matches!(err, ProviderError::Unsupported(_, _)));
    }

    #[tokio::test]
    async fn missing_permission_is_denied() {
        let provider = ProbeProvider;
        let policy = Policy::local_default();
        let err = provider
            .invoke(
                ProviderRequest {
                    capability: Capability::Query,
                    payload: serde_json::json!({}),
                },
                &Policy {
                    allow: BTreeSet::new(),
                    ..policy
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permission(Permission::FilesystemRead)));
    }

    #[tokio::test]
    async fn registry_does_not_invent_providers() {
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(ProbeProvider));
        assert!(registry.require("missing").is_err());
        assert!(Arc::new(registry.require("probe").unwrap().identity().id.clone()).as_str() == "probe");
    }
}
