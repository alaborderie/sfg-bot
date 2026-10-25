---
name: verify-sfg-bot
description: "Drive sfg-bot the way a Discord user does, without production Discord, Riot, or LLM calls. Use to launch a private verification instance, doctor it, exercise /init-sfg-bot, /add-summoner, /list-summoners, /remove-summoner, or /analyze-last-game, and capture proof."
---

# Verify sfg-bot

sfg-bot's user surface is Discord: slash commands and the French messages the bot replies with. Verification does not log into Discord and does not call Riot or an LLM. It starts a private Postgres, runs the same migrations the bot runs, and invokes the same command functions the gateway dispatches (`commands::run_*`). Serenity posts the reply to a stand-in on `127.0.0.1`. Account and match data come from `.claude/skills/verify-sfg-bot/fixtures/riot.json`.

Do not start `sfg-bot` itself for this loop. That binary reads `DISCORD_BOT_TOKEN` and `RIOT_API_KEY` and connects to the Discord gateway and Data Dragon.

`./target/debug/sfg-verify --help` lists commands, flags, and examples. stdout of every command except `--help` is one JSON object. stderr is a sentence when something fails.

## Launch

From the repository root, with PostgreSQL server binaries installed (`initdb`, `pg_ctl`, `postgres` under `/usr/lib/postgresql/<version>/bin`, plus `psql` and `pg_isready` on `PATH`). Debian/Ubuntu: `sudo apt-get install -y postgresql postgresql-client`. Set `SFG_VERIFY_PG_BIN` if the binaries live somewhere else. The harness never attaches to a cluster already listening on port 5432.

```bash
cargo build --bin sfg-verify
./target/debug/sfg-verify launch
```

Launch creates `/tmp/sfg-bot-verify/<run-id>/` (or `--home`), runs `initdb`, starts Postgres on `127.0.0.1` and a free port, creates database `sfg_verify`, and applies the embedded `sqlx::migrate!()` set. It refuses a home that already has files. Evidence is created at `/tmp/sfg-bot-verify-evidence/<run-id>/` (or `--evidence`) and must not be inside the home.

Ready means the JSON has `"ready": true` and `"ok": true`. That is only printed after migrations have run and `SELECT COUNT(*) FROM summoners` returned 0. A process that accepts TCP before migrations finish does not count. Export the home before any other command:

```bash
export SFG_VERIFY_HOME=/tmp/sfg-bot-verify/<run-id>
```

The value is the `home` field, also repeated as `export` in the JSON.

Two launches use two homes, two data directories, and two ports. Drive only the home you just started. Pass `--home` or `SFG_VERIFY_HOME` on every later command.

## Doctor

Run this first whenever a drive looks wrong. It is read-only.

```bash
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" doctor
```

Require all of these before driving:

- `"ok": true` and `"ready": true`
- `"postmaster_owns_data_dir": true` and `"postmaster_pid"` is the process from this home's `postmaster.pid`
- `"server_port"` matches `"pg_port"` and `"database_host_loopback": true`
- `"applied_migrations"` equals `"expected_migrations"`
- `"registered_commands"` is `analyze-last-game`, `init-sfg-bot`, `list-summoners`, `add-summoner`, `remove-summoner` in that order
- `"discord_gateway": "not-started"`, `"discord_bot_token_used": false`, `"riot_api_key_used": false`, `"llm_api_key_used": false`

`discord_bot_token_present` / `riot_api_key_present` / `llm_api_key_present` only say whether those variables are set in the environment. The harness does not read their values. A present token is not used and is not a reason to skip doctor.

Doctor writes a copy of the report into the evidence directory. Exit code is non-zero when `"ok"` is false.

## Drive

Read `features/README.md`, then the feature file for the change. The commands below are the real entry points. Riot lookups hit `FixtureRiot` (`RiotApiClient`), not the network. Discord replies hit the stand-in. The reqwest client passed to serenity is built with `no_proxy()` so `HTTPS_PROXY` cannot redirect it.

Harness identities, not a real Discord server:

| Handle | Value |
|---|---|
| Application | `900000000000000001` |
| Invoking user | `900000000000000002` (`verify-user`) |
| Guild | `900000000000000003` |
| Channel | `900000000000000004` |
| Fixture summoner | `VerifyFox#SFG` (`verify-puuid-sfg-bot-0001`) |
| Fixture match | `EUW1_9000000001` |

