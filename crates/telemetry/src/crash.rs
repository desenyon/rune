use rune_security::redact_secrets;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct CrashBundleRequest {
    pub version: String,
    pub platform: String,
    pub terminal_capabilities: serde_json::Value,
    pub logs: String,
    pub stack_trace: Option<String>,
    pub provider_states: serde_json::Value,
    pub database_diagnostics: serde_json::Value,
}

/// Optional local crash bundle. Secrets in logs are redacted. Repository
/// contents are never included automatically.
pub fn write_crash_bundle(dir: impl AsRef<Path>, request: &CrashBundleRequest) -> std::io::Result<PathBuf> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;
    let (logs, _) = redact_secrets(&request.logs);
    let (stack, _) = match &request.stack_trace {
        Some(stack) => {
            let (text, flag) = redact_secrets(stack);
            (Some(text), flag)
        }
        None => (None, false),
    };
    let bundle = serde_json::json!({
        "version": request.version,
        "platform": request.platform,
        "terminal_capabilities": request.terminal_capabilities,
        "logs": logs,
        "stack_trace": stack,
        "provider_states": request.provider_states,
        "database_diagnostics": request.database_diagnostics,
        "includes_repository_secrets": false,
        "includes_repository_content": false,
    });
    let path = dir.join("crash-bundle.json");
    fs::write(&path, serde_json::to_vec_pretty(&bundle).unwrap())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_bundle_redacts_and_omits_repo_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_crash_bundle(
            dir.path(),
            &CrashBundleRequest {
                version: "0.1.0".into(),
                platform: "macos-arm64".into(),
                terminal_capabilities: serde_json::json!({"level": "enhanced"}),
                logs: "Authorization: Bearer supersecrettokenvalue".into(),
                stack_trace: Some("token=abcdefghijklmnop".into()),
                provider_states: serde_json::json!({"probe": "ok"}),
                database_diagnostics: serde_json::json!({"migrations": 1}),
            },
        )
        .unwrap();
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains("[REDACTED]"));
        assert!(!text.contains("supersecrettokenvalue"));
        assert!(text.contains("\"includes_repository_content\": false"));
    }
}
