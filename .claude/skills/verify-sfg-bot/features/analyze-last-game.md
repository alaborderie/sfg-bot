# Analyze the last game

`/analyze-last-game` resolves a `Name#Tag`, loads that account's latest match, and replies with a French recap embed. When no LLM key is configured, the bot stops after the recap and does not post a coach paragraph. Verification always takes that branch.

## Sub-features

- `analyze-recap` posts the win recap for the fixture match.
- `analyze-unknown` replies that the account does not exist and posts no embed.
- `analyze-invalid` replies that the format must be `Name#Tag`.
- `analyze-coach` the coach paragraph. Unreachable in this harness.

## How to get to it (user POV)

- In the server, run `/analyze-last-game` with option `summoner_name` set to `Name#Tag`.

## Driving it with sfg-verify

Preconditions:

- `doctor` reports `"ok": true` for `$SFG_VERIFY_HOME`.
- The fixture match is `EUW1_9000000001` for `VerifyFox#SFG`: a win on Yasuo, queue 420, role `MIDDLE`, KDA 7/2/11.
- `LLM_API_KEY` is not read. Do not point this command at a live model.

- **Bad format.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash analyze-last-game --summoner-name 'not-a-riot-id'`. `visible_text` contains `Format invalide. Utilise `Nom#Tag` (ex: `Doublelift#NA1`)`.
- **Unknown account.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash analyze-last-game --summoner-name 'Nobody#NOPE'`. `visible_text` contains ``Compte `Nobody#NOPE` introuvable``.
- **Recap.** Run `./target/debug/sfg-verify --home "$SFG_VERIFY_HOME" slash analyze-last-game --summoner-name 'VerifyFox#SFG'`. `visible_text` contains `Partie gagnée !`, `Partie Classée Solo/Duo terminée pour VerifyFox#SFG.`, `💎 Yasuo · MIDDLE · W 7/2/11`, `6.9 CS/min · 463 GPM · 24.9k dmg`, and `Ahri (6.1 CS/min · 394 GPM · 19.3k dmg)`. The Discord array contains a defer (`"type": 5`) and one followup embed. It does not contain a second coach embed. `"non_loopback_sockets"` is `[]`. `summoners` is unchanged.
- **Proof.** Keep the recap JSON. The embed title and the Yasuo line must both be in `visible_text`.

## Gotchas

- The coach paragraph is not produced. The harness forces the same `analysis_pipeline = None` branch the bot uses when `LLM_API_KEY` is unset. Do not describe that skip as a verified coach reply. Driving the coach text needs `LLM_API_KEY` and a local `LLM_BASE_URL`; this CLI does not call one.
- The account does not have to be in the tracked roster. `/analyze-last-game` resolves the fixture account even when `summoners` is empty.
- Arena and other non-classic modes are not in the fixture. There is no drive here for the "Pas d'analyse pour ce mode de jeu" reply.
- Stats lines are computed from the fixture's CS, gold, damage, and duration of 1842 seconds. If the fixture numbers change, the expected `CS/min` line changes with them.
