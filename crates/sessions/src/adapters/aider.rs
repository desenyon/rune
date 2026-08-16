use crate::adapters::{discover_files, SessionSource};
use crate::error::Result;
use crate::io::read_lossy;
use crate::model::{DiscoveredSession, DiscoveryContext, NormalizedSession};
use crate::provider::identity;
use regex::Regex;
use rune_providers::ProviderIdentity;
use serde_json::Value;
use std::sync::OnceLock;

/// Aider chat history: workspace `.aider*` files and `~/.aider*`.
pub struct AiderAdapter;

impl SessionSource for AiderAdapter {
    fn identity(&self) -> ProviderIdentity {
        identity("aider", "Aider")
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredSession>> {
        let mut found = discover_files(
            "aider",
            &[ctx.home_join(&[".aider"])],
            &["md", "txt", "jsonl"],
        )?;
        if let Some(workspace) = &ctx.workspace {
            for name in [
                ".aider.chat.history.md",
                ".aider.input.history",
                ".aider.chat.history",
            ] {
                let path = workspace.join(name);
                if path.is_file() && !found.iter().any(|item| item.path == path) {
                    found.push(DiscoveredSession {
                        provider: "aider".into(),
                        external_id: name.to_string(),
                        path,
                        title: Some(name.to_string()),
                    });
                }
            }
        }
        found.retain(|item| {
            item.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.contains("aider") || name.contains("history"))
                .unwrap_or(false)
        });
        Ok(found)
    }

    fn read_normalized(&self, discovered: &DiscoveredSession) -> Result<NormalizedSession> {
        let raw = read_lossy(&discovered.path)?;
        let mut session = NormalizedSession::empty("aider", &discovered.path);
        session.external_id = discovered.external_id.clone();
        session.raw = raw.clone();
        session.turns = parse_aider_markdown(&raw);
        session.title = session
            .turns
            .iter()
            .find(|turn| turn.role == "user")
            .map(|turn| crate::adapters::truncate(&turn.text, 80));
        Ok(session)
    }
}

fn parse_aider_markdown(raw: &str) -> Vec<crate::model::NormalizedTurn> {
    let heading = heading_re();
    let mut turns = Vec::new();
    let mut current_role = String::from("user");
    let mut current = String::new();
    let mut index = 0usize;
    let flush = |role: &str,
                 body: &str,
                 index: &mut usize,
                 turns: &mut Vec<crate::model::NormalizedTurn>| {
        let text = body.trim();
        if text.is_empty() {
            return;
        }
        turns.push(crate::adapters::turn(
            format!("turn-{index}"),
            role,
            text,
            Value::String(text.to_string()),
            None,
        ));
        *index += 1;
    };
    for line in raw.lines() {
        if let Some(caps) = heading.captures(line) {
            flush(&current_role, &current, &mut index, &mut turns);
            current.clear();
            let label = caps.get(1).map(|m| m.as_str()).unwrap_or("user");
            current_role = crate::adapters::normalize_role(label);
            continue;
        }
        current.push_str(line);
        current.push('\n');
    }
    flush(&current_role, &current, &mut index, &mut turns);
    if turns.is_empty() && !raw.trim().is_empty() {
        turns.push(crate::adapters::turn(
            "body",
            "user",
            raw,
            Value::String(raw.to_string()),
            None,
        ));
    }
    turns
}

fn heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^(?:#{1,6}\s+|####\s+)?(user|assistant|aider|human|ai)\s*:?\s*$")
            .expect("static regex")
    })
}
