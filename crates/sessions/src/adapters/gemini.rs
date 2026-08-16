use crate::adapters::{discover_files, jsonl_session, message_turn, SessionSource};
use crate::error::Result;
use crate::io::{json_text, read_lossy};
use crate::model::{DiscoveredSession, DiscoveryContext, NormalizedSession};
use crate::provider::identity;
use rune_providers::ProviderIdentity;
use serde_json::Value;

/// Gemini CLI local files under `~/.gemini`.
pub struct GeminiAdapter;

impl SessionSource for GeminiAdapter {
    fn identity(&self) -> ProviderIdentity {
        identity("gemini", "Gemini CLI")
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredSession>> {
        discover_files(
            "gemini",
            &[
                ctx.home_join(&[".gemini"]),
                ctx.home_join(&[".config", "gemini"]),
            ],
            &["jsonl", "json", "md"],
        )
    }

    fn read_normalized(&self, discovered: &DiscoveredSession) -> Result<NormalizedSession> {
        let ext = discovered
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        match ext {
            "jsonl" => jsonl_session("gemini", discovered, message_turn),
            "json" => {
                let raw = read_lossy(&discovered.path)?;
                let mut session = NormalizedSession::empty("gemini", &discovered.path);
                session.external_id = discovered.external_id.clone();
                session.raw = raw.clone();
                if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                    session.turns = gemini_turns(&value);
                }
                Ok(session)
            }
            _ => {
                let raw = read_lossy(&discovered.path)?;
                let mut session = NormalizedSession::empty("gemini", &discovered.path);
                session.raw = raw.clone();
                if !raw.trim().is_empty() {
                    session.turns.push(crate::adapters::turn(
                        "body",
                        "assistant",
                        raw,
                        Value::String(session.raw.clone()),
                        None,
                    ));
                }
                Ok(session)
            }
        }
    }
}

fn gemini_turns(value: &Value) -> Vec<crate::model::NormalizedTurn> {
    if let Some(msgs) = value
        .get("messages")
        .or_else(|| value.get("history"))
        .and_then(Value::as_array)
    {
        return msgs
            .iter()
            .enumerate()
            .filter_map(|(i, msg)| message_turn(msg, i))
            .collect();
    }
    if let Some(turn) = message_turn(value, 0) {
        return vec![turn];
    }
    let text = json_text(value);
    if text.is_empty() {
        Vec::new()
    } else {
        vec![crate::adapters::turn(
            "body",
            "assistant",
            text,
            value.clone(),
            None,
        )]
    }
}
