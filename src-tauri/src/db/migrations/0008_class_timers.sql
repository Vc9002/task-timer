ALTER TABLE tasks ADD COLUMN is_class_timer INTEGER NOT NULL DEFAULT 0
    CHECK(is_class_timer IN (0, 1));

CREATE INDEX idx_tasks_class_timer ON tasks(class_id, is_class_timer);
