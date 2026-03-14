use std::error::Error;
use std::fmt;
use std::fs;

use crate::analysis::gemini::{GeminiClient, GeminiError};
use crate::analysis::models::{AnalysisData, AnalysisResult};

#[derive(Debug)]
pub enum AnalysisError {
    GeminiError(GeminiError),
    PromptFileError(std::io::Error),
    SerializationError(serde_json::Error),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalysisError::GeminiError(error) => write!(f, "Gemini error: {error}"),
            AnalysisError::PromptFileError(error) => write!(f, "Prompt file error: {error}"),
            AnalysisError::SerializationError(error) => write!(f, "Serialization error: {error}"),
        }
    }
}

impl Error for AnalysisError {}

#[derive(Clone)]
pub struct AnalysisPipeline {
    gemini_client: GeminiClient,
    prompt_template: String,
}

impl AnalysisPipeline {
    pub fn new(gemini_client: GeminiClient, prompt_path: &str) -> Result<Self, AnalysisError> {
        let prompt_template =
            fs::read_to_string(prompt_path).map_err(AnalysisError::PromptFileError)?;
        Ok(Self {
            gemini_client,
            prompt_template,
        })
    }

    pub async fn analyze_game(&self, data: &AnalysisData) -> AnalysisResult {
        let error_message = match serde_json::to_string_pretty(data) {
            Ok(data_json) => {
                let result = self
                    .gemini_client
                    .analyze(&self.prompt_template, &data_json)
                    .await;

                match result {
                    Ok(text) => {
                        let overall_rating = extract_overall_rating(&text);
                        return AnalysisResult {
                            summoner_name: data.summoner_name.clone(),
                            champion_name: data.champion_name.clone(),
                            overall_rating,
                            summary: text,
                            error: None,
                        };
                    }
                    Err(error) => {
                        tracing::warn!(
                            summoner = data.summoner_name.as_str(),
                            error = %error,
                            "Gemini analysis failed"
                        );
                        error.to_string()
                    }
                }
            }
            Err(error) => {
                tracing::warn!(
                    summoner = data.summoner_name.as_str(),
                    error = %error,
                    "Failed to serialize analysis data"
                );
                error.to_string()
            }
        };

        AnalysisResult {
            summoner_name: data.summoner_name.clone(),
            champion_name: data.champion_name.clone(),
            overall_rating: None,
            summary: format!("Analyse indisponible : {error_message}"),
            error: Some(error_message),
        }
    }
}

fn extract_overall_rating(text: &str) -> Option<String> {
    let lowered = text.to_lowercase();
    if lowered.contains("good") {
        return Some("Good".to_string());
    }
    if lowered.contains("average") {
        return Some("Average".to_string());
    }
    if lowered.contains("poor") {
        return Some("Poor".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::models::AnalysisData;

    fn sample_analysis_data() -> AnalysisData {
        AnalysisData {
            summoner_name: "TestSummoner".to_string(),
            champion_name: "Ahri".to_string(),
            win: true,
            kills: 10,
            deaths: 2,
            assists: 8,
            kda: Some(9.0),
            kill_participation: Some(0.65),
            gold_per_minute: Some(450.0),
            damage_per_minute: Some(600.0),
            vision_score_per_minute: Some(1.2),
            team_damage_percentage: Some(0.28),
            max_cs_advantage_on_lane_opponent: Some(12.0),
            early_laning_phase_gold_exp_advantage: Some(150.0),
            laning_phase_gold_exp_advantage: Some(220.0),
            turret_kills: 2,
            inhibitor_kills: 0,
            objectives_stolen: 0,
            damage_dealt_to_objectives: 3200,
            total_damage_dealt_to_champions: 24000,
            gold_earned: 15000,
            total_cs: 210,
            enemy_champion_name: Some("Orianna".to_string()),
            enemy_cs: Some(190),
            enemy_gold: Some(14000),
            enemy_damage: Some(20000),
            gold_diff_at_10: Some(300),
            gold_diff_at_15: Some(450),
            gold_diff_at_20: Some(600),
            cs_diff_at_10: Some(10),
            cs_diff_at_15: Some(15),
            cs_diff_at_20: Some(20),
            game_duration_secs: 2100,
            role: "MIDDLE".to_string(),
            game_mode: "CLASSIC".to_string(),
        }
    }

    #[test]
    fn extract_overall_rating_detects_good() {
        let result = extract_overall_rating("Good performance overall");
        assert_eq!(result.as_deref(), Some("Good"));
    }

    #[test]
    fn extract_overall_rating_detects_average() {
        let result = extract_overall_rating("An average showing");
        assert_eq!(result.as_deref(), Some("Average"));
    }

    #[test]
    fn extract_overall_rating_detects_poor() {
        let result = extract_overall_rating("Poor lane control");
        assert_eq!(result.as_deref(), Some("Poor"));
    }

    #[test]
    fn extract_overall_rating_returns_none_when_missing() {
        let result = extract_overall_rating("Solid play with no rating keyword");
        assert!(result.is_none());
    }

    #[test]
    fn analysis_data_serializes_to_json() {
        let data = sample_analysis_data();
        let json = serde_json::to_string_pretty(&data).expect("serialize analysis data");
        assert!(json.contains("TestSummoner"));
        assert!(json.contains("Ahri"));
        assert!(json.contains("gold_per_minute"));
    }
}
