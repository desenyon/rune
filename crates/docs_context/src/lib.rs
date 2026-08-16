//! External documentation context (S030) and version freshness (S031).

use async_trait::async_trait;
use rune_core::{Node, NodeKind, Timestamp};
use rune_providers::{
    deny_if_missing, deny_if_unauthorized, Capability, Provider, ProviderError, ProviderIdentity,
    ProviderKind, ProviderRequest, ProviderResponse, Result as ProviderResult,
};
use rune_security::{Permission, Policy};
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocsError {
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("version mismatch for {library}: cached {cached}, project {current}")]
    VersionMismatch {
        library: String,
        cached: String,
        current: String,
    },
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, DocsError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalDoc {
    pub library: String,
    pub version: String,
    pub source: String,
    pub retrieved_at: Timestamp,
    pub section: String,
    pub content: String,
    pub relevance: f32,
}

impl ExternalDoc {
    pub fn into_node(&self) -> Node {
        Node::new(
            NodeKind::ExternalDocument,
            Some(format!(
                "{}@{}#{}",
                self.library, self.version, self.section
            )),
            serde_json::to_value(self).unwrap_or(serde_json::json!({})),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DependencyPin {
    pub library: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessOutcome {
    Current,
    Invalidated,
    MismatchWarning,
}

pub fn evaluate_freshness(doc: &ExternalDoc, pin: &DependencyPin) -> FreshnessOutcome {
    if doc.library != pin.library {
        return FreshnessOutcome::Current;
    }
    if doc.version == pin.version {
        FreshnessOutcome::Current
    } else {
        FreshnessOutcome::MismatchWarning
    }
}

pub struct DocsStore<'a> {
    store: &'a Store,
}

impl<'a> DocsStore<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn upsert(&self, doc: &ExternalDoc) -> Result<Node> {
        let node = doc.into_node();
        self.store.upsert_node(&node)?;
        Ok(node)
    }

    pub fn cached(&self, library: &str, version: &str) -> Result<Vec<ExternalDoc>> {
        let mut out = Vec::new();
        for node in self.store.nodes_of_kind(NodeKind::ExternalDocument)? {
            if let Ok(doc) = serde_json::from_value::<ExternalDoc>(node.payload.clone()) {
                if doc.library == library && doc.version == version {
                    out.push(doc);
                }
            }
        }
        Ok(out)
    }

    /// Invalidate cached docs when the project dependency version changes.
    pub fn invalidate_for_version_change(&self, pin: &DependencyPin) -> Result<Vec<ExternalDoc>> {
        let mut invalidated = Vec::new();
        for node in self.store.nodes_of_kind(NodeKind::ExternalDocument)? {
            let Ok(doc) = serde_json::from_value::<ExternalDoc>(node.payload.clone()) else {
                continue;
            };
            if doc.library != pin.library || doc.version == pin.version {
                continue;
            }
            let mut stale = node;
            stale.validity = rune_core::Validity::Stale;
            stale.touch();
            self.store.upsert_node(&stale)?;
            invalidated.push(doc.clone());
            let _ = doc;
        }
        Ok(invalidated)
    }

    pub fn warn_if_mismatch(doc: &ExternalDoc, pin: &DependencyPin) -> Option<String> {
        match evaluate_freshness(doc, pin) {
            FreshnessOutcome::MismatchWarning => Some(format!(
                "documentation for {}@{} may not apply to project version {}",
                doc.library, doc.version, pin.version
            )),
            _ => None,
        }
    }
}

/// Context7-style documentation provider. Network/policy denial fails clearly.
pub struct Context7Provider {
    pub endpoint: String,
}

impl Default for Context7Provider {
    fn default() -> Self {
        Self {
            endpoint: "https://context7.com/api/docs".into(),
        }
    }
}

impl Context7Provider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}

#[async_trait]
impl Provider for Context7Provider {
    fn identity(&self) -> ProviderIdentity {
        ProviderIdentity {
            id: "context7".into(),
            name: "Context7".into(),
            version: Some("1".into()),
            kind: ProviderKind::Documentation,
        }
    }

    fn capabilities(&self) -> BTreeSet<Capability> {
        BTreeSet::from([Capability::Query])
    }

    fn required_permissions(&self) -> BTreeSet<Permission> {
        BTreeSet::from([Permission::Network])
    }

    fn data_leaving_machine(&self) -> Option<&'static str> {
        Some("library name, version, and query text are sent to the configured documentation endpoint")
    }

    async fn invoke(
        &self,
        request: ProviderRequest,
        policy: &Policy,
    ) -> ProviderResult<ProviderResponse> {
        deny_if_missing(self, &request.capability)?;
        deny_if_unauthorized(policy, Permission::Network)?;
        if request.capability != Capability::Query {
            return Err(ProviderError::Unsupported(
                format!("{:?}", request.capability).to_lowercase(),
                self.identity().id,
            ));
        }
        let library = request
            .payload
            .get("library")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Message("payload.library is required".into()))?;
        let version = request
            .payload
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("latest");
        let query = request
            .payload
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let project_version = request
            .payload
            .get("project_version")
            .and_then(|v| v.as_str());

        let mut warnings = Vec::new();
        if let Some(project_version) = project_version {
            if project_version != version && version != "latest" {
                warnings.push(format!(
                    "requested {library}@{version} but project depends on {project_version}"
                ));
            }
        }

        // Network is permitted. A real fetch is attempted; failures are explicit.
        let url = format!(
            "{}?library={}&version={}&q={}",
            self.endpoint,
            encode(library),
            encode(version),
            encode(query)
        );
        match fetch_text(&url) {
            Ok(body) => Ok(ProviderResponse {
                capability: Capability::Query,
                payload: serde_json::json!({
                    "library": library,
                    "version": version,
                    "section": query,
                    "content": body,
                    "relevance": 0.5,
                    "warnings": warnings,
                    "retrieved_at": Timestamp::now().as_millis(),
                    "source": url,
                }),
                raw: Some(body),
            }),
            Err(err) => Err(ProviderError::Unavailable(
                self.identity().id,
                format!("documentation fetch failed for {url}: {err}"),
            )),
        }
    }
}

