use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde_json::json;

use crate::drive::{ShowWhat, SlashRequest, show, slash};
use crate::error::{VerifyResult, err};
use crate::instance::{cleanup, doctor, launch};
use crate::state::{InstanceState, require_home};

mod discord_standin;
mod drive;
mod error;
mod fixture;
mod instance;
mod net;
mod postgres;
mod state;

const AFTER_HELP: &str = "\
Examples:
  sfg-verify launch
  sfg-verify --home /tmp/sfg-bot-verify/<id> doctor
  sfg-verify --home /tmp/sfg-bot-verify/<id> slash list-summoners
  sfg-verify --home /tmp/sfg-bot-verify/<id> slash add-summoner --summoner-name 'VerifyFox#SFG'
  sfg-verify --home /tmp/sfg-bot-verify/<id> slash remove-summoner --summoner-name 'VerifyFox#SFG' --dry-run
  sfg-verify --home /tmp/sfg-bot-verify/<id> show summoners
  sfg-verify --home /tmp/sfg-bot-verify/<id> cleanup --dry-run
  sfg-verify --home /tmp/sfg-bot-verify/<id> cleanup

Every command except --help prints one JSON object on stdout. Failures also print a sentence on stderr and exit non-zero.
The harness never reads DISCORD_BOT_TOKEN, RIOT_API_KEY, or LLM_API_KEY, and it never opens the Discord gateway.
";

#[derive(Parser)]
#[command(
    name = "sfg-verify",
    version,
    about = "Drive sfg-bot slash commands on a private Postgres with a localhost Discord stand-in",
    long_about = "Launch an isolated sfg-bot verification instance, check that it is the one you started, invoke the real slash-command handlers, and keep the captured Discord payloads after cleanup.\n\nRiot account and match data come from the fixture file. Discord replies are recorded by a stand-in on 127.0.0.1. Nothing is sent to discord.com, the Riot API, or an LLM.",
    after_help = AFTER_HELP
)]
struct Cli {
    /// Verification home printed by `launch`. Overrides SFG_VERIFY_HOME.
    #[arg(long, env = "SFG_VERIFY_HOME", global = true)]
    home: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a private Postgres, run the bot's migrations, and write state.json.
    Launch {
        /// Proof directory. Must not live inside the home. Default: /tmp/sfg-bot-verify-evidence/<run-id>.
        #[arg(long, env = "SFG_VERIFY_EVIDENCE")]
        evidence: Option<PathBuf>,
        /// Riot fixture JSON. Default: .claude/skills/verify-sfg-bot/fixtures/riot.json.
        #[arg(long, env = "SFG_VERIFY_FIXTURE")]
        fixture: Option<PathBuf>,
        /// Region stored on summoners added by /add-summoner. Default: euw1.
        #[arg(long, default_value = "euw1")]
        region: String,
    },
    /// Read-only check: our postmaster, loopback port, migrations, and command list.
    Doctor,
    /// Invoke one real slash-command handler and record the Discord HTTP body.
    Slash {
        #[command(subcommand)]
        command: SlashCommand,
    },
    /// Read roster or channel config back from the verification database.
    Show {
        #[command(subcommand)]
        what: ShowCommand,
    },
    /// Stop the postmaster recorded in state.json and delete the home. Keeps evidence.
    Cleanup {
        /// Print the pid and paths that would be removed. Does not signal any process.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum SlashCommand {
    /// Configure the notification channel for the harness guild.
    InitSfgBot {
        /// Omit the guild id and expect the in-server rejection.
        #[arg(long)]
        dm: bool,
        /// Guild snowflake. Default: the harness guild 900000000000000003.
        #[arg(long)]
        guild_id: Option<u64>,
        /// Channel snowflake. Default: the harness channel 900000000000000004.
        #[arg(long)]
        channel_id: Option<u64>,
    },
    /// Resolve Name#Tag through the Riot fixture and insert the summoner.
    AddSummoner {
        /// Riot id, for example VerifyFox#SFG.
        #[arg(long)]
        summoner_name: String,
    },
    /// Reply with the tracked summoners, or the empty-roster message.
    ListSummoners,
    /// Delete a tracked summoner by Name#Tag.
    RemoveSummoner {
        /// Riot id, for example VerifyFox#SFG.
        #[arg(long)]
        summoner_name: String,
        /// Report whether the row would be deleted. Does not call the handler.
        #[arg(long)]
        dry_run: bool,
    },
    /// Post the last-match recap embed from the fixture. Does not call an LLM.
    AnalyzeLastGame {
        /// Riot id, for example VerifyFox#SFG.
        #[arg(long)]
        summoner_name: String,
    },
}

#[derive(Subcommand)]
enum ShowCommand {
    /// Rows in summoners.
    Summoners,
    /// Rows in bot_config.
    BotConfig,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("sfg_verify=info,sfg_bot=info")
            }),
        )
        .init();

    let cli = Cli::parse();
    let result = dispatch(cli).await;
    match result {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).unwrap_or_else(|error| {
                    format!(r#"{{"ok":false,"error":"could not encode JSON: {error}"}}"#)
                })
            );
            if value.get("ok").and_then(serde_json::Value::as_bool) == Some(false) {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"ok": false, "error": error.to_string()}))
                    .unwrap_or_else(|_| r#"{"ok":false,"error":"unprintable error"}"#.to_string())
            );
            std::process::exit(1);
        }
    }
}

