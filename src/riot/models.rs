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
    pub game_duration_secs: i32,
    pub game_mode: String,
}

/// Represents a change in game state for a summoner
#[derive(Debug, Clone)]
pub enum GameStateChange {
    /// Summoner started a new game
    GameStarted(ActiveGameInfo),
    /// Summoner's game ended (need to fetch match result)
    GameEnded { game_id: i64 },
    /// No change in game state
    NoChange,
}
