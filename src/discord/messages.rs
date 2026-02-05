/// Format message for game start notification
pub fn format_game_started(summoner_name: &str, champion_id: i32, game_mode: &str) -> String {
    format!(
        "🎮 {} started a {} game as Champion #{}!",
        summoner_name, game_mode, champion_id
    )
}

/// Format message for game end notification
pub fn format_game_ended(
    summoner_name: &str,
    win: bool,
    kills: i32,
    deaths: i32,
    assists: i32,
    duration_secs: i32,
) -> String {
    let emoji = if win { "🏆" } else { "💔" };
    let result = if win { "won" } else { "lost" };
    let minutes = duration_secs / 60;
    format!(
        "{} {} {}! KDA: {}/{}/{} | Duration: {}m",
        emoji, summoner_name, result, kills, deaths, assists, minutes
    )
}

/// Format response for @mentions
pub fn format_mention_response() -> String {
    "This feature is not implemented yet!".to_string()
}
