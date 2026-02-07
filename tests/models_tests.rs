use chrono::Utc;
use sfg_bot::db::models::{NewActiveGame, NewMatchResult};
use sfg_bot::riot::models::{ActiveGameInfo, GameStateChange, MatchResult, SummonerInfo};
use uuid::Uuid;

mod summoner_info {
    use super::*;

    #[test]
    fn can_create_summoner_info() {
        let info = SummonerInfo {
            puuid: "test-puuid".to_string(),
            game_name: "TestPlayer".to_string(),
            tag_line: "EUW".to_string(),
        };
        assert_eq!(info.puuid, "test-puuid");
        assert_eq!(info.game_name, "TestPlayer");
        assert_eq!(info.tag_line, "EUW");
    }

    #[test]
    fn can_clone_summoner_info() {
        let info = SummonerInfo {
            puuid: "test-puuid".to_string(),
            game_name: "TestPlayer".to_string(),
            tag_line: "EUW".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.puuid, cloned.puuid);
        assert_eq!(info.game_name, cloned.game_name);
    }

    #[test]
    fn debug_format_works() {
        let info = SummonerInfo {
            puuid: "test".to_string(),
            game_name: "Player".to_string(),
            tag_line: "TAG".to_string(),
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("SummonerInfo"));
        assert!(debug.contains("test"));
    }
}

mod active_game_info {
    use super::*;

    #[test]
    fn can_create_active_game_info() {
        let now = Utc::now();
        let info = ActiveGameInfo {
            game_id: 12345,
            champion_id: 157,
            game_mode: "CLASSIC".to_string(),
            game_start_time: now,
        };
        assert_eq!(info.game_id, 12345);
        assert_eq!(info.champion_id, 157);
        assert_eq!(info.game_mode, "CLASSIC");
    }

    #[test]
    fn can_clone_active_game_info() {
        let info = ActiveGameInfo {
            game_id: 12345,
            champion_id: 157,
            game_mode: "CLASSIC".to_string(),
            game_start_time: Utc::now(),
        };
        let cloned = info.clone();
        assert_eq!(info.game_id, cloned.game_id);
    }

    #[test]
    fn different_game_modes() {
        let modes = vec!["CLASSIC", "ARAM", "RANKED_SOLO_5x5", "RANKED_FLEX_SR"];
        for mode in modes {
            let info = ActiveGameInfo {
                game_id: 1,
                champion_id: 1,
                game_mode: mode.to_string(),
                game_start_time: Utc::now(),
            };
            assert_eq!(info.game_mode, mode);
        }
    }
}

mod match_result {
    use super::*;

    #[test]
    fn can_create_match_result() {
        let result = MatchResult {
            match_id: "EUW1_12345".to_string(),
            game_id: 12345,
            win: true,
            kills: 10,
            deaths: 5,
            assists: 15,
            champion_id: 157,
            game_duration_secs: 1800,
            game_mode: "CLASSIC".to_string(),
            role: "TOP".to_string(),
            total_cs: 200,
            total_gold: 15000,
            total_damage: 25000,
            enemy_champion_name: Some("Darius".to_string()),
            enemy_cs: Some(180),
            enemy_gold: Some(14000),
            enemy_damage: Some(22000),
        };
        assert_eq!(result.match_id, "EUW1_12345");
        assert!(result.win);
        assert_eq!(result.kills, 10);
    }

    #[test]
    fn can_represent_loss() {
        let result = MatchResult {
            match_id: "EUW1_12345".to_string(),
            game_id: 12345,
            win: false,
            kills: 3,
            deaths: 8,
            assists: 5,
            champion_id: 157,
            game_duration_secs: 1500,
            game_mode: "CLASSIC".to_string(),
            role: "MID".to_string(),
            total_cs: 150,
            total_gold: 10000,
            total_damage: 18000,
            enemy_champion_name: Some("Zed".to_string()),
            enemy_cs: Some(180),
            enemy_gold: Some(13000),
            enemy_damage: Some(24000),
        };
        assert!(!result.win);
    }

    #[test]
    fn can_clone_match_result() {
        let result = MatchResult {
            match_id: "test".to_string(),
            game_id: 1,
            win: true,
            kills: 1,
            deaths: 1,
            assists: 1,
            champion_id: 1,
            game_duration_secs: 1000,
            game_mode: "CLASSIC".to_string(),
            role: "JUNGLE".to_string(),
            total_cs: 100,
            total_gold: 8000,
            total_damage: 15000,
            enemy_champion_name: None,
            enemy_cs: None,
            enemy_gold: None,
            enemy_damage: None,
        };
        let cloned = result.clone();
        assert_eq!(result.match_id, cloned.match_id);
        assert_eq!(result.win, cloned.win);
    }

