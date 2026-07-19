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

// riven::RiotApiError is ~200 bytes; boxing it would ripple through every
// call site for no runtime win, so silence result_large_err on the mock.
#[allow(clippy::result_large_err)]
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

            let role_gaps = format_role_gaps(&compute_role_gaps(
                &m.info.participants,
                participant.team_id,
            ));

            Some(MatchResult {
                match_id: m.metadata.match_id,
                game_id: m.info.game_id,
                win: participant.win,
                kills: participant.kills,
                deaths: participant.deaths,
                assists: participant.assists,
                champion_id: participant.champion().map(|c| c.0 as i32).unwrap_or(0),
                champion_name: participant.champion_name.clone(),
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
                role_gaps,
                game_end_timestamp: m.info.game_end_timestamp,
                game_start_timestamp: Some(m.info.game_start_timestamp),
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
            recent_games: Vec::new(),
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

/// Per-lane gold gap from the tracked summoner's perspective.
///
/// `gold_delta = ally_team_gold_in_lane - enemy_team_gold_in_lane`, so a
/// negative value means the tracked summoner's team is behind in that lane.
/// For `BOTLANE`, both BOTTOM and UTILITY are summed on each side.
#[derive(Debug, Clone, PartialEq)]
pub struct RoleGap {
    pub lane: &'static str,
    pub gold_delta: i32,
}

/// Gold-delta thresholds at which a lane is reported as gapped (or diff'd).
/// Solo lanes use a tighter threshold than the combined bot lane.
pub const SOLO_LANE_GAP_THRESHOLD: i32 = 3000;
pub const BOT_LANE_GAP_THRESHOLD: i32 = 5000;

/// Minimum data needed to compute lane gaps. Decoupled from
/// `riven::models::match_v5::Participant` so tests don't have to construct
/// the full Participant struct.
#[derive(Debug, Clone)]
pub struct ParticipantGapInput<'a> {
    pub team_position: &'a str,
    pub team_id_value: u16,
    pub gold_earned: i32,
}

/// Adapter for the production caller that already has riven Participants.
fn participants_to_gap_inputs(
    participants: &[riven::models::match_v5::Participant],
) -> Vec<ParticipantGapInput<'_>> {
    participants
        .iter()
        .map(|p| ParticipantGapInput {
            team_position: p.team_position.as_str(),
            team_id_value: <riven::consts::Team as Into<u16>>::into(p.team_id),
            gold_earned: p.gold_earned,
        })
        .collect()
}

/// Compares per-lane gold totals between teams and returns each lane whose
/// gold delta crosses the gap threshold. Output is from `team_id`'s
/// perspective: positive delta = team_id leads the lane.
///
/// Riot exposes lanes as `team_position` strings (`TOP`, `JUNGLE`, `MIDDLE`,
/// `BOTTOM`, `UTILITY`). `Bot` is synthesised by summing BOTTOM + UTILITY
/// per team.
pub fn compute_role_gaps(
    participants: &[riven::models::match_v5::Participant],
    team_id: riven::consts::Team,
) -> Vec<RoleGap> {
    let inputs = participants_to_gap_inputs(participants);
    compute_role_gaps_from_inputs(&inputs, <riven::consts::Team as Into<u16>>::into(team_id))
}

