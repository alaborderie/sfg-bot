use std::sync::Arc;

use serde_json::{Value, json};
use serenity::http::HttpBuilder;
use serenity::model::application::CommandInteraction;
use serenity::model::id::ApplicationId;

use sfg_bot::db;
use sfg_bot::db::repository::{PgRepository, Repository};
use sfg_bot::discord::commands;

use crate::discord_standin::{self, DiscordStandIn};
use crate::error::{VerifyResult, err};
use crate::fixture::FixtureRiot;
use crate::instance::write_evidence;
use crate::net::non_loopback_sockets;
use crate::state::InstanceState;

#[derive(Debug, Clone, Copy)]
pub enum ShowWhat {
    Summoners,
    BotConfig,
}

pub async fn show(state: &InstanceState, what: ShowWhat) -> VerifyResult<Value> {
    let repository = repository(state).await?;
    let report = match what {
        ShowWhat::Summoners => json!({
            "ok": true,
            "summoners": summoner_rows(repository.as_ref()).await?,
        }),
        ShowWhat::BotConfig => json!({
            "ok": true,
            "bot_configs": config_rows(repository.as_ref()).await?,
        }),
    };
    let name = match what {
        ShowWhat::Summoners => "show-summoners",
        ShowWhat::BotConfig => "show-bot-config",
    };
    let path = write_evidence(&state.evidence_dir, name, &report)?;
    let mut report = report;
    report["evidence_path"] = json!(path);
    Ok(report)
}

pub struct SlashRequest {
    pub command: &'static str,
    pub summoner_name: Option<String>,
    pub guild_id: Option<u64>,
    pub channel_id: u64,
    pub dry_run: bool,
}

pub async fn slash(state: &InstanceState, request: SlashRequest) -> VerifyResult<Value> {
    if request.dry_run {
        return remove_dry_run(state, request.summoner_name.as_deref()).await;
    }

    let repository = repository(state).await?;
    let riot: Arc<dyn sfg_bot::RiotApiClient> = Arc::new(FixtureRiot::load(&state.fixture_path)?);
    let standin = DiscordStandIn::start().await?;
    let client = local_discord_client()?;
    let http = HttpBuilder::new(discord_standin::BOT_TOKEN)
        .client(client)
        .proxy(standin.base_url.clone())
        .ratelimiter_disabled(true)
        .application_id(ApplicationId::new(discord_standin::APP_ID))
        .build();

    let interaction_value = discord_standin::interaction_json(
        request.command,
        request.summoner_name.as_deref(),
        request.guild_id,
        request.channel_id,
    );
    let interaction: CommandInteraction = serde_json::from_value(interaction_value.clone())
        .map_err(|error| {
            err(format!(
                "could not build a CommandInteraction for /{}: {error}",
                request.command
            ))
        })?;

    let repo: Arc<dyn Repository> = repository.clone();
    dispatch(&http, &interaction, &repo, &riot, state, request.command).await;

    let captured = standin.captured().await;
    let discord: Vec<Value> = captured
        .iter()
        .map(|item| {
            json!({
                "method": item.method,
                "path": item.path,
                "body": item.body,
                "visible_text": discord_standin::visible_text(&item.body),
            })
        })
        .collect();
    let visible_text: Vec<String> = discord
        .iter()
        .filter_map(|item| item.get("visible_text"))
        .filter_map(Value::as_array)
        .flat_map(|items| items.iter().filter_map(Value::as_str).map(str::to_string))
        .collect();
    let sockets = non_loopback_sockets()?;
    let mut problems = Vec::new();
    if discord.is_empty() {
        problems.push(
            "the slash handler returned without posting to the local Discord stand-in".to_string(),
        );
    }
    if !sockets.is_empty() {
        problems.push(format!(
            "this process opened a non-loopback socket during the drive: {sockets:?}"
        ));
    }
    let report = json!({
        "ok": problems.is_empty(),
        "command": request.command,
        "summoner_name": request.summoner_name,
        "guild_id": request.guild_id.map(|id| id.to_string()),
        "channel_id": request.channel_id.to_string(),
        "discord_standin": standin.base_url,
        "discord": discord,
        "visible_text": visible_text,
        "summoners": summoner_rows(repo.as_ref()).await?,
        "bot_configs": config_rows(repo.as_ref()).await?,
        "non_loopback_sockets": sockets,
        "problems": problems,
        "riot_transport": "fixture",
        "discord_transport": "127.0.0.1",
    });
    let path = write_evidence(&state.evidence_dir, request.command, &report)?;
    let mut report = report;
    report["evidence_path"] = json!(path);
    if !problems.is_empty() {
        eprintln!("{}", problems.join("; "));
    }
    Ok(report)
}

