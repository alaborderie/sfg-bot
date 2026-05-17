use std::env;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub riot_api_key: String,
    pub discord_bot_token: String,
    pub discord_bot_id: u64,
    pub database_url: String,
    pub default_region: String,
    pub polling_interval_secs: u64,
    pub gemini_api_key: Option<String>,
    pub analysis_prompts_dir: String,
    pub health_check_port: Option<u16>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid environment variable value for {0}: {1}")]
    InvalidValue(String, String),
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

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let default_region = env::var("DEFAULT_REGION").unwrap_or_else(|_| "euw1".to_string());
        let polling_interval_secs = env::var("POLLING_INTERVAL_SECS")
            .unwrap_or_else(|_| "180".to_string())
            .parse()
            .unwrap_or(180);

        let gemini_api_key = env::var("GEMINI_API_KEY").ok();
        let analysis_prompts_dir =
            env::var("ANALYSIS_PROMPTS_DIR").unwrap_or_else(|_| "analysis_prompts".to_string());
        let health_check_port = env::var("HEALTH_CHECK_PORT")
            .ok()
            .filter(|raw| !raw.trim().is_empty())
            .and_then(|raw| {
                raw.parse::<u16>()
                    .inspect_err(|_| {
                        tracing::warn!(
                            value = raw.as_str(),
                            "HEALTH_CHECK_PORT is not a valid u16; health check disabled"
                        );
                    })
                    .ok()
            });

        tracing::info!(
            has_gemini_api_key = gemini_api_key.is_some(),
            analysis_prompts_dir = analysis_prompts_dir.as_str(),
            health_check_port = ?health_check_port,
        );

        Self {
            riot_api_key,
            discord_bot_token,
            discord_bot_id,
            database_url,
            default_region,
            polling_interval_secs,
            gemini_api_key,
            analysis_prompts_dir,
            health_check_port,
        }
    }
}
