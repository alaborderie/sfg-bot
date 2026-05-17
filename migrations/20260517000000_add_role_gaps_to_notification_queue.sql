-- Stores a pre-formatted summary of per-lane gold gaps for a finished game
-- (e.g. "Bot gap (-5800g), Top diff (+4100g)") so the Discord recap can
-- surface lane gaps regardless of which team won. NULL when no gap crosses
-- the threshold for any lane.
ALTER TABLE notification_queue
    ADD COLUMN IF NOT EXISTS role_gaps TEXT;
