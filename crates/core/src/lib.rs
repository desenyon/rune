//! Canonical information model for Rune.
//!
//! Everything is an object. Every object can have relationships, actions, and
//! context contributions. This crate is renderer-free and storage-free.

pub mod action;
pub mod config;
pub mod edge;
pub mod error;
pub mod fingerprint;
pub mod id;
pub mod node;
pub mod payloads;
pub mod provenance;
pub mod time;
pub mod validity;

pub use action::{Action, ActionKind};
pub use config::{ConfigLayer, ConfigValue, LayeredConfig};
pub use edge::{Edge, EdgeKind, EdgeMetadata};
pub use error::{Error, Result};
pub use fingerprint::{ContentHash, Fingerprint};
pub use id::{EdgeId, NodeId, ProvenanceId};
pub use node::{Node, NodeKind};
pub use payloads::{
    CapsuleSelection, FilePayload, MemoryCategory, MemoryOrigin, MemoryPayload, SymbolPayload,
    TaskPayload, TaskStatus,
};
pub use provenance::{Provenance, ProvenanceSource, ProvenanceSubject};
pub use time::Timestamp;
pub use validity::Validity;
