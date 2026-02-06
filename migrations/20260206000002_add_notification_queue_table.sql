-- Add notification_queue table for grouping notifications
CREATE TABLE notification_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    summoner_id UUID NOT NULL REFERENCES summoners(id) ON DELETE CASCADE,
    event_type VARCHAR(20) NOT NULL CHECK (event_type IN ('GAME_STARTED', 'GAME_ENDED')),
    game_id BIGINT NOT NULL,
    match_id VARCHAR(100),
    champion_id INTEGER NOT NULL,
    champion_name VARCHAR(100) NOT NULL,
    role VARCHAR(50),
    win BOOLEAN,
    kills INTEGER,
    deaths INTEGER,
    assists INTEGER,
    game_duration_secs INTEGER,
    game_mode VARCHAR(50) NOT NULL,
    processed BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);

-- Index for efficient querying of pending events
CREATE INDEX idx_notification_queue_pending ON notification_queue(processed, created_at) WHERE NOT processed;

-- Index for grouping by game_id (for GAME_STARTED events)
CREATE INDEX idx_notification_queue_game_id ON notification_queue(event_type, game_id) WHERE NOT processed;

-- Index for grouping by match_id (for GAME_ENDED events)
CREATE INDEX idx_notification_queue_match_id ON notification_queue(event_type, match_id) WHERE NOT processed AND match_id IS NOT NULL;