    #[test]
    fn zero_kda_is_valid() {
        let result = MatchResult {
            match_id: "test".to_string(),
            game_id: 1,
            win: false,
            kills: 0,
            deaths: 0,
            assists: 0,
            champion_id: 1,
            game_duration_secs: 900,
            game_mode: "CLASSIC".to_string(),
            role: "SUPPORT".to_string(),
            total_cs: 50,
            total_gold: 5000,
            total_damage: 8000,
            enemy_champion_name: None,
            enemy_cs: None,
            enemy_gold: None,
            enemy_damage: None,
        };
        assert_eq!(result.kills, 0);
        assert_eq!(result.deaths, 0);
        assert_eq!(result.assists, 0);
    }
}

mod game_state_change {
    use super::*;

    #[test]
    fn can_create_game_started() {
        let game_info = ActiveGameInfo {
            game_id: 12345,
            champion_id: 157,
            game_mode: "CLASSIC".to_string(),
            game_start_time: Utc::now(),
        };
        let change = GameStateChange::GameStarted(game_info);
        assert!(matches!(change, GameStateChange::GameStarted(_)));
    }

    #[test]
    fn can_create_game_ended() {
        let change = GameStateChange::GameEnded {
            game_id: 12345,
            is_featured_mode: false,
        };
        assert!(matches!(change, GameStateChange::GameEnded { .. }));
    }

    #[test]
    fn can_create_no_change() {
        let change = GameStateChange::NoChange;
        assert!(matches!(change, GameStateChange::NoChange));
    }

    #[test]
    fn can_clone_game_state_change() {
        let change = GameStateChange::NoChange;
        let cloned = change.clone();
        assert!(matches!(cloned, GameStateChange::NoChange));
    }

    #[test]
    fn game_ended_contains_game_id() {
        let change = GameStateChange::GameEnded {
            game_id: 99999,
            is_featured_mode: false,
        };
        if let GameStateChange::GameEnded {
            game_id,
            is_featured_mode: _,
        } = change
        {
            assert_eq!(game_id, 99999);
        } else {
            panic!("Expected GameEnded");
        }
    }

    #[test]
    fn game_started_contains_game_info() {
        let info = ActiveGameInfo {
            game_id: 55555,
            champion_id: 1,
            game_mode: "ARAM".to_string(),
            game_start_time: Utc::now(),
        };
        let change = GameStateChange::GameStarted(info);
        if let GameStateChange::GameStarted(game_info) = change {
            assert_eq!(game_info.game_id, 55555);
            assert_eq!(game_info.game_mode, "ARAM");
        } else {
            panic!("Expected GameStarted");
        }
    }
}

mod new_active_game {
    use super::*;

    #[test]
    fn can_create_new_active_game() {
        let game = NewActiveGame {
            summoner_id: Uuid::new_v4(),
            game_id: 12345,
            champion_id: 157,
            game_mode: "CLASSIC".to_string(),
            game_start_time: Utc::now(),
        };
        assert_eq!(game.game_id, 12345);
        assert_eq!(game.champion_id, 157);
    }

    #[test]
    fn can_clone_new_active_game() {
        let game = NewActiveGame {
            summoner_id: Uuid::new_v4(),
            game_id: 12345,
            champion_id: 157,
            game_mode: "CLASSIC".to_string(),
            game_start_time: Utc::now(),
        };
        let cloned = game.clone();
        assert_eq!(game.game_id, cloned.game_id);
    }
}

mod new_match_result {
    use super::*;

    #[test]
    fn can_create_new_match_result() {
        let result = NewMatchResult {
            summoner_id: Uuid::new_v4(),
            match_id: "EUW1_12345".to_string(),
            game_id: 12345,
            win: true,
            kills: 10,
            deaths: 5,
            assists: 15,
            champion_id: 157,
            game_duration_secs: 1800,
            game_mode: "CLASSIC".to_string(),
            role: "TOP".to_string(),
            finished_at: Utc::now(),
            total_cs: 200,
            total_gold: 15000,
            total_damage: 25000,
            enemy_champion_name: Some("Darius".to_string()),
            enemy_cs: Some(180),
            enemy_gold: Some(14000),
            enemy_damage: Some(22000),
        };
        assert!(result.win);
        assert_eq!(result.kills, 10);
    }

    #[test]
    fn can_clone_new_match_result() {
        let result = NewMatchResult {
            summoner_id: Uuid::new_v4(),
            match_id: "test".to_string(),
            game_id: 1,
            win: true,
            kills: 1,
            deaths: 1,
            assists: 1,
            champion_id: 1,
            game_duration_secs: 1000,
            game_mode: "CLASSIC".to_string(),
            role: "MID".to_string(),
            finished_at: Utc::now(),
            total_cs: 100,
            total_gold: 8000,
            total_damage: 15000,
            enemy_champion_name: None,
            enemy_cs: None,
            enemy_gold: None,
            enemy_damage: None,
        };
        let cloned = result.clone();
        assert_eq!(result.match_id, cloned.match_id);
    }
}
