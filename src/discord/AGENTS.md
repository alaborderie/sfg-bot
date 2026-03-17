# discord/

Discord bot integration — event handling, slash commands, and message formatting.

## Files

| File | Purpose |
|---|---|
| `handler.rs` | `Bot` struct implementing serenity `EventHandler`. `ready()` registers commands + spawns polling. `check_and_notify()` polls one summoner, inserts notification events. `spawn_analysis_task()` triggers post-game AI analysis |
| `commands.rs` | `/analyze-last-game` slash command — registration via `CreateCommand` + `run()` handler |
| `messages.rs` | Message formatting functions: `game_started()`, `game_ended()`, `mention_response()`. All output in French |

## Key Details

### Bot / Handler (handler.rs)

- `Bot` holds: `Arc<dyn Repository>`, `Arc<dyn RiotApiClient>`, channel ID, analysis pipeline, config
- On `ready`: registers slash commands globally, then spawns per-summoner polling loops via `tokio::spawn`
- Each polling loop creates its own `GameTracker` and calls `check_and_notify()` in a loop with configurable interval
- `check_and_notify()` — calls `tracker.check_game()`, matches on `GameStateChange`, inserts `NewNotificationEvent` rows to DB
- `spawn_analysis_task()` — spawned as detached `tokio::spawn`, fetches match data + timeline, runs analysis pipeline, sends embed to channel
- Notification delivery handled by `NotificationProcessor` (separate module), NOT by handler directly

### Slash Commands (commands.rs)

- Single command: `/analyze-last-game`
- Uses deferred response pattern (acknowledges immediately, edits later)
- Fetches last match for a summoner, runs analysis pipeline, responds with embed

### Messages (messages.rs)

- Pure formatting functions, no side effects
- All strings in French locale
- Champion names resolved from cache
- Queue type display names mapped to French equivalents
