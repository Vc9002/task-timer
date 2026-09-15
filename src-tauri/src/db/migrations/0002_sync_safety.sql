-- Keep the legacy column for safe upgrades, but retire it as a mapping source.
INSERT INTO todoist_projects(todoist_id, name, class_id)
SELECT todoist_project_id, course_code, id FROM classes WHERE todoist_project_id IS NOT NULL
ON CONFLICT(todoist_id) DO NOTHING;
UPDATE classes SET todoist_project_id = NULL;

ALTER TABLE tasks ADD COLUMN todoist_project_id TEXT;
ALTER TABLE tasks ADD COLUMN external_state TEXT NOT NULL DEFAULT 'active'
    CHECK(external_state IN ('active','completed','deleted','unknown'));
-- Cache unmapped remote tasks so a later mapping can import them without a full resync.
CREATE TABLE todoist_task_cache (todoist_id TEXT PRIMARY KEY, payload TEXT NOT NULL);