async fn dispatch(cli: Cli) -> VerifyResult<serde_json::Value> {
    match cli.command {
        Commands::Launch {
            evidence,
            fixture,
            region,
        } => launch(cli.home, evidence, fixture, region).await,
        Commands::Doctor => {
            let home = require_home(cli.home)?;
            let state = InstanceState::load(&home)?;
            doctor(&state).await
        }
        Commands::Slash { command } => {
            let home = require_home(cli.home)?;
            let state = InstanceState::load(&home)?;
            let request = slash_request(command)?;
            slash(&state, request).await
        }
        Commands::Show { what } => {
            let home = require_home(cli.home)?;
            let state = InstanceState::load(&home)?;
            let what = match what {
                ShowCommand::Summoners => ShowWhat::Summoners,
                ShowCommand::BotConfig => ShowWhat::BotConfig,
            };
            show(&state, what).await
        }
        Commands::Cleanup { dry_run } => {
            let home = require_home(cli.home)?;
            let state = InstanceState::load(&home)?;
            cleanup(&state, dry_run)
        }
    }
}

fn slash_request(command: SlashCommand) -> VerifyResult<SlashRequest> {
    match command {
        SlashCommand::InitSfgBot {
            dm,
            guild_id,
            channel_id,
        } => {
            if dm && guild_id.is_some() {
                return Err(err(
                    "--dm omits the guild, so it cannot be combined with --guild-id. Pass one of them.",
                ));
            }
            Ok(SlashRequest {
                command: "init-sfg-bot",
                summoner_name: None,
                guild_id: if dm {
                    None
                } else {
                    Some(guild_id.unwrap_or(discord_standin::GUILD_ID))
                },
                channel_id: channel_id.unwrap_or(discord_standin::CHANNEL_ID),
                dry_run: false,
            })
        }
        SlashCommand::AddSummoner { summoner_name } => Ok(SlashRequest {
            command: "add-summoner",
            summoner_name: Some(summoner_name),
            guild_id: Some(discord_standin::GUILD_ID),
            channel_id: discord_standin::CHANNEL_ID,
            dry_run: false,
        }),
        SlashCommand::ListSummoners => Ok(SlashRequest {
            command: "list-summoners",
            summoner_name: None,
            guild_id: Some(discord_standin::GUILD_ID),
            channel_id: discord_standin::CHANNEL_ID,
            dry_run: false,
        }),
        SlashCommand::RemoveSummoner {
            summoner_name,
            dry_run,
        } => Ok(SlashRequest {
            command: "remove-summoner",
            summoner_name: Some(summoner_name),
            guild_id: Some(discord_standin::GUILD_ID),
            channel_id: discord_standin::CHANNEL_ID,
            dry_run,
        }),
        SlashCommand::AnalyzeLastGame { summoner_name } => Ok(SlashRequest {
            command: "analyze-last-game",
            summoner_name: Some(summoner_name),
            guild_id: Some(discord_standin::GUILD_ID),
            channel_id: discord_standin::CHANNEL_ID,
            dry_run: false,
        }),
    }
}
