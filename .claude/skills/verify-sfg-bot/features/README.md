# sfg-bot verification map

This directory is the maintained source for verifying the user-facing behavior of sfg-bot. Read this index before driving, then use the matching feature file as the recipe.

The user surface is Discord slash commands and the French replies they produce. There is no web UI. Game-start and game-end notification embeds, the `@mention` reply, and the LLM coach paragraph are not in this map: delivering them requires `DISCORD_BOT_TOKEN` or `LLM_API_KEY`, and the harness does not call those services. Record them as skipped with that reason. Do not substitute a direct database insert.

## Baseline preconditions

- Run from the repository root so the default fixture path resolves.
- `cargo build --bin sfg-verify` has produced `./target/debug/sfg-verify`.
- `./target/debug/sfg-verify launch` has printed `"ready": true` for an empty `summoners` table.
- `SFG_VERIFY_HOME` is that launch's `home`. Pass `--home "$SFG_VERIFY_HOME"` on every command.
- `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" doctor` reports `"ok": true`, loopback Postgres, and the five command names.
- The Riot fixture is `.claude/skills/verify-sfg-bot/fixtures/riot.json`. The only account is `VerifyFox#SFG`. The only match is `EUW1_9000000001`.
- Never drive an instance that was not started by this verification run.

## Driving conventions

- Start every recipe from the baseline state unless its preconditions say otherwise.
- Treat every command as literal. Keep quoted names and flags unchanged.
- Invoke commands through `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash ...`.
- Read rows back through `show summoners` or `show bot-config`, and through `/list-summoners` when the user would look at the roster.
- `--dry-run` is an observation of what a destructive command would do. It is not a successful drive.
- Restore or isolate state between recipes by launching a fresh home. Do not point two recipes at one home unless the feature file says the earlier row is required.
- Leave proof files in the evidence directory. Cleanup deletes the home only.

## Proof and skip reporting

- Capture the user action and the resulting state, not only the final JSON `"ok"` field.
- Slash proof includes `discord[].body`, `visible_text`, and a second read of the stored row.
- Mutation proof includes `show` (or `/list-summoners`) after the slash command.
- `"non_loopback_sockets"` must be `[]` on the drive JSON. If it is not, the drive contacted something other than localhost and does not count.
- Record the feature id and the entry point with every artifact. The evidence file name contains the command.
- Report an unreachable path with the command you tried and the missing variable (`DISCORD_BOT_TOKEN`, `RIOT_API_KEY`, or `LLM_API_KEY`).
- Do not report a skipped entry point as verified through a different path.

## Feature entry contract

Each feature file starts with an H1 and one paragraph describing the user-visible behavior. It then uses exactly four H2 sections in this order.

1. `Sub-features` lists short IDs with one line for each behavior.
2. `How to get to it (user POV)` lists every user entry point.
3. `Driving it with sfg-verify` starts with `Preconditions:` and uses labeled bullets that pair each user action with an exact command and observable result.
4. `Gotchas` lists traps that can waste or invalidate a verification run.

Keep implementation details out of the map. Name only user paths, stable handles, required state, commands, and observable proof.

## Features

- [Configure the notification channel](./init-sfg-bot.md) covers `/init-sfg-bot` in a server and in a DM.
- [Add a summoner](./add-summoner.md) covers a valid fixture account, an unknown account, and a bad `Name#Tag`.
- [List summoners](./list-summoners.md) covers the empty roster and a roster that contains `VerifyFox#SFG`.
- [Remove a summoner](./remove-summoner.md) covers a real removal, a missing name, and `--dry-run`.
- [Analyze the last game](./analyze-last-game.md) covers the recap embed for the fixture match. The coach paragraph is skipped without `LLM_API_KEY`.
