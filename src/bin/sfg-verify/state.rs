use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{VerifyResult, err};

pub const STATE_FILE: &str = "state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceState {
    pub run_id: String,
    pub home: PathBuf,
    pub evidence_dir: PathBuf,
    pub pg_port: u16,
    pub pg_data_dir: PathBuf,
    pub database_url: String,
    pub default_region: String,
    pub fixture_path: PathBuf,
    pub build_version: String,
}

impl InstanceState {
    pub fn path(home: &Path) -> PathBuf {
        home.join(STATE_FILE)
    }

    pub fn load(home: &Path) -> VerifyResult<Self> {
        let path = Self::path(home);
        let raw = std::fs::read_to_string(&path).map_err(|_| {
            err(format!(
                "no verification instance at {} (missing {}). Run `sfg-verify launch` and pass the home it prints via --home or SFG_VERIFY_HOME.",
                home.display(),
                path.display()
            ))
        })?;
        let state: Self = serde_json::from_str(&raw)?;
        if state.home != home {
            return Err(err(format!(
                "state file {} records home {} but the command was pointed at {}",
                path.display(),
                state.home.display(),
                home.display()
            )));
        }
        if !is_loopback_database_url(&state.database_url) {
            return Err(err(format!(
                "refusing to use database URL that is not on 127.0.0.1 or localhost: {}",
                redact_database_url(&state.database_url)
            )));
        }
        Ok(state)
    }

    pub fn save(&self) -> VerifyResult<()> {
        let path = Self::path(&self.home);
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw)?;
        Ok(())
    }
}

pub fn is_loopback_database_url(url: &str) -> bool {
    matches!(
        database_host(url).as_deref(),
        Some("127.0.0.1" | "localhost" | "::1")
    )
}

pub fn redact_database_url(url: &str) -> String {
    match database_host(url) {
        Some(host) => format!("postgres://<redacted>@{host}/<database>"),
        None => "<unparseable database url>".to_string(),
    }
}

fn database_host(url: &str) -> Option<String> {
    let (_, after_scheme) = url.split_once("://")?;
    let hostport = after_scheme.split('@').next_back().unwrap_or(after_scheme);
    let hostport = hostport.split('/').next().unwrap_or(hostport);
    let host = if let Some(stripped) = hostport.strip_prefix('[') {
        stripped.split(']').next().unwrap_or(stripped)
    } else {
        hostport.split(':').next().unwrap_or(hostport)
    };
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

pub fn require_home(home: Option<PathBuf>) -> VerifyResult<PathBuf> {
    home.ok_or_else(|| {
        err(
            "SFG_VERIFY_HOME is unset and --home was not passed. Run `sfg-verify launch` and reuse the home path it prints.",
        )
    })
}
