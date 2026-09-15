CREATE TABLE classes (
  id INTEGER PRIMARY KEY,
  course_code TEXT NOT NULL,
  name TEXT,
  semester TEXT NOT NULL,
  color TEXT,
  active INTEGER NOT NULL DEFAULT 1,
  todoist_project_id TEXT UNIQUE,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE tasks (
  id INTEGER PRIMARY KEY,
  class_id INTEGER NOT NULL REFERENCES classes(id),
  parent_task_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL CHECK (status IN ('not_started','in_progress','completed')) DEFAULT 'not_started',
  priority INTEGER,
  due_at TEXT,
  scheduled_date TEXT,
  estimated_minutes INTEGER,
  source TEXT NOT NULL CHECK (source IN ('local','todoist')) DEFAULT 'local',
  external_id TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  completed_at TEXT,
  UNIQUE(source, external_id)
);

CREATE INDEX idx_tasks_class_id ON tasks(class_id);
CREATE INDEX idx_tasks_parent_task_id ON tasks(parent_task_id);

CREATE TABLE time_sessions (
  id INTEGER PRIMARY KEY,
  task_id INTEGER NOT NULL REFERENCES tasks(id),
  start_ts TEXT NOT NULL,
  end_ts TEXT,
  accumulated_pause_seconds INTEGER NOT NULL DEFAULT 0,
  pause_started_ts TEXT,
  final_duration_seconds INTEGER,
  edited INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_time_sessions_task_id ON time_sessions(task_id);

CREATE UNIQUE INDEX one_active_session
  ON time_sessions ((end_ts IS NULL)) WHERE end_ts IS NULL;

CREATE TABLE todoist_projects (
  todoist_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  class_id INTEGER REFERENCES classes(id),
  synced_at TEXT
);

CREATE TABLE sync_metadata (
  key TEXT PRIMARY KEY,
  value TEXT
);

CREATE TABLE app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
