use crate::analysis::discord::{format_analysis_embed, format_analysis_error_embed};
use crate::analysis::llm::LlmClient;
use crate::analysis::models::AnalysisResult;
use crate::analysis::pipeline::AnalysisPipeline;
use crate::config::Config;
use crate::db::models::{NewNotificationEvent, Summoner};
use crate::db::repository::Repository;
use crate::discord::commands;
use crate::discord::messages::format_mention_response;
use crate::notification::NotificationProcessor;
use crate::notification::messages::format_report_unavailable;
use crate::riot::client::RiotApiClient;
use crate::riot::client::RiotClient;
use crate::riot::models::{GameStateChange, MatchLookup};
use crate::riot::tracker::{GameTracker, MAX_END_RETRY_CYCLES};
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub struct Bot {
    pub repository: Arc<dyn Repository>,
    pub riot_client: Arc<dyn RiotApiClient>,
    pub config: Config,
    pub analysis_pipeline: Option<Arc<AnalysisPipeline>>,
    /// Guards background-task startup so gateway reconnects (which re-fire
    /// `cache_ready`) don't spawn duplicate polling loops and notification
    /// processors.
    background_tasks_started: AtomicBool,
}

impl Bot {
    pub fn new(
        repository: Arc<dyn Repository>,
        riot_client: Arc<dyn RiotApiClient>,
        config: Config,
    ) -> Self {
        let analysis_pipeline = match config.llm_api_key.as_ref() {
            Some(api_key) => {
                let llm_client = match LlmClient::new(
                    api_key.clone(),
                    config.llm_base_url.clone(),
                    config.llm_model.clone(),
                ) {
                    Ok(client) => client,
                    Err(error) => {
                        tracing::warn!(error = %error, "Failed to initialize LLM client");
                        return Self {
                            repository,
                            riot_client,
                            config,
                            analysis_pipeline: None,
                            background_tasks_started: AtomicBool::new(false),
                        };
                    }
                };

                match AnalysisPipeline::new(llm_client, &config.analysis_prompts_dir) {
                    Ok(pipeline) => Some(Arc::new(pipeline)),
                    Err(error) => {
                        tracing::warn!(error = %error, "Failed to load analysis prompt");
                        None
                    }
                }
            }
            None => {
                tracing::info!("LLM API key not configured, analysis disabled");
                None
            }
        };

        Self {
            repository,
            riot_client,
            config,
            analysis_pipeline,
            background_tasks_started: AtomicBool::new(false),
        }
    }

    /// Returns `true` the first time it is called and `false` on every
    /// subsequent call. Used to ensure background tasks (per-summoner polling
    /// loops and the notification processor) are spawned only once, even if
    /// the serenity gateway re-fires `cache_ready` after a reconnect.
    fn try_claim_background_tasks(&self) -> bool {
        self.background_tasks_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("Bot ready as {}", ready.user.name);

        match serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::register_all(),
        )
        .await
        {
            Ok(cmds) => {
                tracing::info!("Registered {} global slash command(s)", cmds.len());
            }
            Err(e) => {
                tracing::error!("Failed to register global slash commands: {}", e);
            }
        }
    }

    async fn cache_ready(&self, ctx: Context, _guilds: Vec<serenity::model::id::GuildId>) {
        if !self.try_claim_background_tasks() {
            tracing::warn!(
                "cache_ready fired again (likely a gateway reconnect); skipping background task startup to avoid duplicate polling loops"
            );
            return;
        }

        tracing::info!("Cache ready, starting background tasks");

        let repository = self.repository.clone();
        let riot_client = self.riot_client.clone();
        let config = self.config.clone();
        let analysis_pipeline = self.analysis_pipeline.clone();
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            start_polling_task(
                ctx_clone,
                repository,
                riot_client,
                config,
                analysis_pipeline,
            )
            .await;
        });

        let repository = self.repository.clone();
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            let processor = NotificationProcessor::new(repository, ctx_clone, 5);
            processor.start().await;
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "analyze-last-game" => {
                    commands::run(
                        &ctx,
                        &command,
                        &self.riot_client,
                        &self.repository,
                        &self.analysis_pipeline,
                        &self.config.default_region,
                    )
                    .await;
                }
                "init-sfg-bot" => {
                    commands::run_init_sfg_bot(&ctx, &command, &self.repository).await;
                }
                "list-summoners" => {
                    commands::run_list_summoners(&ctx, &command, &self.repository).await;
                }
                "add-summoner" => {
                    commands::run_add_summoner(
                        &ctx,
                        &command,
                        &self.repository,
                        &self.riot_client,
                        &self.config.default_region,
                    )
                    .await;
                }
                "remove-summoner" => {
                    commands::run_remove_summoner(&ctx, &command, &self.repository).await;
                }
                _ => {
                    tracing::warn!("Unknown slash command: {}", command.data.name);
                }
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.id == self.config.discord_bot_id {
            return;
        }

        let bot_mentioned = msg
            .mentions
            .iter()
            .any(|u| u.id == self.config.discord_bot_id);

        if bot_mentioned {
            let response = format_mention_response();
            if let Err(e) = msg.channel_id.say(&ctx.http, &response).await {
                tracing::error!("Failed to send mention response: {}", e);
            }
        }
    }
}

