use crate::adapters::{discover_files, jsonl_session, SessionSource};
use crate::error::Result;
use crate::io::{first_string, json_text};
use crate::model::{DiscoveredSession, DiscoveryContext, NormalizedSession, NormalizedTurn};
use crate::provider::identity;
use rune_providers::ProviderIdentity;
use serde_json::Value;

/// Codex rollouts under `~/.codex/sessions` and archived jsonl.
pub struct CodexAdapter;

impl SessionSource for CodexAdapter {
    fn identity(&self) -> ProviderIdentity {
        identity("codex", "Codex")
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredSession>> {
        discover_files(
            "codex",
            &[
                ctx.home_join(&[".codex", "sessions"]),
                ctx.home_join(&[".codex", "archived_sessions"]),
                ctx.home_join(&[".codex"]),
            ],
            &["jsonl"],
        )
    }

    fn read_normalized(&self, discovered: &DiscoveredSession) -> Result<NormalizedSession> {
        jsonl_session("codex", discovered, codex_turn)
    }
}

fn codex_turn(value: &Value, index: usize) -> Option<NormalizedTurn> {
    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
    if kind == "session_meta" {
        return None;
    }
    let payload = value.get("payload").unwrap_or(value);
    let text = json_text(payload);
    let text = if text.is_empty() {
        first_string(payload, &["text", "message", "command"]).unwrap_or_default()
    } else {
        text
    };
    if text.trim().is_empty() {
        return None;
    }
    let role = match kind {
        "user_message" | "user" => "user",
        "agent_message" | "assistant" | "response_item" => "assistant",
        "tool" | "function_call" | "custom_tool_call" => "tool",
        other if !other.is_empty() => other,
        _ => "assistant",
    };
    let id = first_string(value, &["id"])
        .or_else(|| first_string(payload, &["id"]))
        .unwrap_or_else(|| format!("turn-{index}"));
    let timestamp = first_string(value, &["timestamp"]);
    Some(crate::adapters::turn(
        id,
        role,
        text,
        value.clone(),
        timestamp,
    ))
}
