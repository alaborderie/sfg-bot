use std::path::Path;

use async_trait::async_trait;
use riven::consts::{PlatformRoute, RegionalRoute};
use serde::Deserialize;

use sfg_bot::analysis::models::AnalysisData;
use sfg_bot::riot::client::{RiotApiClient, RiotClientError};
use sfg_bot::riot::models::{ActiveGameInfo, MatchResult, SummonerInfo};

use crate::error::{VerifyResult, err};

#[derive(Debug, Clone, Deserialize)]
struct FixtureFile {
    accounts: Vec<FixtureAccount>,
    recent_match_id: String,
    #[serde(rename = "match")]
    match_result: FixtureMatch,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureAccount {
    game_name: String,
    tag_line: String,
    puuid: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureMatch {
    match_id: String,
    game_id: i64,
    win: bool,
    kills: i32,
    deaths: i32,
    assists: i32,
    champion_id: i32,
    champion_name: String,
    game_duration_secs: i32,
    game_mode: String,
    role: String,
    total_cs: i32,
    total_gold: i32,
    total_damage: i32,
    enemy_champion_name: Option<String>,
    enemy_cs: Option<i32>,
    enemy_gold: Option<i32>,
    enemy_damage: Option<i32>,
    queue_id: Option<i32>,
    role_gaps: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FixtureRiot {
    file: FixtureFile,
}

impl FixtureRiot {
    pub fn load(path: &Path) -> VerifyResult<Self> {
        let raw = std::fs::read_to_string(path).map_err(|error| {
            err(format!(
                "could not read Riot fixture {}: {error}. The default path is .claude/skills/verify-sfg-bot/fixtures/riot.json from the repository root.",
                path.display()
            ))
        })?;
        let file: FixtureFile = serde_json::from_str(&raw).map_err(|error| {
            err(format!(
                "Riot fixture {} is not valid JSON: {error}",
                path.display()
            ))
        })?;
        if file.accounts.is_empty() {
            return Err(err(format!(
                "Riot fixture {} has no accounts. Add at least one game_name/tag_line/puuid entry.",
                path.display()
            )));
        }
        for account in &file.accounts {
            if account.tag_line.chars().count() > 5 {
                return Err(err(format!(
                    "fixture tag `{}` is longer than 5 characters; summoners.tag_line is VARCHAR(5) and the insert would fail",
                    account.tag_line
                )));
            }
            if account.game_name.chars().count() > 24 {
                return Err(err(format!(
                    "fixture game name `{}` is longer than 24 characters; summoners.game_name is VARCHAR(24)",
                    account.game_name
                )));
            }
        }
        Ok(Self { file })
    }

    fn account(&self, game_name: &str, tag_line: &str) -> Option<&FixtureAccount> {
        self.file.accounts.iter().find(|account| {
            account.game_name.eq_ignore_ascii_case(game_name)
                && account.tag_line.eq_ignore_ascii_case(tag_line)
        })
    }
}

#[async_trait]
impl RiotApiClient for FixtureRiot {
    async fn get_account_by_riot_id(
        &self,
        game_name: &str,
        tag_line: &str,
        _region: RegionalRoute,
    ) -> Result<SummonerInfo, RiotClientError> {
        self.account(game_name, tag_line)
            .map(|account| SummonerInfo {
                puuid: account.puuid.clone(),
                game_name: account.game_name.clone(),
                tag_line: account.tag_line.clone(),
            })
            .ok_or_else(|| {
                RiotClientError::AccountNotFound(game_name.to_string(), tag_line.to_string())
            })
    }

    async fn get_active_game(
        &self,
        _puuid: &str,
        _platform: PlatformRoute,
    ) -> Result<Option<ActiveGameInfo>, RiotClientError> {
        Ok(None)
    }

    async fn get_match_result(
        &self,
        match_id: &str,
        _puuid: &str,
        _region: RegionalRoute,
    ) -> Result<Option<MatchResult>, RiotClientError> {
        if match_id != self.file.match_result.match_id {
            return Ok(None);
        }
        let fixture = &self.file.match_result;
        Ok(Some(MatchResult {
            match_id: fixture.match_id.clone(),
            game_id: fixture.game_id,
            win: fixture.win,
            kills: fixture.kills,
            deaths: fixture.deaths,
            assists: fixture.assists,
            champion_id: fixture.champion_id,
            champion_name: fixture.champion_name.clone(),
            game_duration_secs: fixture.game_duration_secs,
            game_mode: fixture.game_mode.clone(),
            role: fixture.role.clone(),
            total_cs: fixture.total_cs,
            total_gold: fixture.total_gold,
            total_damage: fixture.total_damage,
            enemy_champion_name: fixture.enemy_champion_name.clone(),
            enemy_cs: fixture.enemy_cs,
            enemy_gold: fixture.enemy_gold,
            enemy_damage: fixture.enemy_damage,
            queue_id: fixture.queue_id,
            role_gaps: fixture.role_gaps.clone(),
            game_end_timestamp: None,
            game_start_timestamp: None,
        }))
    }

    async fn get_match_timeline(
        &self,
        _match_id: &str,
        _region: RegionalRoute,
    ) -> Result<Option<riven::models::match_v5::Timeline>, RiotClientError> {
        Ok(None)
    }

    async fn get_match_analysis_data(
        &self,
        _match_id: &str,
        _puuid: &str,
        _summoner_name: &str,
        _region: RegionalRoute,
    ) -> Result<Option<AnalysisData>, RiotClientError> {
        Ok(None)
    }

    async fn get_recent_match_id(
        &self,
        puuid: &str,
        _region: RegionalRoute,
    ) -> Result<Option<String>, RiotClientError> {
        let known = self
            .file
            .accounts
            .iter()
            .any(|account| account.puuid == puuid);
        if known {
            Ok(Some(self.file.recent_match_id.clone()))
        } else {
            Ok(None)
        }
    }

    async fn get_all_champions(
        &self,
    ) -> Result<std::collections::HashMap<i32, String>, RiotClientError> {
        let mut champions = std::collections::HashMap::new();
        champions.insert(
            self.file.match_result.champion_id,
            self.file.match_result.champion_name.clone(),
        );
        Ok(champions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lookup_is_case_insensitive_and_returns_canonical_names() {
        let path = Path::new(".claude/skills/verify-sfg-bot/fixtures/riot.json");
        let fixture = FixtureRiot::load(path).expect("fixture");
        let info = fixture
            .get_account_by_riot_id("verifyfox", "sfg", RegionalRoute::EUROPE)
            .await
            .expect("account");
        assert_eq!(info.game_name, "VerifyFox");
        assert_eq!(info.tag_line, "SFG");
        assert_eq!(info.puuid, "verify-puuid-sfg-bot-0001");
        let missing = fixture
            .get_account_by_riot_id("Nobody", "NOPE", RegionalRoute::EUROPE)
            .await;
        assert!(missing.is_err());
    }
}
