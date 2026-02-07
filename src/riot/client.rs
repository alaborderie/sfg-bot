use crate::riot::models::{ActiveGameInfo, MatchResult, SummonerInfo};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use riven::RiotApi;
use riven::consts::{PlatformRoute, RegionalRoute};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RiotClientError {
    #[error("Riot API error: {0}")]
    ApiError(#[from] riven::RiotApiError),
    #[error("Account not found: {0}#{1}")]
    AccountNotFound(String, String),
    #[error("Unknown region: {0}")]
    UnknownRegion(String),
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[cfg_attr(feature = "test-mocks", mockall::automock)]
#[async_trait]
pub trait RiotApiClient: Send + Sync {
    async fn get_account_by_riot_id(
        &self,
        game_name: &str,
        tag_line: &str,
        region: RegionalRoute,
    ) -> Result<SummonerInfo, RiotClientError>;

    async fn get_active_game(
        &self,
        puuid: &str,
        platform: PlatformRoute,
    ) -> Result<Option<ActiveGameInfo>, RiotClientError>;

    async fn get_match_result(
        &self,
        match_id: &str,
        puuid: &str,
        region: RegionalRoute,
    ) -> Result<Option<MatchResult>, RiotClientError>;

    /// Get the most recent match ID for a summoner
    async fn get_recent_match_id(
        &self,
        puuid: &str,
        region: RegionalRoute,
    ) -> Result<Option<String>, RiotClientError>;

    async fn get_all_champions(
        &self,
    ) -> Result<std::collections::HashMap<i32, String>, RiotClientError>;
}

pub struct RiotClient {
    api: RiotApi,
}

impl RiotClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api: RiotApi::new(api_key),
        }
    }

    pub fn platform_for_region(region: &str) -> PlatformRoute {
        match region.to_lowercase().as_str() {
            "br1" | "br" => PlatformRoute::BR1,
            "eun1" | "eune" => PlatformRoute::EUN1,
            "euw1" | "euw" => PlatformRoute::EUW1,
            "jp1" | "jp" => PlatformRoute::JP1,
            "kr" => PlatformRoute::KR,
            "la1" | "lan" => PlatformRoute::LA1,
            "la2" | "las" => PlatformRoute::LA2,
            "na1" | "na" => PlatformRoute::NA1,
            "oc1" | "oce" => PlatformRoute::OC1,
            "tr1" | "tr" => PlatformRoute::TR1,
            "ru" => PlatformRoute::RU,
            "sg2" | "sg" => PlatformRoute::SG2,
            "tw2" | "tw" => PlatformRoute::TW2,
            "vn2" | "vn" => PlatformRoute::VN2,
            _ => PlatformRoute::EUW1,
        }
    }

    pub fn regional_for_region(region: &str) -> RegionalRoute {
        match region.to_lowercase().as_str() {
            "br1" | "br" | "la1" | "lan" | "la2" | "las" | "na1" | "na" | "oc1" | "oce" => {
                RegionalRoute::AMERICAS
            }
            "jp1" | "jp" | "kr" => RegionalRoute::ASIA,
            "sg2" | "sg" | "tw2" | "tw" | "vn2" | "vn" => RegionalRoute::SEA,
            "eun1" | "eune" | "euw1" | "euw" | "tr1" | "tr" | "ru" => RegionalRoute::EUROPE,
            _ => RegionalRoute::EUROPE,
        }
    }
}

#[async_trait]
impl RiotApiClient for RiotClient {
    async fn get_account_by_riot_id(
        &self,
        game_name: &str,
        tag_line: &str,
        region: RegionalRoute,
    ) -> Result<SummonerInfo, RiotClientError> {
        let account = self
            .api
            .account_v1()
            .get_by_riot_id(region, game_name, tag_line)
            .await?
            .ok_or_else(|| {
                RiotClientError::AccountNotFound(game_name.to_string(), tag_line.to_string())
            })?;

        Ok(SummonerInfo {
            puuid: account.puuid,
            game_name: account.game_name.unwrap_or_default(),
            tag_line: account.tag_line.unwrap_or_default(),
        })
    }

