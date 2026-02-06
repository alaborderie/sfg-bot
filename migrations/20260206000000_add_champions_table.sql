CREATE TABLE IF NOT EXISTS champions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  champion_id INT UNIQUE NOT NULL,
  champion_name VARCHAR(50) NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_champions_champion_id ON champions(champion_id);