async fn start_polling_task(
    ctx: Context,
    repository: Arc<dyn Repository>,
    riot_client: Arc<dyn RiotApiClient>,
    config: Config,
    analysis_pipeline: Option<Arc<AnalysisPipeline>>,
) {
    let interval_secs = config.polling_interval_secs;

    let summoners = match repository.get_all_summoners().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to get summoners: {}", e);
            return;
        }
    };

    tracing::info!("Starting {} independent polling tasks", summoners.len());

    let mut handles = Vec::new();

    for summoner in summoners {
        let ctx = ctx.clone();
        let riot_client = riot_client.clone();
        let repository = repository.clone();
        let analysis_pipeline = analysis_pipeline.clone();
        let region = config.default_region.clone();

        let handle = tokio::spawn(async move {
            let tracker = GameTracker::new(riot_client, repository, region);

            tracing::info!(
                "Polling task started for {}#{}",
                summoner.game_name,
                summoner.tag_line
            );

            loop {
                if let Err(e) =
                    check_and_notify(&ctx, &tracker, &summoner, analysis_pipeline.clone()).await
                {
                    tracing::error!(
                        "Error checking summoner {}#{}: {}",
                        summoner.game_name,
                        summoner.tag_line,
                        e
                    );
                }

                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks (they run indefinitely, so this blocks forever)
    for handle in handles {
        match handle.await {
            Ok(never) => match never {},
            Err(e) => tracing::error!("Polling task panicked: {}", e),
        }
    }
}

async fn check_and_notify<R: RiotApiClient + ?Sized + 'static, D: Repository + ?Sized + 'static>(
    ctx: &Context,
    tracker: &GameTracker<R, D>,
    summoner: &Summoner,
    analysis_pipeline: Option<Arc<AnalysisPipeline>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state_change = tracker.check_summoner_game_state(summoner).await?;

    match state_change {
        GameStateChange::GameStarted(game_info) => {
            tracing::info!(
                "Game started for {}#{}: game_id={}",
                summoner.game_name,
                summoner.tag_line,
                game_info.game_id
            );

            tracker.handle_game_started(summoner, &game_info).await?;

            let champion_name = tracker
                .repository
                .get_champion_by_id(game_info.champion_id)
                .await?
                .map(|c| c.champion_name)
                .unwrap_or_else(|| format!("Champion #{}", game_info.champion_id));

            let event = NewNotificationEvent {
                summoner_id: summoner.id,
                event_type: "GAME_STARTED".to_string(),
                game_id: game_info.game_id,
                match_id: None,
                champion_id: game_info.champion_id,
                champion_name,
                role: None,
                win: None,
                kills: None,
                deaths: None,
                assists: None,
                game_duration_secs: None,
                game_mode: game_info.game_mode,
                queue_id: game_info.queue_id,
                is_featured_mode: false,
                total_cs: None,
                total_gold: None,
                total_damage: None,
                enemy_champion_name: None,
                enemy_cs: None,
                enemy_gold: None,
                enemy_damage: None,
                role_gaps: None,
            };

            tracker.repository.insert_notification_event(&event).await?;
        }
        GameStateChange::GameEnded { game_id } => {
            tracing::info!(
                "Game ended for {}#{}: game_id={}",
                summoner.game_name,
                summoner.tag_line,
                game_id
            );

            let summoner_clone = summoner.clone();
            let tracker_result = tracker.handle_game_ended(summoner, game_id).await;

            match tracker_result {
                Ok(MatchLookup::Found(match_result)) => {
                    let match_result = *match_result;
                    let analyzable = crate::analysis::is_analyzable_mode(&match_result.game_mode);
                    let champion_name = tracker
                        .repository
                        .get_champion_by_id(match_result.champion_id)
                        .await?
                        .map(|c| c.champion_name)
                        .unwrap_or_else(|| format!("Champion #{}", match_result.champion_id));

                    let match_id = match_result.match_id.clone();
                    let event = NewNotificationEvent {
                        summoner_id: summoner_clone.id,
                        event_type: "GAME_ENDED".to_string(),
                        game_id: match_result.game_id,
                        match_id: Some(match_id.clone()),
                        champion_id: match_result.champion_id,
                        champion_name,
                        role: Some(match_result.role),
                        win: Some(match_result.win),
                        kills: Some(match_result.kills),
                        deaths: Some(match_result.deaths),
                        assists: Some(match_result.assists),
                        game_duration_secs: Some(match_result.game_duration_secs),
                        game_mode: match_result.game_mode,
                        queue_id: match_result.queue_id,
                        is_featured_mode: false,
                        total_cs: Some(match_result.total_cs),
                        total_gold: Some(match_result.total_gold),
                        total_damage: Some(match_result.total_damage),
                        enemy_champion_name: match_result.enemy_champion_name.clone(),
                        enemy_cs: match_result.enemy_cs,
                        enemy_gold: match_result.enemy_gold,
                        enemy_damage: match_result.enemy_damage,
                        role_gaps: match_result.role_gaps.clone(),
                    };

                    tracker.repository.insert_notification_event(&event).await?;

                    if analyzable {
                        spawn_analysis_task(
                            ctx,
                            tracker,
                            &summoner_clone,
                            &match_id,
                            analysis_pipeline.clone(),
                        );
                    } else {
                        tracing::info!(
                            "Skipping analysis for {}#{} match {}: game mode not analyzable",
                            summoner_clone.game_name,
                            summoner_clone.tag_line,
                            match_id
                        );
                    }
                }
                Ok(MatchLookup::Pending { attempts }) => {
                    tracing::info!(
                        "Match data for {}#{} game {} not yet available (cycle {}/{}); will retry next poll",
                        summoner_clone.game_name,
                        summoner_clone.tag_line,
                        game_id,
                        attempts,
                        MAX_END_RETRY_CYCLES
                    );
                }
                Ok(MatchLookup::GaveUp { attempts }) => {
                    tracing::warn!(
                        "Giving up on match data for {}#{} game {} after {} cycles",
                        summoner_clone.game_name,
                        summoner_clone.tag_line,
                        game_id,
                        attempts
                    );
                    send_report_unavailable(ctx, tracker, &summoner_clone, game_id).await;
                }
                Err(e) => {
                    tracing::error!(
                        "Error handling game end for {}#{}: {}",
                        summoner_clone.game_name,
                        summoner_clone.tag_line,
                        e
                    );
                }
            }
        }
        GameStateChange::FeaturedModeGameEnded { game_id } => {
            tracing::info!(
                "Featured mode game ended for {}#{}: game_id={}",
                summoner.game_name,
                summoner.tag_line,
                game_id
            );

            let summoner_clone = summoner.clone();
            let tracker_result = tracker.handle_game_ended(summoner, game_id).await;

            match tracker_result {
                Ok(MatchLookup::Found(match_result)) => {
                    let match_result = *match_result;
                    let analyzable = crate::analysis::is_analyzable_mode(&match_result.game_mode);
                    let champion_name = tracker
                        .repository
                        .get_champion_by_id(match_result.champion_id)
                        .await?
                        .map(|c| c.champion_name)
                        .unwrap_or_else(|| format!("Champion #{}", match_result.champion_id));

                    let match_id = match_result.match_id.clone();
                    let event = NewNotificationEvent {
                        summoner_id: summoner_clone.id,
                        event_type: "GAME_ENDED".to_string(),
                        game_id: match_result.game_id,
                        match_id: Some(match_id.clone()),
                        champion_id: match_result.champion_id,
                        champion_name,
                        role: Some(match_result.role),
                        win: Some(match_result.win),
                        kills: Some(match_result.kills),
                        deaths: Some(match_result.deaths),
                        assists: Some(match_result.assists),
                        game_duration_secs: Some(match_result.game_duration_secs),
                        game_mode: match_result.game_mode,
                        queue_id: match_result.queue_id,
                        is_featured_mode: true,
                        total_cs: Some(match_result.total_cs),
                        total_gold: Some(match_result.total_gold),
                        total_damage: Some(match_result.total_damage),
                        enemy_champion_name: match_result.enemy_champion_name.clone(),
                        enemy_cs: match_result.enemy_cs,
                        enemy_gold: match_result.enemy_gold,
                        enemy_damage: match_result.enemy_damage,
                        role_gaps: match_result.role_gaps.clone(),
                    };

                    tracker.repository.insert_notification_event(&event).await?;

                    if analyzable {
                        spawn_analysis_task(
                            ctx,
                            tracker,
                            &summoner_clone,
                            &match_id,
                            analysis_pipeline.clone(),
                        );
                    } else {
                        tracing::info!(
                            "Skipping analysis for {}#{} match {}: game mode not analyzable",
                            summoner_clone.game_name,
                            summoner_clone.tag_line,
                            match_id
                        );
                    }
                }
                Ok(MatchLookup::Pending { .. }) => {
                    tracing::info!(
                        "Match data for {}#{} featured game {} not yet available; will retry next poll",
                        summoner_clone.game_name,
                        summoner_clone.tag_line,
                        game_id
                    );
                }
                Ok(MatchLookup::GaveUp { attempts }) => {
                    tracing::warn!(
                        "Giving up on match data for {}#{} featured game {} after {} cycles",
                        summoner_clone.game_name,
                        summoner_clone.tag_line,
                        game_id,
                        attempts
                    );
                    send_report_unavailable(ctx, tracker, &summoner_clone, game_id).await;
                }
                Err(e) => {
                    tracing::error!(
                        "Error handling featured mode game end for {}#{}: {}",
                        summoner_clone.game_name,
                        summoner_clone.tag_line,
                        e
                    );
                }
            }
        }
        GameStateChange::NoChange => {}
    }

    Ok(())
}

/// Tells the notification channel that a game's recap will never arrive.
/// Best-effort: a failure here is only logged, the game is already dropped.
async fn send_report_unavailable<R: RiotApiClient + ?Sized, D: Repository + ?Sized>(
    ctx: &Context,
    tracker: &GameTracker<R, D>,
    summoner: &Summoner,
    game_id: i64,
) {
    let channel_id = match tracker.repository.get_all_bot_configs().await {
        Ok(configs) => match configs.first() {
            Some(c) => ChannelId::new(c.channel_id as u64),
            None => return,
        },
        Err(e) => {
            tracing::error!(
                "Failed to fetch bot config for report-unavailable notice: {}",
                e
            );
            return;
        }
    };

    let summoner_name = format!("{}#{}", summoner.game_name, summoner.tag_line);
    let embed = format_report_unavailable(&summoner_name, game_id);
    if let Err(error) = channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        tracing::error!(
            summoner = summoner_name.as_str(),
            error = %error,
            "Failed to send report-unavailable notice"
        );
    }
}

fn spawn_analysis_task<R: RiotApiClient + ?Sized + 'static, D: Repository + ?Sized + 'static>(
    ctx: &Context,
    tracker: &GameTracker<R, D>,
    summoner: &Summoner,
    match_id: &str,
    analysis_pipeline: Option<Arc<AnalysisPipeline>>,
) {
    let Some(analysis_pipeline) = analysis_pipeline else {
        return;
    };

    let ctx = ctx.clone();
    let riot_client = tracker.riot_client();
    let repository = tracker.repository.clone();
    let summoner_clone = summoner.clone();
    let match_id = match_id.to_string();
    let region = RiotClient::regional_for_region(tracker.default_region());
    let analysis_pipeline = analysis_pipeline.clone();

    tokio::spawn(async move {
        let summoner_name = format!("{}#{}", summoner_clone.game_name, summoner_clone.tag_line);

        let channel_id = match repository.get_all_bot_configs().await {
            Ok(configs) => match configs.first() {
                Some(c) => ChannelId::new(c.channel_id as u64),
                None => {
                    tracing::warn!(
                        "No notification channel configured, skipping analysis for {}",
                        summoner_name
                    );
                    return;
                }
            },
            Err(e) => {
                tracing::error!("Failed to fetch bot config for analysis: {}", e);
                return;
            }
        };

        let analysis_data = riot_client
            .get_match_analysis_data(
                &match_id,
                &summoner_clone.riot_puuid,
                &summoner_name,
                region,
            )
            .await;

        let result = match analysis_data {
            Ok(Some(data)) => {
                crate::analysis::history::analyze_with_memory(
                    repository.as_ref(),
                    &analysis_pipeline,
                    data,
                    &summoner_clone.riot_puuid,
                    &match_id,
                )
                .await
            }
            Ok(None) => AnalysisResult {
                summoner_name: summoner_name.clone(),
                champion_name: "Unknown".to_string(),
                overall_rating: None,
                summary: "Analysis unavailable: match data not found".to_string(),
                error: Some("match data not found".to_string()),
            },
            Err(error) => {
                tracing::warn!(
                    summoner = summoner_name.as_str(),
                    error = %error,
                    "Failed to fetch match analysis data"
                );
                AnalysisResult {
                    summoner_name: summoner_name.clone(),
                    champion_name: "Unknown".to_string(),
                    overall_rating: None,
                    summary: "Analysis unavailable: could not retrieve match data".to_string(),
                    error: Some("could not retrieve match data".to_string()),
                }
            }
        };

        let embed = if result.error.is_some() {
            format_analysis_error_embed(&summoner_name, result.error.as_deref().unwrap_or(""))
        } else {
            format_analysis_embed(&result)
        };

        if let Err(error) = channel_id
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await
        {
            tracing::error!(
                summoner = summoner_name.as_str(),
                error = %error,
                "Failed to send analysis message"
            );
        }
    });
}

#[cfg(all(test, feature = "test-mocks"))]
mod tests {
    use super::*;
    use crate::db::repository::MockRepository;
    use crate::riot::client::MockRiotApiClient;

    fn test_config() -> Config {
        Config {
            riot_api_key: "test-riot".to_string(),
            discord_bot_token: "test-token".to_string(),
            discord_bot_id: 0,
            database_url: "postgres://localhost/test".to_string(),
            default_region: "euw1".to_string(),
            polling_interval_secs: 180,
            llm_api_key: None,
            llm_base_url: "http://localhost:8080/v1".to_string(),
            llm_model: "gemma-4-26b".to_string(),
            analysis_prompts_dir: "analysis_prompts".to_string(),
            health_check_port: None,
        }
    }

    fn build_bot() -> Bot {
        let repository: Arc<dyn Repository> = Arc::new(MockRepository::new());
        let riot_client: Arc<dyn RiotApiClient> = Arc::new(MockRiotApiClient::new());
        Bot::new(repository, riot_client, test_config())
    }

    #[test]
    fn try_claim_background_tasks_succeeds_on_first_call() {
        let bot = build_bot();
        assert!(
            bot.try_claim_background_tasks(),
            "first claim should succeed"
        );
    }

    #[test]
    fn try_claim_background_tasks_blocks_subsequent_calls() {
        let bot = build_bot();

        assert!(bot.try_claim_background_tasks(), "first claim should win");

        // Simulates gateway reconnects firing cache_ready again.
        for attempt in 0..5 {
            assert!(
                !bot.try_claim_background_tasks(),
                "claim attempt #{} after the first must be rejected",
                attempt + 2
            );
        }
    }

    #[test]
    fn try_claim_background_tasks_is_thread_safe() {
        use std::sync::Arc;
        use std::thread;

        let bot = Arc::new(build_bot());
        let mut handles = Vec::new();

        for _ in 0..16 {
            let bot = bot.clone();
            handles.push(thread::spawn(move || bot.try_claim_background_tasks()));
        }

        let successes = handles
            .into_iter()
            .map(|h| h.join().expect("thread panicked"))
            .filter(|claimed| *claimed)
            .count();

        assert_eq!(
            successes, 1,
            "exactly one thread should claim the background-task slot"
        );
    }
}