async fn dispatch(
    http: &serenity::http::Http,
    interaction: &CommandInteraction,
    repository: &Arc<dyn Repository>,
    riot: &Arc<dyn sfg_bot::RiotApiClient>,
    state: &InstanceState,
    command: &str,
) {
    let pipeline = None;
    match command {
        "init-sfg-bot" => {
            commands::run_init_sfg_bot(http, interaction, repository).await;
        }
        "list-summoners" => {
            commands::run_list_summoners(http, interaction, repository).await;
        }
        "add-summoner" => {
            commands::run_add_summoner(http, interaction, repository, riot, &state.default_region)
                .await;
        }
        "remove-summoner" => {
            commands::run_remove_summoner(http, interaction, repository).await;
        }
        "analyze-last-game" => {
            commands::run(
                http,
                interaction,
                riot,
                repository,
                &pipeline,
                &state.default_region,
            )
            .await;
        }
        _ => {}
    }
}

async fn remove_dry_run(state: &InstanceState, summoner_name: Option<&str>) -> VerifyResult<Value> {
    let Some(summoner_name) = summoner_name else {
        return Err(err(
            "remove-summoner --dry-run needs --summoner-name Name#Tag. Nothing was deleted.",
        ));
    };
    let repository = repository(state).await?;
    let rows = summoner_rows(repository.as_ref()).await?;
    let parsed = split_riot_id(summoner_name);
    let would_delete = match &parsed {
        Some((game_name, tag_line)) => rows.iter().any(|row| {
            row.get("game_name")
                .and_then(Value::as_str)
                .is_some_and(|value| value.eq_ignore_ascii_case(game_name))
                && row
                    .get("tag_line")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case(tag_line))
        }),
        None => false,
    };
    let report = json!({
        "ok": true,
        "dry_run": true,
        "command": "remove-summoner",
        "summoner_name": summoner_name,
        "parsed": parsed,
        "would_delete": would_delete,
        "summoners": rows,
        "note": "dry-run did not call run_remove_summoner and did not post to Discord",
    });
    let path = write_evidence(&state.evidence_dir, "remove-summoner-dry-run", &report)?;
    let mut report = report;
    report["evidence_path"] = json!(path);
    Ok(report)
}

fn split_riot_id(input: &str) -> Option<(String, String)> {
    let hash = input.rfind('#')?;
    let game_name = input[..hash].trim();
    let tag_line = input[hash + 1..].trim();
    if game_name.is_empty() || tag_line.is_empty() {
        None
    } else {
        Some((game_name.to_string(), tag_line.to_string()))
    }
}

async fn repository(state: &InstanceState) -> VerifyResult<Arc<PgRepository>> {
    let pool = db::create_pool(&state.database_url)
        .await
        .map_err(|error| {
            err(format!(
                "could not connect to the verification database ({}): {error}. Run doctor.",
                crate::state::redact_database_url(&state.database_url)
            ))
        })?;
    Ok(Arc::new(PgRepository::new(pool)))
}

fn local_discord_client() -> VerifyResult<reqwest12::Client> {
    reqwest12::Client::builder()
        .no_proxy()
        .build()
        .map_err(|error| {
            err(format!(
                "failed to build the local Discord HTTP client: {error}"
            ))
        })
}

async fn summoner_rows(repository: &dyn Repository) -> VerifyResult<Vec<Value>> {
    let summoners = repository
        .get_all_summoners()
        .await
        .map_err(|error| err(format!("reading summoners failed: {error}")))?;
    Ok(summoners
        .into_iter()
        .map(|summoner| {
            json!({
                "id": summoner.id.to_string(),
                "riot_puuid": summoner.riot_puuid,
                "game_name": summoner.game_name,
                "tag_line": summoner.tag_line,
                "region": summoner.region,
            })
        })
        .collect())
}

