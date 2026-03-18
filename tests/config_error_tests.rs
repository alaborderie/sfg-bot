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
}
