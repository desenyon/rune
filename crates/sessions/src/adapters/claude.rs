use crate::adapters::{discover_files, jsonl_session, message_turn, SessionSource};
use crate::error::Result;
use crate::model::{DiscoveredSession, DiscoveryContext, NormalizedSession};
use crate::provider::identity;
use rune_providers::ProviderIdentity;
use serde_json::Value;

/// Claude Code stores jsonl transcripts under `~/.claude/projects`.
pub struct ClaudeCodeAdapter;

impl SessionSource for ClaudeCodeAdapter {
    fn identity(&self) -> ProviderIdentity {
        identity("claude-code", "Claude Code")
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredSession>> {
        discover_files(
            "claude-code",
            &[
                ctx.home_join(&[".claude", "projects"]),
                ctx.home_join(&[".config", "claude", "projects"]),
            ],
            &["jsonl"],
        )
    }

    fn read_normalized(&self, discovered: &DiscoveredSession) -> Result<NormalizedSession> {
        jsonl_session("claude-code", discovered, claude_turn)
    }
}

fn claude_turn(value: &Value, index: usize) -> Option<crate::model::NormalizedTurn> {
    match value.get("type").and_then(Value::as_str) {
        Some("user" | "assistant") => message_turn(value, index),
        Some("progress") => None,
        _ => {
            if value.get("message").is_some() {
                message_turn(value, index)
            } else {
                None
            }
        }
    }
}
