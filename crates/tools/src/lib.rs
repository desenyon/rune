//! CLI adapters that prefer machine-readable output and fail clearly when a tool is missing.

use async_trait::async_trait;
use rune_providers::{
    deny_if_missing, deny_if_unauthorized, Capability, Provider, ProviderError, ProviderIdentity,
    ProviderKind, ProviderRequest, ProviderResponse, Result,
};
use rune_security::{Permission, Policy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    JsonStdout,
    JsonLines,
    NullSeparated,
    PlainText,
}

#[derive(Clone, Debug)]
pub struct ToolSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub binaries: &'static [&'static str],
    pub mode: OutputMode,
}

impl ToolSpec {
    pub const fn new(
        id: &'static str,
        name: &'static str,
        binaries: &'static [&'static str],
        mode: OutputMode,
    ) -> Self {
        Self {
            id,
            name,
            binaries,
            mode,
        }
    }
}

pub fn augmented_path(base: Option<&OsStr>) -> OsString {
    let mut parts: Vec<PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir().or_else(|| std::env::var_os("HOME").map(PathBuf::from)) {
        parts.push(home.join(".cargo").join("bin"));
    }
    if let Some(base) = base {
        for segment in std::env::split_paths(base) {
            if !parts.contains(&segment) {
                parts.push(segment);
            }
        }
    } else if let Some(path) = std::env::var_os("PATH") {
        for segment in std::env::split_paths(&path) {
            if !parts.contains(&segment) {
                parts.push(segment);
            }
        }
    }
    std::env::join_paths(parts).unwrap_or_else(|_| OsString::from("/usr/bin:/bin"))
}