async fn config_rows(repository: &dyn Repository) -> VerifyResult<Vec<Value>> {
    let configs = repository
        .get_all_bot_configs()
        .await
        .map_err(|error| err(format!("reading bot_config failed: {error}")))?;
    Ok(configs
        .into_iter()
        .map(|config| {
            json!({
                "guild_id": config.guild_id.to_string(),
                "channel_id": config.channel_id.to_string(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use sfg_bot::db::models::{
        ActiveGame, AnalysisHistoryEntry, BotConfig, Champion, MatchHistory, NewActiveGame,
        NewAnalysisHistory, NewMatchResult, NewNotificationEvent, NotificationEvent, Summoner,
    };
    use sfg_bot::db::repository::RepositoryError;
    use uuid::Uuid;

    struct EmptyRepo;

    fn stub_err() -> RepositoryError {
        RepositoryError::Database(sqlx::Error::Protocol("sfg-verify test stub".into()))
    }

    #[async_trait]
    impl Repository for EmptyRepo {
        async fn upsert_summoner(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Summoner, RepositoryError> {
            Err(stub_err())
        }
        async fn get_summoner_by_puuid(
            &self,
            _: &str,
        ) -> Result<Option<Summoner>, RepositoryError> {
            Err(stub_err())
        }
        async fn get_all_summoners(&self) -> Result<Vec<Summoner>, RepositoryError> {
            Ok(Vec::new())
        }
        async fn insert_active_game(
            &self,
            _: &NewActiveGame,
        ) -> Result<ActiveGame, RepositoryError> {
            Err(stub_err())
        }
        async fn get_active_game(
            &self,
            _: Uuid,
            _: i64,
        ) -> Result<Option<ActiveGame>, RepositoryError> {
            Err(stub_err())
        }
        async fn get_active_games_for_summoner(
            &self,
            _: Uuid,
        ) -> Result<Vec<ActiveGame>, RepositoryError> {
            Err(stub_err())
        }
        async fn delete_active_game(&self, _: Uuid) -> Result<(), RepositoryError> {
            Err(stub_err())
        }
        async fn delete_active_game_by_summoner_and_game(
            &self,
            _: Uuid,
            _: i64,
        ) -> Result<(), RepositoryError> {
            Err(stub_err())
        }
        async fn increment_active_game_end_retry(
            &self,
            _: Uuid,
            _: i64,
        ) -> Result<Option<i32>, RepositoryError> {
            Err(stub_err())
        }
        async fn insert_match_result(
            &self,
            _: &NewMatchResult,
        ) -> Result<MatchHistory, RepositoryError> {
            Err(stub_err())
        }
        async fn get_match_history_by_match_id(
            &self,
            _: Uuid,
            _: &str,
        ) -> Result<Option<MatchHistory>, RepositoryError> {
            Err(stub_err())
        }
        async fn upsert_champion(&self, _: i32, _: &str) -> Result<Champion, RepositoryError> {
            Err(stub_err())
        }
        async fn get_champion_by_id(&self, _: i32) -> Result<Option<Champion>, RepositoryError> {
            Err(stub_err())
        }
        async fn insert_notification_event(
            &self,
            _: &NewNotificationEvent,
        ) -> Result<NotificationEvent, RepositoryError> {
            Err(stub_err())
        }
        async fn get_pending_notification_events(
            &self,
        ) -> Result<Vec<NotificationEvent>, RepositoryError> {
            Err(stub_err())
        }
        async fn mark_notifications_processed(&self, _: &[Uuid]) -> Result<(), RepositoryError> {
            Err(stub_err())
        }
        async fn increment_notification_retry_count(
            &self,
            _: &[Uuid],
            _: &str,
        ) -> Result<(), RepositoryError> {
            Err(stub_err())
        }
        async fn mark_notifications_failed(
            &self,
            _: &[Uuid],
            _: &str,
        ) -> Result<(), RepositoryError> {
            Err(stub_err())
        }
        async fn upsert_bot_config(&self, _: i64, _: i64) -> Result<BotConfig, RepositoryError> {
            Err(stub_err())
        }
        async fn get_bot_config(&self, _: i64) -> Result<Option<BotConfig>, RepositoryError> {
            Err(stub_err())
        }
        async fn get_all_bot_configs(&self) -> Result<Vec<BotConfig>, RepositoryError> {
            Err(stub_err())
        }
        async fn insert_analysis_history(
            &self,
            _: &NewAnalysisHistory,
        ) -> Result<(), RepositoryError> {
            Err(stub_err())
        }
        async fn get_recent_analysis_history(
            &self,
            _: &str,
            _: i64,
        ) -> Result<Vec<AnalysisHistoryEntry>, RepositoryError> {
            Err(stub_err())
        }
        async fn delete_summoner_by_name_and_tag(
            &self,
            _: &str,
            _: &str,
        ) -> Result<bool, RepositoryError> {
            Err(stub_err())
        }
    }

    #[tokio::test]
    async fn list_summoners_posts_the_empty_roster_to_the_stand_in() {
        let standin = DiscordStandIn::start().await.expect("stand-in");
        let client = local_discord_client().expect("client");
        let http = HttpBuilder::new(discord_standin::BOT_TOKEN)
            .client(client)
            .proxy(standin.base_url.clone())
            .ratelimiter_disabled(true)
            .application_id(ApplicationId::new(discord_standin::APP_ID))
            .build();
        let interaction: CommandInteraction =
            serde_json::from_value(discord_standin::interaction_json(
                "list-summoners",
                None,
                Some(discord_standin::GUILD_ID),
                discord_standin::CHANNEL_ID,
            ))
            .expect("interaction");
        let repository: Arc<dyn Repository> = Arc::new(EmptyRepo);
        commands::run_list_summoners(&http, &interaction, &repository).await;
        let captured = standin.captured().await;
        assert_eq!(captured.len(), 1);
        let text = discord_standin::visible_text(&captured[0].body);
        assert!(
            text.iter()
                .any(|line| line.contains("Aucun invocateur suivi")),
            "visible text: {text:?} body: {}",
            captured[0].body
        );
        let sockets = non_loopback_sockets().expect("sockets");
        assert!(sockets.is_empty(), "non-loopback sockets: {sockets:?}");
    }
}
