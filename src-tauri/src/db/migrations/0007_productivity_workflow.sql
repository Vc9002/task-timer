ALTER TABLE tasks ADD COLUMN task_type TEXT NOT NULL DEFAULT 'assignment'
    CHECK(task_type IN ('assignment','reading','problem_set','exam','project','other'));
ALTER TABLE tasks ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'
    CHECK(json_valid(tags));
ALTER TABLE tasks ADD COLUMN time_budget_minutes INTEGER
    CHECK(time_budget_minutes IS NULL OR time_budget_minutes >= 0);

CREATE INDEX idx_tasks_task_type ON tasks(task_type);
