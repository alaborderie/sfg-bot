-- Add queue_id column to active_games table
ALTER TABLE active_games
ADD COLUMN queue_id INT NULL;

-- Add queue_id column to match_history table
ALTER TABLE match_history
ADD COLUMN queue_id INT NULL;

-- Add queue_id column to notification_queue table
ALTER TABLE notification_queue
ADD COLUMN queue_id INT NULL;
