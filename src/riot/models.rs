use chrono::{DateTime, Utc};

/// Summoner account information resolved from Riot API
#[derive(Debug, Clone)]
pub struct SummonerInfo {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
}

/// Information about an active game from Spectator API
#[derive(Debug, Clone)]
pub struct ActiveGameInfo {
    pub game_id: i64,
    pub champion_id: i32,
    pub game_mode: String,
    pub game_start_time: DateTime<Utc>,
    pub queue_id: Option<i32>,
}

/// Result of a completed match from Match API
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub match_id: String,
    pub game_id: i64,
    pub win: bool,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub champion_id: i32,
    pub champion_name: String,
    pub game_duration_secs: i32,
    pub game_mode: String,
    pub role: String,
    pub total_cs: i32,
    pub total_gold: i32,
    pub total_damage: i32,
    pub enemy_champion_name: Option<String>,
    pub enemy_cs: Option<i32>,
    pub enemy_gold: Option<i32>,
    pub enemy_damage: Option<i32>,
    pub queue_id: Option<i32>,
    /// Pre-formatted per-lane gold-gap summary from the tracked summoner's
    /// perspective (e.g. `"Bot gap (-5.8k), Top diff (+4.1k)"`). `None` when
    /// no lane crosses the gap threshold.
    pub role_gaps: Option<String>,
    /// Unix timestamp (milliseconds) when the match ended on the game server.
    /// Available from match-v5 since patch 11.20.
    pub game_end_timestamp: Option<i64>,
    /// Unix timestamp (milliseconds) when the match started on the game server.
    /// Available from match-v5 since patch 11.20.
    pub game_start_timestamp: Option<i64>,
}

/// Outcome of the match lookup performed when a game ends
#[derive(Debug, Clone)]
pub enum MatchLookup {
    /// Match data retrieved; the game is fully resolved. Boxed to keep the
    /// enum small next to the unit-like retry variants.
    Found(Box<MatchResult>),
    /// Match data not available yet; the active game is kept so the next
    /// poll cycle retries the lookup
    Pending { attempts: i32 },
    /// Retry budget exhausted; the active game was dropped
    GaveUp { attempts: i32 },
}

/// Represents a change in game state for a summoner
#[derive(Debug, Clone)]
pub enum GameStateChange {
    /// Summoner started a new game
    GameStarted(ActiveGameInfo),
    /// Summoner's game ended (need to fetch match result)
    GameEnded { game_id: i64 },
    /// Summoner's featured mode game ended (detected via match history)
    FeaturedModeGameEnded { game_id: i64 },
    /// No change in game state
    NoChange,
}
