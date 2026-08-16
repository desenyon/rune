//! Local-first SQLite storage for the canonical graph.

mod blobs;
mod cache;
mod db;
mod edges;
mod error;
mod fts;
mod integrity;
mod migrations;
mod nodes;
mod provenance;
mod settings;

pub use blobs::BlobStore;
pub use cache::ContentCache;
pub use db::{open, Store};
pub use error::{Result, StorageError};
pub use integrity::{IntegrityFinding, IntegrityKind, IntegritySeverity};
pub use migrations::{applied_migrations, migrate, Migration};

pub use rusqlite;
