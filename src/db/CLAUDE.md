# db/

PostgreSQL database layer — connection pool, repository trait, and data models.

## Files

| File | Purpose |
|---|---|
| `mod.rs` | `create_pool(database_url)` — creates `PgPool` with max 5 connections, 30s acquire timeout |
| `models.rs` | 8 structs for DB rows and input types. DB-mapped structs use `#[derive(FromRow)]`, input structs use plain `#[derive(Debug, Clone)]` |
| `repository.rs` | `Repository` trait (19 async methods, `#[automock]` + `#[async_trait]`) + `PgRepository` implementation with raw SQL |

## Key Details

### Models (models.rs)

| Struct | Usage |
|---|---|
| `Summoner` | DB row — tracked summoner (id, game_name, tag_line, puuid, region) |
| `ActiveGame` | DB row — currently active game |
| `NewActiveGame` | Input — data for inserting a new active game |
| `MatchHistory` | DB row — completed match record |
| `NewMatchResult` | Input — data for inserting match result |
| `Champion` | DB row — cached champion data |
| `NotificationEvent` / `NewNotificationEvent` | DB row / input — notification queue entries |
| `BotConfig` | DB row — per-guild bot configuration (notification channel) |

### Repository Trait (repository.rs)

- 19 async methods covering CRUD for: summoners, active_games, match_history, champions, notification_events, bot_config
- `RepositoryError` wraps `sqlx::Error` via thiserror `#[from]`
- `#[cfg_attr(feature = "test-mocks", mockall::automock)]` enables mock generation for tests

### PgRepository Implementation

- Holds `PgPool`, all queries use `sqlx::query_as` with raw SQL
- **UPSERT pattern**: `ON CONFLICT ... DO UPDATE` for summoners and champions
- **Case-insensitive delete**: `LOWER()` comparison for summoner name/tag deletion
- **Batch operations**: `ANY($1)` for updating multiple notification events at once
- **Note**: DB table is named `notification_queue` (not `notification_events`)
