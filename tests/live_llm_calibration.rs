//! Live calibration tests against the real LLM server.
//!
//! Ignored by default: they need the local LLM server running and each call
//! can take minutes. Run explicitly with:
//!
//! ```sh
//! cargo test --test live_llm_calibration -- --ignored --test-threads=1
//! ```
//!
//! Environment overrides: `LLM_BASE_URL` (default `http://jarvis:8080/v1`),
//! `LLM_MODEL` (default `gemma-4-26b`), `LLM_API_KEY` (default `test`).

use sfg_bot::analysis::llm::LlmClient;
use sfg_bot::analysis::models::{AnalysisData, AnalysisResult};
use sfg_bot::analysis::pipeline::AnalysisPipeline;

fn live_pipeline() -> AnalysisPipeline {
    let base_url =
        std::env::var("LLM_BASE_URL").unwrap_or_else(|_| "http://jarvis:8080/v1".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "gemma-4-26b".to_string());
    let api_key = std::env::var("LLM_API_KEY").unwrap_or_else(|_| "test".to_string());
    let client = LlmClient::new(api_key, base_url, model).expect("build LLM client");
    AnalysisPipeline::new(client, "analysis_prompts").expect("load analysis prompts")
}

fn base_data() -> AnalysisData {
    AnalysisData {
        summoner_name: "Calibration#TEST".to_string(),
        champion_name: "Gangplank".to_string(),
        win: true,
        kills: 5,
        deaths: 5,
        assists: 5,
        kda: Some(2.0),
        kill_participation: Some(0.5),
        gold_per_minute: Some(380.0),
        damage_per_minute: Some(500.0),
        vision_score_per_minute: Some(1.0),
        team_damage_percentage: Some(0.22),
        max_cs_advantage_on_lane_opponent: Some(5.0),
        early_laning_phase_gold_exp_advantage: Some(0.0),
        laning_phase_gold_exp_advantage: Some(0.0),
        turret_kills: 1,
        inhibitor_kills: 0,
        objectives_stolen: 0,
        damage_dealt_to_objectives: 5000,
        total_damage_dealt_to_champions: 15000,
        gold_earned: 11000,
        total_cs: 180,
        enemy_champion_name: Some("Tryndamere".to_string()),
        enemy_cs: Some(180),
        enemy_gold: Some(11000),
        enemy_damage: Some(15000),
        gold_diff_at_10: Some(0),
        gold_diff_at_15: Some(0),
        gold_diff_at_20: Some(0),
        cs_diff_at_10: Some(0),
        cs_diff_at_15: Some(0),
        cs_diff_at_20: Some(0),
        game_duration_secs: 1800,
        role: "TOP".to_string(),
        game_mode: "CLASSIC".to_string(),
    }
}

fn assert_well_formed(result: &AnalysisResult) -> String {
    assert!(
        result.error.is_none(),
        "analysis errored: {:?}",
        result.error
    );
    let rating = result
        .overall_rating
        .clone()
        .expect("no rating parsed from response");
    let first_word: String = result
        .summary
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect();
    assert_eq!(
        first_word, rating,
        "rating must be the first word, got summary: {}",
        result.summary
    );
    // The prompt contract asks for 150-250 words; LLM compliance is
    // approximate, so leave headroom while still catching a regression to
    // terse 3-sentence answers or runaway rambling.
    let words = result.summary.split_whitespace().count();
    assert!(
        (120..=320).contains(&words),
        "expected 120-320 words, got {words}: {}",
        result.summary
    );
    rating
}

