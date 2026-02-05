pub mod config;
pub mod db;
pub mod discord;
pub mod riot;

pub use config::{Config, ConfigError, SummonerConfig, parse_summoner_names};
pub use db::repository::{PgRepository, Repository, RepositoryError};
pub use riot::client::{RiotApiClient, RiotClient, RiotClientError};
pub use riot::tracker::{GameTracker, TrackerError};

#[cfg(feature = "test-mocks")]
pub use db::repository::MockRepository;
#[cfg(feature = "test-mocks")]
pub use riot::client::MockRiotApiClient;
