use sfg_bot::config::Config;
use sfg_bot::db;
use sfg_bot::db::repository::PgRepository;
use sfg_bot::discord::handler::Bot;
use sfg_bot::riot::client::RiotClient;
use sfg_bot::{Repository, RiotApiClient};

use serenity::prelude::*;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

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
    tracing::info!(
        "Loaded configuration with {} summoners",
        config.summoner_names.len()
    );

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

    // Create Riot API client
    let riot_client: Arc<dyn RiotApiClient> = Arc::new(RiotClient::new(&config.riot_api_key));
    tracing::info!("Riot API client initialized");

    // Create repository
    let repository = Arc::new(PgRepository::new(db_pool.clone()));

    // Resolve summoner PUUIDs and upsert to database
    for summoner_config in &config.summoner_names {
        match riot_client
            .get_account_by_riot_id(
                &summoner_config.name,
                &summoner_config.tag,
                RiotClient::regional_for_region(&config.default_region),
            )
            .await
        {
            Ok(summoner_info) => {
                if let Err(e) = repository
                    .upsert_summoner(
                        &summoner_info.puuid,
                        &summoner_info.game_name,
                        &summoner_info.tag_line,
                        &config.default_region,
                    )
                    .await
                {
                    tracing::error!(
                        "Failed to upsert summoner {}#{}: {}",
                        summoner_config.name,
                        summoner_config.tag,
                        e
                    );
                } else {
                    tracing::info!(
                        "Registered summoner: {}#{}",
                        summoner_info.game_name,
                        summoner_info.tag_line
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to resolve summoner {}#{}: {}",
                    summoner_config.name,
                    summoner_config.tag,
                    e
                );
            }
        }
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
