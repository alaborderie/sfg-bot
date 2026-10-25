# Configure the notification channel

`/init-sfg-bot` stores the current Discord channel as the place where game alerts will be posted, and replies in French with that channel mention. In a direct message it refuses.

## Sub-features

- `init-save` stores the guild and channel and replies with the channel mention.
- `init-dm` refuses when the command is not used in a server.
- `init-replace` a second save for the same guild replaces the channel.

## How to get to it (user POV)

- In a server channel, run `/init-sfg-bot`. There are no options.
- In a DM with the bot, run `/init-sfg-bot`. The bot replies that the command only works in a server.

## Driving it with sfg-verify

Preconditions:

- `doctor` reports `"ok": true` for `$SFG_VERIFY_HOME`.
- `show bot-config` returns `"bot_configs": []`.

- **Save the harness channel.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash init-sfg-bot`. `visible_text` contains `✅ Notifications configurées dans <#900000000000000004>. Les alertes de parties seront envoyées ici !`. `bot_configs` contains one object whose `guild_id` is `900000000000000003` and whose `channel_id` is `900000000000000004`. `"non_loopback_sockets"` is `[]`.
- **Confirm the stored channel.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" show bot-config`. The same guild and channel pair is still there.
- **Replace the channel.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash init-sfg-bot --channel-id 900000000000000099`. `visible_text` mentions `<#900000000000000099>`. `show bot-config` returns one row, with `channel_id` `900000000000000099`, not two rows.
- **DM rejection.** On a fresh home, run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash init-sfg-bot --dm`. `visible_text` contains `❌ Cette commande ne peut être utilisée que dans un serveur.`. `show bot-config` is still empty.
- **Proof.** Keep the save JSON and the following `show bot-config` JSON in the evidence directory. Both must name guild `900000000000000003`.

## Gotchas

- The harness Discord cache is empty, so the permission check cannot see the bot's roles. The command still saves, which is the same branch the bot takes when the guild is not in cache. A reply of `❌ Droits manquants` is not reachable here. Do not treat a successful save as proof that permissions were checked.
- `--dm` cannot be combined with `--guild-id`. The CLI exits non-zero and does not write a row.
- The channel mention in the reply is the interaction's channel id, not a channel the bot created.
- Game alerts are not posted by this command. Saving the channel does not prove that a later game-start embed arrives in Discord.
