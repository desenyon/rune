use crate::adapters::{discover_files, jsonl_session, message_turn, SessionSource};
use crate::error::Result;
use crate::model::{DiscoveredSession, DiscoveryContext, NormalizedSession};
use crate::provider::identity;
use rune_providers::ProviderIdentity;
use serde_json::Value;

/// Cursor agent transcripts under `~/.cursor/projects/*/agent-transcripts`.
pub struct CursorAdapter;

impl SessionSource for CursorAdapter {
    fn identity(&self) -> ProviderIdentity {
        identity("cursor", "Cursor")
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredSession>> {
        let mut roots = vec![ctx.home_join(&[".cursor", "projects"])];
        if let Some(workspace) = &ctx.workspace {
            roots.push(workspace.join(".cursor"));
        }
        discover_files("cursor", &roots, &["jsonl"])
    }

    fn read_normalized(&self, discovered: &DiscoveredSession) -> Result<NormalizedSession> {
        jsonl_session("cursor", discovered, cursor_turn)
    }
}

fn cursor_turn(value: &Value, index: usize) -> Option<crate::model::NormalizedTurn> {
    message_turn(value, index)
}
