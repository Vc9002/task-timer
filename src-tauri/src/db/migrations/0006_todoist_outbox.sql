CREATE TABLE integration_outbox (
    id INTEGER PRIMARY KEY,
    provider TEXT NOT NULL CHECK(provider = 'todoist'),
    entity_type TEXT NOT NULL CHECK(entity_type = 'task'),
    external_id TEXT NOT NULL,
    command_uuid TEXT NOT NULL UNIQUE,
    command_type TEXT NOT NULL CHECK(command_type = 'item_complete'),
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','sending','completed','failed')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK(attempts >= 0),
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_integration_outbox_pending ON integration_outbox(provider,status,created_at);
