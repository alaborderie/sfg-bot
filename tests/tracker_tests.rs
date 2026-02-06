use chrono::Utc;
use mockall::predicate::*;
use riven::consts::PlatformRoute;
use sfg_bot::db::models::{ActiveGame, MatchHistory, NewActiveGame, Summoner};
use sfg_bot::riot::client::RiotClientError;
use sfg_bot::riot::models::{ActiveGameInfo, GameStateChange, MatchResult};
use sfg_bot::riot::tracker::GameTracker;
use sfg_bot::{MockRepository, MockRiotApiClient, RepositoryError};
use std::sync::Arc;
use uuid::Uuid;

fn create_test_summoner() -> Summoner {
    Summoner {
        id: Uuid::new_v4(),
        riot_puuid: "test-puuid-12345".to_string(),
        game_name: "TestPlayer".to_string(),
        tag_line: "NA1".to_string(),
        region: "na1".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_test_active_game_info(game_id: i64) -> ActiveGameInfo {
    ActiveGameInfo {
        game_id,
        champion_id: 1,
        game_mode: "CLASSIC".to_string(),
        game_start_time: Utc::now(),
    }
}

fn create_test_active_game(summoner_id: Uuid, game_id: i64) -> ActiveGame {
    ActiveGame {
        id: Uuid::new_v4(),
        summoner_id,
        game_id,
        champion_id: 1,
        game_mode: "CLASSIC".to_string(),
        game_start_time: Utc::now(),
        created_at: Utc::now(),
    }
}

fn create_test_match_result(game_id: i64) -> MatchResult {
    MatchResult {
        match_id: format!("NA1_{}", game_id),
        game_id,
        win: true,
        kills: 10,
        deaths: 2,
        assists: 5,
        champion_id: 1,
        game_duration_secs: 1800,
        game_mode: "CLASSIC".to_string(),
        role: "JUNGLE".to_string(),
        total_cs: 180,
        total_gold: 13500,
        total_damage: 20000,
        enemy_champion_name: Some("Lee Sin".to_string()),
        enemy_cs: Some(170),
        enemy_gold: Some(12000),
        enemy_damage: Some(18000),
    }
}

fn create_test_match_history(summoner_id: Uuid, game_id: i64) -> MatchHistory {
    MatchHistory {
        id: Uuid::new_v4(),
        summoner_id,
        match_id: format!("NA1_{}", game_id),
        game_id,
        win: true,
        kills: 10,
        deaths: 2,
        assists: 5,
        champion_id: 1,
        game_duration_secs: 1800,
        game_mode: "CLASSIC".to_string(),
        role: Some("JUNGLE".to_string()),
        finished_at: Utc::now(),
        created_at: Utc::now(),
        total_cs: 180,
        total_gold: 13500,
        total_damage: 20000,
        enemy_champion_name: Some("Lee Sin".to_string()),
        enemy_cs: Some(170),
        enemy_gold: Some(12000),
        enemy_damage: Some(18000),
    }
}
mod check_summoner_game_state {
    use super::*;

    #[tokio::test]
    async fn returns_game_started_when_in_game_but_not_in_db() {
        let summoner = create_test_summoner();

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_active_game()
            .with(eq(summoner.riot_puuid.clone()), eq(PlatformRoute::NA1))
            .times(1)
            .returning(move |_, _| Ok(Some(create_test_active_game_info(12345))));

        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_get_active_games_for_summoner()
            .with(eq(summoner.id))
            .times(1)
            .returning(|_| Ok(vec![]));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.check_summoner_game_state(&summoner).await.unwrap();

        match result {
            GameStateChange::GameStarted(info) => {
                assert_eq!(info.game_id, 12345);
            }
            _ => panic!("Expected GameStarted"),
        }
    }

    #[tokio::test]
    async fn returns_game_ended_when_not_in_game_but_in_db() {
        let summoner = create_test_summoner();

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_active_game()
            .with(eq(summoner.riot_puuid.clone()), eq(PlatformRoute::NA1))
            .times(1)
            .returning(|_, _| Ok(None));

        let summoner_id = summoner.id;
        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_get_active_games_for_summoner()
            .with(eq(summoner.id))
            .times(1)
            .returning(move |_| Ok(vec![create_test_active_game(summoner_id, 12345)]));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.check_summoner_game_state(&summoner).await.unwrap();

        match result {
            GameStateChange::GameEnded { game_id } => {
                assert_eq!(game_id, 12345);
            }
            _ => panic!("Expected GameEnded"),
        }
    }

    #[tokio::test]
    async fn returns_game_ended_when_different_game_started() {
        let summoner = create_test_summoner();

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_active_game()
            .with(eq(summoner.riot_puuid.clone()), eq(PlatformRoute::NA1))
            .times(1)
            .returning(|_, _| Ok(Some(create_test_active_game_info(22222))));

        let summoner_id = summoner.id;
        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_get_active_games_for_summoner()
            .with(eq(summoner.id))
            .times(1)
            .returning(move |_| Ok(vec![create_test_active_game(summoner_id, 11111)]));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.check_summoner_game_state(&summoner).await.unwrap();

        match result {
            GameStateChange::GameEnded { game_id } => {
                assert_eq!(game_id, 11111);
            }
            _ => panic!("Expected GameEnded for old game"),
        }
    }

    #[tokio::test]
    async fn returns_no_change_when_not_in_game_and_not_in_db() {
        let summoner = create_test_summoner();

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_active_game()
            .with(eq(summoner.riot_puuid.clone()), eq(PlatformRoute::NA1))
            .times(1)
            .returning(|_, _| Ok(None));

        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_get_active_games_for_summoner()
            .with(eq(summoner.id))
            .times(1)
            .returning(|_| Ok(vec![]));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.check_summoner_game_state(&summoner).await.unwrap();

        assert!(matches!(result, GameStateChange::NoChange));
    }

    #[tokio::test]
    async fn returns_no_change_when_in_same_game() {
        let summoner = create_test_summoner();
        let summoner_id = summoner.id;

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_active_game()
            .with(eq(summoner.riot_puuid.clone()), eq(PlatformRoute::NA1))
            .times(1)
            .returning(|_, _| Ok(Some(create_test_active_game_info(12345))));

        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_get_active_games_for_summoner()
            .with(eq(summoner.id))
            .times(1)
            .returning(move |_| Ok(vec![create_test_active_game(summoner_id, 12345)]));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.check_summoner_game_state(&summoner).await.unwrap();

        assert!(matches!(result, GameStateChange::NoChange));
    }

    #[tokio::test]
    async fn propagates_riot_api_error() {
        let summoner = create_test_summoner();

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_active_game()
            .times(1)
            .returning(|_, _| Err(RiotClientError::UnknownRegion("test".to_string())));

        let mock_repo = MockRepository::new();

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.check_summoner_game_state(&summoner).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn propagates_repository_error() {
        let summoner = create_test_summoner();

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_active_game()
            .times(1)
            .returning(|_, _| Ok(None));

        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_get_active_games_for_summoner()
            .times(1)
            .returning(|_| Err(RepositoryError::Database(sqlx::Error::RowNotFound)));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.check_summoner_game_state(&summoner).await;

        assert!(result.is_err());
    }
}

mod handle_game_started {
    use super::*;

    #[tokio::test]
    async fn inserts_active_game_to_repository() {
        let summoner = create_test_summoner();
        let game_info = create_test_active_game_info(12345);

        let mock_riot = MockRiotApiClient::new();

        let summoner_id = summoner.id;
        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_insert_active_game()
            .withf(move |new_game: &NewActiveGame| {
                new_game.summoner_id == summoner_id
                    && new_game.game_id == 12345
                    && new_game.champion_id == 1
                    && new_game.game_mode == "CLASSIC"
            })
            .times(1)
            .returning(move |new_game| {
                Ok(ActiveGame {
                    id: Uuid::new_v4(),
                    summoner_id: new_game.summoner_id,
                    game_id: new_game.game_id,
                    champion_id: new_game.champion_id,
                    game_mode: new_game.game_mode.clone(),
                    game_start_time: new_game.game_start_time,
                    created_at: Utc::now(),
                })
            });

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.handle_game_started(&summoner, &game_info).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn propagates_repository_error() {
        let summoner = create_test_summoner();
        let game_info = create_test_active_game_info(12345);

        let mock_riot = MockRiotApiClient::new();

        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_insert_active_game()
            .times(1)
            .returning(|_| Err(RepositoryError::Database(sqlx::Error::RowNotFound)));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.handle_game_started(&summoner, &game_info).await;

        assert!(result.is_err());
    }
}

mod handle_game_ended {
    use super::*;

    #[tokio::test]
    async fn deletes_active_game_and_returns_match_result() {
        let summoner = create_test_summoner();
        let game_id = 12345i64;

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_match_result()
            .withf(|match_id, _, _| match_id == "NA1_12345")
            .times(1)
            .returning(move |_, _, _| Ok(Some(create_test_match_result(12345))));

        let summoner_id = summoner.id;
        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_delete_active_game_by_summoner_and_game()
            .with(eq(summoner.id), eq(game_id))
            .times(1)
            .returning(|_, _| Ok(()));
        mock_repo
            .expect_insert_match_result()
            .times(1)
            .returning(move |new_match| {
                Ok(create_test_match_history(summoner_id, new_match.game_id))
            });

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.handle_game_ended(&summoner, game_id).await.unwrap();

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.game_id, 12345);
        assert!(match_result.win);
    }

    #[tokio::test]
    async fn returns_none_when_match_not_found() {
        tokio::time::pause();

        let summoner = create_test_summoner();
        let game_id = 12345i64;

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_match_result()
            .withf(|match_id, _, _| match_id == "NA1_12345")
            .times(6)
            .returning(|_, _, _| Ok(None));

        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_delete_active_game_by_summoner_and_game()
            .with(eq(summoner.id), eq(game_id))
            .times(1)
            .returning(|_, _| Ok(()));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.handle_game_ended(&summoner, game_id).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn propagates_delete_error() {
        let summoner = create_test_summoner();
        let game_id = 12345i64;

        let mock_riot = MockRiotApiClient::new();

        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_delete_active_game_by_summoner_and_game()
            .times(1)
            .returning(|_, _| Err(RepositoryError::Database(sqlx::Error::RowNotFound)));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.handle_game_ended(&summoner, game_id).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn continues_when_insert_match_result_fails() {
        let summoner = create_test_summoner();
        let game_id = 12345i64;

        let mut mock_riot = MockRiotApiClient::new();
        mock_riot
            .expect_get_match_result()
            .withf(|match_id, _, _| match_id == "NA1_12345")
            .times(1)
            .returning(move |_, _, _| Ok(Some(create_test_match_result(12345))));

        let mut mock_repo = MockRepository::new();
        mock_repo
            .expect_delete_active_game_by_summoner_and_game()
            .times(1)
            .returning(|_, _| Ok(()));
        mock_repo
            .expect_insert_match_result()
            .times(1)
            .returning(|_| Err(RepositoryError::Database(sqlx::Error::RowNotFound)));

        let tracker = GameTracker::new(Arc::new(mock_riot), Arc::new(mock_repo), "na1".to_string());

        let result = tracker.handle_game_ended(&summoner, game_id).await.unwrap();

        assert!(result.is_some());
    }
}
