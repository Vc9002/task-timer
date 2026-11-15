-- When the timer was last resumed. An idle auto-pause is backdated to when the
-- user actually went idle, but never to before this moment: time before it is
-- already accounted for (as work, or as an earlier pause).
ALTER TABLE time_sessions ADD COLUMN last_resumed_ts TEXT;
