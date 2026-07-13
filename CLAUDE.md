# sfg-bot

Discord bot that tracks SouthFoxGaming League of Legends games — detects game start/end, posts notifications to a Discord channel, and optionally runs AI post-game analysis via a self-hosted LLM (Gemma 4 behind an OpenAI-compatible API).

## Tech Stack

- **Rust** (edition 2024, rust-version 1.93)
- **serenity 0.12** — Discord gateway, slash commands, embeds
- **riven 2** — Riot Games API client
- **sqlx 0.8** — Async PostgreSQL (raw SQL, no ORM)
- **tokio 1** — Async runtime
- **reqwest** — HTTP client (LLM API)
- **tracing** — Structured logging (env-filter, JSON format)
- **thiserror 2** — Error type derivation
- **mockall 0.13** — Mock generation (behind `test-mocks` feature flag)

## Architecture

### Core Flow

1. **main.rs** — Init tracing, load config from env, create DB pool, run migrations, cache champions, start Discord client
2. **Per-summoner polling** — Each tracked summoner gets a `tokio::spawn` loop that polls Riot API for active games
3. **State machine** — `GameTracker` detects `GameStarted`, `GameEnded`, `FeaturedModeGameEnded`, `NoChange`
4. **Event queue** — State changes insert `NotificationEvent` rows into PostgreSQL
5. **Notification processor** — Polls pending events, groups by game_id/match_id, waits 30s to batch multi-player notifications, sends Discord embeds
6. **AI analysis** (optional) — On game end, if `LLM_API_KEY` is set, fetches match timeline + stats, sends to the LLM (Gemma 4) for French-language analysis

### Key Patterns

- **Trait-based abstraction** — `Repository` and `RiotApiClient` traits with `#[automock]` enable testing without external services
- **Generic tracker** — `GameTracker<R: RiotApiClient, D: Repository>` is fully generic over both traits
- **Event-driven notifications** — DB-backed event queue decouples detection from delivery; 30s grouping window batches concurrent game events
- **Optional AI pipeline** — LLM analysis gated on env var presence, non-blocking (spawned as separate task)

### Module Map

| Module | Purpose |
|---|---|
| `discord/` | Bot event handler, slash commands, message formatting |
| `riot/` | Riot API client, game models, state tracker |
| `db/` | PostgreSQL pool, repository trait + impl, DB models |
| `notification/` | Event processor, Discord embed builders |
| `analysis/` | LLM client, analysis data models, prompt pipeline, embed formatters |
| `config.rs` | Env var parsing |

## Conventions

### Code Style

- `cargo fmt` — enforced in CI
- `cargo clippy -- -D warnings` — all warnings are errors
- `#[async_trait]` on all async trait definitions
- Error types use `thiserror` derive macros
- Raw SQL with `sqlx::query_as!` / `sqlx::query!` — no ORM, no query builder
- All user-facing text is in **French**

### Testing

- **Unit tests** — Inline `#[cfg(test)] mod tests` in: `riot/client.rs`, `analysis/llm.rs`, `analysis/pipeline.rs`, `analysis/discord.rs`
- **Integration tests** — `tests/` directory (8 files): messages, notification_messages, models, tracker, tracker_error, config_error, riot_error, riot_client
- **Mock system** — `mockall` behind `test-mocks` feature flag; mocks for `Repository` and `RiotApiClient`
- **Run**: `cargo test --features test-mocks`

### Database

- **Migrations** — `migrations/` directory (9 migrations), run automatically on startup via `sqlx::migrate!()`
- **Offline mode** — `SQLX_OFFLINE=true` in CI; update with `cargo sqlx prepare`
- **Schema** — summoners, active_games, match_history, champions, notification_events, bot_config tables
- **Pool** — max 5 connections (PgPoolOptions)

### CI/CD

- **CI** (`ci.yml`): Test → Lint (fmt + clippy) → Build. All use `SQLX_OFFLINE=true`
- **Deploy** (`deploy.yml`): Multi-stage Docker build (cargo-chef) → push to ghcr.io → K3s rollout restart via SSH
- **Docker** — Alpine-based, cargo-chef for layer caching, runtime image alpine:3.23

