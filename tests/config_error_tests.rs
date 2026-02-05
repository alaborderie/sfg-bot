use sfg_bot::config::{ConfigError, parse_summoner_names};

mod error_messages {
    use super::*;

    #[test]
    fn invalid_summoner_format_error_contains_input() {
        let result = parse_summoner_names("InvalidInput");
        match result {
            Err(ConfigError::InvalidSummonerFormat(msg)) => {
                assert!(msg.contains("InvalidInput"));
            }
            _ => panic!("Expected InvalidSummonerFormat error"),
        }
    }

    #[test]
    fn invalid_name_error_contains_invalid_name() {
        let result = parse_summoner_names("Bad@Name#TAG");
        match result {
            Err(ConfigError::InvalidSummonerFormat(msg)) => {
                assert!(msg.contains("Invalid name"));
            }
            _ => panic!("Expected InvalidSummonerFormat error with name info"),
        }
    }

    #[test]
    fn invalid_tag_error_contains_invalid_tag() {
        let result = parse_summoner_names("GoodName#BAD!");
        match result {
            Err(ConfigError::InvalidSummonerFormat(msg)) => {
                assert!(msg.contains("Invalid tag"));
            }
            _ => panic!("Expected InvalidSummonerFormat error with tag info"),
        }
    }
}

mod config_error_display {
    use sfg_bot::config::ConfigError;

    #[test]
    fn missing_env_var_display() {
        let err = ConfigError::MissingEnvVar("TEST_VAR".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Missing required environment variable"));
        assert!(display.contains("TEST_VAR"));
    }

    #[test]
    fn invalid_value_display() {
        let err = ConfigError::InvalidValue("MY_VAR".to_string(), "bad value".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Invalid environment variable value"));
        assert!(display.contains("MY_VAR"));
        assert!(display.contains("bad value"));
    }

    #[test]
    fn invalid_summoner_format_display() {
        let err = ConfigError::InvalidSummonerFormat("BadFormat".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Invalid summoner name format"));
        assert!(display.contains("BadFormat"));
    }
}

mod summoner_config_struct {
    use sfg_bot::config::SummonerConfig;

    #[test]
    fn can_create_summoner_config() {
        let config = SummonerConfig {
            name: "TestPlayer".to_string(),
            tag: "EUW".to_string(),
        };
        assert_eq!(config.name, "TestPlayer");
        assert_eq!(config.tag, "EUW");
    }

    #[test]
    fn can_clone_summoner_config() {
        let config = SummonerConfig {
            name: "Player".to_string(),
            tag: "TAG".to_string(),
        };
        let cloned = config.clone();
        assert_eq!(config.name, cloned.name);
        assert_eq!(config.tag, cloned.tag);
    }

    #[test]
    fn debug_format_works() {
        let config = SummonerConfig {
            name: "Debug".to_string(),
            tag: "TEST".to_string(),
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("SummonerConfig"));
        assert!(debug.contains("Debug"));
        assert!(debug.contains("TEST"));
    }
}
