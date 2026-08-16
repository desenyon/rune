use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub user: Option<String>,
    pub command: String,
    pub cwd: PathBuf,
}

/// Discover processes whose current working directory is inside `repo`.
/// Never terminates processes.
pub fn discover_processes(repo: &Path) -> Result<Vec<ProcessInfo>> {
    let repo = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    #[cfg(target_os = "linux")]
    {
        return discover_linux(&repo);
    }
    #[cfg(target_os = "macos")]
    {
        return discover_macos(&repo);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        tracing::info!("process discovery is not available on this platform");
        let _ = repo;
        Ok(Vec::new())
    }
}

#[cfg(target_os = "linux")]
fn discover_linux(repo: &Path) -> Result<Vec<ProcessInfo>> {
    parse_proc_linux(Path::new("/proc"), repo)
}

pub fn parse_proc_linux(proc_root: &Path, repo: &Path) -> Result<Vec<ProcessInfo>> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(proc_root) {
        Ok(entries) => entries,
        Err(err) => {
            tracing::warn!(error = %err, "unable to read proc filesystem");
            return Ok(out);
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let cwd_link = entry.path().join("cwd");
        let cwd = match std::fs::read_link(&cwd_link) {
            Ok(cwd) => cwd,
            Err(_) => continue,
        };
        if !cwd_in_repo(&cwd, repo) {
            continue;
        }
        let cmdline = std::fs::read(entry.path().join("cmdline")).unwrap_or_default();
        let command = if cmdline.is_empty() {
            std::fs::read_to_string(entry.path().join("comm"))
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            String::from_utf8_lossy(&cmdline)
                .split('\0')
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let ppid = std::fs::read_to_string(entry.path().join("stat"))
            .ok()
            .and_then(|stat| parse_ppid_from_stat(&stat));
        out.push(ProcessInfo {
            pid,
            ppid,
            user: None,
            command,
            cwd,
        });
    }
    Ok(out)
}

fn parse_ppid_from_stat(stat: &str) -> Option<u32> {
    let close = stat.rfind(')')?;
    let rest = stat[close + 1..].split_whitespace().collect::<Vec<_>>();
    rest.get(1)?.parse().ok()
}

#[cfg(target_os = "macos")]
fn discover_macos(repo: &Path) -> Result<Vec<ProcessInfo>> {
    let lsof = match which::which("lsof") {
        Ok(path) => path,
        Err(_) => {
            tracing::warn!("lsof not found; cannot associate process working directories");
            return Ok(Vec::new());
        }
    };
    let output = match std::process::Command::new(lsof)
        .args(["-nP", "-d", "cwd", "-a", "-F", "pn"])
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            tracing::warn!(error = %err, "lsof cwd listing unavailable");
            return Ok(Vec::new());
        }
    };
    if !output.status.success() {
        tracing::warn!(
            stderr = %String::from_utf8_lossy(&output.stderr),
            "lsof cwd listing failed"
        );
        return Ok(Vec::new());
    }
    let cwds = parse_lsof_cwd(&String::from_utf8_lossy(&output.stdout));
    let ps = match which::which("ps") {
        Ok(path) => path,
        Err(_) => return Ok(filter_cwds(cwds, repo, &HashMap::new())),
    };
    let ps_out = match std::process::Command::new(ps)
        .args(["-ax", "-o", "pid=", "-o", "ppid=", "-o", "user=", "-o", "command="])
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            tracing::warn!(error = %err, "ps listing unavailable");
            return Ok(filter_cwds(cwds, repo, &HashMap::new()));
        }
    };
    let meta = parse_ps(&String::from_utf8_lossy(&ps_out.stdout));
    Ok(filter_cwds(cwds, repo, &meta))
}

#[derive(Clone, Debug)]
pub struct ProcessMeta {
    pub ppid: Option<u32>,
    pub user: Option<String>,
    pub command: String,
}

pub fn parse_lsof_cwd(output: &str) -> HashMap<u32, PathBuf> {
    let mut out = HashMap::new();
    let mut pid = None;
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix('p') {
            pid = rest.parse().ok();
        } else if let Some(rest) = line.strip_prefix('n') {
            if let Some(pid) = pid {
                out.insert(pid, PathBuf::from(rest));
            }
        }
    }
    out
}

pub fn parse_ps(output: &str) -> HashMap<u32, ProcessMeta> {
    let mut out = HashMap::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(pid) = parts.next().and_then(|s| s.parse().ok()) else {
            continue;
        };
        let ppid = parts.next().and_then(|s| s.parse().ok());
        let user = parts.next().map(ToOwned::to_owned);
        let command = parts.collect::<Vec<_>>().join(" ");
        out.insert(
            pid,
            ProcessMeta {
                ppid,
                user,
                command,
            },
        );
    }
    out
}

fn filter_cwds(
    cwds: HashMap<u32, PathBuf>,
    repo: &Path,
    meta: &HashMap<u32, ProcessMeta>,
) -> Vec<ProcessInfo> {
    let mut out = Vec::new();
    for (pid, cwd) in cwds {
        if !cwd_in_repo(&cwd, repo) {
            continue;
        }
        let info = meta.get(&pid);
        out.push(ProcessInfo {
            pid,
            ppid: info.and_then(|m| m.ppid),
            user: info.and_then(|m| m.user.clone()),
            command: info.map(|m| m.command.clone()).unwrap_or_default(),
            cwd,
        });
    }
    out.sort_by_key(|p| p.pid);
    out
}

pub fn cwd_in_repo(cwd: &Path, repo: &Path) -> bool {
    cwd == repo || cwd.starts_with(repo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsof_and_ps_parsers_associate_cwd() {
        let lsof = "p42\nfcwd\nn/tmp/repo\np7\nfcwd\nn/elsewhere\n";
        let ps = "   42     1 alice cargo test\n    7     1 bob vim\n";
        let cwds = parse_lsof_cwd(lsof);
        let meta = parse_ps(ps);
        let found = filter_cwds(cwds, Path::new("/tmp/repo"), &meta);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pid, 42);
        assert_eq!(found[0].command, "cargo test");
    }

    #[test]
    fn linux_stat_ppid_parses() {
        let stat = "42 (bash) S 17 42 42 0 -1 4194304 0 0 0 0 0 0 0 0 20 0 1 0 0 0 0";
        assert_eq!(parse_ppid_from_stat(stat), Some(17));
    }
}
