pub fn format_game_started(summoner_name: &str, champion_name: &str, game_mode: &str) -> String {
    format!(
        "🎮 {} started a {} game as {}!",
        summoner_name, game_mode, champion_name
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
    role: Option<&str>,
) -> String {
    let emoji = if win { "🏆" } else { "💔" };
    let result = if win { "won" } else { "lost" };
    let minutes = duration_secs / 60;

    let role_str = role
        .filter(|r| !r.is_empty() && *r != "Invalid")
        .map(|r| {
            let formatted = match r {
                "TOP" => "Top",
                "JUNGLE" => "Jungle",
                "MIDDLE" => "Mid",
                "BOTTOM" => "Bot",
                "UTILITY" => "Support",
                _ => r,
            };
            format!(" on role {}", formatted)
        })
        .unwrap_or_default();

    format!(
        "{} {} {}! KDA: {}/{}/{} | Duration: {}m{}",
        emoji, summoner_name, result, kills, deaths, assists, minutes, role_str
    )
}

/// Format response for @mentions
pub fn format_mention_response() -> String {
    "This feature is not implemented yet!".to_string()
}
