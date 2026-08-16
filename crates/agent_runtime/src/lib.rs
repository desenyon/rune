//! Local subprocess agent runtime with policy gates, event normalization, and output policy.

use regex::Regex;
use rune_core::{NodeId, Timestamp};
use rune_security::{redact_secrets, Permission, Policy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("permission denied: {0:?}")]
    Denied(Permission),
    #[error("auto command execution is disabled")]
    AutoExecuteDisabled,
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Denied,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Thinking,
    Search,
    Read,
    Write,
    Command,
    Test,
    Error,
    Warning,
    Decision,
    Question,
    Result,
    Handoff,
    Completion,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub kind: EventKind,
    pub summary: String,
    pub raw: String,
    pub timestamp: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Cost {
    pub amount: f64,
    pub currency: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessRef {
    pub pid: u32,
    pub program: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentExecution {
    pub provider: String,
    pub model: Option<String>,
    pub task: Option<NodeId>,
    pub context_capsule_id: Option<NodeId>,
    pub working_directory: PathBuf,
    pub worktree: Option<PathBuf>,
    pub environment: BTreeMap<String, String>,
    pub permissions: Policy,
    pub process: Option<ProcessRef>,
    pub status: ExecutionStatus,
    pub token_usage: Option<TokenUsage>,
    pub cost: Option<Cost>,
    pub events: Vec<NormalizedEvent>,
    pub result: Option<ExecutionResult>,
}

#[derive(Clone, Debug)]
pub struct LaunchSpec {
    pub provider: String,
    pub model: Option<String>,
    pub task: Option<NodeId>,
    pub context_capsule_id: Option<NodeId>,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub worktree: Option<PathBuf>,
    pub environment: BTreeMap<String, String>,
    pub network: bool,
    pub timeout: Duration,
}

impl Default for LaunchSpec {
    fn default() -> Self {
        Self {
            provider: "local".into(),
            model: None,
            task: None,
            context_capsule_id: None,
            program: PathBuf::from("/bin/true"),
            args: Vec::new(),
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            worktree: None,
            environment: BTreeMap::new(),
            network: false,
            timeout: Duration::from_secs(120),
        }
    }
}

pub fn sanitize_env(env: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    env.iter()
        .map(|(key, value)| {
            let upper = key.to_ascii_uppercase();
            if upper.contains("SECRET")
                || upper.contains("TOKEN")
                || upper.contains("PASSWORD")
                || upper.contains("API_KEY")
                || upper.ends_with("_KEY")
            {
                (key.clone(), "[REDACTED]".into())
            } else {
                let (body, _) = redact_secrets(value);
                (key.clone(), body)
            }
        })
        .collect()
}

pub fn normalize_line(raw: &str) -> NormalizedEvent {
    let trimmed = raw.trim();
    let kind = classify_line(trimmed);
    NormalizedEvent {
        kind: kind.clone(),
        summary: summarize(&kind, trimmed),
        raw: raw.to_string(),
        timestamp: Timestamp::now(),
    }
}

pub fn normalize_output(stdout: &str, stderr: &str) -> Vec<NormalizedEvent> {
    let mut events = Vec::new();
    for line in stdout.lines().chain(stderr.lines()) {
        if line.trim().is_empty() {
            continue;
        }
        events.push(normalize_line(line));
    }
    events
}

fn classify_line(line: &str) -> EventKind {
    let lower = line.to_ascii_lowercase();
    if complete_re().is_match(&lower) {
        EventKind::Completion
    } else if search_re().is_match(&lower) {
        EventKind::Search
    } else if read_re().is_match(&lower) {
        EventKind::Read
    } else if write_re().is_match(&lower) {
        EventKind::Write
    } else if test_re().is_match(&lower) {
        EventKind::Test
    } else if command_re().is_match(&lower) || line.starts_with("$ ") {
        EventKind::Command
    } else if lower.contains("handoff") {
        EventKind::Handoff
    } else if lower.contains("error") || lower.contains("failed") {
        EventKind::Error
    } else if lower.contains("warning") || lower.contains("warn:") {
        EventKind::Warning
    } else if decision_re().is_match(&lower) {
        EventKind::Decision
    } else if line.trim_end().ends_with('?') || lower.contains("open question") {
        EventKind::Question
    } else if lower.contains("result") || lower.starts_with("found ") {
        EventKind::Result
    } else if thinking_re().is_match(&lower) {
        EventKind::Thinking
    } else {
        EventKind::Result
    }
}

fn summarize(kind: &EventKind, line: &str) -> String {
    let compact = line.split_whitespace().collect::<Vec<_>>().join(" ");
    match kind {
        EventKind::Search => format!("SEARCH {}", tail(&compact)),
        EventKind::Read => format!("READ {}", tail(&compact)),
        EventKind::Write => format!("WRITE {}", tail(&compact)),
        EventKind::Command => format!("CMD {}", tail(&compact)),
        EventKind::Test => format!("TEST {}", tail(&compact)),
        EventKind::Error => format!("ERROR {}", tail(&compact)),
        EventKind::Warning => format!("WARN {}", tail(&compact)),
        EventKind::Decision => format!("DECIDE {}", tail(&compact)),
        EventKind::Question => format!("ASK {}", tail(&compact)),
        EventKind::Handoff => format!("HANDOFF {}", tail(&compact)),
        EventKind::Completion => "COMPLETE".into(),
        EventKind::Thinking => format!("THINK {}", tail(&compact)),
        EventKind::Result => format!("RESULT {}", tail(&compact)),
    }
}

fn tail(line: &str) -> String {
    line.splitn(2, char::is_whitespace)
        .nth(1)
        .unwrap_or(line)
        .chars()
        .take(80)
        .collect()
}

fn search_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*(search|grep|rg|found)\b").expect("static regex"))
}
fn read_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*(read|reading|open)\b").expect("static regex"))
}
fn write_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*(write|edit|wrote|edited)\b").expect("static regex"))
}
fn test_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^\s*(test|cargo test|passed|failed tests)\b").expect("static regex")
    })
}
fn command_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*(cmd|command|running)\b").expect("static regex"))
}
fn decision_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\b(decided|decision:|going with)\b").expect("static regex"))
}
fn thinking_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*(think|thinking|let me)\b").expect("static regex"))
}
fn complete_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^\s*(complete|completed|done|finished)\b").expect("static regex")
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunicationPolicy {
    Full,
    Concise,
    Minimal,
    Machine,
}

