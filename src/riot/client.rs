use crate::riot::models::{ActiveGameInfo, MatchResult, SummonerInfo};
use chrono::{TimeZone, Utc};
use riven::consts::{PlatformRoute, RegionalRoute};
use riven::RiotApi;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RiotClientError {
    #[error("Riot API error: {0}")]
    ApiError(#[from] riven::RiotApiError),
    #[error("Account not found: {0}#{1}")]
    AccountNotFound(String, String),
    #[error("Unknown region: {0}")]
    UnknownRegion(String),
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

    /// Get account info by Riot ID (game_name#tag_line)
    pub async fn get_account_by_riot_id(
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

    /// Get active game for a summoner (None if not in game)
    pub async fn get_active_game(
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
            // Find the participant matching our puuid
            let participant = g
                .participants
                .iter()
                .find(|p| p.puuid.as_deref() == Some(puuid));

            let champion_id = participant.map(|p| p.champion_id.0 as i32).unwrap_or(0);

            let game_start = Utc
                .timestamp_millis_opt(g.game_start_time)
                .single()
                .unwrap_or_else(Utc::now);

            ActiveGameInfo {
                game_id: g.game_id,
                champion_id,
                game_mode: g.game_mode.to_string(),
                game_start_time: game_start,
            }
        }))
    }

    /// Get match result by match ID
    pub async fn get_match_result(
        &self,
        match_id: &str,
        puuid: &str,
        region: RegionalRoute,
    ) -> Result<Option<MatchResult>, RiotClientError> {
        let match_data = self.api.match_v5().get_match(region, match_id).await?;

        Ok(match_data.and_then(|m| {
            // Find the participant matching our puuid
            let participant = m.info.participants.iter().find(|p| p.puuid == puuid)?;

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
            })
        }))
    }

    /// Get recent match IDs for a summoner
    pub async fn get_recent_match_ids(
        &self,
        puuid: &str,
        region: RegionalRoute,
        count: i32,
    ) -> Result<Vec<String>, RiotClientError> {
        let matches = self
            .api
            .match_v5()
            .get_match_ids_by_puuid(region, puuid, Some(count), None, None, None, None, None)
            .await?;
        Ok(matches)
    }

    /// Convert region string to PlatformRoute (for Spectator-V5)
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
            _ => PlatformRoute::EUW1, // Default to EUW
        }
    }

    /// Convert region string to RegionalRoute (for Account-V1, Match-V5)
    pub fn regional_for_region(region: &str) -> RegionalRoute {
        match region.to_lowercase().as_str() {
            "br1" | "br" | "la1" | "lan" | "la2" | "las" | "na1" | "na" | "oc1" | "oce" => {
                RegionalRoute::AMERICAS
            }
            "jp1" | "jp" | "kr" => RegionalRoute::ASIA,
            "sg2" | "sg" | "tw2" | "tw" | "vn2" | "vn" => RegionalRoute::SEA,
            "eun1" | "eune" | "euw1" | "euw" | "tr1" | "tr" | "ru" => RegionalRoute::EUROPE,
            _ => RegionalRoute::EUROPE, // Default to Europe
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_for_region_euw() {
        assert!(matches!(
            RiotClient::platform_for_region("euw1"),
            PlatformRoute::EUW1
        ));
        assert!(matches!(
            RiotClient::platform_for_region("EUW"),
            PlatformRoute::EUW1
        ));
    }

    #[test]
    fn test_platform_for_region_na() {
        assert!(matches!(
            RiotClient::platform_for_region("na1"),
            PlatformRoute::NA1
        ));
        assert!(matches!(
            RiotClient::platform_for_region("NA"),
            PlatformRoute::NA1
        ));
    }

    #[test]
    fn test_regional_for_region_europe() {
        assert!(matches!(
            RiotClient::regional_for_region("euw1"),
            RegionalRoute::EUROPE
        ));
        assert!(matches!(
            RiotClient::regional_for_region("eune"),
            RegionalRoute::EUROPE
        ));
    }

    #[test]
    fn test_regional_for_region_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("na1"),
            RegionalRoute::AMERICAS
        ));
        assert!(matches!(
            RiotClient::regional_for_region("br"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn test_regional_for_region_asia() {
        assert!(matches!(
            RiotClient::regional_for_region("kr"),
            RegionalRoute::ASIA
        ));
        assert!(matches!(
            RiotClient::regional_for_region("jp1"),
            RegionalRoute::ASIA
        ));
    }
}
