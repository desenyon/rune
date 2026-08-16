use rune_core::{Node, NodeKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;

use crate::{redact_secrets, SecretFinding};

/// Project-scoped environment facts. Secrets are never persisted.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub facts: BTreeMap<String, String>,
    pub redacted_keys: Vec<String>,
    pub findings: Vec<SecretFinding>,
}

const SECRET_NAMES: &[&str] = &[
    "AWS_SECRET_ACCESS_KEY",
    "AWS_ACCESS_KEY_ID",
    "GITHUB_TOKEN",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "API_KEY",
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "PRIVATE_KEY",
    "DATABASE_URL",
];

pub fn capture_environment(allowlist: &[&str]) -> EnvironmentSnapshot {
    let mut snapshot = EnvironmentSnapshot::default();
    for (key, value) in env::vars() {
        if is_secret_name(&key) {
            snapshot.redacted_keys.push(key);
            continue;
        }
        if !allowlist.is_empty() && !allowlist.iter().any(|allowed| *allowed == key) {
            continue;
        }
        let (redacted, changed) = redact_secrets(&value);
        if changed {
            snapshot.redacted_keys.push(key);
            continue;
        }
        snapshot.facts.insert(key, redacted);
    }
    snapshot
}

pub fn is_secret_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    SECRET_NAMES.iter().any(|pattern| upper.contains(pattern))
}

pub fn environment_node(snapshot: &EnvironmentSnapshot) -> Node {
    Node::new(
        NodeKind::Constraint,
        Some("environment".into()),
        serde_json::json!({
            "category": "environment_detail",
            "facts": snapshot.facts,
            "redacted_keys": snapshot.redacted_keys,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_names_are_not_persisted() {
        // Safety: test process isolation; restore after.
        env::set_var("RUNE_TEST_SAFE_FACT", "rustc-1.97");
        env::set_var("OPENAI_API_KEY", "sk-test-not-for-production-use");
        let snapshot = capture_environment(&["RUNE_TEST_SAFE_FACT", "OPENAI_API_KEY"]);
        assert_eq!(
            snapshot.facts.get("RUNE_TEST_SAFE_FACT").map(String::as_str),
            Some("rustc-1.97")
        );
        assert!(!snapshot.facts.contains_key("OPENAI_API_KEY"));
        assert!(snapshot
            .redacted_keys
            .iter()
            .any(|key| key == "OPENAI_API_KEY"));
        env::remove_var("RUNE_TEST_SAFE_FACT");
        env::remove_var("OPENAI_API_KEY");
    }
}
