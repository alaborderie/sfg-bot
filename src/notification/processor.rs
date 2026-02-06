use crate::db::models::{NotificationEvent, Summoner};
use crate::db::repository::Repository;
use crate::notification::messages::{format_grouped_game_ended, format_grouped_game_started};
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct NotificationProcessor {
    repository: Arc<dyn Repository>,
    ctx: Context,
    channel_id: ChannelId,
    interval_secs: u64,
}

impl NotificationProcessor {
    pub fn new(
        repository: Arc<dyn Repository>,
        ctx: Context,
        channel_id: ChannelId,
        interval_secs: u64,
    ) -> Self {
        Self {
            repository,
            ctx,
            channel_id,
            interval_secs,
        }
    }

    pub async fn start(self) {
        loop {
            if let Err(e) = self.process_pending_events().await {
                tracing::error!("Error processing notification events: {}", e);
            }

            tokio::time::sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }

    async fn process_pending_events(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let events = self.repository.get_pending_notification_events().await?;

        if events.is_empty() {
            return Ok(());
        }

        let now = chrono::Utc::now();
        let wait_threshold = Duration::from_secs(30);

        let mut game_started_groups: HashMap<i64, Vec<NotificationEvent>> = HashMap::new();
        let mut game_ended_groups: HashMap<String, Vec<NotificationEvent>> = HashMap::new();

        for event in events {
            let event_age = now.signed_duration_since(event.created_at);
            
            if event_age.to_std().unwrap_or(Duration::ZERO) < wait_threshold {
                continue;
            }

            match event.event_type.as_str() {
                "GAME_STARTED" => {
                    game_started_groups
                        .entry(event.game_id)
                        .or_insert_with(Vec::new)
                        .push(event);
                }
                "GAME_ENDED" => {
                    if let Some(ref match_id) = event.match_id {
                        game_ended_groups
                            .entry(match_id.clone())
                            .or_insert_with(Vec::new)
                            .push(event);
                    }
                }
                _ => {}
            }
        }

        for (game_id, group_events) in game_started_groups {
            if let Err(e) = self
                .send_grouped_game_started(game_id, group_events)
                .await
            {
                tracing::error!("Failed to send grouped game started notification: {}", e);
            }
        }

        for (match_id, group_events) in game_ended_groups {
            if let Err(e) = self
                .send_grouped_game_ended(&match_id, group_events)
                .await
            {
                tracing::error!("Failed to send grouped game ended notification: {}", e);
            }
        }

        Ok(())
    }

    async fn send_grouped_game_started(
        &self,
        game_id: i64,
        events: Vec<NotificationEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let summoner_ids: Vec<_> = events.iter().map(|e| e.summoner_id).collect();
        let event_ids: Vec<_> = events.iter().map(|e| e.id).collect();

        let summoners = self.fetch_summoners(&summoner_ids).await?;

        let champions: Vec<_> = events
            .iter()
            .map(|e| (e.summoner_id, e.champion_name.clone()))
            .collect();

        let game_mode = events
            .first()
            .map(|e| e.game_mode.as_str())
            .unwrap_or("UNKNOWN");

        let message = format_grouped_game_started(&summoners, &champions, game_mode);

        self.channel_id.say(&self.ctx.http, &message).await?;

        self.repository
            .mark_notifications_processed(&event_ids)
            .await?;

        tracing::info!(
            "Sent grouped game started notification for game {} with {} players",
            game_id,
            events.len()
        );

        Ok(())
    }

    async fn send_grouped_game_ended(
        &self,
        match_id: &str,
        events: Vec<NotificationEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let summoner_ids: Vec<_> = events.iter().map(|e| e.summoner_id).collect();
        let event_ids: Vec<_> = events.iter().map(|e| e.id).collect();

        let summoners = self.fetch_summoners(&summoner_ids).await?;

        let message = format_grouped_game_ended(&summoners, &events);

        self.channel_id.say(&self.ctx.http, &message).await?;

        self.repository
            .mark_notifications_processed(&event_ids)
            .await?;

        tracing::info!(
            "Sent grouped game ended notification for match {} with {} players",
            match_id,
            events.len()
        );

        Ok(())
    }

    async fn fetch_summoners(
        &self,
        summoner_ids: &[uuid::Uuid],
    ) -> Result<Vec<Summoner>, Box<dyn std::error::Error + Send + Sync>> {
        let all_summoners = self.repository.get_all_summoners().await?;
        let summoners: Vec<_> = all_summoners
            .into_iter()
            .filter(|s| summoner_ids.contains(&s.id))
            .collect();
        Ok(summoners)
    }
}
