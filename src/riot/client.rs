use crate::analysis::models::AnalysisData;
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

    async fn get_match_timeline(
        &self,
        match_id: &str,
        region: RegionalRoute,
    ) -> Result<Option<riven::models::match_v5::Timeline>, RiotClientError>;

    async fn get_match_analysis_data(
        &self,
        match_id: &str,
        puuid: &str,
        summoner_name: &str,
        region: RegionalRoute,
    ) -> Result<Option<AnalysisData>, RiotClientError>;

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

    async fn get_match_timeline(
        &self,
        match_id: &str,
        region: RegionalRoute,
    ) -> Result<Option<riven::models::match_v5::Timeline>, RiotClientError> {
        let timeline = self.api.match_v5().get_timeline(region, match_id).await?;
        Ok(timeline)
    }

    async fn get_match_analysis_data(
        &self,
        match_id: &str,
        puuid: &str,
        summoner_name: &str,
        region: RegionalRoute,
    ) -> Result<Option<AnalysisData>, RiotClientError> {
        let (match_data, timeline) = tokio::try_join!(
            self.api.match_v5().get_match(region, match_id),
            self.api.match_v5().get_timeline(region, match_id)
        )?;

        let Some(match_data) = match_data else {
            return Ok(None);
        };

        let Some(participant) = match_data
            .info
            .participants
            .iter()
            .find(|p| p.puuid == puuid)
        else {
            return Ok(None);
        };

        let total_cs = participant.total_minions_killed + participant.neutral_minions_killed;

        let is_same_role_different_team = |p: &&riven::models::match_v5::Participant| {
            p.team_position == participant.team_position
                && p.team_id != participant.team_id
                && p.puuid != puuid
        };

        let enemy_data = match_data
            .info
            .participants
            .iter()
            .find(is_same_role_different_team);

        let (enemy_champion_name, enemy_cs, enemy_gold, enemy_damage) = enemy_data
            .map(|enemy| {
                let enemy_cs = enemy.total_minions_killed + enemy.neutral_minions_killed;
                (
                    Some(enemy.champion_name.clone()),
                    Some(enemy_cs),
                    Some(enemy.gold_earned),
                    Some(enemy.total_damage_dealt_to_champions),
                )
            })
            .unwrap_or((None, None, None, None));

        let (gold_diff_at_10, gold_diff_at_15, gold_diff_at_20) =
            extract_timeline_diff(timeline.as_ref(), participant, enemy_data, |f| f.total_gold);

        let (cs_diff_at_10, cs_diff_at_15, cs_diff_at_20) =
            extract_timeline_diff(timeline.as_ref(), participant, enemy_data, |f| {
                f.minions_killed + f.jungle_minions_killed
            });

        let challenges = participant.challenges.as_ref();

        Ok(Some(AnalysisData {
            summoner_name: summoner_name.to_string(),
            champion_name: participant.champion_name.clone(),
            win: participant.win,
            kills: participant.kills,
            deaths: participant.deaths,
            assists: participant.assists,
            kda: challenges.and_then(|c| c.kda),
            kill_participation: challenges.and_then(|c| c.kill_participation),
            gold_per_minute: challenges.and_then(|c| c.gold_per_minute),
            damage_per_minute: challenges.and_then(|c| c.damage_per_minute),
            vision_score_per_minute: challenges.and_then(|c| c.vision_score_per_minute),
            team_damage_percentage: challenges.and_then(|c| c.team_damage_percentage),
            max_cs_advantage_on_lane_opponent: challenges
                .and_then(|c| c.max_cs_advantage_on_lane_opponent),
            early_laning_phase_gold_exp_advantage: challenges
                .and_then(|c| c.early_laning_phase_gold_exp_advantage.map(|v| v as f32)),
            laning_phase_gold_exp_advantage: challenges
                .and_then(|c| c.laning_phase_gold_exp_advantage.map(|v| v as f32)),
            turret_kills: participant.turret_kills,
            inhibitor_kills: participant.inhibitor_kills,
            objectives_stolen: participant.objectives_stolen,
            damage_dealt_to_objectives: participant.damage_dealt_to_objectives,
            total_damage_dealt_to_champions: participant.total_damage_dealt_to_champions,
            gold_earned: participant.gold_earned,
            total_cs,
            enemy_champion_name,
            enemy_cs,
            enemy_gold,
            enemy_damage,
            gold_diff_at_10,
            gold_diff_at_15,
            gold_diff_at_20,
            cs_diff_at_10,
            cs_diff_at_15,
            cs_diff_at_20,
            game_duration_secs: match_data.info.game_duration as i32,
            role: participant.team_position.clone(),
            game_mode: match_data.info.game_mode.to_string(),
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

fn extract_timeline_diff(
    timeline: Option<&riven::models::match_v5::Timeline>,
    participant: &riven::models::match_v5::Participant,
    enemy: Option<&riven::models::match_v5::Participant>,
    metric: impl Fn(&riven::models::match_v5::ParticipantFrame) -> i32,
) -> (Option<i32>, Option<i32>, Option<i32>) {
    let Some(timeline) = timeline else {
        return (None, None, None);
    };

    let Some(enemy) = enemy else {
        return (None, None, None);
    };

    let participant_id = participant.participant_id;
    let enemy_id = enemy.participant_id;

    let frames = &timeline.info.frames;

    (
        diff_at_frame(frames, 10, participant_id, enemy_id, &metric),
        diff_at_frame(frames, 15, participant_id, enemy_id, &metric),
        diff_at_frame(frames, 20, participant_id, enemy_id, &metric),
    )
}

fn diff_at_frame(
    frames: &[riven::models::match_v5::FramesTimeLine],
    minute: usize,
    participant_id: i32,
    enemy_id: i32,
    metric: &impl Fn(&riven::models::match_v5::ParticipantFrame) -> i32,
) -> Option<i32> {
    if frames.is_empty() {
        return None;
    }

    // Return None if the match ended before the requested minute.
    // Clamping to the last frame would produce misleading diff values.
    if frames.len() <= minute {
        return None;
    }

    for idx in (0..=minute).rev() {
        let frame = frames.get(idx)?;
        let participant_frames = match frame.participant_frames.as_ref() {
            Some(frames) => frames,
            None => continue,
        };

        let participant_frame = match participant_frames.get(&participant_id) {
            Some(frame) => frame,
            None => continue,
        };

        let enemy_frame = match participant_frames.get(&enemy_id) {
            Some(frame) => frame,
            None => continue,
        };

        return Some(metric(participant_frame) - metric(enemy_frame));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::diff_at_frame;
    use riven::models::match_v5::{FramesTimeLine, ParticipantFrame, Position};
    use std::collections::HashMap;

    fn build_frame(participant_id: i32, enemy_id: i32) -> FramesTimeLine {
        let mut participant_frames = HashMap::new();
        let participant_frame = ParticipantFrame {
            champion_stats: riven::models::match_v5::ChampionStats {
                ability_haste: None,
                ability_power: 0,
                armor: 0,
                armor_pen: 0,
                armor_pen_percent: 0,
                attack_damage: 0,
                attack_speed: 0,
                bonus_armor_pen_percent: 0,
                bonus_magic_pen_percent: 0,
                cc_reduction: 0,
                cooldown_reduction: 0,
                health: 0,
                health_max: 0,
                health_regen: 0,
                lifesteal: 0,
                magic_pen: 0,
                magic_pen_percent: 0,
                magic_resist: 0,
                movement_speed: 0,
                omnivamp: None,
                physical_vamp: None,
                power: 0,
                power_max: 0,
                power_regen: 0,
                spell_vamp: 0,
            },
            current_gold: 0,
            damage_stats: riven::models::match_v5::DamageStats {
                magic_damage_done: 0,
                magic_damage_done_to_champions: 0,
                magic_damage_taken: 0,
                physical_damage_done: 0,
                physical_damage_done_to_champions: 0,
                physical_damage_taken: 0,
                total_damage_done: 0,
                total_damage_done_to_champions: 0,
                total_damage_taken: 0,
                true_damage_done: 0,
                true_damage_done_to_champions: 0,
                true_damage_taken: 0,
            },
            gold_per_second: 0,
            jungle_minions_killed: 5,
            level: 1,
            minions_killed: 15,
            participant_id,
            position: Position { x: 0, y: 0 },
            time_enemy_spent_controlled: 0,
            total_gold: 3000,
            xp: 0,
        };

        let enemy_frame = ParticipantFrame {
            champion_stats: participant_frame.champion_stats.clone(),
            current_gold: 0,
            damage_stats: participant_frame.damage_stats.clone(),
            gold_per_second: 0,
            jungle_minions_killed: 3,
            level: 1,
            minions_killed: 12,
            participant_id: enemy_id,
            position: Position { x: 0, y: 0 },
            time_enemy_spent_controlled: 0,
            total_gold: 2800,
            xp: 0,
        };

        participant_frames.insert(participant_id, participant_frame);
        participant_frames.insert(enemy_id, enemy_frame);

        FramesTimeLine {
            events: Vec::new(),
            participant_frames: Some(participant_frames),
            timestamp: 600000,
        }
    }

    fn build_empty_frame() -> FramesTimeLine {
        FramesTimeLine {
            events: Vec::new(),
            participant_frames: None,
            timestamp: 600000,
        }
    }

    #[test]
    fn diff_at_frame_returns_gold_difference() {
        let frames = vec![build_frame(1, 2)];
        let diff = diff_at_frame(&frames, 0, 1, 2, &|frame| frame.total_gold);
        assert_eq!(diff, Some(200));
    }

    #[test]
    fn diff_at_frame_returns_none_when_frame_missing() {
        let frames = vec![];
        let diff = diff_at_frame(&frames, 10, 1, 2, &|frame| frame.total_gold);
        assert!(diff.is_none());
    }

    #[test]
    fn diff_at_frame_returns_none_when_participant_missing() {
        let frames = vec![build_frame(1, 2)];
        let diff = diff_at_frame(&frames, 0, 1, 999, &|frame| frame.total_gold);
        assert!(diff.is_none());
    }

    #[test]
    fn diff_at_frame_falls_back_to_previous_frame() {
        let frames = vec![build_frame(1, 2), build_empty_frame()];
        let diff = diff_at_frame(&frames, 1, 1, 2, &|frame| frame.total_gold);
        assert_eq!(diff, Some(200));
    }
}
