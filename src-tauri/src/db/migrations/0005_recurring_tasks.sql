CREATE TABLE recurring_task_templates (
    id INTEGER PRIMARY KEY,
    class_id INTEGER NOT NULL REFERENCES classes(id),
    title TEXT NOT NULL,
    description TEXT,
    priority INTEGER NOT NULL DEFAULT 2 CHECK(priority BETWEEN 1 AND 4),
    estimated_minutes INTEGER CHECK(estimated_minutes >= 0),
    recurrence_type TEXT NOT NULL CHECK(recurrence_type IN ('daily','weekly','weekdays')),
    interval INTEGER NOT NULL DEFAULT 1 CHECK(interval > 0),
    weekdays TEXT,
    start_date TEXT NOT NULL,
    end_date TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK(end_date IS NULL OR end_date >= start_date)
);
ALTER TABLE tasks ADD COLUMN recurring_template_id INTEGER REFERENCES recurring_task_templates(id);
ALTER TABLE tasks ADD COLUMN occurrence_date TEXT;
CREATE UNIQUE INDEX idx_tasks_recurring_occurrence ON tasks(recurring_template_id, occurrence_date) WHERE recurring_template_id IS NOT NULL AND occurrence_date IS NOT NULL;
CREATE INDEX idx_recurring_templates_active ON recurring_task_templates(active, start_date, end_date);
