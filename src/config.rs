use std::env;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub riot_api_key: String,
    pub discord_bot_token: String,
    pub discord_bot_id: u64,
    pub discord_server_id: u64,
    pub discord_channel_id: u64,
    pub summoner_names: Vec<SummonerConfig>,
    pub database_url: String,
    pub default_region: String,
    pub polling_interval_secs: u64,
}

#[derive(Debug, Clone)]
pub struct SummonerConfig {
    pub name: String,
    pub tag: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid environment variable value for {0}: {1}")]
    InvalidValue(String, String),
    #[error("Invalid summoner name format: {0}")]
    InvalidSummonerFormat(String),
}

impl Config {
    pub fn from_env() -> Self {
        match dotenvy::dotenv() {
            Ok(path) => tracing::debug!("Loaded .env from: {:?}", path),
            Err(e) => tracing::warn!("Could not load .env: {}", e),
        }

        let riot_api_key = env::var("RIOT_API_KEY").expect("RIOT_API_KEY must be set");
        let discord_bot_token =
            env::var("DISCORD_BOT_TOKEN").expect("DISCORD_BOT_TOKEN must be set");
        let discord_bot_id = env::var("DISCORD_BOT_ID")
            .expect("DISCORD_BOT_ID must be set")
            .parse()
            .expect("DISCORD_BOT_ID must be a valid u64");
        let discord_server_id = env::var("DISCORD_SERVER_ID")
            .expect("DISCORD_SERVER_ID must be set")
            .parse()
            .expect("DISCORD_SERVER_ID must be a valid u64");
        let discord_channel_id = env::var("DISCORD_CHANNEL_ID")
            .expect("DISCORD_CHANNEL_ID must be set")
            .parse()
            .expect("DISCORD_CHANNEL_ID must be a valid u64");

        let summoner_names_raw = env::var("SUMMONER_NAMES").expect("SUMMONER_NAMES must be set");
        let summoner_names =
            parse_summoner_names(&summoner_names_raw).expect("SUMMONER_NAMES must be valid format");

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let default_region = env::var("DEFAULT_REGION").unwrap_or_else(|_| "euw1".to_string());
        let polling_interval_secs = env::var("POLLING_INTERVAL_SECS")
            .unwrap_or_else(|_| "180".to_string())
            .parse()
            .unwrap_or(180);

        Self {
            riot_api_key,
            discord_bot_token,
            discord_bot_id,
            discord_server_id,
            discord_channel_id,
            summoner_names,
            database_url,
            default_region,
            polling_interval_secs,
        }
    }
}

pub fn parse_summoner_names(input: &str) -> Result<Vec<SummonerConfig>, ConfigError> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }

    input
        .split('|')
        .map(|entry| {
            let entry = entry.trim();
            let hash_pos = entry
                .rfind('#')
                .ok_or_else(|| ConfigError::InvalidSummonerFormat(entry.to_string()))?;

            let name = entry[..hash_pos].trim().to_string();
            let tag = entry[hash_pos + 1..].trim().to_string();

            if name.is_empty() || tag.is_empty() {
                return Err(ConfigError::InvalidSummonerFormat(entry.to_string()));
            }

            if !is_valid_summoner_name(&name) {
                return Err(ConfigError::InvalidSummonerFormat(format!(
                    "Invalid name: {}",
                    name
                )));
            }

            if !is_valid_tag(&tag) {
                return Err(ConfigError::InvalidSummonerFormat(format!(
                    "Invalid tag: {}",
                    tag
                )));
            }

            Ok(SummonerConfig { name, tag })
        })
        .collect()
}

fn is_valid_summoner_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 24
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
}

fn is_valid_tag(tag: &str) -> bool {
    !tag.is_empty() && tag.len() <= 5 && tag.chars().all(|c| c.is_alphanumeric())
}
