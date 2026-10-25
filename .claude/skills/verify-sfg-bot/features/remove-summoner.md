# Remove a summoner

`/remove-summoner` deletes one tracked account by `Name#Tag` and replies in French. A name that is not in the roster does not delete anything. `--dry-run` does not delete anything either.

## Sub-features

- `remove-save` deletes `VerifyFox#SFG` and confirms the reply.
- `remove-missing` replies that the account is not in the roster.
- `remove-invalid` replies that the format must be `Name#Tag`.
- `remove-dry-run` reports `would_delete` and leaves the row in place.

## How to get to it (user POV)

- In the server, run `/remove-summoner` with option `summoner_name` set to `Name#Tag`.

## Driving it with sfg-verify

Preconditions:

- `doctor` reports `"ok": true` for `$SFG_VERIFY_HOME`.
- For `remove-save` and `remove-dry-run`, `show summoners` already contains `VerifyFox#SFG`. If it does not, the journey fails. Add it with `/add-summoner` first. Do not insert it by SQL.
- For `remove-missing`, the name you type must not be in `show summoners`.

- **Dry-run.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash remove-summoner --summoner-name 'VerifyFox#SFG' --dry-run`. The JSON has `"dry_run": true` and `"would_delete": true`. `show summoners` still contains `VerifyFox#SFG`. There is no `discord` array.
- **Remove.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash remove-summoner --summoner-name 'VerifyFox#SFG'`. `visible_text` contains `✅ **VerifyFox#SFG** retiré de la liste de suivi.`. `summoners` is `[]`. `"non_loopback_sockets"` is `[]`.
- **Confirm.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash list-summoners`. `visible_text` contains `Aucun invocateur suivi`. Then run `show summoners` and require `"summoners": []`.
- **Missing name.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash remove-summoner --summoner-name 'VerifyFox#SFG'` again. `visible_text` contains ``❌ Invocateur `VerifyFox#SFG` introuvable dans la liste de suivi.``.
- **Bad format.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash remove-summoner --summoner-name 'not-a-riot-id'`. `visible_text` contains `❌ Format invalide. Utilise `Nom#Tag` (ex: `Doublelift#NA1`)`.
- **Proof.** Keep the dry-run JSON, the real removal JSON, and the following empty `list-summoners` JSON. The dry-run file is not the removal proof.

## Gotchas

- Removal matches `game_name` and `tag_line` case-insensitively. `verifyfox#sfg` removes `VerifyFox#SFG`.
- `--dry-run` never calls the handler. A later real removal is still required.
- The command does not ask Riot whether the account exists. An unknown Riot id that was never added takes the "not in the roster" reply, not the "account not found" reply from `/add-summoner`.