#[tokio::test]
#[ignore = "requires the live LLM server"]
async fn stomp_win_is_never_poor() {
    let pipeline = live_pipeline();
    let mut data = base_data();
    data.champion_name = "Darius".to_string();
    data.enemy_champion_name = Some("Teemo".to_string());
    data.kills = 12;
    data.deaths = 2;
    data.assists = 8;
    data.kda = Some(10.0);
    data.kill_participation = Some(0.65);
    data.gold_per_minute = Some(480.0);
    data.damage_per_minute = Some(820.0);
    data.team_damage_percentage = Some(0.32);
    data.max_cs_advantage_on_lane_opponent = Some(40.0);
    data.gold_diff_at_10 = Some(800);
    data.gold_diff_at_15 = Some(1500);
    data.gold_diff_at_20 = Some(2400);
    data.cs_diff_at_10 = Some(25);
    data.cs_diff_at_15 = Some(35);
    data.cs_diff_at_20 = Some(48);
    data.turret_kills = 4;
    data.total_cs = 240;
    data.gold_earned = 14500;
    data.total_damage_dealt_to_champions = 24500;

    let result = pipeline.analyze_game(&data).await;
    let rating = assert_well_formed(&result);
    assert_ne!(
        rating, "Poor",
        "a dominant win must not rate Poor. Summary: {}",
        result.summary
    );
    println!("stomp_win rating={rating}\n{}", result.summary);
}

#[tokio::test]
#[ignore = "requires the live LLM server"]
async fn disaster_loss_is_never_good() {
    let pipeline = live_pipeline();
    let mut data = base_data();
    data.champion_name = "Zed".to_string();
    data.enemy_champion_name = Some("Ahri".to_string());
    data.role = "MIDDLE".to_string();
    data.win = false;
    data.kills = 1;
    data.deaths = 11;
    data.assists = 3;
    data.kda = Some(0.36);
    data.kill_participation = Some(0.25);
    data.gold_per_minute = Some(280.0);
    data.damage_per_minute = Some(240.0);
    data.team_damage_percentage = Some(0.12);
    data.max_cs_advantage_on_lane_opponent = Some(0.0);
    data.gold_diff_at_10 = Some(-900);
    data.gold_diff_at_15 = Some(-1800);
    data.gold_diff_at_20 = Some(-3200);
    data.cs_diff_at_10 = Some(-30);
    data.cs_diff_at_15 = Some(-45);
    data.cs_diff_at_20 = Some(-60);
    data.turret_kills = 0;
    data.damage_dealt_to_objectives = 800;
    data.total_cs = 130;
    data.gold_earned = 8000;
    data.total_damage_dealt_to_champions = 7000;

    let result = pipeline.analyze_game(&data).await;
    let rating = assert_well_formed(&result);
    assert_ne!(
        rating, "Good",
        "a 1/11 loss with -3.2k gold must not rate Good. Summary: {}",
        result.summary
    );
    println!("disaster_loss rating={rating}\n{}", result.summary);
}

#[tokio::test]
#[ignore = "requires the live LLM server"]
async fn lost_lane_won_game_parses_and_is_not_poor() {
    // The real Gangplank vs Tryndamere game that exposed the calibration
    // gap: lane lost hard, game won through scaling. Rubric says >= Average.
    let pipeline = live_pipeline();
    let mut data = base_data();
    data.kills = 10;
    data.deaths = 8;
    data.assists = 5;
    data.kda = Some(1.9);
    data.kill_participation = Some(0.52);
    data.gold_diff_at_10 = Some(-1287);
    data.gold_diff_at_15 = Some(-1500);
    data.gold_diff_at_20 = Some(-1100);
    data.cs_diff_at_10 = Some(-22);
    data.cs_diff_at_15 = Some(-38);
    data.cs_diff_at_20 = Some(-52);
    data.turret_kills = 4;
    data.total_cs = 165;
    data.damage_per_minute = Some(720.0);
    data.team_damage_percentage = Some(0.28);
    data.game_duration_secs = 2100;

    let result = pipeline.analyze_game(&data).await;
    let rating = assert_well_formed(&result);
    assert_ne!(
        rating, "Poor",
        "lost lane converted into a win must rate at least Average. Summary: {}",
        result.summary
    );
    println!("lost_lane_won_game rating={rating}\n{}", result.summary);
}
