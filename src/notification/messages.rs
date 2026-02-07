use crate::db::models::{NotificationEvent, Summoner};
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::model::{Colour, Timestamp};
use std::collections::HashMap;
use uuid::Uuid;

/// Convert a Riot API queue_id to a friendly name
fn get_queue_type_name(queue_id: Option<i32>) -> String {
    match queue_id {
        Some(420) => "Ranked Solo/Duo".to_string(),
        Some(440) => "Ranked Flex".to_string(),
        Some(400) => "Draft Pick".to_string(),
        Some(430) => "Blind Pick".to_string(),
        Some(450) => "ARAM".to_string(),
        Some(490) => "Quickplay".to_string(),
        Some(1700) => "Arena".to_string(),
        Some(id) => format!("Queue {}", id),
        None => "Unknown".to_string(),
    }
}

pub fn format_grouped_game_started(
    summoners: &[Summoner],
    champions: &[(Uuid, String, String)],
    game_mode: &str,
    queue_id: Option<i32>,
) -> CreateEmbed {
    let summoner_names: Vec<String> = summoners.iter().map(|s| s.game_name.clone()).collect();

    let queue_type = get_queue_type_name(queue_id);
    let description = format!(
        "{} started a {} game ({})",
        format_list(&summoner_names),
        game_mode,
        queue_type
    );

    let champion_map: HashMap<Uuid, String> = champions
        .iter()
        .map(|(id, name, _)| (*id, name.clone()))
        .collect();

    let footer_text = format!("League of Legends · {} · {}", game_mode, queue_type);

    let mut embed = CreateEmbed::new()
        .title("🎮 Game Started!")
        .description(description)
        .colour(Colour::from_rgb(52, 152, 219))
        .footer(CreateEmbedFooter::new(footer_text))
        .timestamp(Timestamp::now());

    for summoner in summoners {
        let name = format!("{}#{}", summoner.game_name, summoner.tag_line);
        let champion = champion_map
            .get(&summoner.id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        embed = embed.field(name, champion, true);
    }

    embed
}

pub fn format_grouped_game_ended(
    summoners: &[Summoner],
    events: &[NotificationEvent],
    game_mode: &str,
) -> CreateEmbed {
    let event_map: HashMap<Uuid, &NotificationEvent> =
        events.iter().map(|e| (e.summoner_id, e)).collect();

    let mut wins = 0;
    let mut losses = 0;

    for event in events {
        if event.win.unwrap_or(false) {
            wins += 1;
        } else {
            losses += 1;
        }
    }

    let color = if losses == 0 {
        Colour::from_rgb(46, 204, 113)
    } else if wins == 0 {
        Colour::from_rgb(231, 76, 60)
    } else {
        Colour::from_rgb(241, 196, 15)
    };

    let is_featured_mode = events
        .first()
        .map(|event| event.is_featured_mode)
        .unwrap_or(false);

    let queue_type = events
        .first()
        .and_then(|event| event.queue_id)
        .map(|qid| get_queue_type_name(Some(qid)));

    let description = if is_featured_mode {
        format!(
            "{} featured mode ended! Match history may take a bit to update.",
            game_mode
        )
    } else {
        format!("{} game ended! Check your stats.", game_mode)
    };

    let footer_text = if let Some(queue) = queue_type {
        if is_featured_mode {
            format!("League of Legends · {} · {} (Featured)", game_mode, queue)
        } else {
            format!("League of Legends · {} · {}", game_mode, queue)
        }
    } else if is_featured_mode {
        format!("League of Legends · {} (Featured)", game_mode)
    } else {
        format!("League of Legends · {}", game_mode)
    };

    let mut embed = CreateEmbed::new()
        .title(if wins > losses {
            "Game Won!"
        } else {
            "Game Lost!"
        })
        .description(description)
        .colour(color)
        .footer(CreateEmbedFooter::new(footer_text))
        .timestamp(Timestamp::now());

    for summoner in summoners {
        if let Some(event) = event_map.get(&summoner.id) {
            let is_win = event.win.unwrap_or(false);

            let name_prefix = if is_win { "🏆" } else { "💔" };
            let name_field = format!("{} {}", name_prefix, summoner.game_name);

            let kda = format!(
                "{}/{}/{}",
                event.kills.unwrap_or(0),
                event.deaths.unwrap_or(0),
                event.assists.unwrap_or(0)
            );

            let result_char = if is_win { "W" } else { "L" };
            let role = event.role.as_deref().unwrap_or("Unknown");
            let champion_name = &event.champion_name;

            let value_field = format!("💎 {} · {} · {} {}", champion_name, role, result_char, kda);

            let stats_line = format_stats_line(
                event.total_cs.unwrap_or(0),
                event.total_gold.unwrap_or(0),
                event.total_damage.unwrap_or(0),
                event.game_duration_secs.unwrap_or(0),
            );

            let enemy_line = format_enemy_comparison(
                event.enemy_champion_name.as_deref(),
                event.enemy_cs,
                event.enemy_gold,
                event.enemy_damage,
                event.game_duration_secs.unwrap_or(0),
            );

            embed = embed
                .field(name_field, value_field, true)
                .field("📊 Stats", stats_line, true)
                .field("⚔️ vs", enemy_line, true);
        }
    }

    embed
}

fn format_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let last = items.last().unwrap();
            let rest = &items[..items.len() - 1];
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

fn format_stats_line(cs: i32, gold: i32, damage: i32, game_duration_secs: i32) -> String {
    let minutes = game_duration_secs as f64 / 60.0;
    let cs_per_min = if minutes > 0.0 {
        cs as f64 / minutes
    } else {
        0.0
    };
    let gold_per_min = if minutes > 0.0 {
        gold as f64 / minutes
    } else {
        0.0
    };

    let dmg_str = if damage > 1000 {
        format!("{:.1}k", damage as f64 / 1000.0)
    } else {
        format!("{}", damage)
    };

    format!(
        "{:.1} CS/min · {:.0} GPM · {} dmg",
        cs_per_min, gold_per_min, dmg_str
    )
}

fn format_enemy_comparison(
    enemy_champion: Option<&str>,
    enemy_cs: Option<i32>,
    enemy_gold: Option<i32>,
    enemy_damage: Option<i32>,
    game_duration_secs: i32,
) -> String {
    match (enemy_champion, enemy_cs, enemy_gold, enemy_damage) {
        (Some(champ), Some(cs), Some(gold), Some(dmg)) => {
            let stats = format_stats_line(cs, gold, dmg, game_duration_secs);
            format!("{} ({})", champ, stats)
        }
        _ => "⚔️ vs Unknown (no role data)".to_string(),
    }
}
