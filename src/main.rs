use sfg_bot::config::Config;
use sfg_bot::db;
use sfg_bot::db::repository::PgRepository;
use sfg_bot::discord::handler::Bot;
use sfg_bot::health;
use sfg_bot::riot::client::RiotClient;
use sfg_bot::{Repository, RiotApiClient};

use serenity::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{MissedTickBehavior, interval};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const CHAMPION_REFRESH_INTERVAL_SECS: u64 = 24 * 60 * 60;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("sfg_bot=info,serenity=warn")),
        )
        .init();

    tracing::info!("Starting SFG Bot...");

    // Load configuration
    let config = Config::from_env();
    tracing::info!("Configuration loaded");

    // Create database pool
    let db_pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to database");
    tracing::info!("Connected to database");

    // Run migrations
    sqlx::migrate!()
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");
    tracing::info!("Database migrations complete");

    let riot_client: Arc<dyn RiotApiClient> = Arc::new(RiotClient::new(&config.riot_api_key));
    tracing::info!("Riot API client initialized");

    let repository: Arc<dyn Repository> = Arc::new(PgRepository::new(db_pool.clone()));

    refresh_champion_cache(repository.as_ref(), riot_client.as_ref()).await;

    spawn_champion_refresh_task(repository.clone(), riot_client.clone());

    if let Some(port) = config.health_check_port {
        health::spawn(port);
    }

    // Set up Discord client with required intents
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let bot = Bot::new(repository, riot_client, config);

    let mut client = Client::builder(&bot.config.discord_bot_token, intents)
        .event_handler(bot)
        .await
        .expect("Failed to create Discord client");

    tracing::info!("Starting Discord client...");

    // Start the Discord client
    if let Err(e) = client.start().await {
        tracing::error!("Discord client error: {}", e);
    }
}

/// Fetches the champion list from Data Dragon and upserts every entry into
/// the repository. Failures are logged, not propagated, so a transient Data
/// Dragon outage never blocks bot startup or the periodic refresh.
async fn refresh_champion_cache(repository: &dyn Repository, riot_client: &dyn RiotApiClient) {
    tracing::info!("Fetching champion data from Data Dragon...");
    match riot_client.get_all_champions().await {
        Ok(champions) => {
            tracing::info!(
                "Fetched {} champions, upserting to database...",
                champions.len()
            );
            let mut success_count = 0;
            for (champion_id, champion_name) in champions {
                if let Err(e) = repository
                    .upsert_champion(champion_id, &champion_name)
                    .await
                {
                    tracing::error!("Failed to upsert champion {}: {}", champion_name, e);
                } else {
                    success_count += 1;
                }
            }
            tracing::info!("Successfully cached {} champions", success_count);
        }
        Err(e) => {
            tracing::warn!(
                "Failed to fetch champion data: {}. Champion names will fall back to IDs.",
                e
            );
        }
    }
}

/// Spawns a background task that refreshes the champion cache once per day.
/// Riot releases new champions and renames assets ~every 2 weeks, so an
/// idle-running bot would otherwise drift until the next deploy.
fn spawn_champion_refresh_task(
    repository: Arc<dyn Repository>,
    riot_client: Arc<dyn RiotApiClient>,
) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(CHAMPION_REFRESH_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // Skip the immediate tick — startup already warmed the cache.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            tracing::info!("Refreshing champion cache (periodic)");
            refresh_champion_cache(repository.as_ref(), riot_client.as_ref()).await;
        }
    });
}
