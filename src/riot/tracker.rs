use crate::db::models::{NewActiveGame, NewMatchResult, Summoner};
use crate::db::repository::{Repository, RepositoryError};
use crate::riot::client::{RiotApiClient, RiotClient, RiotClientError};
use crate::riot::models::{ActiveGameInfo, GameStateChange, MatchResult};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrackerError {
    #[error("Database error: {0}")]
    Database(#[from] RepositoryError),
    #[error("Riot API error: {0}")]
    RiotApi(#[from] RiotClientError),
}

pub struct GameTracker<R: RiotApiClient + ?Sized, D: Repository + ?Sized> {
    riot_client: Arc<R>,
    pub repository: Arc<D>,
    default_region: String,
}

impl<R: RiotApiClient + ?Sized, D: Repository + ?Sized> GameTracker<R, D> {
    pub fn new(riot_client: Arc<R>, repository: Arc<D>, default_region: String) -> Self {
        Self {
            riot_client,
            repository,
            default_region,
        }
    }

    /// Check if summoner's game state has changed
    pub async fn check_summoner_game_state(
        &self,
        summoner: &Summoner,
    ) -> Result<GameStateChange, TrackerError> {
        let platform = RiotClient::platform_for_region(&self.default_region);

        // Get current game from Spectator API
        let current_game = self
            .riot_client
            .get_active_game(&summoner.riot_puuid, platform)
            .await?;

        tracing::info!(
            "Current game for {}#{}: {}",
            summoner.game_name,
            summoner.tag_line,
            if current_game.is_some() {
                "In Game"
            } else {
                "Not In Game"
            }
        );

        // Get active games from database
        let db_games = self
            .repository
            .get_active_games_for_summoner(summoner.id)
            .await?;

        match (current_game, db_games.first()) {
            // Started a new game (not in DB yet)
            (Some(game_info), None) => Ok(GameStateChange::GameStarted(game_info)),
            // Started a different game (different game_id)
            (Some(game_info), Some(db_game)) if game_info.game_id != db_game.game_id => {
                // Old game ended, new game started
                // Return GameEnded first, next poll will catch the new game
                Ok(GameStateChange::GameEnded {
                    game_id: db_game.game_id,
                })
            }
            // Game ended (was in DB, no longer in Spectator)
            (None, Some(db_game)) => Ok(GameStateChange::GameEnded {
                game_id: db_game.game_id,
            }),
            // Still in same game or still not in game
            _ => Ok(GameStateChange::NoChange),
        }
    }

    /// Handle game started: insert into active_games table
    pub async fn handle_game_started(
        &self,
        summoner: &Summoner,
        game_info: &ActiveGameInfo,
    ) -> Result<(), TrackerError> {
        let new_game = NewActiveGame {
            summoner_id: summoner.id,
            game_id: game_info.game_id,
            champion_id: game_info.champion_id,
            game_mode: game_info.game_mode.clone(),
            game_start_time: game_info.game_start_time,
        };

        self.repository.insert_active_game(&new_game).await?;
        Ok(())
    }

    /// Handle game ended: delete from active_games, try to fetch match result
    pub async fn handle_game_ended(
        &self,
        summoner: &Summoner,
        game_id: i64,
    ) -> Result<Option<MatchResult>, TrackerError> {
        // Delete from active games
        self.repository
            .delete_active_game_by_summoner_and_game(summoner.id, game_id)
            .await?;

        // Try to fetch match result with retries
        let result = self.fetch_match_with_retry(summoner, game_id, 6).await?;

        // If we got a result, save it to match_history
        if let Some(ref match_result) = result {
            let new_match = NewMatchResult {
                summoner_id: summoner.id,
                match_id: match_result.match_id.clone(),
                game_id: match_result.game_id,
                win: match_result.win,
                kills: match_result.kills,
                deaths: match_result.deaths,
                assists: match_result.assists,
                champion_id: match_result.champion_id,
                game_duration_secs: match_result.game_duration_secs,
                game_mode: match_result.game_mode.clone(),
                role: match_result.role.clone(),
                finished_at: chrono::Utc::now(),
            };

            // Ignore errors on insert (might be duplicate)
            let _ = self.repository.insert_match_result(&new_match).await;
        }

        Ok(result)
    }

    /// Fetch match result with retries (match data appears delayed)
    async fn fetch_match_with_retry(
        &self,
        summoner: &Summoner,
        game_id: i64,
        max_retries: u32,
    ) -> Result<Option<MatchResult>, TrackerError> {
        let region = RiotClient::regional_for_region(&self.default_region);
        let platform = RiotClient::platform_for_region(&self.default_region);
        let match_id = format!("{}_{}", platform, game_id);

        tracing::info!(
            "Looking up match {} for {}#{}",
            match_id,
            summoner.game_name,
            summoner.tag_line
        );

        for attempt in 0..max_retries {
            if attempt > 0 {
                // Wait 10 seconds between retries
                tokio::time::sleep(Duration::from_secs(10)).await;
            }

            match self
                .riot_client
                .get_match_result(&match_id, &summoner.riot_puuid, region)
                .await?
            {
                Some(result) => return Ok(Some(result)),
                None => {
                    tracing::debug!(
                        "Match {} not yet available (attempt {}/{})",
                        match_id,
                        attempt + 1,
                        max_retries
                    );
                }
            }
        }

        tracing::warn!(
            "Could not find match data for {} after {} retries",
            match_id,
            max_retries
        );
        Ok(None)
    }
}