### Configuration

All config via environment variables (see `.env.example`):

| Variable | Required | Purpose |
|---|---|---|
| `DISCORD_BOT_ID` | Yes | Bot application ID |
| `DISCORD_TOKEN` | Yes | Bot authentication token |
| `RIOT_API_KEY` | Yes | Riot Games API key |
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `POLLING_INTERVAL_SECS` | No | Polling frequency (default in config) |
| `LLM_API_KEY` | No | Enables AI analysis (key for the OpenAI-compatible LLM server; any non-empty value if the server does not check auth) |
| `LLM_BASE_URL` | No | Base URL of the OpenAI-compatible LLM server (default: `http://jarvis:8080/v1`) |
| `LLM_MODEL` | No | Model name/alias requested from the LLM server (default: `gemma-4-26b`) |
| `ANALYSIS_PROMPTS_DIR` | No | Directory with role-specific prompts (default: `analysis_prompts`) |
| `DEFAULT_REGION` | No | Riot API region routing |
| `HEALTH_CHECK_PORT` | No | If set, bind a `0.0.0.0:port` HTTP/TCP listener that responds `200 OK` (for K8s readiness/liveness probes) |

**Note:** Notification channel is configured at runtime via the `/init-sfg-bot` slash command (stored in `bot_config` DB table).
Summoners are managed at runtime via `/add-summoner`, `/remove-summoner`, and `/list-summoners` slash commands (stored in `summoners` DB table).

## Analysis Prompts (Claude agent format)

Files in `analysis_prompts/` (`default.md`, `top.md`, `jungle.md`, `middle.md`, `bottom.md`, `support.md`) are Claude-style agent definitions: a YAML frontmatter block (`name`, `description`, `model`) followed by the prompt body. The frontmatter is metadata only — `AnalysisPipeline::new` calls `strip_frontmatter` so only the body is sent to the LLM.

When editing prompts:
- Keep the frontmatter block intact (opening `---` on line 1, closing `---` on its own line, prompt body after).
- Update `description` if the prompt's focus shifts.
- The body is in French and instructs the LLM to respond in French — preserve that.
- Add new role-specific prompts by creating `{role}.md` and registering the filename in `ROLE_PROMPT_FILES` in `src/analysis/pipeline.rs`.

## Claude Code workflow

- `.claude/settings.json` preallows the common cargo/sqlx/git commands used in this repo and pins `SQLX_OFFLINE=true` so `cargo check` / `clippy` work without a live database.
- Per-module guidance lives in `src/{discord,riot,db,notification,analysis}/CLAUDE.md`.

## Deployment (K3s)

The runtime config — Deployment manifest, container env, and **secrets** — lives in a sibling repo at `../k3s-ovh-config` (GitHub: `alaborderie/k3s-ovh-config`). Relevant files:

| File | Purpose |
|---|---|
| `sfg-bot.yaml` | Kubernetes `Deployment` — image, resources, env vars, probes, container ports |
| `sfg-bot-secrets.yaml` | `Secret` (`sfg-bot-secrets`) holding `RIOT_API_KEY`, `DISCORD_BOT_TOKEN`, `DISCORD_BOT_ID`, `DATABASE_URL`, `LLM_API_KEY`, etc. |
| `sfg-bot-postgres-init.yaml` | Postgres bootstrap (database + user) |

**Rule (for Claude across sessions):** whenever a change in *this* repo introduces or renames a runtime env var, exposes a new port, changes resource needs, or otherwise alters what the cluster needs to provide, **also update `../k3s-ovh-config`** and open a PR there. Merge the `k3s-ovh-config` PR **first**, then merge the matching sfg-bot PR — otherwise the next deploy will roll out a binary the cluster can't satisfy (missing env, failing probes, etc.).

Secrets values live only in `sfg-bot-secrets.yaml` (Kubernetes `stringData`). Do not copy real secret values into this repo — `.env.example` is for shape only.
