use sfg_bot::discord::messages::{format_game_ended, format_game_started, format_mention_response};

mod format_game_started_tests {
    use super::*;

    #[test]
    fn contains_emoji() {
        let msg = format_game_started("Catory", 157, "CLASSIC");
        assert!(msg.contains("🎮"));
    }

    #[test]
    fn contains_summoner_name() {
        let msg = format_game_started("Catory", 157, "CLASSIC");
        assert!(msg.contains("Catory"));
    }

    #[test]
    fn contains_game_mode() {
        let msg = format_game_started("Catory", 157, "CLASSIC");
        assert!(msg.contains("CLASSIC"));
    }

    #[test]
    fn contains_champion_id() {
        let msg = format_game_started("Catory", 157, "CLASSIC");
        assert!(msg.contains("Champion #157"));
    }

    #[test]
    fn aram_game_mode() {
        let msg = format_game_started("Player", 1, "ARAM");
        assert!(msg.contains("ARAM"));
    }

    #[test]
    fn ranked_game_mode() {
        let msg = format_game_started("Player", 1, "RANKED_SOLO_5x5");
        assert!(msg.contains("RANKED_SOLO_5x5"));
    }

    #[test]
    fn summoner_name_with_spaces() {
        let msg = format_game_started("Player With Spaces", 157, "CLASSIC");
        assert!(msg.contains("Player With Spaces"));
    }

    #[test]
    fn champion_id_zero() {
        let msg = format_game_started("Player", 0, "CLASSIC");
        assert!(msg.contains("Champion #0"));
    }

    #[test]
    fn high_champion_id() {
        let msg = format_game_started("Player", 999, "CLASSIC");
        assert!(msg.contains("Champion #999"));
    }
}

mod format_game_ended_tests {
    use super::*;

    #[test]
    fn win_contains_trophy_emoji() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800);
        assert!(msg.contains("🏆"));
    }

    #[test]
    fn win_contains_won_text() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800);
        assert!(msg.contains("won"));
    }

    #[test]
    fn loss_contains_broken_heart_emoji() {
        let msg = format_game_ended("Catory", false, 3, 8, 5, 1500);
        assert!(msg.contains("💔"));
    }

    #[test]
    fn loss_contains_lost_text() {
        let msg = format_game_ended("Catory", false, 3, 8, 5, 1500);
        assert!(msg.contains("lost"));
    }

    #[test]
    fn contains_summoner_name() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800);
        assert!(msg.contains("Catory"));
    }

    #[test]
    fn contains_kda() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800);
        assert!(msg.contains("10/5/15"));
    }

    #[test]
    fn duration_30_minutes() {
        let msg = format_game_ended("Catory", true, 10, 5, 15, 1800);
        assert!(msg.contains("30m"));
    }

    #[test]
    fn duration_25_minutes() {
        let msg = format_game_ended("Catory", false, 3, 8, 5, 1500);
        assert!(msg.contains("25m"));
    }

    #[test]
    fn duration_45_minutes() {
        let msg = format_game_ended("Player", true, 15, 3, 20, 2700);
        assert!(msg.contains("45m"));
    }

    #[test]
    fn short_game_duration_15_minutes() {
        let msg = format_game_ended("Player", true, 5, 0, 10, 900);
        assert!(msg.contains("15m"));
    }

    #[test]
    fn zero_kills() {
        let msg = format_game_ended("Player", false, 0, 10, 5, 1800);
        assert!(msg.contains("0/10/5"));
    }

    #[test]
    fn zero_deaths() {
        let msg = format_game_ended("Player", true, 20, 0, 15, 1800);
        assert!(msg.contains("20/0/15"));
    }

    #[test]
    fn zero_assists() {
        let msg = format_game_ended("Player", true, 10, 5, 0, 1800);
        assert!(msg.contains("10/5/0"));
    }

    #[test]
    fn perfect_game() {
        let msg = format_game_ended("Player", true, 15, 0, 10, 1200);
        assert!(msg.contains("🏆"));
        assert!(msg.contains("15/0/10"));
    }

    #[test]
    fn high_kda_numbers() {
        let msg = format_game_ended("Player", true, 99, 99, 99, 3600);
        assert!(msg.contains("99/99/99"));
    }

    #[test]
    fn summoner_name_with_special_chars() {
        let msg = format_game_ended("Player-Name_123", true, 10, 5, 15, 1800);
        assert!(msg.contains("Player-Name_123"));
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
