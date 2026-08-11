//! SQL schema (v2 adds sessions).

pub const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS experiments (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    parent_id TEXT,
    candidate_json TEXT NOT NULL,
    machine_fingerprint TEXT NOT NULL,
    kraftverk_version TEXT NOT NULL,
    os_info TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    samples_json TEXT NOT NULL,
    index_samples_json TEXT NOT NULL,
    index_summary_json TEXT,
    kraft_index_json TEXT,
    comparison_json TEXT,
    decision TEXT NOT NULL,
    decision_reason TEXT NOT NULL,
    stability TEXT NOT NULL,
    comparison_class TEXT,
    telemetry_json TEXT NOT NULL,
    recovery_token TEXT,
    kraft_index_score REAL,
    hardware_policy TEXT NOT NULL DEFAULT 'amd-only-v1',
    FOREIGN KEY(parent_id) REFERENCES experiments(id)
);

CREATE INDEX IF NOT EXISTS idx_experiments_created ON experiments(created_at);
CREATE INDEX IF NOT EXISTS idx_experiments_fingerprint ON experiments(machine_fingerprint);
CREATE INDEX IF NOT EXISTS idx_experiments_kind ON experiments(kind);
CREATE INDEX IF NOT EXISTS idx_experiments_decision ON experiments(decision);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    status TEXT NOT NULL,
    goal TEXT NOT NULL,
    config_json TEXT NOT NULL,
    constraints_json TEXT NOT NULL,
    checkpoint_json TEXT,
    failure_streak INTEGER NOT NULL DEFAULT 0,
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
"#;
