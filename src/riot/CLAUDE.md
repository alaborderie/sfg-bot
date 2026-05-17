# riot/

Riot Games API integration — client abstraction, game models, and state tracking.

## Files

| File | Purpose |
|---|---|
| `client.rs` | `RiotApiClient` trait (`#[automock]`) + `RiotClient` implementation. Wraps `riven::RiotApi` for account lookup, active game detection, match results, match timeline, analysis data aggregation, and champion data |
| `models.rs` | Domain models: `SummonerInfo`, `ActiveGameInfo`, `MatchResult`, `GameStateChange` enum |
| `tracker.rs` | `GameTracker<R: RiotApiClient, D: Repository>` — stateful per-summoner tracker implementing game state machine |

## Key Details

### RiotApiClient Trait (client.rs)

Methods:
- `get_account_by_riot_id(name, tag, region)` → `SummonerInfo`
- `get_active_game(puuid, region)` → `Option<ActiveGameInfo>`
- `get_match_result(match_id, puuid, region)` → `MatchResult`
- `get_match_timeline(match_id, region)` → timeline JSON
- `get_match_analysis_data(match_id, puuid, summoner_name, region)` → `Option<AnalysisData>`
- `get_recent_match_id(puuid, region)` → `Option<String>`
- `get_all_champions(region)` → champion map for cache

The trait uses `#[async_trait]` and `#[automock]` (behind `test-mocks` feature).

### Helper Functions (outside trait)

- `extract_timeline_diff(timeline, participant_id)` — extracts gold/xp/cs diffs from timeline frames
- `diff_at_frame(frames, participant_id, frame_index)` — computes diff at a specific frame
- `compute_role_gaps(participants, team_id) -> Vec<RoleGap>` — scans all 10 participants and returns lanes whose gold delta crosses the gap threshold (3000g for solo lanes, 5000g for combined bot lane). Output is from `team_id`'s perspective: positive delta = ally leads.
- `format_role_gaps(&[RoleGap]) -> Option<String>` — renders as `"Bot gap (-5.5k), Top diff (+4.2k)"`. The result is attached to `MatchResult::role_gaps` and ultimately rendered as a 🎯 Écarts par rôle field on the Discord recap embed.

### Region Routing

- `RiotClient::platform_for_region(region)` — maps region string to `riven` platform route enum (e.g., `EUW1`)
- `RiotClient::regional_for_region(region)` — maps region string to `riven` regional route enum (e.g., `EUROPE`)
- Platform routes used for summoner/spectator endpoints
- Regional routes used for match endpoints

### GameTracker State Machine (tracker.rs)

- Tracks `current_game_id: Option<i64>` per summoner
- `check_game()` returns `GameStateChange`:
  - `GameStarted` — new active game detected, inserts `active_games` + `notification_events` rows
  - `GameEnded` — previously tracked game no longer active, fetches match result, inserts `match_history` + `notification_events`
  - `FeaturedModeGameEnded` — game ended but no match data found (featured/spectator mode)
  - `NoChange` — no state transition
- Includes retry logic for match data fetching (Riot API can delay match availability)
- Generic over `R: RiotApiClient` and `D: Repository` for testability
