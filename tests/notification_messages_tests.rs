use chrono::Utc;
use sfg_bot::db::models::{NotificationEvent, Summoner};
use sfg_bot::notification::messages::format_grouped_game_ended;
use uuid::Uuid;

fn create_dummy_summoner() -> Summoner {
    Summoner {
        id: Uuid::new_v4(),
        riot_puuid: "test_puuid".to_string(),
        game_name: "TestUser".to_string(),
        tag_line: "EUW".to_string(),
        region: "euw1".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_dummy_event(win: bool) -> NotificationEvent {
    NotificationEvent {
        id: Uuid::new_v4(),
        summoner_id: Uuid::new_v4(),
        event_type: "GAME_ENDED".to_string(),
        game_id: 12345,
        match_id: Some("EUW1_12345".to_string()),
        champion_id: 1,
        champion_name: "Annie".to_string(),
        role: Some("MIDDLE".to_string()),
        win: Some(win),
        kills: Some(10),
        deaths: Some(2),
        assists: Some(5),
        game_duration_secs: Some(1800),
        game_mode: "CLASSIC".to_string(),
        queue_id: Some(420),
        is_featured_mode: false,
        total_cs: Some(200),
        total_gold: Some(12000),
        total_damage: Some(25000),
        enemy_champion_name: Some("Ahri".to_string()),
        enemy_cs: Some(180),
        enemy_gold: Some(11000),
        enemy_damage: Some(24000),
        processed: false,
        created_at: Utc::now(),
        processed_at: None,
    }
}

#[test]
fn test_game_won_title() {
    let summoner = create_dummy_summoner();
    let mut event = create_dummy_event(true); // Win
    event.summoner_id = summoner.id;

    let embed = format_grouped_game_ended(&[summoner], &[event], "Ranked Solo/Duo");

    // Debug print the embed to inspect the title, as fields might be private
    let debug_str = format!("{:?}", embed);
    assert!(debug_str.contains("Game Won!"));
}

#[test]
fn test_game_lost_title() {
    let summoner = create_dummy_summoner();
    let mut event = create_dummy_event(false); // Loss
    event.summoner_id = summoner.id;

    let embed = format_grouped_game_ended(&[summoner], &[event], "Ranked Solo/Duo");

    let debug_str = format!("{:?}", embed);
    assert!(debug_str.contains("Game Lost!"));
}
