ALTER TABLE notification_queue
    ADD COLUMN IF NOT EXISTS is_featured_mode BOOLEAN NOT NULL DEFAULT false;
