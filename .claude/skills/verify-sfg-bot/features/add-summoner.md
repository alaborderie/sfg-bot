# Add a summoner

`/add-summoner` takes a `Name#Tag`, resolves it, and adds that account to the tracked roster. The reply is French. An unknown account or a bad format does not insert a row.

## Sub-features

- `add-save` resolves `VerifyFox#SFG` and inserts one roster row.
- `add-unknown` replies that the account does not exist and inserts nothing.
- `add-invalid` replies that the format must be `Name#Tag` and inserts nothing.
- `add-again` running the successful command a second time keeps a single row for the same puuid.

## How to get to it (user POV)

- In the server, run `/add-summoner` with option `summoner_name` set to `Name#Tag`, for example `VerifyFox#SFG`.

## Driving it with sfg-verify

Preconditions:

- `doctor` reports `"ok": true` for `$SFG_VERIFY_HOME`.
- `show summoners` returns `"summoners": []`.
- The fixture file contains `VerifyFox#SFG` and does not contain `Nobody#NOPE`.

- **Reject a bad format.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash add-summoner --summoner-name 'not-a-riot-id'`. `visible_text` contains `❌ Format invalide. Utilise `Nom#Tag` (ex: `Doublelift#NA1`)`. `summoners` is `[]`.
- **Reject an unknown account.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash add-summoner --summoner-name 'Nobody#NOPE'`. `visible_text` contains ``❌ Compte `Nobody#NOPE` introuvable. Vérifie le nom et le tag.``. `summoners` is `[]`.
- **Add the fixture account.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash add-summoner --summoner-name 'VerifyFox#SFG'`. `visible_text` contains `✅ **VerifyFox#SFG** ajouté à la liste de suivi !`. `summoners` has one object with `game_name` `VerifyFox`, `tag_line` `SFG`, `region` `euw1`, and `riot_puuid` `verify-puuid-sfg-bot-0001`. The Discord array has a defer callback (`"type": 5`) and a followup whose `content` is that success sentence. `"non_loopback_sockets"` is `[]`.
- **Confirm from the roster command.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash list-summoners`. `visible_text` contains `VerifyFox#SFG` and `euw1`.
- **Confirm the row.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" show summoners`. The same puuid is present.
- **Add the same account again.** Run the successful `add-summoner` command a second time. `show summoners` still has one row, with that puuid.
- **Proof.** Keep the success JSON, the `list-summoners` JSON, and the `show summoners` JSON. The success reply alone is not enough.

## Gotchas

- Lookup is case-insensitive. `verifyfox#sfg` resolves, and the stored name is the fixture spelling `VerifyFox#SFG`. Assert the stored name, not the typed casing.
- The tag in the fixture must be at most 5 characters or the insert fails on `summoners.tag_line`.
- A defer that never reaches the stand-in returns before the insert. If `discord` is empty, do not expect a row.
- This command does not call Riot. `"non_loopback_sockets"` must stay empty. A drive that opens a public address is not a proof.
- Re-adding updates the existing puuid. A second row with the same name is a failure.
