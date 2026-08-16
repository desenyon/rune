use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, GitIntelError>;

#[derive(Debug, Error)]
pub enum GitIntelError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("storage error: {0}")]
    Storage(#[from] rune_storage::StorageError),
    #[error("serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("git not found: {0}")]
    GitMissing(String),
    #[error("git error: {0}")]
    Git(String),
    #[error("not a git repository: {0}")]
    NotARepository(PathBuf),
    #[error("{0}")]
    Message(String),
}

impl GitIntelError {
    pub fn git(message: impl Into<String>) -> Self {
        Self::Git(message.into())
    }
}