```bash
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash init-sfg-bot
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash init-sfg-bot --dm
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash add-summoner --summoner-name 'VerifyFox#SFG'
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash list-summoners
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash remove-summoner --summoner-name 'VerifyFox#SFG'
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash remove-summoner --summoner-name 'VerifyFox#SFG' --dry-run
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash analyze-last-game --summoner-name 'VerifyFox#SFG'
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" show summoners
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" show bot-config
```

A drive is worth keeping when `"ok": true`, `"non_loopback_sockets"` is `[]`, and `visible_text` contains the French string the feature file names. `show` is the database read-back. It does not replace the slash command the user would type.

`remove-summoner --dry-run` does not call `run_remove_summoner` and does not post to the stand-in. Confirm that by the `"dry_run": true` field and by the summoner still being present in a following `show summoners`. Do not treat a dry-run as proof that removal works.

Paths the harness does not drive, and the variable that would be required to drive them for real:

- Discord gateway session, slash-command registration, and channel delivery of game-start/end embeds: `DISCORD_BOT_TOKEN` plus `DISCORD_BOT_ID`. Skipped.
- Riot account, spectator, and match HTTP: `RIOT_API_KEY`. Skipped. The fixture is the stand-in at the existing `RiotApiClient` boundary.
- LLM coach text inside `/analyze-last-game`: `LLM_API_KEY`. Skipped. The harness always passes `analysis_pipeline = None`, which is the same branch the bot uses when the key is unset. The recap embed is still produced.
- Data Dragon champion refresh at process start. Skipped, because that is the production CDN.

## Evidence

Write proof under the `evidence_dir` from launch (`/tmp/sfg-bot-verify-evidence/<run-id>/` unless `--evidence` was set). Each command writes `{unix-millis}-{name}.json` and refreshes `latest.json`. Cleanup never deletes this directory.

A proof contains the action and the resulting state:

- The `slash` JSON: `discord` (method, path, body the handler posted) and `visible_text` (the French message or embed title, description, and fields).
- The same command's `summoners` or `bot_configs` array, plus a later `show` or `slash list-summoners` so the side effect is visible from a second user-facing read.
- `non_loopback_sockets: []` while the database pool and the stand-in are still open. That is the check that the drive did not open discord.com, Riot, or an LLM host. Do not trust the name of the fixture alone.
- Doctor's `"ready": true` from the same home, captured before the drive.

Screenshots do not apply. There is no browser UI. Do not cite `cargo test` as a substitute for a slash drive.

## Cleanup

Stop only the postmaster recorded for this home, then delete the home. Evidence stays.

```bash
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" cleanup --dry-run
./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" cleanup
```

`--dry-run` prints `would_stop_pid`, `would_remove`, and `evidence_kept` and does not signal a process. The real cleanup refuses to stop a pid whose `/proc/<pid>/cmdline` is not `postgres` for this data directory, and it does not delete the home in that case. It does not kill by process name. After cleanup, `"evidence_survived": true` and `test -d` on `evidence_dir` must succeed. Run cleanup after a failed launch too, if a home was printed, so a half-started cluster is not left behind. Launch itself stops Postgres and deletes the home when migrations fail before it prints a home.

## Helpers

The control CLI is the `sfg-verify` binary.

```bash
cargo build --bin sfg-verify
./target/debug/sfg-verify --help
./target/debug/sfg-verify launch --help
./target/debug/sfg-verify slash --help
./target/debug/sfg-verify cleanup --help
```

| Command | What it does |
|---|---|
| `launch` | Private Postgres, migrations, empty `summoners`, state file, evidence directory |
| `doctor` | Read-only ownership, migration, and command-list check |
| `slash <command>` | Calls `commands::run_*` and records the Discord HTTP body |
| `show summoners` / `show bot-config` | Reads those tables through `PgRepository` |
| `cleanup` | Stops this home's postmaster and deletes the home |

`--dry-run` exists on `cleanup` and on `slash remove-summoner`. Both leave the database and the process table unchanged.

New slash commands stay drivable only if their first parameter is `&impl serenity::http::CacheHttp`, which is what `interaction_create` already passes as `&ctx`. See `.claude/skills/add-sfg-bot-feature/SKILL.md`.

## Feature map

The behavior inventory is in [`features/README.md`](features/README.md). Each feature file uses the same four H2s: `Sub-features`, `How to get to it (user POV)`, `Driving it with sfg-verify`, `Gotchas`.

## Maintenance

When the commands, French copy, or fixture change, run `/maintain-verification-skill` so this map stays honest. Do not invent a cadence here.
