use sfg_bot::config::{parse_summoner_names, ConfigError};

mod parse_summoner_names {
    use super::*;

    #[test]
    fn single_summoner() {
        let result = parse_summoner_names("Catory#6433").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Catory");
        assert_eq!(result[0].tag, "6433");
    }

    #[test]
    fn multiple_summoners() {
        let result = parse_summoner_names("Catory#6433|AnotherPlayer#EUW").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Catory");
        assert_eq!(result[0].tag, "6433");
        assert_eq!(result[1].name, "AnotherPlayer");
        assert_eq!(result[1].tag, "EUW");
    }

    #[test]
    fn three_summoners() {
        let result = parse_summoner_names("Player1#TAG1|Player2#TAG2|Player3#TAG3").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "Player1");
        assert_eq!(result[1].name, "Player2");
        assert_eq!(result[2].name, "Player3");
    }

    #[test]
    fn summoner_with_spaces() {
        let result = parse_summoner_names("Player With Spaces#TAG1").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Player With Spaces");
        assert_eq!(result[0].tag, "TAG1");
    }

    #[test]
    fn summoner_with_hyphens() {
        let result = parse_summoner_names("Player-With-Hyphens#TAG1").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Player-With-Hyphens");
    }

    #[test]
    fn summoner_with_underscores() {
        let result = parse_summoner_names("Player_With_Underscores#TAG1").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Player_With_Underscores");
    }

    #[test]
    fn summoner_with_numbers() {
        let result = parse_summoner_names("Player123#TAG1").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Player123");
    }

    #[test]
    fn summoner_name_all_numbers() {
        let result = parse_summoner_names("12345#TAG1").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "12345");
    }

    #[test]
    fn empty_string() {
        let result = parse_summoner_names("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn whitespace_only() {
        let result = parse_summoner_names("   ").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn whitespace_handling() {
        let result = parse_summoner_names("  Catory#6433  |  Another#TAG  ").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Catory");
        assert_eq!(result[0].tag, "6433");
        assert_eq!(result[1].name, "Another");
        assert_eq!(result[1].tag, "TAG");
    }

    #[test]
    fn tag_with_numbers() {
        let result = parse_summoner_names("Player#12345").unwrap();
        assert_eq!(result[0].tag, "12345");
    }

    #[test]
    fn tag_mixed_case() {
        let result = parse_summoner_names("Player#EuW1").unwrap();
        assert_eq!(result[0].tag, "EuW1");
    }

    #[test]
    fn invalid_no_hash() {
        let result = parse_summoner_names("InvalidName");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidSummonerFormat(_)
        ));
    }

    #[test]
    fn invalid_empty_name() {
        let result = parse_summoner_names("#TAG");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_empty_tag() {
        let result = parse_summoner_names("Name#");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_name_with_special_chars() {
        let result = parse_summoner_names("Player@Name#TAG");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_name_with_exclamation() {
        let result = parse_summoner_names("Player!Name#TAG");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_tag_with_special_chars() {
        let result = parse_summoner_names("Player#TAG!");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_tag_with_hash() {
        let result = parse_summoner_names("Player#TAG#1");
        assert!(result.is_err());
    }

    #[test]
    fn mixed_valid_and_invalid() {
        let result = parse_summoner_names("ValidPlayer#TAG|Invalid@Player#TAG");
        assert!(result.is_err());
    }

    #[test]
    fn name_at_max_length_24_chars() {
        let result = parse_summoner_names("ABCDEFGHIJKLMNOPQRSTUVWX#TAG").unwrap();
        assert_eq!(result[0].name.len(), 24);
    }

    #[test]
    fn name_exceeds_max_length_25_chars() {
        let result = parse_summoner_names("ABCDEFGHIJKLMNOPQRSTUVWXY#TAG");
        assert!(result.is_err());
    }

    #[test]
    fn tag_at_max_length_5_chars() {
        let result = parse_summoner_names("Player#ABCDE").unwrap();
        assert_eq!(result[0].tag.len(), 5);
    }

    #[test]
    fn tag_exceeds_max_length_6_chars() {
        let result = parse_summoner_names("Player#ABCDEF");
        assert!(result.is_err());
    }

    #[test]
    fn single_char_name() {
        let result = parse_summoner_names("A#TAG").unwrap();
        assert_eq!(result[0].name, "A");
    }

    #[test]
    fn single_char_tag() {
        let result = parse_summoner_names("Player#A").unwrap();
        assert_eq!(result[0].tag, "A");
    }
}
