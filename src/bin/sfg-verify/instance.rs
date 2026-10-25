use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sqlx::Row;

use sfg_bot::db;
use sfg_bot::discord::commands;

use crate::error::{VerifyResult, err};
use crate::net::non_loopback_sockets;
use crate::postgres::{self, find_bins};
use crate::state::{InstanceState, is_loopback_database_url, redact_database_url};

pub async fn launch(
    home: Option<PathBuf>,
    evidence: Option<PathBuf>,
    fixture: Option<PathBuf>,
    region: String,
) -> VerifyResult<Value> {
    let run_id = new_run_id();
    let home = home.unwrap_or_else(|| PathBuf::from(format!("/tmp/sfg-bot-verify/{run_id}")));
    let evidence_dir =
        evidence.unwrap_or_else(|| PathBuf::from(format!("/tmp/sfg-bot-verify-evidence/{run_id}")));
    if evidence_dir.starts_with(&home) {
        return Err(err(format!(
            "evidence directory {} is inside the verification home {}. Put proof outside the home so cleanup can delete the instance without deleting it.",
            evidence_dir.display(),
            home.display()
        )));
    }
    if home.join(crate::state::STATE_FILE).exists() {
        return Err(err(format!(
            "verification home {} already has a state file. Run `sfg-verify --home {} cleanup` before launching again.",
            home.display(),
            home.display()
        )));
    }
    if home.exists() && fs::read_dir(&home)?.next().is_some() {
        return Err(err(format!(
            "verification home {} is not empty. Pick a new --home or remove it yourself; launch will not reuse an existing directory.",
            home.display()
        )));
    }
    let fixture_path = fixture
        .unwrap_or_else(|| PathBuf::from(".claude/skills/verify-sfg-bot/fixtures/riot.json"));
    if !fixture_path.is_file() {
        return Err(err(format!(
            "Riot fixture not found at {}. Run sfg-verify from the repository root, or pass --fixture.",
            fixture_path.display()
        )));
    }
    crate::fixture::FixtureRiot::load(&fixture_path)?;

    fs::create_dir_all(&home)?;
    fs::create_dir_all(&evidence_dir)?;
    let data_dir = home.join("pg");
    let bins = find_bins()?;
    let port = free_port()?;
    if let Err(error) = postgres::init_and_start(&data_dir, port, &bins) {
        let _ = fs::remove_dir_all(&home);
        return Err(error);
    }
    if let Err(error) = postgres::create_database(port) {
        let _ = postgres::stop(&data_dir, &bins);
        let _ = fs::remove_dir_all(&home);
        return Err(error);
    }

    let database_url = format!("postgres://sfgverify@127.0.0.1:{port}/sfg_verify");
    if !is_loopback_database_url(&database_url) {
        let _ = postgres::stop(&data_dir, &bins);
        let _ = fs::remove_dir_all(&home);
        return Err(err(
            "internal error: generated database URL is not loopback",
        ));
    }

    if let Err(error) = apply_migrations(&database_url).await {
        let _ = postgres::stop(&data_dir, &bins);
        let _ = fs::remove_dir_all(&home);
        return Err(error);
    }

    let state = InstanceState {
        run_id: run_id.clone(),
        home: home.clone(),
        evidence_dir: evidence_dir.clone(),
        pg_port: port,
        pg_data_dir: data_dir,
        database_url,
        default_region: region,
        fixture_path,
        build_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    state.save()?;
    let report = json!({
        "ok": true,
        "ready": true,
        "run_id": state.run_id,
        "home": state.home,
        "evidence_dir": state.evidence_dir,
        "pg_port": state.pg_port,
        "database_url": redact_database_url(&state.database_url),
        "default_region": state.default_region,
        "fixture_path": state.fixture_path,
        "build_version": state.build_version,
        "export": format!("SFG_VERIFY_HOME={}", state.home.display()),
        "note": "Migrations are applied and the summoners table is empty. The Discord gateway is not running. Pass this home to doctor, slash, show, and cleanup."
    });
    write_evidence(&evidence_dir, "launch", &report)?;
    Ok(report)
}

async fn apply_migrations(database_url: &str) -> VerifyResult<()> {
    let pool = db::create_pool(database_url).await?;
    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(|error| err(format!("database migrations failed: {error}")))?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM summoners")
        .fetch_one(&pool)
        .await?;
    if count != 0 {
        return Err(err(format!(
            "fresh database already contains {count} summoners after migrations; refusing to drive a non-empty roster"
        )));
    }
    Ok(())
}

pub async fn doctor(state: &InstanceState) -> VerifyResult<Value> {
    let mut problems = Vec::new();
    let pid = match postgres::postmaster_pid(&state.pg_data_dir) {
        Ok(pid) => Some(pid),
        Err(error) => {
            problems.push(error.to_string());
            None
        }
    };
    let mut owns_data_dir = false;
    if let Some(pid) = pid {
        match postgres::pid_owns_data_dir(pid, &state.pg_data_dir) {
            Ok(true) => owns_data_dir = true,
            Ok(false) => problems.push(format!(
                "pid {pid} from postmaster.pid is not a postgres process for {}",
                state.pg_data_dir.display()
            )),
            Err(error) => problems.push(error.to_string()),
        }
    }

    let mut server_port = None;
    let mut server_addr = None;
    let mut applied_migrations = Vec::new();
    let mut summoner_count = None;
    match db::create_pool(&state.database_url).await {
        Ok(pool) => {
            match sqlx::query("SELECT inet_server_addr()::text AS addr, inet_server_port() AS port")
                .fetch_one(&pool)
                .await
            {
                Ok(row) => {
                    server_addr = row.try_get::<String, _>("addr").ok();
                    server_port = row.try_get::<i32, _>("port").ok();
                    if server_port != Some(i32::from(state.pg_port)) {
                        problems.push(format!(
                            "connected port {server_port:?} does not match state port {}",
                            state.pg_port
                        ));
                    }
                    if server_addr.as_deref() != Some("127.0.0.1/32")
                        && server_addr.as_deref() != Some("127.0.0.1")
                    {
                        problems.push(format!(
                            "database server address is {server_addr:?}, expected 127.0.0.1"
                        ));
                    }
                }
                Err(error) => problems.push(format!("could not read server address: {error}")),
            }
            match sqlx::query("SELECT version FROM _sqlx_migrations WHERE success ORDER BY version")
                .fetch_all(&pool)
                .await
            {
                Ok(rows) => {
                    for row in rows {
                        if let Ok(version) = row.try_get::<i64, _>("version") {
                            applied_migrations.push(version);
                        }
                    }
                }
                Err(error) => problems.push(format!("could not read _sqlx_migrations: {error}")),
            }
            match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM summoners")
                .fetch_one(&pool)
                .await
            {
                Ok(count) => summoner_count = Some(count),
                Err(error) => problems.push(format!("summoners table is not usable: {error}")),
            }
        }
        Err(error) => problems.push(format!(
            "could not connect to the private database: {error}"
        )),
    }

    let expected: Vec<i64> = sqlx::migrate!()
        .migrations
        .iter()
        .map(|migration| migration.version)
        .collect();
    if applied_migrations != expected {
        problems.push(format!(
            "applied migrations {applied_migrations:?} do not match the binary's embedded migrations {expected:?}"
        ));
    }

    let registered_commands = registered_command_names();
    let expected_commands = [
        "analyze-last-game",
        "init-sfg-bot",
        "list-summoners",
        "add-summoner",
        "remove-summoner",
    ];
    if registered_commands != expected_commands {
        problems.push(format!(
            "register_all() returned {registered_commands:?}, expected {expected_commands:?}"
        ));
    }

    if !state.fixture_path.is_file() {
        problems.push(format!(
            "Riot fixture {} is missing",
            state.fixture_path.display()
        ));
    }

    let sockets = non_loopback_sockets().unwrap_or_default();
    let report = json!({
        "ok": problems.is_empty(),
        "ready": problems.is_empty(),
        "run_id": state.run_id,
        "home": state.home,
        "evidence_dir": state.evidence_dir,
        "build_version": env!("CARGO_PKG_VERSION"),
        "state_build_version": state.build_version,
        "git_rev": git_rev(),
        "pg_port": state.pg_port,
        "postmaster_pid": pid,
        "postmaster_owns_data_dir": owns_data_dir,
        "server_addr": server_addr,
        "server_port": server_port,
        "database_url": redact_database_url(&state.database_url),
        "database_host_loopback": is_loopback_database_url(&state.database_url),
        "applied_migrations": applied_migrations,
        "expected_migrations": expected,
        "summoner_count": summoner_count,
        "registered_commands": registered_commands,
        "default_region": state.default_region,
        "fixture_path": state.fixture_path,
        "discord_gateway": "not-started",
        "discord_bot_token_present": std::env::var_os("DISCORD_BOT_TOKEN").is_some(),
        "discord_bot_token_used": false,
        "riot_api_key_present": std::env::var_os("RIOT_API_KEY").is_some(),
        "riot_api_key_used": false,
        "llm_api_key_present": std::env::var_os("LLM_API_KEY").is_some(),
        "llm_api_key_used": false,
        "non_loopback_sockets": sockets,
        "problems": problems,
    });
    write_evidence(&state.evidence_dir, "doctor", &report)?;
    if !problems.is_empty() {
        eprintln!("doctor failed: {}", problems.join("; "));
    }
    Ok(report)
}

pub fn cleanup(state: &InstanceState, dry_run: bool) -> VerifyResult<Value> {
    let pid = postgres::postmaster_pid(&state.pg_data_dir).ok();
    let owns = match pid {
        Some(pid) => postgres::pid_owns_data_dir(pid, &state.pg_data_dir).unwrap_or(false),
        None => false,
    };
    if dry_run {
        let report = json!({
            "ok": true,
            "dry_run": true,
            "would_stop_pid": pid,
            "pid_owns_data_dir": owns,
            "would_remove": state.home,
            "evidence_kept": state.evidence_dir,
            "pg_port": state.pg_port,
        });
        write_evidence(&state.evidence_dir, "cleanup-dry-run", &report)?;
        return Ok(report);
    }
    if let Some(pid) = pid {
        if !owns {
            return Err(err(format!(
                "refusing to stop pid {pid}: its command line is not postgres for {}. No files were deleted.",
                state.pg_data_dir.display()
            )));
        }
        let bins = find_bins()?;
        postgres::stop(&state.pg_data_dir, &bins)?;
    }
    if state.home.exists() {
        fs::remove_dir_all(&state.home).map_err(|error| {
            err(format!(
                "stopped postgres but failed to remove {}: {error}. Evidence remains at {}.",
                state.home.display(),
                state.evidence_dir.display()
            ))
        })?;
    }
    let evidence_survived = state.evidence_dir.is_dir();
    let report = json!({
        "ok": true,
        "dry_run": false,
        "stopped_pid": pid,
        "removed": state.home,
        "evidence_dir": state.evidence_dir,
        "evidence_survived": evidence_survived,
    });
    write_evidence(&state.evidence_dir, "cleanup", &report)?;
    if !evidence_survived {
        return Err(err(format!(
            "cleanup removed the instance but the evidence directory {} is gone",
            state.evidence_dir.display()
        )));
    }
    Ok(report)
}

pub fn write_evidence(dir: &Path, name: &str, value: &Value) -> VerifyResult<PathBuf> {
    fs::create_dir_all(dir)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let path = dir.join(format!("{stamp}-{name}.json"));
    fs::write(&path, serde_json::to_string_pretty(value)?)?;
    fs::write(
        dir.join("latest.json"),
        serde_json::to_string_pretty(value)?,
    )?;
    Ok(path)
}

fn free_port() -> VerifyResult<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener
        .local_addr()
        .map_err(|error| err(error.to_string()))?
        .port();
    drop(listener);
    Ok(port)
}

fn new_run_id() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{secs}-{}", std::process::id())
}

fn git_rev() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|rev| rev.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn registered_command_names() -> Vec<String> {
    commands::register_all()
        .into_iter()
        .filter_map(|command| {
            serde_json::to_value(command)
                .ok()
                .and_then(|value| value.get("name")?.as_str().map(str::to_string))
        })
        .collect()
}
