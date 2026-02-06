use crate::db::models::{NotificationEvent, Summoner};
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::model::{Colour, Timestamp};
use std::collections::HashMap;
use uuid::Uuid;

pub fn format_grouped_game_started(
    summoners: &[Summoner],
    champions: &[(Uuid, String, String)],
    game_mode: &str,
) -> CreateEmbed {
    let summoner_names: Vec<String> = summoners.iter().map(|s| s.game_name.clone()).collect();

    let description = format!(
        "{} started a {} game",
        format_list(&summoner_names),
        game_mode
    );

    let champion_map: HashMap<Uuid, String> = champions
        .iter()
        .map(|(id, name, _)| (*id, name.clone()))
        .collect();

    let mut embed = CreateEmbed::new()
        .title("🎮 Game Started!")
        .description(description)
        .colour(Colour::from_rgb(52, 152, 219))
        .footer(CreateEmbedFooter::new(format!(
            "League of Legends · {}",
            game_mode
        )))
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
        Colour::from_rgb(46, 204, 113) // Green
    } else if wins == 0 {
        Colour::from_rgb(231, 76, 60) // Red
    } else {
        Colour::from_rgb(241, 196, 15) // Gold/Orange
    };

    let mut embed = CreateEmbed::new()
        .title(format!("{} Wins, {} Losses", wins, losses))
        .description("Game ended! Check your stats.")
        .colour(color)
        .footer(CreateEmbedFooter::new("Match results saved to history"))
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

            // "💎 Champion · Role · W 10/2/5"
            let value_field = format!("💎 {} · {} · {} {}", champion_name, role, result_char, kda);

            embed = embed.field(name_field, value_field, true);
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
