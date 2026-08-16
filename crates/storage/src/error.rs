use thiserror::Error;

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration `{name}` failed: {source}")]
    Migration {
        name: String,
        #[source]
        source: Box<StorageError>,
    },
    #[error("incompatible database: {0}")]
    Incompatible(String),
    #[error("object not found: {0}")]
    NotFound(String),
    #[error("serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("identifier parse error: {0}")]
    Id(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("blob missing on disk: {0}")]
    MissingBlob(String),
    #[error("{0}")]
    Message(String),
}

impl StorageError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}
