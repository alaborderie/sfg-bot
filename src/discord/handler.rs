use crate::config::Config;
use crate::db::models::Summoner;
use crate::db::repository::Repository;
use crate::discord::messages::{format_game_ended, format_game_started, format_mention_response};
use crate::riot::client::RiotApiClient;
use crate::riot::models::GameStateChange;
use crate::riot::tracker::GameTracker;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub struct Bot {
    pub repository: Arc<dyn Repository>,
    pub riot_client: Arc<dyn RiotApiClient>,
    pub config: Config,
}

impl Bot {
    pub fn new(
        repository: Arc<dyn Repository>,
        riot_client: Arc<dyn RiotApiClient>,
        config: Config,
    ) -> Self {
        Self {
            repository,
            riot_client,
            config,
        }
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        tracing::info!("Bot ready as {}", ready.user.name);
    }

    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        tracing::info!("Cache ready, starting polling task");

        let repository = self.repository.clone();
        let riot_client = self.riot_client.clone();
        let config = self.config.clone();
        let ctx = ctx.clone();

        tokio::spawn(async move {
            start_polling_task(ctx, repository, riot_client, config).await;
        });
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
) {
    let channel_id = ChannelId::new(config.discord_channel_id);
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
        let region = config.default_region.clone();

        let handle = tokio::spawn(async move {
            let tracker = GameTracker::new(riot_client, repository, region);

            tracing::info!(
                "Polling task started for {}#{}",
                summoner.game_name,
                summoner.tag_line
            );

            loop {
                if let Err(e) = check_and_notify(&ctx, &tracker, &summoner, channel_id).await {
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

async fn check_and_notify<R: RiotApiClient + ?Sized, D: Repository + ?Sized>(
    ctx: &Context,
    tracker: &GameTracker<R, D>,
    summoner: &Summoner,
    channel_id: ChannelId,
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

            let msg = format_game_started(
                &format!("{}#{}", summoner.game_name, summoner.tag_line),
                game_info.champion_id,
                &game_info.game_mode,
            );
            channel_id.say(&ctx.http, &msg).await?;
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
                Ok(Some(match_result)) => {
                    let msg = format_game_ended(
                        &format!("{}#{}", summoner_clone.game_name, summoner_clone.tag_line),
                        match_result.win,
                        match_result.kills,
                        match_result.deaths,
                        match_result.assists,
                        match_result.game_duration_secs,
                    );
                    channel_id.say(&ctx.http, &msg).await?;
                }
                Ok(None) => {
                    tracing::warn!(
                        "Could not get match result for {}#{} game {}",
                        summoner_clone.game_name,
                        summoner_clone.tag_line,
                        game_id
                    );
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
        GameStateChange::NoChange => {}
    }

    Ok(())
}
