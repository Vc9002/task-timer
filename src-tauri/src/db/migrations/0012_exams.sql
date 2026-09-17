CREATE TABLE exams (
    id INTEGER PRIMARY KEY,
    class_id INTEGER NOT NULL REFERENCES classes(id),
    title TEXT NOT NULL,
    exam_date TEXT NOT NULL,
    target_study_minutes INTEGER,
    study_task_id INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_exams_date ON exams(exam_date);
CREATE INDEX idx_exams_class ON exams(class_id);
