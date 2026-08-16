use thiserror::Error;

pub type Result<T> = std::result::Result<T, SearchError>;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error("{0}")]
    Message(String),
    #[error("invalid search request: {0}")]
    InvalidRequest(String),
    #[error("node not found: {0}")]
    NotFound(String),
    #[error("ambiguous name `{name}` matched {count} nodes")]
    Ambiguous { name: String, count: usize },
}

impl SearchError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidRequest(message.into())
    }
}
