-- Remove duplicate summoners, keeping the most recently updated row for each riot_puuid
DELETE FROM summoners
WHERE id NOT IN (
    SELECT DISTINCT ON (riot_puuid) id
    FROM summoners
    ORDER BY riot_puuid, updated_at DESC
);

-- Also remove duplicates by game_name + tag_line (case-insensitive), keeping the most recent
DELETE FROM summoners
WHERE id NOT IN (
    SELECT DISTINCT ON (LOWER(game_name), LOWER(tag_line)) id
    FROM summoners
    ORDER BY LOWER(game_name), LOWER(tag_line), updated_at DESC
);

-- Add unique constraint on game_name + tag_line (case-insensitive) to prevent future duplicates
CREATE UNIQUE INDEX IF NOT EXISTS idx_summoners_name_tag_unique
    ON summoners (LOWER(game_name), LOWER(tag_line));
