use crate::db::models::{ActiveGame, MatchHistory, NewActiveGame, NewMatchResult, Summoner};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn upsert_summoner(
    pool: &PgPool,
    puuid: &str,
    game_name: &str,
    tag_line: &str,
    region: &str,
) -> Result<Summoner, sqlx::Error> {
    sqlx::query_as::<_, Summoner>(
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
    .fetch_one(pool)
    .await
}

pub async fn get_summoner_by_puuid(
    pool: &PgPool,
    puuid: &str,
) -> Result<Option<Summoner>, sqlx::Error> {
    sqlx::query_as::<_, Summoner>("SELECT * FROM summoners WHERE riot_puuid = $1")
        .bind(puuid)
        .fetch_optional(pool)
        .await
}

pub async fn get_all_summoners(pool: &PgPool) -> Result<Vec<Summoner>, sqlx::Error> {
    sqlx::query_as::<_, Summoner>("SELECT * FROM summoners")
        .fetch_all(pool)
        .await
}

pub async fn insert_active_game(
    pool: &PgPool,
    game: &NewActiveGame,
) -> Result<ActiveGame, sqlx::Error> {
    sqlx::query_as::<_, ActiveGame>(
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
    .fetch_one(pool)
    .await
}

pub async fn get_active_game(
    pool: &PgPool,
    summoner_id: Uuid,
    game_id: i64,
) -> Result<Option<ActiveGame>, sqlx::Error> {
    sqlx::query_as::<_, ActiveGame>(
        "SELECT * FROM active_games WHERE summoner_id = $1 AND game_id = $2",
    )
    .bind(summoner_id)
    .bind(game_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_active_games_for_summoner(
    pool: &PgPool,
    summoner_id: Uuid,
) -> Result<Vec<ActiveGame>, sqlx::Error> {
    sqlx::query_as::<_, ActiveGame>("SELECT * FROM active_games WHERE summoner_id = $1")
        .bind(summoner_id)
        .fetch_all(pool)
        .await
}

pub async fn delete_active_game(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_games WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_active_game_by_summoner_and_game(
    pool: &PgPool,
    summoner_id: Uuid,
    game_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM active_games WHERE summoner_id = $1 AND game_id = $2")
        .bind(summoner_id)
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_match_result(
    pool: &PgPool,
    result: &NewMatchResult,
) -> Result<MatchHistory, sqlx::Error> {
    sqlx::query_as::<_, MatchHistory>(
        r#"
        INSERT INTO match_history (summoner_id, match_id, game_id, win, kills, deaths, assists, champion_id, game_duration_secs, game_mode, finished_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (summoner_id, match_id) DO NOTHING
        RETURNING id, summoner_id, match_id, game_id, win, kills, deaths, assists, champion_id, game_duration_secs, game_mode, finished_at, created_at
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
    .bind(result.finished_at)
    .fetch_one(pool)
    .await
}