fn encode(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(ch),
            ' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", ch as u32)),
        }
    }
    out
}

fn parse_url(url: &str) -> std::result::Result<ParsedUrl, String> {
    let (https, rest) = if let Some(rest) = url.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (false, rest)
    } else {
        return Err("endpoint must be http:// or https://".into());
    };
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, "/"));
    let (host, port) = if let Some((h, p)) = hostport.split_once(':') {
        (h.to_string(), p.parse().unwrap_or(if https { 443 } else { 80 }))
    } else {
        (hostport.to_string(), if https { 443 } else { 80 })
    };
    Ok(ParsedUrl {
        https,
        host,
        port,
        path: format!("/{path}"),
    })
}

fn fetch_text(url: &str) -> std::result::Result<String, String> {
    let parsed = parse_url(url)?;
    if parsed.https {
        https_get(url)
    } else {
        http_get(&parsed)
    }
}

fn https_get(url: &str) -> std::result::Result<String, String> {
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(10))
        .set("User-Agent", "rune-docs-context")
        .set("Accept", "text/plain, application/json")
        .call()
        .map_err(|err| err.to_string())?;
    response.into_string().map_err(|err| err.to_string())
}

fn http_get(parsed: &ParsedUrl) -> std::result::Result<String, String> {
    let mut stream = std::net::TcpStream::connect(format!("{}:{}", parsed.host, parsed.port))
        .map_err(|err| err.to_string())?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .map_err(|err| err.to_string())?;
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(10)))
        .map_err(|err| err.to_string())?;
    let request = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: rune-docs-context\r\nAccept: text/plain, application/json\r\nConnection: close\r\n\r\n",
        parsed.path, parsed.host
    );
    use std::io::{Read, Write};
    stream
        .write_all(request.as_bytes())
        .map_err(|err| err.to_string())?;
    let mut buf = String::new();
    stream
        .read_to_string(&mut buf)
        .map_err(|err| err.to_string())?;
    let (headers, body) = buf.split_once("\r\n\r\n").unwrap_or((buf.as_str(), ""));
    if !headers.contains("200") {
        return Err(format!(
            "unexpected response: {}",
            headers.lines().next().unwrap_or("")
        ));
    }
    Ok(body.to_string())
}

struct ParsedUrl {
    https: bool,
    host: String,
    port: u16,
    path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_change_invalidates_cached_docs() {
        let store = Store::open_in_memory().unwrap();
        let docs = DocsStore::new(&store);
        let doc = ExternalDoc {
            library: "tokio".into(),
            version: "1.0.0".into(),
            source: "context7".into(),
            retrieved_at: Timestamp::now(),
            section: "spawn".into(),
            content: "spawn a task".into(),
            relevance: 0.9,
        };
        docs.upsert(&doc).unwrap();
        let pin = DependencyPin {
            library: "tokio".into(),
            version: "1.44.0".into(),
        };
        assert_eq!(
            evaluate_freshness(&doc, &pin),
            FreshnessOutcome::MismatchWarning
        );
        assert!(DocsStore::warn_if_mismatch(&doc, &pin).is_some());
        let invalidated = docs.invalidate_for_version_change(&pin).unwrap();
        assert_eq!(invalidated.len(), 1);
        let stored = store.nodes_of_kind(NodeKind::ExternalDocument).unwrap();
        assert_eq!(stored[0].validity, rune_core::Validity::Stale);
    }

    #[tokio::test]
    async fn network_policy_denied_fails_clearly() {
        let provider = Context7Provider::default();
        let policy = Policy::local_default();
        let err = provider
            .invoke(
                ProviderRequest {
                    capability: Capability::Query,
                    payload: serde_json::json!({"library": "tokio", "query": "spawn"}),
                },
                &policy,
            )
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ProviderError::Permission(Permission::Network)
        ));
    }

    #[test]
    fn https_urls_parse_to_port_443() {
        let parsed = parse_url("https://context7.com/api/docs?library=tokio").unwrap();
        assert!(parsed.https);
        assert_eq!(parsed.host, "context7.com");
        assert_eq!(parsed.port, 443);
        assert!(parsed.path.starts_with("/api/docs"));
    }

    #[test]
    fn http_urls_still_parse() {
        let parsed = parse_url("http://127.0.0.1:9/docs").unwrap();
        assert!(!parsed.https);
        assert_eq!(parsed.port, 9);
    }
}
