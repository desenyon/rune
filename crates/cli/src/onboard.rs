use rune_terminal::TerminalCapabilities;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct OnboardingReport {
    pub workspace: String,
    pub git: bool,
    pub languages: Vec<String>,
    pub coding_agents: Vec<String>,
    pub tools: Vec<String>,
    pub terminal: serde_json::Value,
    pub account_required: bool,
}

pub fn inspect_environment(workspace: &Path) -> OnboardingReport {
    let git = workspace.join(".git").exists() || which::which("git").is_ok();
    let mut languages = Vec::new();
    if workspace.join("Cargo.toml").exists() {
        languages.push("rust".into());
    }
    if workspace.join("package.json").exists() {
        languages.push("javascript".into());
    }
    if workspace.join("go.mod").exists() {
        languages.push("go".into());
    }
    if workspace.join("pyproject.toml").exists() || workspace.join("requirements.txt").exists() {
        languages.push("python".into());
    }
    if workspace.join("tsconfig.json").exists() {
        languages.push("typescript".into());
    }

    let mut coding_agents = Vec::new();
    for (dir, name) in [
        (".claude", "claude"),
        (".cursor", "cursor"),
        (".codex", "codex"),
        (".aider", "aider"),
        (".opencode", "opencode"),
        (".gemini", "gemini"),
    ] {
        if workspace.join(dir).exists() {
            coding_agents.push(name.into());
        }
    }

    let tools = [
        "git", "rg", "fd", "bat", "jq", "cargo", "npm", "pnpm", "bun", "uv", "docker", "kubectl",
    ]
    .into_iter()
    .filter(|name| which::which(name).is_ok())
    .map(|s| s.to_string())
    .collect();

    let caps = TerminalCapabilities::detect();
    OnboardingReport {
        workspace: workspace.display().to_string(),
        git,
        languages,
        coding_agents,
        tools,
        terminal: serde_json::json!({
            "renderer": format!("{:?}", caps.renderer_level),
            "true_color": caps.true_color,
            "unicode": caps.unicode,
            "mouse": caps.mouse,
            "tty": caps.is_tty,
        }),
        account_required: false,
    }
}
