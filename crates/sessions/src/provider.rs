use crate::error::Result;
use crate::model::{DiscoveredSession, DiscoveryContext, NormalizedSession};
use async_trait::async_trait;
use rune_providers::{
    deny_if_missing, deny_if_unauthorized, Capability, Provider, ProviderIdentity, ProviderKind,
    ProviderRequest, ProviderResponse,
};
use rune_security::{Permission, Policy};
use std::collections::BTreeSet;

/// Local-file session source. Continuation, injection, handoff, and streaming are not declared.
pub trait SessionSource: Send + Sync {
    fn identity(&self) -> ProviderIdentity;
    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredSession>>;
    fn read_normalized(&self, discovered: &DiscoveredSession) -> Result<NormalizedSession>;
}

impl SessionSource for Box<dyn SessionSource> {
    fn identity(&self) -> ProviderIdentity {
        self.as_ref().identity()
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredSession>> {
        self.as_ref().discover(ctx)
    }

    fn read_normalized(&self, discovered: &DiscoveredSession) -> Result<NormalizedSession> {
        self.as_ref().read_normalized(discovered)
    }
}

pub fn file_only_capabilities() -> BTreeSet<Capability> {
    BTreeSet::from([
        Capability::Query,
        Capability::Inspect,
        Capability::Import,
        Capability::SessionDiscovery,
        Capability::SessionImport,
    ])
}

pub struct SessionProvider<S> {
    inner: S,
}

impl<S: SessionSource> SessionProvider<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    pub fn source(&self) -> &S {
        &self.inner
    }
}

#[async_trait]
impl<S: SessionSource + 'static> Provider for SessionProvider<S> {
    fn identity(&self) -> ProviderIdentity {
        self.inner.identity()
    }

    fn capabilities(&self) -> BTreeSet<Capability> {
        file_only_capabilities()
    }

    fn required_permissions(&self) -> BTreeSet<Permission> {
        BTreeSet::from([Permission::FilesystemRead])
    }

    async fn invoke(
        &self,
        request: ProviderRequest,
        policy: &Policy,
    ) -> rune_providers::Result<ProviderResponse> {
        deny_if_missing(self, &request.capability)?;
        deny_if_unauthorized(policy, Permission::FilesystemRead)?;
        let ctx = context_from_payload(&request.payload);
        match request.capability {
            Capability::SessionDiscovery | Capability::Query | Capability::Inspect => {
                let found = self
                    .inner
                    .discover(&ctx)
                    .map_err(|err| rune_providers::ProviderError::Message(err.to_string()))?;
                Ok(ProviderResponse {
                    capability: request.capability,
                    payload: serde_json::to_value(found)
                        .map_err(|err| rune_providers::ProviderError::Message(err.to_string()))?,
                    raw: None,
                })
            }
            Capability::SessionImport | Capability::Import => {
                let discovered =
                    discovered_from_payload(&request.payload, &self.inner.identity().id)?;
                let session = self
                    .inner
                    .read_normalized(&discovered)
                    .map_err(|err| rune_providers::ProviderError::Message(err.to_string()))?;
                Ok(ProviderResponse {
                    capability: request.capability,
                    payload: serde_json::to_value(&session)
                        .map_err(|err| rune_providers::ProviderError::Message(err.to_string()))?,
                    raw: Some(session.raw.clone()),
                })
            }
            other => Err(rune_providers::ProviderError::Unsupported(
                format!("{other:?}").to_lowercase(),
                self.inner.identity().id,
            )),
        }
    }
}

fn context_from_payload(payload: &serde_json::Value) -> DiscoveryContext {
    let home = payload
        .get("home")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default());
    let workspace = payload
        .get("workspace")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from);
    DiscoveryContext {
        home,
        workspace,
        extra_roots: Vec::new(),
    }
}

fn discovered_from_payload(
    payload: &serde_json::Value,
    provider: &str,
) -> rune_providers::Result<DiscoveredSession> {
    if let Ok(discovered) = serde_json::from_value::<DiscoveredSession>(payload.clone()) {
        return Ok(discovered);
    }
    let path = payload
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            rune_providers::ProviderError::Message("session import requires path".into())
        })?;
    Ok(DiscoveredSession {
        provider: payload
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or(provider)
            .to_string(),
        external_id: payload
            .get("external_id")
            .and_then(|v| v.as_str())
            .unwrap_or(path)
            .to_string(),
        path: std::path::PathBuf::from(path),
        title: payload
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

pub fn identity(id: &str, name: &str) -> ProviderIdentity {
    ProviderIdentity {
        id: id.to_string(),
        name: name.to_string(),
        version: None,
        kind: ProviderKind::SessionAdapter,
    }
}
