use crate::error::Result;
use crate::languages::{looks_like_temp_write, path_has_storm_component};
use notify::event::{EventKind, ModifyKind, RenameMode};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchConfig {
    pub debounce: Duration,
    pub storm_threshold: usize,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(150),
            storm_threshold: 500,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoalescedChange {
    Index(PathBuf),
    Remove(PathBuf),
    Rename { from: PathBuf, to: PathBuf },
}

pub fn classify_event(root: &Path, event: &Event) -> Option<CoalescedChange> {
    let paths: Vec<&Path> = event
        .paths
        .iter()
        .map(PathBuf::as_path)
        .filter(|path| !path_has_storm_component(root, path) && !looks_like_temp_write(path))
        .collect();
    if paths.is_empty() {
        return None;
    }
    match event.kind {
        EventKind::Remove(_) => Some(CoalescedChange::Remove(paths[0].to_path_buf())),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if event.paths.len() >= 2 => {
            Some(CoalescedChange::Rename {
                from: event.paths[0].clone(),
                to: event.paths[1].clone(),
            })
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            Some(CoalescedChange::Index(paths[0].to_path_buf()))
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            Some(CoalescedChange::Remove(paths[0].to_path_buf()))
        }
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Any => {
            Some(CoalescedChange::Index(paths[0].to_path_buf()))
        }
        _ => None,
    }
}

pub fn coalesce_events(root: &Path, events: &[Event], storm_threshold: usize) -> Vec<CoalescedChange> {
    if events.len() > storm_threshold {
        tracing::warn!(
            count = events.len(),
            "dropping generated-file storm above threshold"
        );
        return Vec::new();
    }
    let mut index = BTreeSet::new();
    let mut remove = BTreeSet::new();
    let mut renames = Vec::new();
    for event in events {
        match classify_event(root, event) {
            Some(CoalescedChange::Index(path)) => {
                remove.remove(&path);
                index.insert(path);
            }
            Some(CoalescedChange::Remove(path)) => {
                index.remove(&path);
                remove.insert(path);
            }
            Some(CoalescedChange::Rename { from, to }) => {
                index.remove(&from);
                remove.remove(&to);
                index.insert(to.clone());
                renames.push(CoalescedChange::Rename { from, to });
            }
            None => {}
        }
    }
    let mut out = renames;
    out.extend(index.into_iter().map(CoalescedChange::Index));
    out.extend(remove.into_iter().map(CoalescedChange::Remove));
    out
}

pub struct WorkspaceWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    root: PathBuf,
    config: WatchConfig,
}

impl WorkspaceWatcher {
    pub fn start(root: impl Into<PathBuf>) -> Result<Self> {
        Self::start_with(root, WatchConfig::default())
    }

    pub fn start_with(root: impl Into<PathBuf>, config: WatchConfig) -> Result<Self> {
        let root = root.into();
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(&root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            rx,
            root,
            config,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn poll_debounced(&self, wait: Duration) -> Result<Vec<CoalescedChange>> {
        let deadline = Instant::now() + wait;
        let mut batch = Vec::new();
        let mut last = Instant::now();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() && batch.is_empty() {
                break;
            }
            let timeout = if batch.is_empty() {
                remaining
            } else {
                self.config.debounce.saturating_sub(last.elapsed()).min(remaining)
            };
            match self.rx.recv_timeout(timeout) {
                Ok(Ok(event)) => {
                    batch.push(event);
                    last = Instant::now();
                    if batch.len() > self.config.storm_threshold {
                        tracing::warn!("watch storm detected; discarding batch");
                        batch.clear();
                        drain_pending(&self.rx);
                        break;
                    }
                }
                Ok(Err(err)) => return Err(err.into()),
                Err(RecvTimeoutError::Timeout) => {
                    if !batch.is_empty() && last.elapsed() >= self.config.debounce {
                        break;
                    }
                    if remaining.is_zero() {
                        break;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(coalesce_events(&self.root, &batch, self.config.storm_threshold))
    }
}

fn drain_pending(rx: &Receiver<notify::Result<Event>>) {
    while rx.try_recv().is_ok() {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::Event;

    #[test]
    fn ignores_target_and_node_modules() {
        let root = Path::new("/repo");
        let event = Event::new(EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Any)))
            .add_path(PathBuf::from("/repo/target/debug/foo"));
        assert!(classify_event(root, &event).is_none());
        let ok = Event::new(EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Any)))
            .add_path(PathBuf::from("/repo/src/lib.rs"));
        assert!(matches!(classify_event(root, &ok), Some(CoalescedChange::Index(_))));
    }

    #[test]
    fn rename_both_is_atomic_save() {
        let root = Path::new("/repo");
        let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from("/repo/src/lib.rs.tmp"))
            .add_path(PathBuf::from("/repo/src/lib.rs"));
        let change = classify_event(root, &event).unwrap();
        assert!(matches!(change, CoalescedChange::Rename { .. }));
    }
}
