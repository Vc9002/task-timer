CREATE TABLE study_capacity_overrides (
    date TEXT PRIMARY KEY,
    available_minutes INTEGER NOT NULL CHECK(available_minutes >= 0)
);

CREATE INDEX idx_tasks_due_at ON tasks(due_at);
CREATE INDEX idx_tasks_scheduled_date ON tasks(scheduled_date);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_time_sessions_start_ts ON time_sessions(start_ts);
