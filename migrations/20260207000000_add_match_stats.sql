ALTER TABLE match_history ADD COLUMN total_cs INT NOT NULL DEFAULT 0;
ALTER TABLE match_history ADD COLUMN total_gold INT NOT NULL DEFAULT 0;
ALTER TABLE match_history ADD COLUMN total_damage INT NOT NULL DEFAULT 0;
ALTER TABLE match_history ADD COLUMN enemy_champion_name VARCHAR NULL;
ALTER TABLE match_history ADD COLUMN enemy_cs INT NULL;
ALTER TABLE match_history ADD COLUMN enemy_gold INT NULL;
ALTER TABLE match_history ADD COLUMN enemy_damage INT NULL;

ALTER TABLE notification_queue ADD COLUMN total_cs INT NULL;
ALTER TABLE notification_queue ADD COLUMN total_gold INT NULL;
ALTER TABLE notification_queue ADD COLUMN total_damage INT NULL;
ALTER TABLE notification_queue ADD COLUMN enemy_champion_name VARCHAR NULL;
ALTER TABLE notification_queue ADD COLUMN enemy_cs INT NULL;
ALTER TABLE notification_queue ADD COLUMN enemy_gold INT NULL;
ALTER TABLE notification_queue ADD COLUMN enemy_damage INT NULL;
