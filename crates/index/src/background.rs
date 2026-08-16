use crate::error::Result;
use crate::indexer::Indexer;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexJob {
    pub path: PathBuf,
    pub priority: u64,
}

#[derive(Eq, PartialEq)]
struct HeapJob {
    priority: u64,
    seq: u64,
    path: PathBuf,
}

impl Ord for HeapJob {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

impl PartialOrd for HeapJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct IndexQueue {
    inner: Mutex<BinaryHeap<HeapJob>>,
    seq: AtomicU64,
    pause: AtomicBool,
}

impl Default for IndexQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(BinaryHeap::new()),
            seq: AtomicU64::new(0),
            pause: AtomicBool::new(false),
        }
    }

    pub fn set_paused(&self, paused: bool) {
        self.pause.store(paused, AtomicOrdering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
        self.pause.load(AtomicOrdering::SeqCst)
    }

    pub fn enqueue(&self, path: PathBuf, recently_touched: bool) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let priority = if recently_touched { now + 1_000_000_000 } else { now };
        let seq = self.seq.fetch_add(1, AtomicOrdering::SeqCst);
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).push(HeapJob {
            priority,
            seq,
            path,
        });
    }

    pub fn pending(&self) -> usize {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn pop(&self) -> Option<IndexJob> {
        if self.is_paused() {
            return None;
        }
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop()
            .map(|job| IndexJob {
                path: job.path,
                priority: job.priority,
            })
    }
}

pub struct BackgroundIndexer {
    pub queue: IndexQueue,
}

impl Default for BackgroundIndexer {
    fn default() -> Self {
        Self {
            queue: IndexQueue::new(),
        }
    }
}

impl BackgroundIndexer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&self, path: PathBuf, recently_touched: bool) {
        self.queue.enqueue(path, recently_touched);
    }

    pub fn set_paused(&self, paused: bool) {
        self.queue.set_paused(paused);
    }

    pub fn is_paused(&self) -> bool {
        self.queue.is_paused()
    }

    /// Process queued work until empty, paused, or `max_jobs` is reached.
    /// Yields immediately when the pause flag is set.
    pub fn pump(&self, indexer: &Indexer, max_jobs: usize) -> Result<usize> {
        let mut processed = 0;
        while processed < max_jobs {
            if self.queue.is_paused() {
                break;
            }
            let Some(job) = self.queue.pop() else {
                break;
            };
            indexer.index_path(&job.path)?;
            processed += 1;
        }
        Ok(processed)
    }

    pub async fn run_until_idle(&self, indexer: &Indexer) -> Result<usize> {
        let mut total = 0;
        loop {
            if self.queue.is_paused() {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            }
            if self.queue.pending() == 0 {
                break;
            }
            total += self.pump(indexer, 32)?;
            tokio::task::yield_now().await;
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_prevents_pop() {
        let queue = IndexQueue::new();
        queue.enqueue(PathBuf::from("a.rs"), true);
        queue.set_paused(true);
        assert!(queue.pop().is_none());
        queue.set_paused(false);
        assert!(queue.pop().is_some());
    }

    #[test]
    fn recently_touched_sorts_first() {
        let queue = IndexQueue::new();
        queue.enqueue(PathBuf::from("old.rs"), false);
        queue.enqueue(PathBuf::from("new.rs"), true);
        let first = queue.pop().unwrap();
        assert_eq!(first.path, PathBuf::from("new.rs"));
    }
}
