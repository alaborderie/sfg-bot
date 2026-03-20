-- Add retry tracking to notification_queue
ALTER TABLE notification_queue ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE notification_queue ADD COLUMN error_message TEXT;
