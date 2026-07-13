//! Per-player analysis memory.
//!
//! After each successful analysis a compact `AnalysisData` snapshot is stored
//! in the `analysis_history` table; before the next analysis the most recent
//! snapshots are summarized into `AnalysisData::recent_games` so the coach
//! can comment on progression across games ("your early CS improved since
//! last game").

use crate::analysis::models::{AnalysisData, AnalysisResult, RecentGameSummary};
use crate::analysis::pipeline::AnalysisPipeline;
use crate::db::models::{AnalysisHistoryEntry, NewAnalysisHistory};
use crate::db::repository::Repository;

/// How many previous games are surfaced to the coach.
pub const RECENT_GAMES_LIMIT: i64 = 5;

/// Fetches the player's recent history, runs the analysis with it, then
/// stores this game's snapshot. History failures are logged and degrade to
/// a memory-less analysis — they never block the analysis itself.
pub async fn analyze_with_memory<D: Repository + ?Sized>(
    repository: &D,
    pipeline: &AnalysisPipeline,
    mut data: AnalysisData,
    riot_puuid: &str,
    match_id: &str,
) -> AnalysisResult {
    // Fetch one extra row: the current match may be among the most recent
    // snapshots (e.g. /analyze-last-game reruns) and is excluded below.
    match repository
        .get_recent_analysis_history(riot_puuid, RECENT_GAMES_LIMIT + 1)
        .await
    {
        Ok(entries) => data.recent_games = summaries_from_entries(&entries, match_id),
        Err(error) => tracing::warn!(
            error = %error,
            "Failed to fetch analysis history; analyzing without memory"
        ),
    }

    let result = pipeline.analyze_game(&data).await;

    if result.error.is_none() {
        match snapshot_json(&data) {
            Ok(json) => {
                let entry = NewAnalysisHistory {
                    riot_puuid: riot_puuid.to_string(),
                    match_id: match_id.to_string(),
                    role: data.role.clone(),
                    champion_name: data.champion_name.clone(),
                    win: data.win,
                    overall_rating: result.overall_rating.clone(),
                    analysis_data: json,
                };
                if let Err(error) = repository.insert_analysis_history(&entry).await {
                    tracing::warn!(error = %error, "Failed to store analysis history");
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "Failed to serialize analysis snapshot")
            }
        }
    }

    result
}

/// Maps stored history rows to prompt-ready summaries, most recent first.
/// The current match is excluded (relevant when a game is re-analyzed via
/// `/analyze-last-game`); unparseable snapshots are skipped with a warning.
pub fn summaries_from_entries(
    entries: &[AnalysisHistoryEntry],
    exclude_match_id: &str,
) -> Vec<RecentGameSummary> {
    entries
        .iter()
        .filter(|entry| entry.match_id != exclude_match_id)
        .filter_map(|entry| {
            let data: AnalysisData = match serde_json::from_str(&entry.analysis_data) {
                Ok(data) => data,
                Err(error) => {
                    tracing::warn!(
                        match_id = entry.match_id.as_str(),
                        error = %error,
                        "Skipping unparseable analysis history snapshot"
                    );
                    return None;
                }
            };
            Some(RecentGameSummary {
                champion_name: data.champion_name,
                role: data.role,
                win: data.win,
                overall_rating: entry.overall_rating.clone(),
                kills: data.kills,
                deaths: data.deaths,
                assists: data.assists,
                cs_per_minute: cs_per_minute(data.total_cs, data.game_duration_secs),
                cs_diff_at_10: data.cs_diff_at_10,
                gold_diff_at_10: data.gold_diff_at_10,
                damage_per_minute: data.damage_per_minute,
                vision_score_per_minute: data.vision_score_per_minute,
            })
        })
        .take(RECENT_GAMES_LIMIT as usize)
        .collect()
}

/// Serializes the game's own data for storage, with history stripped so
/// snapshots never nest snapshots.
pub fn snapshot_json(data: &AnalysisData) -> Result<String, serde_json::Error> {
    let mut snapshot = data.clone();
    snapshot.recent_games = Vec::new();
    serde_json::to_string(&snapshot)
}

