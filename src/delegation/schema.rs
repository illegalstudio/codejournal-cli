pub const SQL: &str = r#"
CREATE TABLE IF NOT EXISTS delegations (
 id TEXT PRIMARY KEY, protocol_version INTEGER NOT NULL DEFAULT 2,
 target TEXT NOT NULL, mode TEXT NOT NULL, cwd TEXT NOT NULL,
 model TEXT NOT NULL DEFAULT '', task TEXT NOT NULL, status TEXT NOT NULL,
 created_at REAL NOT NULL, updated_at REAL NOT NULL, host_heartbeat REAL,
 guest_heartbeat REAL, last_activity REAL NOT NULL, host_ack_seq INTEGER NOT NULL DEFAULT 0,
 guest_ack_seq INTEGER NOT NULL DEFAULT 0, attempt INTEGER NOT NULL DEFAULT 0,
 attempt_token TEXT NOT NULL DEFAULT '', stall_timeout REAL NOT NULL DEFAULT 600,
 diff_timeout REAL NOT NULL DEFAULT 0, workspace_root TEXT NOT NULL DEFAULT '',
 baseline_sha TEXT NOT NULL DEFAULT '', diff_detected_at REAL, integrated_at REAL,
 supervisor_pid INTEGER, guest_pid INTEGER, provider_session_id TEXT,
 exit_code INTEGER, result TEXT, error TEXT,
 journal_context TEXT NOT NULL DEFAULT '{}', supervisor_identity TEXT, guest_identity TEXT
);
CREATE TABLE IF NOT EXISTS events (
 seq INTEGER PRIMARY KEY AUTOINCREMENT,
 delegation_id TEXT NOT NULL REFERENCES delegations(id) ON DELETE CASCADE,
 sender TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL,
 reply_to INTEGER, idempotency_key TEXT, created_at REAL NOT NULL,
 UNIQUE(delegation_id, sender, idempotency_key)
);
CREATE INDEX IF NOT EXISTS events_delegation_seq ON events(delegation_id, seq);
CREATE TABLE IF NOT EXISTS journal_delivery (
 id TEXT PRIMARY KEY, payload TEXT NOT NULL, queued_at TEXT NOT NULL
);
"#;
