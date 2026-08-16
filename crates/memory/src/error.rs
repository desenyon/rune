use thiserror::Error;

pub type Result<T> = std::result::Result<T, MemoryError>;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error("{0}")]
    Message(String),
    #[error("invalid memory: {0}")]
    Invalid(String),
    #[error("node not found: {0}")]
    NotFound(String),
    #[error("merge `{0}` is not reversible because its snapshot is missing")]
    IrreversibleMerge(String),
    #[error("conflicting memories preserved; refusing to overwrite `{0}`")]
    RefuseOverwrite(String),
}

impl MemoryError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
}
