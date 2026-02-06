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
    pub role: String,
    pub total_cs: i32,
    pub total_gold: i32,
    pub total_damage: i32,
    pub enemy_champion_name: Option<String>,
    pub enemy_cs: Option<i32>,
    pub enemy_gold: Option<i32>,
    pub enemy_damage: Option<i32>,
}

/// Represents a change in game state for a summoner
#[derive(Debug, Clone)]
pub enum GameStateChange {
    /// Summoner started a new game
    GameStarted(ActiveGameInfo),
    /// Summoner's game ended (need to fetch match result)
    /// is_featured_mode: true if game is likely ARAM Mayhem/Arena (featured mode)
    GameEnded {
        game_id: i64,
        is_featured_mode: bool,
    },
    /// No change in game state
    NoChange,
}
