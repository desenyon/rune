use rune_providers::ProviderError;
use rune_storage::StorageError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, SessionError>;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

impl SessionError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}
