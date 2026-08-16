use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, IndexError>;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("storage error: {0}")]
    Storage(#[from] rune_storage::StorageError),
    #[error("serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("tree-sitter error: {0}")]
    TreeSitter(String),
    #[error("git error: {0}")]
    Git(String),
    #[error("notify error: {0}")]
    Notify(String),
    #[error("path is not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("identifier parse error: {0}")]
    Id(String),
    #[error("{0}")]
    Message(String),
}

impl IndexError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn git(message: impl Into<String>) -> Self {
        Self::Git(message.into())
    }
}

impl From<notify::Error> for IndexError {
    fn from(value: notify::Error) -> Self {
        Self::Notify(value.to_string())
    }
}
