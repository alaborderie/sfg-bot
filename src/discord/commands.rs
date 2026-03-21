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
use serenity::model::permissions::Permissions;
use serenity::prelude::*;
use std::sync::Arc;

pub fn register_all() -> Vec<CreateCommand> {
    vec![
        register_analyze_last_game(),
        register_init_sfg_bot(),
        register_list_summoners(),
        register_add_summoner(),
        register_remove_summoner(),
    ]
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

fn register_list_summoners() -> CreateCommand {
    CreateCommand::new("list-summoners").description("Affiche la liste des invocateurs suivis")
}

fn register_add_summoner() -> CreateCommand {
    CreateCommand::new("add-summoner")
        .description("Ajoute un invocateur à la liste de suivi")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "summoner_name",
                "Nom d'invocateur au format : Nom#Tag",
            )
            .required(true),
        )
}

fn register_remove_summoner() -> CreateCommand {
    CreateCommand::new("remove-summoner")
        .description("Retire un invocateur de la liste de suivi")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "summoner_name",
                "Nom d'invocateur au format : Nom#Tag",
            )
            .required(true),
        )
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

    let required_permissions =
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::EMBED_LINKS;

    let permission_check = (|| {
        let guild = ctx.cache.guild(guild_id)?;
        let channel = guild.channels.get(&channel_id)?.clone();
        let bot_id = ctx.cache.current_user().id;
        let member = guild.members.get(&bot_id)?.clone();
        Some(guild.user_permissions_in(&channel, &member))
    })();

    match permission_check {
        Some(permissions) if !permissions.contains(required_permissions) => {
            let missing = required_permissions - permissions;
            let mut missing_names = Vec::new();
            if missing.contains(Permissions::VIEW_CHANNEL) {
                missing_names.push("Voir le salon");
            }
            if missing.contains(Permissions::SEND_MESSAGES) {
                missing_names.push("Envoyer des messages");
            }
            if missing.contains(Permissions::EMBED_LINKS) {
                missing_names.push("Intégrer des liens");
            }
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!(
                                "❌ Droits manquants : {}",
                                missing_names.join(", ")
                            ))
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
        None => {
            tracing::warn!(
                guild_id = guild_id.get(),
                channel_id = channel_id.get(),
                "Could not check bot permissions (guild/channel/member not in cache)"
            );
        }
        _ => {}
    }

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

pub async fn run_list_summoners(
    ctx: &Context,
    command: &serenity::model::application::CommandInteraction,
    repository: &Arc<dyn Repository>,
) {
    let summoners = match repository.get_all_summoners().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch summoners");
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(
                                "❌ Erreur lors de la récupération de la liste des invocateurs.",
                            )
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    let content = if summoners.is_empty() {
        "📋 Aucun invocateur suivi pour le moment.\nUtilise `/add-summoner` pour en ajouter."
            .to_string()
    } else {
        let mut lines = vec![format!("📋 **Invocateurs suivis ({})** :", summoners.len())];
        for summoner in &summoners {
            lines.push(format!(
                "• **{}#{}** ({})",
                summoner.game_name, summoner.tag_line, summoner.region
            ));
        }
        lines.join("\n")
    };

    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(content),
            ),
        )
        .await;
}

pub async fn run_add_summoner(
    ctx: &Context,
    command: &serenity::model::application::CommandInteraction,
    repository: &Arc<dyn Repository>,
    riot_client: &Arc<dyn RiotApiClient>,
    default_region: &str,
) {
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
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Merci de fournir un nom d'invocateur au format : `Nom#Tag`")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let Some(hash_pos) = summoner_input.rfind('#') else {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Format invalide. Utilise `Nom#Tag` (ex: `Doublelift#NA1`)")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    };

    let game_name = summoner_input[..hash_pos].trim();
    let tag_line = summoner_input[hash_pos + 1..].trim();

    if game_name.is_empty() || tag_line.is_empty() {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Format invalide. Utilise `Nom#Tag` (ex: `Doublelift#NA1`)")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let defer = CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new());
    if let Err(e) = command.create_response(&ctx.http, defer).await {
        tracing::error!("Failed to defer interaction: {}", e);
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
                "Failed to resolve summoner account for add-summoner"
            );
            let _ = command
                .create_followup(
                    &ctx.http,
                    CreateInteractionResponseFollowup::new().content(format!(
                        "❌ Compte `{summoner_input}` introuvable. Vérifie le nom et le tag."
                    )),
                )
                .await;
            return;
        }
    };

    match repository
        .upsert_summoner(
            &summoner_info.puuid,
            &summoner_info.game_name,
            &summoner_info.tag_line,
            default_region,
        )
        .await
    {
        Ok(_) => {
            tracing::info!(
                game_name = summoner_info.game_name.as_str(),
                tag_line = summoner_info.tag_line.as_str(),
                "Summoner added via /add-summoner"
            );
            let _ = command
                .create_followup(
                    &ctx.http,
                    CreateInteractionResponseFollowup::new().content(format!(
                        "✅ **{}#{}** ajouté à la liste de suivi !",
                        summoner_info.game_name, summoner_info.tag_line
                    )),
                )
                .await;
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to upsert summoner");
            let _ = command
                .create_followup(
                    &ctx.http,
                    CreateInteractionResponseFollowup::new()
                        .content("❌ Erreur lors de l'ajout de l'invocateur."),
                )
                .await;
        }
    }
}

pub async fn run_remove_summoner(
    ctx: &Context,
    command: &serenity::model::application::CommandInteraction,
    repository: &Arc<dyn Repository>,
) {
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
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Merci de fournir un nom d'invocateur au format : `Nom#Tag`")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let Some(hash_pos) = summoner_input.rfind('#') else {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Format invalide. Utilise `Nom#Tag` (ex: `Doublelift#NA1`)")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    };

    let game_name = summoner_input[..hash_pos].trim();
    let tag_line = summoner_input[hash_pos + 1..].trim();

    if game_name.is_empty() || tag_line.is_empty() {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Format invalide. Utilise `Nom#Tag` (ex: `Doublelift#NA1`)")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    match repository
        .delete_summoner_by_name_and_tag(game_name, tag_line)
        .await
    {
        Ok(true) => {
            tracing::info!(
                game_name = game_name,
                tag_line = tag_line,
                "Summoner removed via /remove-summoner"
            );
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!(
                            "✅ **{game_name}#{tag_line}** retiré de la liste de suivi."
                        )),
                    ),
                )
                .await;
        }
        Ok(false) => {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!(
                                "❌ Invocateur `{game_name}#{tag_line}` introuvable dans la liste de suivi."
                            ))
                            .ephemeral(true),
                    ),
                )
                .await;
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to delete summoner");
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("❌ Erreur lors de la suppression de l'invocateur.")
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
