use sfg_bot::riot::client::RiotClientError;

mod riot_client_error {
    use super::*;

    #[test]
    fn account_not_found_display() {
        let err = RiotClientError::AccountNotFound("TestPlayer".to_string(), "EUW".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Account not found"));
        assert!(display.contains("TestPlayer"));
        assert!(display.contains("EUW"));
    }

    #[test]
    fn unknown_region_display() {
        let err = RiotClientError::UnknownRegion("invalid".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Unknown region"));
        assert!(display.contains("invalid"));
    }

    #[test]
    fn error_is_debug() {
        let err = RiotClientError::AccountNotFound("Test".to_string(), "TAG".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("AccountNotFound"));
    }
}