pub fn which_in(binaries: &[&str], path: &OsStr) -> Option<PathBuf> {
    for binary in binaries {
        if let Ok(found) = which::which_in(
            binary,
            Some(path),
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        ) {
            return Some(found);
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct CliAdapter {
    spec: ToolSpec,
    path_override: Option<OsString>,
}

impl CliAdapter {
    pub fn new(spec: ToolSpec) -> Self {
        Self {
            spec,
            path_override: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<OsString>) -> Self {
        self.path_override = Some(path.into());
        self
    }

    pub fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    pub fn search_path(&self) -> OsString {
        match &self.path_override {
            Some(path) => path.clone(),
            None => augmented_path(std::env::var_os("PATH").as_deref()),
        }
    }

    pub fn locate(&self) -> Option<PathBuf> {
        which_in(self.spec.binaries, &self.search_path())
    }

    pub fn prepare_args(&self, args: &[String]) -> Vec<String> {
        inject_machine_flags(self.spec.id, self.spec.mode, args)
    }

    pub async fn run(&self, args: &[String], policy: &Policy) -> Result<ProviderResponse> {
        deny_if_unauthorized(policy, Permission::ProcessExecute)?;
        let program = self.locate().ok_or_else(|| {
            ProviderError::Unavailable(
                self.spec.id.to_string(),
                format!("{} is not installed", self.spec.binaries.join("/")),
            )
        })?;
        if needs_network(self.spec.id, args) && !policy.permits(Permission::Network) {
            return Err(ProviderError::Permission(Permission::Network));
        }
        let prepared = self.prepare_args(args);
        let mut cmd = Command::new(&program);
        cmd.args(&prepared)
            .env("PATH", self.search_path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let output = tokio::time::timeout(Duration::from_secs(60), cmd.output())
            .await
            .map_err(|_| ProviderError::Message(format!("{} timed out after 60s", self.spec.id)))?
            .map_err(|err| ProviderError::Message(err.to_string()))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if !output.status.success() {
            return Err(ProviderError::Message(format!(
                "{} exited {}: {}",
                self.spec.id,
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )));
        }
        let payload = parse_output(self.spec.mode, &stdout)?;
        Ok(ProviderResponse {
            capability: Capability::Execute,
            payload: serde_json::json!({
                "tool": self.spec.id,
                "program": program,
                "args": prepared,
                "output": payload,
                "stderr": stderr,
                "exit_code": output.status.code(),
            }),
            raw: Some(stdout),
        })
    }
}

fn needs_network(id: &str, args: &[String]) -> bool {
    matches!(id, "curl" | "gh" | "kubectl")
        || args
            .iter()
            .any(|arg| arg.starts_with("http://") || arg.starts_with("https://"))
}

fn already_has(args: &[String], flag: &str) -> bool {
    args.iter()
        .any(|arg| arg == flag || arg.starts_with(&format!("{flag}=")))
}

fn inject_machine_flags(id: &str, mode: OutputMode, args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    match id {
        "git" => {
            out.extend(["--no-pager".into(), "-c".into(), "color.ui=never".into()]);
            if args.first().map(String::as_str) == Some("status")
                && !already_has(args, "--porcelain")
            {
                out.extend(args.iter().cloned());
                if !out.iter().any(|a| a.starts_with("--porcelain")) {
                    out.push("--porcelain=v2".into());
                }
                return out;
            }
            if args.first().map(String::as_str) == Some("diff") && !already_has(args, "--numstat") {
                out.extend(args.iter().cloned());
                out.push("--numstat".into());
                return out;
            }
        }
        "rg" if !already_has(args, "--json") => {
            out.push("--json".into());
        }
        "fd" if !already_has(args, "-0") && !already_has(args, "--print0") => {
            out.push("-0".into());
        }
        "bat" => {
            if !already_has(args, "--paging") {
                out.push("--paging=never".into());
            }
            if !already_has(args, "--style") {
                out.push("--style=plain".into());
            }
            if !already_has(args, "--color") {
                out.push("--color=never".into());
            }
        }
        "curl" if !already_has(args, "-s") && !already_has(args, "--silent") => {
            out.extend(["-sS".into()]);
        }
        "docker" => {
            if matches!(args.first().map(String::as_str), Some("ps" | "images"))
                && !already_has(args, "--format")
            {
                out.extend(args.iter().cloned());
                out.extend(["--format".into(), "json".into()]);
                return out;
            }
        }
        "kubectl" if !already_has(args, "-o") && !already_has(args, "--output") => {
            out.extend(args.iter().cloned());
            out.extend(["-o".into(), "json".into()]);
            return out;
        }
        "cargo" => {
            if matches!(
                args.first().map(String::as_str),
                Some("build" | "check" | "test" | "clippy")
            ) && !already_has(args, "--message-format")
            {
                out.extend(args.iter().cloned());
                out.extend(["--message-format".into(), "json".into()]);
                return out;
            }
            if args.first().map(String::as_str) == Some("metadata")
                && !already_has(args, "--format-version")
            {
                out.extend(args.iter().cloned());
                out.extend(["--format-version".into(), "1".into()]);
                return out;
            }
        }
        "npm" | "pnpm" | "bun" if !already_has(args, "--json") => {
            out.push("--json".into());
        }
        "go" if args.first().map(String::as_str) == Some("env") && !already_has(args, "-json") => {
            out.extend(args.iter().cloned());
            out.push("-json".into());
            return out;
        }
        "brew"
            if args.first().map(String::as_str) == Some("info") && !already_has(args, "--json") =>
        {
            out.extend(args.iter().cloned());
            out.push("--json=v2".into());
            return out;
        }
        "hyperfine" if !already_has(args, "--export-json") => {
            out.extend(args.iter().cloned());
            out.extend(["--export-json".into(), "-".into()]);
            return out;
        }
        "gh" => {
            if !already_has(args, "--json")
                && matches!(
                    args.first().map(String::as_str),
                    Some("pr" | "issue" | "run" | "repo")
                )
            {
                return ErrFlags::unsupported_gh(args);
            }
        }
        _ => {}
    }
    let _ = mode;
    out.extend(args.iter().cloned());
    out
}

struct ErrFlags;

impl ErrFlags {
    fn unsupported_gh(args: &[String]) -> Vec<String> {
        let mut out = args.to_vec();
        if args.first().map(String::as_str) == Some("pr")
            && args.get(1).map(String::as_str) == Some("list")
        {
            out.extend(["--json".into(), "number,title,state,url".into()]);
        } else if args.first().map(String::as_str) == Some("issue")
            && args.get(1).map(String::as_str) == Some("list")
        {
            out.extend(["--json".into(), "number,title,state,url".into()]);
        }
        out
    }
}

fn parse_output(mode: OutputMode, stdout: &str) -> Result<serde_json::Value> {
    let trimmed = stdout.trim();
    match mode {
        OutputMode::JsonStdout => {
            if trimmed.is_empty() {
                return Ok(serde_json::json!(null));
            }
            serde_json::from_str(trimmed).map_err(|err| {
                ProviderError::Message(format!(
                    "expected JSON output, refusing to parse decorative text: {err}"
                ))
            })
        }
        OutputMode::JsonLines => {
            let mut rows = Vec::new();
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                rows.push(
                    serde_json::from_str::<serde_json::Value>(line).map_err(|err| {
                        ProviderError::Message(format!(
                            "expected JSONL output, refusing to parse decorative text: {err}"
                        ))
                    })?,
                );
            }
            Ok(serde_json::Value::Array(rows))
        }
        OutputMode::NullSeparated => {
            let parts: Vec<_> = stdout.split('\0').filter(|part| !part.is_empty()).collect();
            Ok(serde_json::json!(parts))
        }
        OutputMode::PlainText => Ok(serde_json::json!({ "text": stdout })),
    }
}

#[async_trait]
impl Provider for CliAdapter {
    fn identity(&self) -> ProviderIdentity {
        ProviderIdentity {
            id: self.spec.id.to_string(),
            name: self.spec.name.to_string(),
            version: None,
            kind: ProviderKind::DeveloperTool,
        }
    }

    fn capabilities(&self) -> BTreeSet<Capability> {
        BTreeSet::from([Capability::Query, Capability::Inspect, Capability::Execute])
    }

    fn required_permissions(&self) -> BTreeSet<Permission> {
        let mut set = BTreeSet::from([Permission::ProcessExecute]);
        if matches!(self.spec.id, "curl" | "gh" | "kubectl") {
            set.insert(Permission::Network);
        }
        set
    }

    async fn invoke(&self, request: ProviderRequest, policy: &Policy) -> Result<ProviderResponse> {
        deny_if_missing(self, &request.capability)?;
        if self.locate().is_none() {
            return Err(ProviderError::Unavailable(
                self.spec.id.to_string(),
                format!("{} is not installed", self.spec.binaries.join("/")),
            ));
        }
        let args = match &request.payload {
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            serde_json::Value::Object(map) => map
                .get("args")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            serde_json::Value::String(arg) => vec![arg.clone()],
            _ => Vec::new(),
        };
        match request.capability {
            Capability::Inspect | Capability::Query if args.is_empty() => {
                self.run(&inspect_args(self.spec.id), policy).await
            }
            Capability::Execute | Capability::Query | Capability::Inspect => {
                self.run(&args, policy).await
            }
            other => Err(ProviderError::Unsupported(
                format!("{other:?}").to_lowercase(),
                self.spec.id.to_string(),
            )),
        }
    }
}

fn inspect_args(id: &str) -> Vec<String> {
    match id {
        "git" => vec!["rev-parse".into(), "--is-inside-work-tree".into()],
        "go" => vec!["env".into()],
        "python" => vec![
            "-c".into(),
            "import json,sys; print(json.dumps({\"version\": sys.version}))".into(),
        ],
        "cargo" => vec!["metadata".into(), "--no-deps".into()],
        "jq" => vec!["-n".into(), "{}".into()],
        "uv" => vec!["--version".into()],
        "hyperfine" => vec!["--version".into()],
        "ssh" => vec!["-V".into()],
        _ => vec!["--version".into()],
    }
}

pub fn catalog() -> Vec<ToolSpec> {
    vec![
        ToolSpec::new("git", "Git", &["git"], OutputMode::PlainText),
        ToolSpec::new("gh", "GitHub CLI", &["gh"], OutputMode::JsonStdout),
        ToolSpec::new("rg", "ripgrep", &["rg"], OutputMode::JsonLines),
        ToolSpec::new("fd", "fd", &["fd"], OutputMode::NullSeparated),
        ToolSpec::new("bat", "bat", &["bat"], OutputMode::PlainText),
        ToolSpec::new("jq", "jq", &["jq"], OutputMode::JsonStdout),
        ToolSpec::new("curl", "curl", &["curl"], OutputMode::PlainText),
        ToolSpec::new("docker", "Docker", &["docker"], OutputMode::JsonLines),
        ToolSpec::new("kubectl", "kubectl", &["kubectl"], OutputMode::JsonStdout),
        ToolSpec::new("ssh", "SSH", &["ssh"], OutputMode::PlainText),
        ToolSpec::new("cargo", "Cargo", &["cargo"], OutputMode::JsonLines),
        ToolSpec::new("npm", "npm", &["npm"], OutputMode::JsonStdout),
        ToolSpec::new("pnpm", "pnpm", &["pnpm"], OutputMode::JsonStdout),
        ToolSpec::new("bun", "bun", &["bun"], OutputMode::JsonStdout),
        ToolSpec::new("uv", "uv", &["uv"], OutputMode::PlainText),
        ToolSpec::new(
            "python",
            "Python",
            &["python3", "python"],
            OutputMode::JsonStdout,
        ),
        ToolSpec::new("go", "Go", &["go"], OutputMode::JsonStdout),
        ToolSpec::new("brew", "Homebrew", &["brew"], OutputMode::JsonStdout),
        ToolSpec::new(
            "hyperfine",
            "hyperfine",
            &["hyperfine"],
            OutputMode::JsonStdout,
        ),
    ]
}

pub fn adapter(id: &str) -> Option<CliAdapter> {
    catalog()
        .into_iter()
        .find(|spec| spec.id == id)
        .map(CliAdapter::new)
}

pub fn jq_adapter() -> CliAdapter {
    adapter("jq").expect("jq is in the catalog")
}

pub fn all_adapters() -> Vec<CliAdapter> {
    catalog().into_iter().map(CliAdapter::new).collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolStatus {
    pub id: String,
    pub installed: bool,
    pub path: Option<PathBuf>,
}

pub fn status_report() -> Vec<ToolStatus> {
    all_adapters()
        .into_iter()
        .map(|adapter| {
            let path = adapter.locate();
            ToolStatus {
                id: adapter.spec().id.to_string(),
                installed: path.is_some(),
                path,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_security::Policy;

    #[tokio::test]
    async fn missing_jq_returns_structured_unavailable_error() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = jq_adapter().with_path(dir.path().as_os_str().to_os_string());
        assert!(adapter.locate().is_none());
        let err = adapter
            .invoke(
                ProviderRequest {
                    capability: Capability::Execute,
                    payload: serde_json::json!(["-n", "{}"]),
                },
                &Policy::local_default(),
            )
            .await
            .unwrap_err();
        match err {
            ProviderError::Unavailable(id, message) => {
                assert_eq!(id, "jq");
                assert!(message.contains("not installed"));
            }
            ProviderError::Unsupported(cap, id) => {
                assert!(id.contains("jq") || cap.contains("jq"));
            }
            other => panic!("expected unavailable/unsupported, got {other}"),
        }
    }

    #[test]
    fn git_status_injects_porcelain_not_human_output() {
        let adapter = adapter("git").unwrap();
        let args = adapter.prepare_args(&["status".into()]);
        assert!(args.iter().any(|a| a.contains("porcelain")));
        assert!(!args.iter().any(|a| a == "--short"));
    }

    #[test]
    fn catalog_covers_required_tools() {
        let ids: BTreeSet<_> = catalog().into_iter().map(|s| s.id).collect();
        for required in [
            "git",
            "gh",
            "rg",
            "fd",
            "bat",
            "jq",
            "curl",
            "docker",
            "kubectl",
            "ssh",
            "cargo",
            "npm",
            "pnpm",
            "bun",
            "uv",
            "python",
            "go",
            "brew",
            "hyperfine",
        ] {
            assert!(ids.contains(required), "missing {required}");
        }
    }
}
