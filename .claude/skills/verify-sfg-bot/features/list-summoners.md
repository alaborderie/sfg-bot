# List summoners

`/list-summoners` replies with every tracked account, or with the empty-roster hint when nobody is tracked. It does not change the roster.

## Sub-features

- `list-empty` shows the empty hint and names `/add-summoner`.
- `list-one` shows `VerifyFox#SFG` and the stored region after that account has been added.
- `list-unchanged` the roster row count is the same after the command as before it.

## How to get to it (user POV)

- In the server, run `/list-summoners`. There are no options.

## Driving it with sfg-verify

Preconditions:

- `doctor` reports `"ok": true` for `$SFG_VERIFY_HOME`.
- For `list-empty`, `show summoners` returns `"summoners": []`.
- For `list-one`, `/add-summoner` has already stored `VerifyFox#SFG`. If that row is missing, this journey fails. Do not insert the row by SQL.

- **Empty roster.** On a fresh home, run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash list-summoners`. `visible_text` contains `📋 Aucun invocateur suivi pour le moment.` and `/add-summoner`. `summoners` is `[]`. The Discord body is a channel message (`"type": 4`), not a defer.
- **Populated roster.** After the add-summoner success path, run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash list-summoners`. `visible_text` contains `📋 **Invocateurs suivis (1)** :`, `VerifyFox#SFG`, and `euw1`.
- **No write.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" show summoners` before and after the populated list. The puuid set is unchanged.
- **Proof.** Keep the empty-roster JSON from a fresh home and the populated JSON from a home where add-summoner succeeded. `"non_loopback_sockets"` is `[]` on both.

## Gotchas

- `SELECT` order is not part of the contract. Assert that the expected name is present, not that it is on a particular line, once more than one summoner exists.
- The empty hint and the populated list are different homes unless you remove the summoner between them. Do not claim the empty state from a home that already has a row.
- The region printed is the region stored at add time (`euw1` unless launch was given another `--region`).