pub fn format_events(policy: CommunicationPolicy, events: &[NormalizedEvent]) -> String {
    match policy {
        CommunicationPolicy::Full => events
            .iter()
            .map(|event| format!("{:?}: {}\n{}", event.kind, event.summary, event.raw))
            .collect::<Vec<_>>()
            .join("\n\n"),
        CommunicationPolicy::Concise => events
            .iter()
            .map(|event| {
                format!(
                    "{}: {}",
                    format!("{:?}", event.kind).to_lowercase(),
                    event.summary
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        CommunicationPolicy::Minimal => events
            .iter()
            .filter(|event| !matches!(event.kind, EventKind::Thinking))
            .map(|event| event.summary.clone())
            .collect::<Vec<_>>()
            .join("\n"),
        CommunicationPolicy::Machine => events
            .iter()
            .map(|event| machine_line(event))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn machine_line(event: &NormalizedEvent) -> String {
    match event.kind {
        EventKind::Search => format!("SEARCH {}", machine_token(&event.raw)),
        EventKind::Read => format!("READ {}", machine_token(&event.raw)),
        EventKind::Write => format!("EDIT {}", machine_token(&event.raw)),
        EventKind::Command => format!("CMD {}", machine_token(&event.raw)),
        EventKind::Test => format!("TEST {}", machine_token(&event.raw)),
        EventKind::Error => format!("ERROR {}", machine_token(&event.raw)),
        EventKind::Warning => format!("WARN {}", machine_token(&event.raw)),
        EventKind::Decision => format!("DECIDE {}", machine_token(&event.raw)),
        EventKind::Question => format!("ASK {}", machine_token(&event.raw)),
        EventKind::Result => format!("FOUND {}", machine_token(&event.raw)),
        EventKind::Handoff => format!("HANDOFF {}", machine_token(&event.raw)),
        EventKind::Thinking => format!("THINK {}", machine_token(&event.raw)),
        EventKind::Completion => "COMPLETE".into(),
    }
}

fn machine_token(raw: &str) -> String {
    raw.split_whitespace()
        .find(|part| {
            let lower = part.to_ascii_lowercase();
            !matches!(
                lower.as_str(),
                "search" | "read" | "write" | "edit" | "complete" | "found" | "test" | "cmd"
            )
        })
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.')
        .to_string()
}

pub fn refuse_if_denied(policy: &Policy) -> Result<()> {
    if !policy.permits(Permission::ProcessExecute) {
        return Err(RuntimeError::Denied(Permission::ProcessExecute));
    }
    if !policy.permits(Permission::AgentSubprocess) {
        return Err(RuntimeError::Denied(Permission::AgentSubprocess));
    }
    Ok(())
}

pub async fn launch(spec: LaunchSpec, policy: &Policy) -> Result<AgentExecution> {
    if let Err(err) = refuse_if_denied(policy) {
        return Ok(denied_execution(&spec, policy, err));
    }
    if spec.network && !policy.permits(Permission::Network) {
        return Ok(denied_execution(
            &spec,
            policy,
            RuntimeError::Denied(Permission::Network),
        ));
    }
    if !spec.program.exists() {
        return Err(RuntimeError::Message(format!(
            "agent program not found: {}",
            spec.program.display()
        )));
    }

    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .current_dir(workdir(&spec))
        .env("PATH", tool_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (key, value) in &spec.environment {
        cmd.env(key, value);
    }
    if !policy.permits(Permission::Network) {
        cmd.env_remove("http_proxy")
            .env_remove("https_proxy")
            .env_remove("HTTP_PROXY")
            .env_remove("HTTPS_PROXY");
    }

    let mut child = cmd.spawn()?;
    let pid = child.id().unwrap_or(0);
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut events = Vec::new();

    let stdout_task = async {
        let mut buf = String::new();
        if let Some(out) = stdout {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                buf.push_str(&line);
                buf.push('\n');
            }
        }
        buf
    };
    let stderr_task = async {
        let mut buf = String::new();
        if let Some(err) = stderr {
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                buf.push_str(&line);
                buf.push('\n');
            }
        }
        buf
    };

    let wait = tokio::time::timeout(spec.timeout, async {
        let (out, err) = tokio::join!(stdout_task, stderr_task);
        let status = child.wait().await?;
        Ok::<_, std::io::Error>((status, out, err))
    })
    .await;

    let (status, stdout, stderr) = match wait {
        Ok(Ok((status, out, err))) => (status, out, err),
        Ok(Err(err)) => return Err(err.into()),
        Err(_) => {
            let _ = child.start_kill();
            return Err(RuntimeError::Message("agent process timed out".into()));
        }
    };
    events.extend(normalize_output(&stdout, &stderr));
    if !policy.auto_execute_commands {
        for event in &events {
            if event.kind == EventKind::Command {
                tracing::debug!("ignoring command event because auto_execute_commands is false");
            }
        }
    }

    let success = status.success();
    Ok(AgentExecution {
        provider: spec.provider,
        model: spec.model,
        task: spec.task,
        context_capsule_id: spec.context_capsule_id,
        working_directory: spec.working_directory,
        worktree: spec.worktree,
        environment: sanitize_env(&spec.environment),
        permissions: policy.clone(),
        process: Some(ProcessRef {
            pid,
            program: spec.program,
        }),
        status: if success {
            ExecutionStatus::Succeeded
        } else {
            ExecutionStatus::Failed
        },
        token_usage: None,
        cost: None,
        events,
        result: Some(ExecutionResult {
            exit_code: status.code(),
            stdout,
            stderr,
        }),
    })
}

fn denied_execution(spec: &LaunchSpec, policy: &Policy, err: RuntimeError) -> AgentExecution {
    AgentExecution {
        provider: spec.provider.clone(),
        model: spec.model.clone(),
        task: spec.task,
        context_capsule_id: spec.context_capsule_id,
        working_directory: spec.working_directory.clone(),
        worktree: spec.worktree.clone(),
        environment: sanitize_env(&spec.environment),
        permissions: policy.clone(),
        process: None,
        status: ExecutionStatus::Denied,
        token_usage: None,
        cost: None,
        events: vec![NormalizedEvent {
            kind: EventKind::Error,
            summary: err.to_string(),
            raw: err.to_string(),
            timestamp: Timestamp::now(),
        }],
        result: None,
    }
}

fn workdir(spec: &LaunchSpec) -> &Path {
    spec.worktree
        .as_deref()
        .unwrap_or(spec.working_directory.as_path())
}

fn tool_path() -> std::ffi::OsString {
    let mut parts = Vec::new();
    if let Some(home) = dirs::home_dir() {
        parts.push(home.join(".cargo").join("bin"));
    }
    if let Some(path) = std::env::var_os("PATH") {
        parts.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(parts).unwrap_or_else(|_| std::env::var_os("PATH").unwrap_or_default())
}

/// Never spawn extra commands from model output unless auto_execute_commands is on.
pub fn maybe_execute_command(policy: &Policy, _command: &str) -> Result<()> {
    if !policy.auto_execute_commands {
        return Err(RuntimeError::AutoExecuteDisabled);
    }
    if !policy.permits(Permission::ProcessExecute) {
        return Err(RuntimeError::Denied(Permission::ProcessExecute));
    }
    Err(RuntimeError::Message(
        "command passthrough is not enabled; use an explicit tool adapter".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_security::Policy;

    #[tokio::test]
    async fn refuses_execute_when_policy_denies_process_execute() {
        let policy = Policy::local_default();
        assert!(!policy.permits(Permission::ProcessExecute));
        let spec = LaunchSpec {
            program: PathBuf::from("/bin/echo"),
            args: vec!["hello".into()],
            ..LaunchSpec::default()
        };
        let execution = launch(spec, &policy).await.unwrap();
        assert_eq!(execution.status, ExecutionStatus::Denied);
        assert!(execution.process.is_none());
        match refuse_if_denied(&policy) {
            Err(RuntimeError::Denied(Permission::ProcessExecute)) => {}
            other => panic!("expected ProcessExecute denial, got {other:?}"),
        }
    }

    #[test]
    fn machine_policy_matches_expected_style() {
        let events = vec![
            normalize_line("SEARCH auth"),
            normalize_line("READ TokenStore"),
            normalize_line("COMPLETE"),
        ];
        let rendered = format_events(CommunicationPolicy::Machine, &events);
        assert_eq!(rendered, "SEARCH auth\nREAD TokenStore\nCOMPLETE");
    }

    #[test]
    fn default_policy_does_not_auto_execute_commands() {
        let policy = Policy::local_default();
        assert!(!policy.auto_execute_commands);
        assert!(matches!(
            maybe_execute_command(&policy, "rm -rf /").unwrap_err(),
            RuntimeError::AutoExecuteDisabled
        ));
    }
}
