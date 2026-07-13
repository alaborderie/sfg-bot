-- Per-player memory of AI post-game analyses. The last few snapshots are
-- injected into the analysis prompt so the coach can comment on progression
-- across games (e.g. "your early CS improved since last game").
CREATE TABLE IF NOT EXISTS analysis_history (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  riot_puuid TEXT NOT NULL,
  match_id TEXT NOT NULL,
  role TEXT NOT NULL,
  champion_name TEXT NOT NULL,
  win BOOLEAN NOT NULL,
  overall_rating TEXT,
  -- JSON-serialized AnalysisData snapshot (history stripped, so snapshots
  -- never nest snapshots). TEXT because the sqlx "json" feature is not
  -- enabled; parsed back with serde_json on read.
  analysis_data TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (riot_puuid, match_id)
);

CREATE INDEX IF NOT EXISTS idx_analysis_history_puuid_created
  ON analysis_history (riot_puuid, created_at DESC);
