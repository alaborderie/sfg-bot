use crate::db::models::{
    ActiveGame, Champion, MatchHistory, NewActiveGame, NewMatchResult, Summoner,
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

#[cfg_attr(feature = "test-mocks", mockall::automock)]
#[async_trait]
pub trait Repository: Send + Sync {
    async fn upsert_summoner(
        &self,
        puuid: &str,
        game_name: &str,
        tag_line: &str,
        region: &str,
    ) -> Result<Summoner, RepositoryError>;

    async fn get_summoner_by_puuid(&self, puuid: &str)
    -> Result<Option<Summoner>, RepositoryError>;

    async fn get_all_summoners(&self) -> Result<Vec<Summoner>, RepositoryError>;

    async fn insert_active_game(&self, game: &NewActiveGame)
    -> Result<ActiveGame, RepositoryError>;

    async fn get_active_game(
        &self,
        summoner_id: Uuid,
        game_id: i64,
    ) -> Result<Option<ActiveGame>, RepositoryError>;

    async fn get_active_games_for_summoner(
        &self,
        summoner_id: Uuid,
    ) -> Result<Vec<ActiveGame>, RepositoryError>;

    async fn delete_active_game(&self, id: Uuid) -> Result<(), RepositoryError>;

    async fn delete_active_game_by_summoner_and_game(
        &self,
        summoner_id: Uuid,
        game_id: i64,
    ) -> Result<(), RepositoryError>;

    async fn insert_match_result(
        &self,
        result: &NewMatchResult,
    ) -> Result<MatchHistory, RepositoryError>;

    async fn upsert_champion(
        &self,
        champion_id: i32,
        champion_name: &str,
    ) -> Result<Champion, RepositoryError>;

    async fn get_champion_by_id(
        &self,
        champion_id: i32,
    ) -> Result<Option<Champion>, RepositoryError>;
}

pub struct PgRepository {
    pool: PgPool,
}

impl PgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Repository for PgRepository {
    async fn upsert_summoner(
        &self,
        puuid: &str,
        game_name: &str,
        tag_line: &str,
        region: &str,
    ) -> Result<Summoner, RepositoryError> {
        let summoner = sqlx::query_as::<_, Summoner>(
            r#"
            INSERT INTO summoners (riot_puuid, game_name, tag_line, region)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (riot_puuid) DO UPDATE SET
                game_name = EXCLUDED.game_name,
                tag_line = EXCLUDED.tag_line,
                updated_at = NOW()
            RETURNING id, riot_puuid, game_name, tag_line, region, created_at, updated_at
            "#,
        )
        .bind(puuid)
        .bind(game_name)
        .bind(tag_line)
        .bind(region)
        .fetch_one(&self.pool)
        .await?;
        Ok(summoner)
    }

    async fn get_summoner_by_puuid(
        &self,
        puuid: &str,
    ) -> Result<Option<Summoner>, RepositoryError> {
        let summoner =
            sqlx::query_as::<_, Summoner>("SELECT * FROM summoners WHERE riot_puuid = $1")
                .bind(puuid)
                .fetch_optional(&self.pool)
                .await?;
        Ok(summoner)
    }

    async fn get_all_summoners(&self) -> Result<Vec<Summoner>, RepositoryError> {
        let summoners = sqlx::query_as::<_, Summoner>("SELECT * FROM summoners")
            .fetch_all(&self.pool)
            .await?;
        Ok(summoners)
    }

    async fn insert_active_game(
        &self,
        game: &NewActiveGame,
    ) -> Result<ActiveGame, RepositoryError> {
        let active_game = sqlx::query_as::<_, ActiveGame>(
            r#"
            INSERT INTO active_games (summoner_id, game_id, champion_id, game_mode, game_start_time)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, summoner_id, game_id, champion_id, game_mode, game_start_time, created_at
            "#,
        )
        .bind(game.summoner_id)
        .bind(game.game_id)
        .bind(game.champion_id)
        .bind(&game.game_mode)
        .bind(game.game_start_time)
        .fetch_one(&self.pool)
        .await?;
        Ok(active_game)
    }

    async fn get_active_game(
        &self,
        summoner_id: Uuid,
        game_id: i64,
    ) -> Result<Option<ActiveGame>, RepositoryError> {
        let game = sqlx::query_as::<_, ActiveGame>(
            "SELECT * FROM active_games WHERE summoner_id = $1 AND game_id = $2",
        )
        .bind(summoner_id)
        .bind(game_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(game)
    }

    async fn get_active_games_for_summoner(
        &self,
        summoner_id: Uuid,
    ) -> Result<Vec<ActiveGame>, RepositoryError> {
        let games =
            sqlx::query_as::<_, ActiveGame>("SELECT * FROM active_games WHERE summoner_id = $1")
                .bind(summoner_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(games)
    }

    async fn delete_active_game(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM active_games WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_active_game_by_summoner_and_game(
        &self,
        summoner_id: Uuid,
        game_id: i64,
    ) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM active_games WHERE summoner_id = $1 AND game_id = $2")
            .bind(summoner_id)
            .bind(game_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn insert_match_result(
        &self,
        result: &NewMatchResult,
    ) -> Result<MatchHistory, RepositoryError> {
        let match_history = sqlx::query_as::<_, MatchHistory>(
            r#"
            INSERT INTO match_history (summoner_id, match_id, game_id, win, kills, deaths, assists, champion_id, game_duration_secs, game_mode, role, finished_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (summoner_id, match_id) DO NOTHING
            RETURNING id, summoner_id, match_id, game_id, win, kills, deaths, assists, champion_id, game_duration_secs, game_mode, role, finished_at, created_at
            "#,
        )
        .bind(result.summoner_id)
        .bind(&result.match_id)
        .bind(result.game_id)
        .bind(result.win)
        .bind(result.kills)
        .bind(result.deaths)
        .bind(result.assists)
        .bind(result.champion_id)
        .bind(result.game_duration_secs)
        .bind(&result.game_mode)
        .bind(&result.role)
        .bind(result.finished_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(match_history)
    }

    async fn upsert_champion(
        &self,
        champion_id: i32,
        champion_name: &str,
    ) -> Result<Champion, RepositoryError> {
        let champion = sqlx::query_as::<_, Champion>(
            r#"
            INSERT INTO champions (champion_id, champion_name)
            VALUES ($1, $2)
            ON CONFLICT (champion_id) DO UPDATE SET
                champion_name = EXCLUDED.champion_name
            RETURNING id, champion_id, champion_name, created_at
            "#,
        )
        .bind(champion_id)
        .bind(champion_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(champion)
    }

    async fn get_champion_by_id(
        &self,
        champion_id: i32,
    ) -> Result<Option<Champion>, RepositoryError> {
        let champion =
            sqlx::query_as::<_, Champion>("SELECT * FROM champions WHERE champion_id = $1")
                .bind(champion_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(champion)
    }
}
