-- Track how many poll cycles a finished game has failed its match lookup.
-- The active_games row is now kept until the lookup succeeds (or the retry
-- budget is exhausted), so a Riot API hiccup no longer loses the game.
ALTER TABLE active_games
    ADD COLUMN IF NOT EXISTS end_retry_count INT NOT NULL DEFAULT 0;
