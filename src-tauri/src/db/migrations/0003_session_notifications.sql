CREATE TABLE session_notifications (
    session_id INTEGER NOT NULL REFERENCES time_sessions(id) ON DELETE CASCADE,
    notification_type TEXT NOT NULL CHECK(notification_type = 'overrun'),
    threshold INTEGER NOT NULL CHECK(threshold IN (0,25,50)),
    status TEXT NOT NULL CHECK(status IN ('attempted','sent','failed','legacy_unknown')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK(attempts >= 0),
    attempted_at TEXT,
    sent_at TEXT,
    PRIMARY KEY(session_id, notification_type, threshold)
);

-- Old markers recorded attempts, not OS success. Preserve that uncertainty.
INSERT INTO session_notifications(session_id, notification_type, threshold, status, attempts)
SELECT s.id, 'overrun', t.threshold, 'legacy_unknown', 1
FROM time_sessions s
CROSS JOIN (SELECT 0 AS threshold UNION ALL SELECT 25 UNION ALL SELECT 50) t
JOIN app_settings a ON a.key = 'overrun_sent_' || s.id || '_' || t.threshold;

DELETE FROM app_settings WHERE key GLOB 'overrun_sent_[0-9]*_[0-9]*';
