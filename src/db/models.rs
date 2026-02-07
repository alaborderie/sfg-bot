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
    pub total_cs: i32,
    pub total_gold: i32,
    pub total_damage: i32,
    pub enemy_champion_name: Option<String>,
    pub enemy_cs: Option<i32>,
    pub enemy_gold: Option<i32>,
    pub enemy_damage: Option<i32>,
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
    pub total_cs: i32,
    pub total_gold: i32,
    pub total_damage: i32,
    pub enemy_champion_name: Option<String>,
    pub enemy_cs: Option<i32>,
    pub enemy_gold: Option<i32>,
    pub enemy_damage: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Champion {
    pub id: Uuid,
    pub champion_id: i32,
    pub champion_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct NotificationEvent {
    pub id: Uuid,
    pub summoner_id: Uuid,
    pub event_type: String,
    pub game_id: i64,
    pub match_id: Option<String>,
    pub champion_id: i32,
    pub champion_name: String,
    pub role: Option<String>,
    pub win: Option<bool>,
    pub kills: Option<i32>,
    pub deaths: Option<i32>,
    pub assists: Option<i32>,
    pub game_duration_secs: Option<i32>,
    pub game_mode: String,
    pub is_featured_mode: bool,
    pub total_cs: Option<i32>,
    pub total_gold: Option<i32>,
    pub total_damage: Option<i32>,
    pub enemy_champion_name: Option<String>,
    pub enemy_cs: Option<i32>,
    pub enemy_gold: Option<i32>,
    pub enemy_damage: Option<i32>,
    pub processed: bool,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewNotificationEvent {
    pub summoner_id: Uuid,
    pub event_type: String,
    pub game_id: i64,
    pub match_id: Option<String>,
    pub champion_id: i32,
    pub champion_name: String,
    pub role: Option<String>,
    pub win: Option<bool>,
    pub kills: Option<i32>,
    pub deaths: Option<i32>,
    pub assists: Option<i32>,
    pub game_duration_secs: Option<i32>,
    pub game_mode: String,
    pub is_featured_mode: bool,
    pub total_cs: Option<i32>,
    pub total_gold: Option<i32>,
    pub total_damage: Option<i32>,
    pub enemy_champion_name: Option<String>,
    pub enemy_cs: Option<i32>,
    pub enemy_gold: Option<i32>,
    pub enemy_damage: Option<i32>,
}