    async fn get_active_game(
        &self,
        puuid: &str,
        platform: PlatformRoute,
    ) -> Result<Option<ActiveGameInfo>, RiotClientError> {
        let game = self
            .api
            .spectator_v5()
            .get_current_game_info_by_puuid(platform, puuid)
            .await?;

        Ok(game.map(|g| {
            let participant = g
                .participants
                .iter()
                .find(|p| p.puuid.as_deref() == Some(puuid));

            let champion_id = participant.map(|p| p.champion_id.0 as i32).unwrap_or(0);

            let game_start = Utc
                .timestamp_millis_opt(g.game_start_time)
                .single()
                .unwrap_or_else(Utc::now);

            let queue_id = g.game_queue_config_id.map(|q| q.0 as i32);

            ActiveGameInfo {
                game_id: g.game_id,
                champion_id,
                game_mode: g.game_mode.to_string(),
                game_start_time: game_start,
                queue_id,
            }
        }))
    }

    async fn get_match_result(
        &self,
        match_id: &str,
        puuid: &str,
        region: RegionalRoute,
    ) -> Result<Option<MatchResult>, RiotClientError> {
        let match_data = self.api.match_v5().get_match(region, match_id).await?;

        Ok(match_data.and_then(|m| {
            let participant = m.info.participants.iter().find(|p| p.puuid == puuid)?;

            let total_cs = participant.total_minions_killed + participant.neutral_minions_killed;

            let is_same_role_different_team = |p: &&riven::models::match_v5::Participant| {
                p.team_position == participant.team_position
                    && p.team_id != participant.team_id
                    && p.puuid != puuid
            };

            let enemy_data = m
                .info
                .participants
                .iter()
                .find(is_same_role_different_team)
                .map(|enemy| {
                    let enemy_cs = enemy.total_minions_killed + enemy.neutral_minions_killed;
                    (
                        enemy.champion_name.clone(),
                        enemy_cs,
                        enemy.gold_earned,
                        enemy.total_damage_dealt_to_champions,
                    )
                });

            let (enemy_champion_name, enemy_cs, enemy_gold, enemy_damage) = match enemy_data {
                Some((name, cs, gold, dmg)) => (Some(name), Some(cs), Some(gold), Some(dmg)),
                None => (None, None, None, None),
            };

            let queue_id = Some(m.info.queue_id.0 as i32);

            Some(MatchResult {
                match_id: m.metadata.match_id,
                game_id: m.info.game_id,
                win: participant.win,
                kills: participant.kills,
                deaths: participant.deaths,
                assists: participant.assists,
                champion_id: participant.champion().map(|c| c.0 as i32).unwrap_or(0),
                game_duration_secs: m.info.game_duration as i32,
                game_mode: m.info.game_mode.to_string(),
                role: participant.team_position.clone(),
                total_cs,
                total_gold: participant.gold_earned,
                total_damage: participant.total_damage_dealt_to_champions,
                enemy_champion_name,
                enemy_cs,
                enemy_gold,
                enemy_damage,
                queue_id,
            })
        }))
    }

    async fn get_recent_match_id(
        &self,
        puuid: &str,
        region: RegionalRoute,
    ) -> Result<Option<String>, RiotClientError> {
        let matches = self
            .api
            .match_v5()
            .get_match_ids_by_puuid(region, puuid, Some(1), None, None, None, None, None)
            .await?;

        Ok(matches.first().cloned())
    }

    async fn get_all_champions(
        &self,
    ) -> Result<std::collections::HashMap<i32, String>, RiotClientError> {
        #[derive(Deserialize)]
        struct ChampionData {
            key: String, // This is the champion_id as a string
            name: String,
        }

        #[derive(Deserialize)]
        struct ChampionsResponse {
            data: std::collections::HashMap<String, ChampionData>,
        }

        let client = reqwest::Client::new();
        let response = client
            .get("https://ddragon.leagueoflegends.com/cdn/14.1.1/data/en_US/champion.json")
            .send()
            .await?;

        let champions: ChampionsResponse = response.json().await?;

        let mut result = std::collections::HashMap::new();
        for champion_data in champions.data.values() {
            if let Ok(champion_id) = champion_data.key.parse::<i32>() {
                result.insert(champion_id, champion_data.name.clone());
            }
        }

        Ok(result)
    }
}
