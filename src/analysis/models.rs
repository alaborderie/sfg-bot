use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisData {
    pub summoner_name: String,
    pub champion_name: String,
    pub win: bool,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub kda: Option<f32>,
    pub kill_participation: Option<f32>,
    pub gold_per_minute: Option<f32>,
    pub damage_per_minute: Option<f32>,
    pub vision_score_per_minute: Option<f32>,
    pub team_damage_percentage: Option<f32>,
    pub max_cs_advantage_on_lane_opponent: Option<f32>,
    pub early_laning_phase_gold_exp_advantage: Option<f32>,
    pub laning_phase_gold_exp_advantage: Option<f32>,
    pub turret_kills: i32,
    pub inhibitor_kills: i32,
    pub objectives_stolen: i32,
    pub damage_dealt_to_objectives: i32,
    pub total_damage_dealt_to_champions: i32,
    pub gold_earned: i32,
    pub total_cs: i32,
    pub enemy_champion_name: Option<String>,
    pub enemy_cs: Option<i32>,
    pub enemy_gold: Option<i32>,
    pub enemy_damage: Option<i32>,
    pub gold_diff_at_10: Option<i32>,
    pub gold_diff_at_15: Option<i32>,
    pub gold_diff_at_20: Option<i32>,
    pub cs_diff_at_10: Option<i32>,
    pub cs_diff_at_15: Option<i32>,
    pub cs_diff_at_20: Option<i32>,
    pub game_duration_secs: i32,
    pub role: String,
    pub game_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub summoner_name: String,
    pub champion_name: String,
    pub overall_rating: Option<String>,
    pub summary: String,
    pub error: Option<String>,
}
