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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_game_started() {
        let msg = format_game_started("Catory", 157, "CLASSIC");
        assert!(msg.contains("🎮"));
        assert!(msg.contains("Catory"));
        assert!(msg.contains("CLASSIC"));
        assert!(msg.contains("Champion #157"));
    }

    #[test]
    fn test_format_game_ended_win() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800);
        assert!(msg.contains("🏆"));
        assert!(msg.contains("Catory"));
        assert!(msg.contains("won"));
        assert!(msg.contains("10/5/15"));
        assert!(msg.contains("30m"));
    }

    #[test]
    fn test_format_game_ended_loss() {
        let msg = format_game_ended("Catory", false, 3, 8, 5, 1500);
        assert!(msg.contains("💔"));
        assert!(msg.contains("lost"));
        assert!(msg.contains("3/8/5"));
        assert!(msg.contains("25m"));
    }

    #[test]
    fn test_format_mention_response() {
        let msg = format_mention_response();
        assert_eq!(msg, "This feature is not implemented yet!");
    }
}
