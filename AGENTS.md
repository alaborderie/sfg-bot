# sfg-bot

Discord bot that tracks SouthFoxGaming League of Legends games — detects game start/end, posts notifications to a Discord channel, and optionally runs AI post-game analysis via Google Gemini.

## Tech Stack

- **Rust** (edition 2024, rust-version 1.93)
- **serenity 0.12** — Discord gateway, slash commands, embeds
- **riven 2** — Riot Games API client
- **sqlx 0.8** — Async PostgreSQL (raw SQL, no ORM)
- **tokio 1** — Async runtime
- **reqwest** — HTTP client (Gemini API)
- **tracing** — Structured logging (env-filter, JSON format)
- **thiserror 2** — Error type derivation
- **mockall 0.13** — Mock generation (behind `test-mocks` feature flag)

## Architecture

### Core Flow

1. **main.rs** — Init tracing, load config from env, create DB pool, run migrations, cache champions, resolve summoner PUUIDs, start Discord client
2. **Per-summoner polling** — Each tracked summoner gets a `tokio::spawn` loop that polls Riot API for active games
3. **State machine** — `GameTracker` detects `GameStarted`, `GameEnded`, `FeaturedModeGameEnded`, `NoChange`
4. **Event queue** — State changes insert `NotificationEvent` rows into PostgreSQL
5. **Notification processor** — Polls pending events, groups by game_id/match_id, waits 30s to batch multi-player notifications, sends Discord embeds
6. **AI analysis** (optional) — On game end, if `GEMINI_API_KEY` is set, fetches match timeline + stats, sends to Gemini for French-language analysis

### Key Patterns

- **Trait-based abstraction** — `Repository` and `RiotApiClient` traits with `#[automock]` enable testing without external services
- **Generic tracker** — `GameTracker<R: RiotApiClient, D: Repository>` is fully generic over both traits
- **Event-driven notifications** — DB-backed event queue decouples detection from delivery; 30s grouping window batches concurrent game events
- **Optional AI pipeline** — Gemini analysis gated on env var presence, non-blocking (spawned as separate task)

### Module Map

| Module | Purpose |
|---|---|
| `discord/` | Bot event handler, slash commands, message formatting |
| `riot/` | Riot API client, game models, state tracker |
| `db/` | PostgreSQL pool, repository trait + impl, DB models |
| `notification/` | Event processor, Discord embed builders |
| `analysis/` | Gemini client, analysis data models, prompt pipeline, embed formatters |
| `config.rs` | Env var parsing, summoner name config |

## Conventions

### Code Style

- `cargo fmt` — enforced in CI
- `cargo clippy -- -D warnings` — all warnings are errors
- `#[async_trait]` on all async trait definitions
- Error types use `thiserror` derive macros exclusively
- Raw SQL with `sqlx::query_as!` / `sqlx::query!` — no ORM, no query builder
- All user-facing text is in **French**

### Testing

- **Unit tests** — Inline `#[cfg(test)] mod tests` in: `riot/client.rs`, `analysis/gemini.rs`, `analysis/pipeline.rs`, `analysis/discord.rs`
- **Integration tests** — `tests/` directory (9 files): messages, notification_messages, models, tracker, tracker_error, config, config_error, riot_error, riot_client
- **Mock system** — `mockall` behind `test-mocks` feature flag; mocks for `Repository` and `RiotApiClient`
- **Run**: `cargo test --features test-mocks`

### Database

- **Migrations** — `migrations/` directory (7 migrations), run automatically on startup via `sqlx::migrate!()`
- **Offline mode** — `SQLX_OFFLINE=true` in CI; update with `cargo sqlx prepare`
- **Schema** — summoners, active_games, match_history, champions, notification_events tables
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
| `SUMMONER_NAMES` | Yes | Comma-separated `name#tag` list |
| `POLLING_INTERVAL_SECS` | No | Polling frequency (default in config) |
| `GEMINI_API_KEY` | No | Enables AI analysis |
| `ANALYSIS_PROMPTS_DIR` | No | Directory with role-specific prompts (default: `analysis_prompts`) |
| `DEFAULT_REGION` | No | Riot API region routing |

**Note:** Notification channel is configured at runtime via the `/init-sfg-bot` slash command (stored in `bot_config` DB table).
