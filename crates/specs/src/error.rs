use thiserror::Error;

pub type Result<T> = std::result::Result<T, SpecError>;

#[derive(Debug, Error)]
pub enum SpecError {
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error("{0}")]
    Message(String),
    #[error("invalid specification: {0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
}

impl SpecError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
}