pub fn compute_role_gaps_from_inputs(
    participants: &[ParticipantGapInput<'_>],
    ally_team_id: u16,
) -> Vec<RoleGap> {
    let solo_lanes: &[(&'static str, &str)] =
        &[("Top", "TOP"), ("Jungle", "JUNGLE"), ("Mid", "MIDDLE")];

    let mut gaps = Vec::new();
    for (display, riot_pos) in solo_lanes {
        let ally = lane_gold_input(participants, riot_pos, Some(ally_team_id));
        let enemy = lane_gold_input_excluding(participants, riot_pos, ally_team_id);
        if let (Some(ally), Some(enemy)) = (ally, enemy) {
            let delta = ally - enemy;
            if delta.abs() >= SOLO_LANE_GAP_THRESHOLD {
                gaps.push(RoleGap {
                    lane: display,
                    gold_delta: delta,
                });
            }
        }
    }

    let ally_bot = lane_gold_input(participants, "BOTTOM", Some(ally_team_id))
        .zip(lane_gold_input(participants, "UTILITY", Some(ally_team_id)))
        .map(|(a, b)| a + b);
    let enemy_bot = lane_gold_input_excluding(participants, "BOTTOM", ally_team_id)
        .zip(lane_gold_input_excluding(
            participants,
            "UTILITY",
            ally_team_id,
        ))
        .map(|(a, b)| a + b);
    if let (Some(ally), Some(enemy)) = (ally_bot, enemy_bot) {
        let delta = ally - enemy;
        if delta.abs() >= BOT_LANE_GAP_THRESHOLD {
            gaps.push(RoleGap {
                lane: "Bot",
                gold_delta: delta,
            });
        }
    }

    gaps
}

fn lane_gold_input(
    participants: &[ParticipantGapInput<'_>],
    riot_position: &str,
    team_filter: Option<u16>,
) -> Option<i32> {
    let mut total = 0;
    let mut found = false;
    for p in participants {
        if p.team_position != riot_position {
            continue;
        }
        if let Some(team) = team_filter
            && p.team_id_value != team
        {
            continue;
        }
        total += p.gold_earned;
        found = true;
    }
    found.then_some(total)
}

fn lane_gold_input_excluding(
    participants: &[ParticipantGapInput<'_>],
    riot_position: &str,
    excluded_team: u16,
) -> Option<i32> {
    let mut total = 0;
    let mut found = false;
    for p in participants {
        if p.team_position != riot_position {
            continue;
        }
        if p.team_id_value == excluded_team {
            continue;
        }
        total += p.gold_earned;
        found = true;
    }
    found.then_some(total)
}

/// Renders a list of gaps as a single Discord-friendly summary string, or
/// `None` when the list is empty. Uses the convention "<Lane> gap" when the
/// tracked summoner's team is behind in that lane and "<Lane> diff" when
/// ahead. Gold deltas are formatted in thousands with one decimal.
pub fn format_role_gaps(gaps: &[RoleGap]) -> Option<String> {
    if gaps.is_empty() {
        return None;
    }
    let parts: Vec<String> = gaps
        .iter()
        .map(|g| {
            let kind = if g.gold_delta < 0 { "gap" } else { "diff" };
            let magnitude = (g.gold_delta as f32 / 1000.0).abs();
            let sign = if g.gold_delta < 0 { "-" } else { "+" };
            format!("{} {} ({}{:.1}k)", g.lane, kind, sign, magnitude)
        })
        .collect();
    Some(parts.join(", "))
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
    use super::{
        ParticipantGapInput, RoleGap, compute_role_gaps_from_inputs, diff_at_frame,
        format_role_gaps,
    };
    use riven::models::match_v5::{FramesTimeLine, ParticipantFrame, Position};
    use std::collections::HashMap;

    const BLUE: u16 = 100;
    const RED: u16 = 200;

    fn p(
        team_position: &'static str,
        team_id_value: u16,
        gold_earned: i32,
    ) -> ParticipantGapInput<'static> {
        ParticipantGapInput {
            team_position,
            team_id_value,
            gold_earned,
        }
    }

    /// Builds a stock 10-participant 5v5 vector with gold totals supplied
    /// per (team, position). Other positions get default gold for sanity.
    fn ten_participants(
        top: (i32, i32),
        jungle: (i32, i32),
        mid: (i32, i32),
        adc: (i32, i32),
        sup: (i32, i32),
    ) -> Vec<ParticipantGapInput<'static>> {
        vec![
            p("TOP", BLUE, top.0),
            p("TOP", RED, top.1),
            p("JUNGLE", BLUE, jungle.0),
            p("JUNGLE", RED, jungle.1),
            p("MIDDLE", BLUE, mid.0),
            p("MIDDLE", RED, mid.1),
            p("BOTTOM", BLUE, adc.0),
            p("BOTTOM", RED, adc.1),
            p("UTILITY", BLUE, sup.0),
            p("UTILITY", RED, sup.1),
        ]
    }

    #[test]
    fn no_gaps_when_lanes_are_close() {
        let participants = ten_participants(
            (12_000, 12_500),
            (10_000, 10_400),
            (13_000, 12_800),
            (14_000, 13_900),
            (8_000, 8_100),
        );
        let gaps = compute_role_gaps_from_inputs(&participants, BLUE);
        assert!(gaps.is_empty(), "unexpected gaps: {gaps:?}");
    }

    #[test]
    fn solo_lane_gap_above_threshold_is_reported() {
        let participants = ten_participants(
            (9_000, 13_500),
            (10_000, 10_000),
            (13_000, 13_000),
            (14_000, 13_900),
            (8_000, 8_100),
        );
        let gaps = compute_role_gaps_from_inputs(&participants, BLUE);
        assert_eq!(
            gaps,
            vec![RoleGap {
                lane: "Top",
                gold_delta: -4_500,
            }]
        );
    }

    #[test]
    fn perspective_flips_for_enemy_team() {
        let participants = ten_participants(
            (9_000, 13_500),
            (10_000, 10_000),
            (13_000, 13_000),
            (14_000, 13_900),
            (8_000, 8_100),
        );
        let gaps = compute_role_gaps_from_inputs(&participants, RED);
        assert_eq!(
            gaps,
            vec![RoleGap {
                lane: "Top",
                gold_delta: 4_500,
            }]
        );
    }

    #[test]
    fn bot_lane_combines_adc_and_support() {
        // ADC delta -2k, Support delta -3.5k → combined -5.5k crosses 5k threshold.
        // Neither solo-lane delta crosses 3k on its own.
        let participants = ten_participants(
            (12_000, 12_500),
            (10_000, 10_400),
            (13_000, 12_800),
            (14_000, 16_000),
            (6_000, 9_500),
        );
        let gaps = compute_role_gaps_from_inputs(&participants, BLUE);
        assert_eq!(
            gaps,
            vec![RoleGap {
                lane: "Bot",
                gold_delta: -5_500,
            }]
        );
    }

    #[test]
    fn multiple_gaps_returned_in_order() {
        let participants = ten_participants(
            (15_500, 10_000), // Top diff +5500
            (10_000, 13_500), // Jungle gap -3500
            (13_000, 12_800), // no gap
            (14_000, 16_000), // ADC -2k
            (6_000, 9_500),   // Sup -3500 → bot combined -5500
        );
        let gaps = compute_role_gaps_from_inputs(&participants, BLUE);
        assert_eq!(
            gaps,
            vec![
                RoleGap {
                    lane: "Top",
                    gold_delta: 5_500,
                },
                RoleGap {
                    lane: "Jungle",
                    gold_delta: -3_500,
                },
                RoleGap {
                    lane: "Bot",
                    gold_delta: -5_500,
                },
            ]
        );
    }

    #[test]
    fn threshold_is_inclusive_at_3000_for_solo_lanes() {
        // Exactly 3000 difference qualifies for the "gap" report.
        let participants = ten_participants(
            (12_000, 9_000),
            (10_000, 10_400),
            (13_000, 12_800),
            (14_000, 13_900),
            (8_000, 8_100),
        );
        let gaps = compute_role_gaps_from_inputs(&participants, BLUE);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].lane, "Top");
        assert_eq!(gaps[0].gold_delta, 3_000);
    }

    #[test]
    fn format_role_gaps_uses_gap_for_negative_diff_for_positive() {
        let summary = format_role_gaps(&[
            RoleGap {
                lane: "Bot",
                gold_delta: -5_500,
            },
            RoleGap {
                lane: "Top",
                gold_delta: 4_200,
            },
        ]);
        assert_eq!(
            summary.as_deref(),
            Some("Bot gap (-5.5k), Top diff (+4.2k)")
        );
    }

    #[test]
    fn format_role_gaps_returns_none_for_empty_list() {
        assert!(format_role_gaps(&[]).is_none());
    }

    #[test]
    fn partial_participant_list_only_reports_gaps_for_lanes_present() {
        // Only Top + Middle are populated (e.g. malformed match data missing other roles).
        // Top has both sides → 5k delta → gap. Mid is 1k → no gap. Jungle/Bot/Sup are absent → ignored.
        let participants = vec![
            p("TOP", BLUE, 10_000),
            p("TOP", RED, 15_000),
            p("MIDDLE", BLUE, 12_000),
            p("MIDDLE", RED, 13_000),
        ];
        let gaps = compute_role_gaps_from_inputs(&participants, BLUE);
        assert_eq!(
            gaps,
            vec![RoleGap {
                lane: "Top",
                gold_delta: -5_000,
            }]
        );
    }

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
