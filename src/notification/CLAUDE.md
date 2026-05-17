# notification/

Event-driven notification processor — polls DB for pending events, groups multi-player notifications, sends Discord embeds.

## Files

| File | Purpose |
|---|---|
| `mod.rs` | Re-exports `NotificationProcessor` from `processor.rs` |
| `processor.rs` | `NotificationProcessor` — long-lived async loop that polls pending events, groups them by game, and sends Discord embeds |
| `messages.rs` | French-locale embed builders for game start/end notifications. Formats player stats, queue types, and win/loss results |

## Key Details

### NotificationProcessor (processor.rs)

- Fields: `repository: Arc<dyn Repository>`, `ctx: Context` (serenity), `interval_secs: u64`
- `start()` — infinite loop calling `process_pending_events()` + sleep
- **30s grouping window**: waits before sending to batch events from multi-player games (groups by `game_id` for started, `match_id` for ended)
- Fetches notification channel from `bot_config` table
- Sends embeds via `channel_id.send_message()`
- Marks events as processed after successful delivery

### Message Formatting (messages.rs)

| Function | Purpose |
|---|---|
| `format_grouped_game_started()` | Multi-player game start embed (blue) |
| `format_grouped_game_ended()` | Multi-player game end embed — green (all won), red (all lost), yellow (mixed). Appends a 🎯 Écarts par rôle field when any grouped event carries a non-empty `role_gaps` summary (e.g. `"Bot gap (-5.5k)"`) — surfaces lane-imbalance context regardless of who won. |
| `format_single_game_ended()` | Single player embed, used by `/analyze-last-game` command |
| `get_queue_type_name()` | Maps Riot queue IDs to French display names |
| `format_list()` | French-style list formatting (e.g., "A, B et C") |
| `format_stats_line()` | Player stats line: CS/min, gold/min, damage |
| `format_enemy_comparison()` | Comparison against lane opponent stats |
