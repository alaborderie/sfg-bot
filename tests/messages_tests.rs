use sfg_bot::discord::messages::{format_game_ended, format_game_started, format_mention_response};

mod format_game_started_tests {
    use super::*;

    #[test]
    fn contains_emoji() {
        let msg = format_game_started("Catory", "Yasuo", "CLASSIC");
        assert!(msg.contains("🎮"));
    }

    #[test]
    fn contains_summoner_name() {
        let msg = format_game_started("Catory", "Yasuo", "CLASSIC");
        assert!(msg.contains("Catory"));
    }

    #[test]
    fn contains_game_mode() {
        let msg = format_game_started("Catory", "Yasuo", "CLASSIC");
        assert!(msg.contains("CLASSIC"));
    }

    #[test]
    fn contains_champion_name() {
        let msg = format_game_started("Catory", "Yasuo", "CLASSIC");
        assert!(msg.contains("Yasuo"));
    }

    #[test]
    fn aram_game_mode() {
        let msg = format_game_started("Player", "Annie", "ARAM");
        assert!(msg.contains("ARAM"));
    }

    #[test]
    fn ranked_game_mode() {
        let msg = format_game_started("Player", "Annie", "RANKED_SOLO_5x5");
        assert!(msg.contains("RANKED_SOLO_5x5"));
    }

    #[test]
    fn summoner_name_with_spaces() {
        let msg = format_game_started("Player With Spaces", "Yasuo", "CLASSIC");
        assert!(msg.contains("Player With Spaces"));
    }

    #[test]
    fn champion_with_spaces() {
        let msg = format_game_started("Player", "Twisted Fate", "CLASSIC");
        assert!(msg.contains("Twisted Fate"));
    }

    #[test]
    fn fallback_champion_format() {
        let msg = format_game_started("Player", "Champion #999", "CLASSIC");
        assert!(msg.contains("Champion #999"));
    }
}

mod format_game_ended_tests {
    use super::*;

    #[test]
    fn win_contains_trophy_emoji() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800, None);
        assert!(msg.contains("🏆"));
    }

    #[test]
    fn win_contains_won_text() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800, None);
        assert!(msg.contains("won"));
    }

    #[test]
    fn loss_contains_broken_heart_emoji() {
        let msg = format_game_ended("Catory", false, 3, 8, 5, 1500, None);
        assert!(msg.contains("💔"));
    }

    #[test]
    fn loss_contains_lost_text() {
        let msg = format_game_ended("Catory", false, 3, 8, 5, 1500, None);
        assert!(msg.contains("lost"));
    }

    #[test]
    fn contains_summoner_name() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800, None);
        assert!(msg.contains("Catory"));
    }

    #[test]
    fn contains_kda() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800, None);
        assert!(msg.contains("10/5/15"));
    }

    #[test]
    fn duration_30_minutes() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800, None);
        assert!(msg.contains("30m"));
    }

    #[test]
    fn duration_25_minutes() {
        let msg = format_game_ended("Catory", false, 3, 8, 5, 1500, None);
        assert!(msg.contains("25m"));
    }

    #[test]
    fn duration_45_minutes() {
        let msg = format_game_ended("Player", true, 15, 3, 20, 2700, None);
        assert!(msg.contains("45m"));
    }

    #[test]
    fn short_game_duration_15_minutes() {
        let msg = format_game_ended("Player", true, 5, 0, 10, 900, None);
        assert!(msg.contains("15m"));
    }

    #[test]
    fn zero_kills() {
        let msg = format_game_ended("Player", false, 0, 10, 5, 1800, None);
        assert!(msg.contains("0/10/5"));
    }

    #[test]
    fn zero_deaths() {
        let msg = format_game_ended("Player", true, 20, 0, 15, 1800, None);
        assert!(msg.contains("20/0/15"));
    }

    #[test]
    fn zero_assists() {
        let msg = format_game_ended("Player", true, 10, 5, 0, 1800, None);
        assert!(msg.contains("10/5/0"));
    }

    #[test]
    fn perfect_game() {
        let msg = format_game_ended("Player", true, 15, 0, 10, 1200, None);
        assert!(msg.contains("🏆"));
        assert!(msg.contains("15/0/10"));
    }

    #[test]
    fn high_kda_numbers() {
        let msg = format_game_ended("Player", true, 99, 99, 99, 3600, None);
        assert!(msg.contains("99/99/99"));
    }

    #[test]
    fn summoner_name_with_special_chars() {
        let msg = format_game_ended("Player-Name_123", true, 10, 5, 15, 1800, None);
        assert!(msg.contains("Player-Name_123"));
    }

    #[test]
    fn role_jungle() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800, Some("JUNGLE"));
        assert!(msg.contains("on role Jungle"));
    }

    #[test]
    fn role_top() {
        let msg = format_game_ended("Player", true, 10, 5, 15, 1800, Some("TOP"));
        assert!(msg.contains("on role Top"));
    }

    #[test]
    fn role_middle() {
        let msg = format_game_ended("Player", true, 10, 5, 15, 1800, Some("MIDDLE"));
        assert!(msg.contains("on role Mid"));
    }

    #[test]
    fn role_bottom() {
        let msg = format_game_ended("Player", true, 10, 5, 15, 1800, Some("BOTTOM"));
        assert!(msg.contains("on role Bot"));
    }

    #[test]
    fn role_utility() {
        let msg = format_game_ended("Player", true, 10, 5, 15, 1800, Some("UTILITY"));
        assert!(msg.contains("on role Support"));
    }

    #[test]
    fn role_empty_string() {
        let msg = format_game_ended("Player", true, 10, 5, 15, 1800, Some(""));
        assert!(!msg.contains("on role"));
    }

    #[test]
    fn role_invalid() {
        let msg = format_game_ended("Player", true, 10, 5, 15, 1800, Some("Invalid"));
        assert!(!msg.contains("on role"));
    }

    #[test]
    fn role_none() {
        let msg = format_game_ended("Player", true, 10, 5, 15, 1800, None);
        assert!(!msg.contains("on role"));
    }
}

mod format_mention_response_tests {
    use super::*;

    #[test]
    fn returns_not_implemented_message() {
        let msg = format_mention_response();
        assert_eq!(msg, "This feature is not implemented yet!");
    }

    #[test]
    fn is_not_empty() {
        let msg = format_mention_response();
        assert!(!msg.is_empty());
    }

    #[test]
    fn returns_same_response_on_multiple_calls() {
        let msg1 = format_mention_response();
        let msg2 = format_mention_response();
        assert_eq!(msg1, msg2);
    }
}
