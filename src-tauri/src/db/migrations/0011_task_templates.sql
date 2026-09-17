CREATE TABLE task_templates (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    class_id INTEGER REFERENCES classes(id),
    task_type TEXT,
    default_estimated_minutes INTEGER,
    default_time_budget_minutes INTEGER,
    priority INTEGER,
    tags TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(tags)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE task_template_milestones (
    id INTEGER PRIMARY KEY,
    template_id INTEGER NOT NULL REFERENCES task_templates(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    offset_days_before_due INTEGER,
    position INTEGER NOT NULL DEFAULT 0
);
