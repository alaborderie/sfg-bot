# riot/

Riot Games API integration — client abstraction, game models, and state tracking.

## Files

| File | Purpose |
|---|---|
| `client.rs` | `RiotApiClient` trait (`#[automock]`) + `RiotClient` implementation. Wraps `riven::RiotApi` for summoner lookup, active game detection, match history, match timeline, and Data Dragon champion data |
| `models.rs` | Domain models: `SummonerInfo`, `ActiveGameInfo`, `MatchResult`, `GameStateChange` enum |
| `tracker.rs` | `GameTracker<R: RiotApiClient, D: Repository>` — stateful per-summoner tracker implementing game state machine |

## Key Details

### RiotApiClient Trait (client.rs)

Methods:
- `get_summoner_by_name(name, tag, region)` → `SummonerInfo`
- `get_active_game(puuid)` → `Option<ActiveGameInfo>`
- `get_match_history(puuid, count)` → `Vec<String>` (match IDs)
- `get_match_result(match_id)` → `MatchResult`
- `get_match_timeline(match_id)` → timeline JSON
- `get_all_champions()` → champion map for cache

The trait uses `#[async_trait]` and `#[automock]` (behind `test-mocks` feature).

### Region Routing

- `RiotClient` maps region strings to `riven` platform/region route enums
- Platform routes (e.g., `EUW1`) for summoner/spectator endpoints
- Regional routes (e.g., `EUROPE`) for match endpoints

### GameTracker State Machine (tracker.rs)

- Tracks `current_game_id: Option<i64>` per summoner
- `check_game()` returns `GameStateChange`:
  - `GameStarted` — new active game detected, inserts `active_games` + `notification_events` rows
  - `GameEnded` — previously tracked game no longer active, fetches match result, inserts `match_history` + `notification_events`
  - `FeaturedModeGameEnded` — game ended but no match data found (featured/spectator mode)
  - `NoChange` — no state transition
- Includes retry logic for match data fetching (Riot API can delay match availability)
- Generic over `R: RiotApiClient` and `D: Repository` for testability
