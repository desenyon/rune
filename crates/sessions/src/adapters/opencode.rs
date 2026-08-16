use crate::adapters::{discover_files, jsonl_session, message_turn, SessionSource};
use crate::error::Result;
use crate::io::{json_text, read_lossy};
use crate::model::{DiscoveredSession, DiscoveryContext, NormalizedSession, NormalizedTurn};
use crate::provider::identity;
use rune_providers::ProviderIdentity;
use serde_json::Value;

/// OpenCode local state under `~/.opencode` plus well-known XDG storage.
pub struct OpenCodeAdapter;

impl SessionSource for OpenCodeAdapter {
    fn identity(&self) -> ProviderIdentity {
        identity("opencode", "OpenCode")
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredSession>> {
        discover_files(
            "opencode",
            &[
                ctx.home_join(&[".opencode"]),
                ctx.home_join(&[".config", "opencode"]),
                ctx.home_join(&[".local", "share", "opencode"]),
            ],
            &["jsonl", "json"],
        )
    }

    fn read_normalized(&self, discovered: &DiscoveredSession) -> Result<NormalizedSession> {
        let ext = discovered
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if ext == "jsonl" {
            return jsonl_session("opencode", discovered, message_turn);
        }
        let raw = read_lossy(&discovered.path)?;
        let mut session = NormalizedSession::empty("opencode", &discovered.path);
        session.external_id = discovered.external_id.clone();
        session.raw = raw.clone();
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            session.turns = turns_from_json(&value);
        }
        if session.title.is_none() {
            session.title = discovered.title.clone();
        }
        Ok(session)
    }
}

fn turns_from_json(value: &Value) -> Vec<NormalizedTurn> {
    match value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                if let Some(turn) = message_turn(item, index) {
                    return Some(turn);
                }
                let file = item.get("file").and_then(Value::as_str);
                let patch = item.get("patch").and_then(Value::as_str);
                match (file, patch) {
                    (Some(file), Some(patch)) => Some(crate::adapters::turn(
                        format!("diff-{index}"),
                        "assistant",
                        format!("changed {file}\n{patch}"),
                        item.clone(),
                        None,
                    )),
                    _ => {
                        let text = json_text(item);
                        if text.is_empty() {
                            None
                        } else {
                            Some(crate::adapters::turn(
                                format!("turn-{index}"),
                                "assistant",
                                text,
                                item.clone(),
                                None,
                            ))
                        }
                    }
                }
            })
            .collect(),
        other => {
            if let Some(turn) = message_turn(other, 0) {
                vec![turn]
            } else {
                Vec::new()
            }
        }
    }
}
