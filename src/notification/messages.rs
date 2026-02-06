use crate::db::models::{NotificationEvent, Summoner};
use uuid::Uuid;

pub fn format_grouped_game_started(
    summoners: &[Summoner],
    champions: &[(Uuid, String)],
    game_mode: &str,
) -> String {
    let summoner_names: Vec<String> = summoners
        .iter()
        .map(|s| format!("{}#{}", s.game_name, s.tag_line))
        .collect();

    let champion_map: std::collections::HashMap<Uuid, String> = champions.iter().cloned().collect();

    let champion_names: Vec<String> = summoners
        .iter()
        .filter_map(|s| champion_map.get(&s.id).cloned())
        .collect();

    let players = format_list(&summoner_names);
    let champs = format_list(&champion_names);

    format!("🎮 {} started a {} game as {}!", players, game_mode, champs)
}

pub fn format_grouped_game_ended(summoners: &[Summoner], events: &[NotificationEvent]) -> String {
    let event_map: std::collections::HashMap<Uuid, &NotificationEvent> =
        events.iter().map(|e| (e.summoner_id, e)).collect();

    let mut parts = Vec::new();

    for summoner in summoners {
        if let Some(event) = event_map.get(&summoner.id) {
            let summoner_name = format!("{}#{}", summoner.game_name, summoner.tag_line);
            let result = if event.win.unwrap_or(false) {
                "🏆"
            } else {
                "💔"
            };

            let kda = format!(
                "{}/{}/{}",
                event.kills.unwrap_or(0),
                event.deaths.unwrap_or(0),
                event.assists.unwrap_or(0)
            );

            let role = event
                .role
                .as_ref()
                .map(|r| format!(" | {}", r))
                .unwrap_or_default();

            let part = format!(
                "{} {} | KDA: {} | {}{}",
                result, summoner_name, kda, event.champion_name, role
            );
            parts.push(part);
        }
    }

    parts.join("\n")
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
