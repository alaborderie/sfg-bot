use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Summoner {
    pub id: Uuid,
    pub riot_puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub region: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ActiveGame {
    pub id: Uuid,
    pub summoner_id: Uuid,
    pub game_id: i64,
    pub champion_id: i32,
    pub game_mode: String,
    pub game_start_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewActiveGame {
    pub summoner_id: Uuid,
    pub game_id: i64,
    pub champion_id: i32,
    pub game_mode: String,
    pub game_start_time: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MatchHistory {
    pub id: Uuid,
    pub summoner_id: Uuid,
    pub match_id: String,
    pub game_id: i64,
    pub win: bool,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub champion_id: i32,
    pub game_duration_secs: i32,
    pub game_mode: String,
    pub role: Option<String>,
    pub finished_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewMatchResult {
    pub summoner_id: Uuid,
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
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Champion {
    pub id: Uuid,
    pub champion_id: i32,
    pub champion_name: String,
    pub created_at: DateTime<Utc>,
}
