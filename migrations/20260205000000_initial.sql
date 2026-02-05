CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE summoners (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  riot_puuid VARCHAR(78) UNIQUE NOT NULL,
  game_name VARCHAR(24) NOT NULL,
  tag_line VARCHAR(5) NOT NULL,
  region VARCHAR(10) NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE active_games (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  summoner_id UUID NOT NULL REFERENCES summoners(id) ON DELETE CASCADE,
  game_id BIGINT NOT NULL,
  champion_id INT NOT NULL,
  game_mode VARCHAR(50) NOT NULL,
  game_start_time TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(summoner_id, game_id)
);

CREATE TABLE match_history (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  summoner_id UUID NOT NULL REFERENCES summoners(id) ON DELETE CASCADE,
  match_id VARCHAR(50) NOT NULL,
  game_id BIGINT NOT NULL,
  win BOOLEAN NOT NULL,
  kills INT NOT NULL,
  deaths INT NOT NULL,
  assists INT NOT NULL,
  champion_id INT NOT NULL,
  game_duration_secs INT NOT NULL,
  game_mode VARCHAR(50) NOT NULL,
  finished_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(summoner_id, match_id)
);

CREATE INDEX idx_summoners_puuid ON summoners(riot_puuid);
CREATE INDEX idx_active_games_summoner ON active_games(summoner_id);
CREATE INDEX idx_match_history_summoner ON match_history(summoner_id);
CREATE INDEX idx_match_history_finished ON match_history(finished_at);
