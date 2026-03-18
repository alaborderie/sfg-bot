use crate::analysis::discord::{format_analysis_embed, format_analysis_error_embed};
use crate::analysis::pipeline::AnalysisPipeline;
use crate::db::repository::Repository;
use crate::notification::messages::format_single_game_ended;
use crate::riot::client::{RiotApiClient, RiotClient};
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
};
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::prelude::*;
use std::sync::Arc;

pub fn register_all() -> Vec<CreateCommand> {
    vec![register_analyze_last_game(), register_init_sfg_bot()]
}

fn register_analyze_last_game() -> CreateCommand {
    CreateCommand::new("analyze-last-game")
        .description("Analyse la dernière partie d'un invocateur")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "summoner_name",
                "Nom d'invocateur au format : Nom#Tag",
            )
            .required(true),
        )
}

fn register_init_sfg_bot() -> CreateCommand {
    CreateCommand::new("init-sfg-bot")
        .description("Configure ce salon comme salon de notifications du bot")
}

pub async fn run_init_sfg_bot(
    ctx: &Context,
    command: &serenity::model::application::CommandInteraction,
    repository: &Arc<dyn Repository>,
) {
    let Some(guild_id) = command.guild_id else {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Cette commande ne peut être utilisée que dans un serveur.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    };

    let channel_id = command.channel_id;

    match repository
        .upsert_bot_config(guild_id.get() as i64, channel_id.get() as i64)
        .await
    {
        Ok(_) => {
            tracing::info!(
                guild_id = guild_id.get(),
                channel_id = channel_id.get(),
                "Bot config saved via /init-sfg-bot"
            );
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!(
                            "✅ Notifications configurées dans <#{}>. Les alertes de parties seront envoyées ici !",
                            channel_id.get()
                        )),
                    ),
                )
                .await;
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to save bot config");
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("❌ Erreur lors de la sauvegarde de la configuration.")
                            .ephemeral(true),
                    ),
                )
                .await;
        }
    }
}

pub async fn run(
    ctx: &Context,
    command: &serenity::model::application::CommandInteraction,
    riot_client: &Arc<dyn RiotApiClient>,
    analysis_pipeline: &Option<Arc<AnalysisPipeline>>,
    default_region: &str,
) {
    let defer = CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new());
    if let Err(e) = command.create_response(&ctx.http, defer).await {
        tracing::error!("Failed to defer interaction: {}", e);
        return;
    }

    let options = command.data.options();
    let summoner_input = options
        .iter()
        .find_map(|opt| {
            if opt.name == "summoner_name" {
                if let ResolvedValue::String(s) = opt.value {
                    Some(s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or_default();

    if summoner_input.is_empty() {
        send_error_followup(
            ctx,
            command,
            "Merci de fournir un nom d'invocateur au format : `Nom#Tag`",
        )
        .await;
        return;
    }

    let Some(hash_pos) = summoner_input.rfind('#') else {
        send_error_followup(
            ctx,
            command,
            "Format invalide. Utilise `Nom#Tag` (ex: `Doublelift#NA1`)",
        )
        .await;
        return;
    };

    let game_name = summoner_input[..hash_pos].trim();
    let tag_line = summoner_input[hash_pos + 1..].trim();

    if game_name.is_empty() || tag_line.is_empty() {
        send_error_followup(
            ctx,
            command,
            "Format invalide. Utilise `Nom#Tag` (ex: `Doublelift#NA1`)",
        )
        .await;
        return;
    }

    let region = RiotClient::regional_for_region(default_region);

    let summoner_info = match riot_client
        .get_account_by_riot_id(game_name, tag_line, region)
        .await
    {
        Ok(info) => info,
        Err(e) => {
            tracing::warn!(
                summoner = summoner_input.as_str(),
                error = %e,
                "Failed to resolve summoner account"
            );
            send_error_followup(
                ctx,
                command,
                &format!("Compte `{summoner_input}` introuvable. Vérifie le nom et le tag."),
            )
            .await;
            return;
        }
    };

    let match_id = match riot_client
        .get_recent_match_id(&summoner_info.puuid, region)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            send_error_followup(
                ctx,
                command,
                &format!("Aucune partie récente trouvée pour `{summoner_input}`."),
            )
            .await;
            return;
        }
        Err(e) => {
            tracing::warn!(
                summoner = summoner_input.as_str(),
                error = %e,
                "Failed to fetch recent match ID"
            );
            send_error_followup(
                ctx,
                command,
                "Impossible de récupérer les données de la dernière partie depuis l'API Riot.",
            )
            .await;
            return;
        }
    };

    let match_result = match riot_client
        .get_match_result(&match_id, &summoner_info.puuid, region)
        .await
    {
        Ok(Some(result)) => result,
        Ok(None) => {
            send_error_followup(
                ctx,
                command,
                "Impossible de récupérer les détails de la partie.",
            )
            .await;
            return;
        }
        Err(e) => {
            tracing::warn!(
                summoner = summoner_input.as_str(),
                match_id = match_id.as_str(),
                error = %e,
                "Failed to fetch match result"
            );
            send_error_followup(
                ctx,
                command,
                "Impossible de récupérer les détails de la partie depuis l'API Riot.",
            )
            .await;
            return;
        }
    };

    let summoner_display = format!("{}#{}", summoner_info.game_name, summoner_info.tag_line);
    let recap_embed = format_single_game_ended(&summoner_display, &match_result);

    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new().embed(recap_embed),
        )
        .await
    {
        tracing::error!("Failed to send recap followup: {}", e);
    }

    let Some(pipeline) = analysis_pipeline else {
        return;
    };

    let analysis_data = match riot_client
        .get_match_analysis_data(&match_id, &summoner_info.puuid, &summoner_display, region)
        .await
    {
        Ok(Some(data)) => data,
        Ok(None) => {
            let embed = format_analysis_error_embed(&summoner_display, "match data not found");
            let _ = command
                .create_followup(
                    &ctx.http,
                    CreateInteractionResponseFollowup::new().embed(embed),
                )
                .await;
            return;
        }
        Err(e) => {
            tracing::warn!(
                summoner = summoner_display.as_str(),
                error = %e,
                "Failed to fetch analysis data"
            );
            let embed =
                format_analysis_error_embed(&summoner_display, "could not retrieve match data");
            let _ = command
                .create_followup(
                    &ctx.http,
                    CreateInteractionResponseFollowup::new().embed(embed),
                )
                .await;
            return;
        }
    };

    let result = pipeline.analyze_game(&analysis_data).await;

    let embed = if result.error.is_some() {
        format_analysis_error_embed(
            &summoner_display,
            result.error.as_deref().unwrap_or("unknown error"),
        )
    } else {
        format_analysis_embed(&result)
    };

    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new().embed(embed),
        )
        .await
    {
        tracing::error!("Failed to send analysis followup: {}", e);
    }
}

async fn send_error_followup(
    ctx: &Context,
    command: &serenity::model::application::CommandInteraction,
    message: &str,
) {
    let _ = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new().content(message),
        )
        .await;
}
