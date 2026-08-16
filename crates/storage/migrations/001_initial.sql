-- Canonical Rune store. Unknown future node kinds use JSON payloads.

PRAGMA foreign_keys = ON;

CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    name TEXT,
    payload TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    content_hash TEXT,
    validity TEXT NOT NULL DEFAULT 'active'
);

CREATE INDEX idx_nodes_kind ON nodes(kind);
CREATE INDEX idx_nodes_name ON nodes(name);
CREATE INDEX idx_nodes_hash ON nodes(content_hash);
CREATE INDEX idx_nodes_updated ON nodes(updated_at);
CREATE INDEX idx_nodes_kind_name ON nodes(kind, name);
CREATE INDEX idx_nodes_validity ON nodes(validity);

CREATE TABLE edges (
    id TEXT PRIMARY KEY,
    from_id TEXT NOT NULL,
    to_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    metadata TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    validity TEXT NOT NULL DEFAULT 'active',
    FOREIGN KEY (from_id) REFERENCES nodes(id) ON DELETE CASCADE,
    FOREIGN KEY (to_id) REFERENCES nodes(id) ON DELETE CASCADE
);

CREATE INDEX idx_edges_from ON edges(from_id);
CREATE INDEX idx_edges_to ON edges(to_id);
CREATE INDEX idx_edges_kind ON edges(kind);
CREATE INDEX idx_edges_from_kind ON edges(from_id, kind);
CREATE INDEX idx_edges_to_kind ON edges(to_id, kind);
CREATE INDEX idx_edges_validity ON edges(validity);

CREATE TABLE provenance (
    id TEXT PRIMARY KEY,
    node_id TEXT,
    edge_id TEXT,
    source_kind TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    source_payload TEXT NOT NULL,
    observed_at INTEGER NOT NULL,
    confidence REAL NOT NULL,
    derived INTEGER NOT NULL,
    details TEXT,
    FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE,
    FOREIGN KEY (edge_id) REFERENCES edges(id) ON DELETE CASCADE,
    CHECK (node_id IS NOT NULL OR edge_id IS NOT NULL)
);

CREATE INDEX idx_provenance_node ON provenance(node_id);
CREATE INDEX idx_provenance_edge ON provenance(edge_id);
CREATE INDEX idx_provenance_source ON provenance(source_kind, source_ref);
CREATE INDEX idx_provenance_derived ON provenance(derived);

CREATE VIRTUAL TABLE nodes_fts USING fts5(
    id UNINDEXED,
    kind UNINDEXED,
    name,
    body,
    tokenize = 'porter unicode61'
);

CREATE TABLE blobs (
    hash TEXT PRIMARY KEY,
    size INTEGER NOT NULL,
    media_type TEXT,
    created_at INTEGER NOT NULL,
    last_accessed_at INTEGER NOT NULL
);

CREATE TABLE cache_entries (
    cache_key TEXT PRIMARY KEY,
    fingerprint TEXT NOT NULL,
    blob_hash TEXT NOT NULL,
    kind TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    FOREIGN KEY (blob_hash) REFERENCES blobs(hash)
);

CREATE INDEX idx_cache_fingerprint ON cache_entries(fingerprint);
CREATE INDEX idx_cache_kind ON cache_entries(kind);

CREATE TABLE settings (
    scope TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (scope, key)
);

CREATE TABLE integrity_findings (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    severity TEXT NOT NULL,
    subject_id TEXT,
    message TEXT NOT NULL,
    repair_action TEXT,
    created_at INTEGER NOT NULL,
    resolved_at INTEGER
);

CREATE INDEX idx_integrity_open ON integrity_findings(resolved_at, severity);
