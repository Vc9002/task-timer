CREATE TABLE study_blocks (
    id INTEGER PRIMARY KEY,
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    planned_date TEXT NOT NULL,
    planned_start_time TEXT,
    planned_minutes INTEGER NOT NULL CHECK(planned_minutes > 0),
    completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_study_blocks_date ON study_blocks(planned_date);
CREATE INDEX idx_study_blocks_task ON study_blocks(task_id);
