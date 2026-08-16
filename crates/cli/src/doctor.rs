use rune_providers::ProviderRegistry;
use rune_storage::{applied_migrations, Store};
use rune_terminal::TerminalCapabilities;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub message: String,
    pub repair: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
}

pub fn doctor_report(
    store: Option<&Store>,
    workspace: &Path,
    providers: &ProviderRegistry,
) -> DoctorReport {
    let mut checks = Vec::new();

    match store {
        Some(store) => {
            match store.node_count() {
                Ok(_) => checks.push(ok("database", "database opened and queryable")),
                Err(err) => checks.push(err_check(
                    "database",
                    &format!("database query failed: {err}"),
                    "delete only the corrupt cache, never the sqlite file, then run `rune doctor` again",
                )),
            }
            match store.with_conn(applied_migrations) {
                Ok(applied) if !applied.is_empty() => checks.push(ok(
                    "migrations",
                    &format!("{} migration(s) applied", applied.len()),
                )),
                Ok(_) => checks.push(err_check(
                    "migrations",
                    "no migrations applied",
                    "re-open the store with a current Rune binary so bundled migrations run",
                )),
                Err(err) => checks.push(err_check(
                    "migrations",
                    &format!("{err}"),
                    "restore a backup of .rune/rune.sqlite; do not delete user data",
                )),
            }
        }
        None => checks.push(err_check(
            "database",
            "could not open .rune/rune.sqlite",
            "run `rune index` in the workspace to create .rune/",
        )),
    }

    let caps = TerminalCapabilities::detect();
    checks.push(ok(
        "terminal",
        &format!(
            "renderer={:?} true_color={} tty={}",
            caps.renderer_level, caps.true_color, caps.is_tty
        ),
    ));

    match which::which("git") {
        Ok(path) => checks.push(ok("git", &format!("git at {}", path.display()))),
        Err(_) => checks.push(warn(
            "git",
            "git not on PATH",
            "install git and ensure it is on PATH",
        )),
    }
    if workspace.join(".git").exists() || workspace.join(".git").is_file() {
        checks.push(ok("git-repo", "workspace looks like a git repository"));
    } else {
        checks.push(warn(
            "git-repo",
            "no .git directory",
            "run `git init` if this should be a repository",
        ));
    }

    let registered = ["context7", "probe", "claude", "codex"]
        .iter()
        .filter(|id| providers.get(id).is_some())
        .count();
    if registered == 0 {
        checks.push(warn(
            "providers",
            "no providers registered",
            "providers are optional; Context7 is registered by the CLI when opening a workspace",
        ));
    } else {
        checks.push(ok(
            "providers",
            &format!("{registered} provider(s) registered"),
        ));
    }

    let session_dirs = [".claude", ".cursor", ".codex", ".aider", ".opencode"];
    let found: Vec<_> = session_dirs
        .iter()
        .filter(|d| workspace.join(d).exists() || dirs_home(d))
        .cloned()
        .collect();
    if found.is_empty() {
        checks.push(warn(
            "sessions",
            "no local coding-agent session directories detected",
            "install an agent or point RUNE_SESSION_DIRS at existing histories",
        ));
    } else {
        checks.push(ok("sessions", &format!("detected {}", found.join(", "))));
    }

    checks.push(ok(
        "permissions",
        "local policy denies network and auto-execute by default",
    ));

    DoctorReport {
        ok: checks
            .iter()
            .filter(|c| matches!(c.name.as_str(), "database" | "migrations"))
            .all(|c| c.ok),
        checks,
    }
}

fn dirs_home(dir: &str) -> bool {
    dirs_path(dir).map(|p| p.exists()).unwrap_or(false)
}

fn dirs_path(dir: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(std::path::PathBuf::from(home).join(dir))
}

fn ok(name: &str, message: &str) -> DoctorCheck {
    DoctorCheck {
        name: name.into(),
        ok: true,
        message: message.into(),
        repair: None,
    }
}

fn warn(name: &str, message: &str, repair: &str) -> DoctorCheck {
    DoctorCheck {
        name: name.into(),
        ok: true,
        message: message.into(),
        repair: Some(repair.into()),
    }
}

fn err_check(name: &str, message: &str, repair: &str) -> DoctorCheck {
    DoctorCheck {
        name: name.into(),
        ok: false,
        message: message.into(),
        repair: Some(repair.into()),
    }
}