fn cs_per_minute(total_cs: i32, duration_secs: i32) -> Option<f32> {
    (duration_secs > 0).then(|| total_cs as f32 / (duration_secs as f32 / 60.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn sample_data() -> AnalysisData {
        AnalysisData {
            summoner_name: "Test#EUW".to_string(),
            champion_name: "Gangplank".to_string(),
            win: true,
            kills: 10,
            deaths: 8,
            assists: 5,
            kda: Some(1.9),
            kill_participation: Some(0.52),
            gold_per_minute: Some(400.0),
            damage_per_minute: Some(720.0),
            vision_score_per_minute: Some(0.9),
            team_damage_percentage: Some(0.28),
            max_cs_advantage_on_lane_opponent: Some(0.0),
            early_laning_phase_gold_exp_advantage: Some(-1.0),
            laning_phase_gold_exp_advantage: Some(-1.0),
            turret_kills: 4,
            inhibitor_kills: 0,
            objectives_stolen: 0,
            damage_dealt_to_objectives: 9000,
            total_damage_dealt_to_champions: 25000,
            gold_earned: 14000,
            total_cs: 180,
            enemy_champion_name: Some("Tryndamere".to_string()),
            enemy_cs: Some(232),
            enemy_gold: Some(15000),
            enemy_damage: Some(20000),
            gold_diff_at_10: Some(-1287),
            gold_diff_at_15: Some(-1500),
            gold_diff_at_20: Some(-1100),
            cs_diff_at_10: Some(-22),
            cs_diff_at_15: Some(-38),
            cs_diff_at_20: Some(-52),
            game_duration_secs: 1800,
            role: "TOP".to_string(),
            game_mode: "CLASSIC".to_string(),
            recent_games: Vec::new(),
        }
    }

    fn entry_for(match_id: &str, data: &AnalysisData) -> AnalysisHistoryEntry {
        AnalysisHistoryEntry {
            id: Uuid::new_v4(),
            riot_puuid: "puuid".to_string(),
            match_id: match_id.to_string(),
            role: data.role.clone(),
            champion_name: data.champion_name.clone(),
            win: data.win,
            overall_rating: Some("Average".to_string()),
            analysis_data: snapshot_json(data).unwrap(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn snapshot_json_strips_recent_games() {
        let mut data = sample_data();
        data.recent_games = summaries_from_entries(&[entry_for("EUW1_1", &sample_data())], "");
        assert!(!data.recent_games.is_empty());

        let json = snapshot_json(&data).unwrap();
        assert!(!json.contains("recent_games"));

        let parsed: AnalysisData = serde_json::from_str(&json).unwrap();
        assert!(parsed.recent_games.is_empty());
        assert_eq!(parsed.champion_name, "Gangplank");
    }

    #[test]
    fn summaries_exclude_current_match_and_compute_cs_per_minute() {
        let data = sample_data();
        let entries = vec![
            entry_for("EUW1_current", &data),
            entry_for("EUW1_old", &data),
        ];

        let summaries = summaries_from_entries(&entries, "EUW1_current");
        assert_eq!(summaries.len(), 1);
        let summary = &summaries[0];
        assert_eq!(summary.champion_name, "Gangplank");
        assert_eq!(summary.overall_rating.as_deref(), Some("Average"));
        assert_eq!(summary.cs_diff_at_10, Some(-22));
        let cs_min = summary.cs_per_minute.unwrap();
        assert!(
            (cs_min - 6.0).abs() < 0.01,
            "expected 6.0 cs/min, got {cs_min}"
        );
    }

    #[test]
    fn summaries_skip_unparseable_snapshots() {
        let data = sample_data();
        let mut broken = entry_for("EUW1_broken", &data);
        broken.analysis_data = "not json".to_string();
        let entries = vec![broken, entry_for("EUW1_ok", &data)];

        let summaries = summaries_from_entries(&entries, "");
        assert_eq!(summaries.len(), 1);
    }

    #[test]
    fn summaries_cap_at_recent_games_limit() {
        let data = sample_data();
        let entries: Vec<_> = (0..10)
            .map(|i| entry_for(&format!("EUW1_{i}"), &data))
            .collect();

        let summaries = summaries_from_entries(&entries, "");
        assert_eq!(summaries.len(), RECENT_GAMES_LIMIT as usize);
    }
}
