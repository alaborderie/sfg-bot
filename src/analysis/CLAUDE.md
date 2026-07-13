# analysis/

AI-powered post-game analysis using a self-hosted LLM (Gemma 4) behind an OpenAI-compatible API.

## Files

| File | Purpose |
|---|---|
| `llm.rs` | `LlmClient` — HTTP client for the OpenAI-compatible LLM API (`/chat/completions`). Handles request formatting, retry with exponential backoff, rate limit (429) handling |
| `models.rs` | `AnalysisData` (match stats + timeline diffs for prompt context), `AnalysisResult` (rating enum + summary text) |
| `pipeline.rs` | `AnalysisPipeline` — loads role intro + shared skill blocks, composes a per-role prompt at startup, serializes match data as JSON, calls the LLM, extracts rating |
| `roles.rs` | `RoleSpec` table — for each Riot role, lists which shared skills apply with `SkillImportance` and role-specific benchmark / tactical-note strings |
| `discord.rs` | Embed formatters for analysis results — rating-based color coding, description truncation for Discord limits |
| `history.rs` | Per-player analysis memory — `analyze_with_memory` fetches the last 5 snapshots from `analysis_history`, injects them as `AnalysisData::recent_games`, runs the analysis, then stores this game's snapshot (history stripped so snapshots never nest) |

## Key Details

### Pipeline Flow

1. `AnalysisPipeline::new(llm_client, prompts_dir)` loads role-specific prompt files (`top.md`, `jungle.md`, `middle.md`, `bottom.md`, `support.md`) into a `HashMap` + a `default.md` fallback
2. `analyze_game(&self, data: &AnalysisData)` called after game ends
3. Selects prompt by player's role; falls back to `default.md` if role not found
4. Serializes `AnalysisData` as JSON (`data_json`)
5. Calls `llm_client.analyze(prompt, &data_json)`
6. Extracts rating via `extract_overall_rating()` — parses `Good`/`Average`/`Poor` from response
7. Returns `AnalysisResult` for embed formatting

### LLM Client (llm.rs)

- Uses `reqwest` for HTTP
- Endpoint: `{LLM_BASE_URL}/chat/completions` (OpenAI-compatible; default base URL `http://jarvis:8080/v1`)
- Model name sent in the request body comes from `LLM_MODEL` (default `gemma-4-26b` — the llama.cpp alias for the local Gemma 4 model)
- Retry logic: exponential backoff on 429 (rate limit) and 5xx errors
- API key passed as `Authorization: Bearer` header
- Gemma 4 is a reasoning model: it spends tokens on `reasoning_content` before the visible answer, so `MAX_TOKENS` is 4096 and the HTTP timeout is 300s (local generation runs ~30 tokens/s); an empty `content` field is treated as `ParseError`
- Temperature is 0.35: ratings must be stable across reruns of similar games

### Error Handling

- `LlmError` — thiserror derive: `HttpError`, `ApiError`, `ParseError`, `RateLimited`, `Timeout`
- `AnalysisError` — thiserror derive: `LlmError`, `PromptFileError`, `PromptDirError`, `SerializationError`

### Analysis Output

- **Rating**: `Good`, `Average`, or `Poor` — determines embed color (green/yellow/red)
- **Summary**: French-language analysis text, truncated to fit Discord embed limits
- All analysis text generated in French (prompt templates are in French)

### Prompt Templates

- Directory configured via `ANALYSIS_PROMPTS_DIR` env var (default: `analysis_prompts/`)
- Role-specific files at the root: `top.md`, `jungle.md`, `middle.md`, `bottom.md`, `support.md` — these are the **role intro** (identity, tactics, matchup advice).
- Fallback: `default.md` used when role has no specific prompt.
- Role files carry ONLY the role intro, analysis axes, coaching tips, and role-specific response priorities. Generic response rules live in `analysis_prompts/shared/` (see below) — do not re-add per-file "Règles de réponse" blocks.
- Each prompt file is a Claude agent definition: YAML frontmatter (`name`, `description`, `model`) followed by the prompt body. `pipeline::strip_frontmatter` removes the frontmatter before the body is sent to the LLM, so the metadata never reaches the model.
- Prompts written in French, instruct the LLM to produce French output.
- When adding a new role: create `{role}.md` with a frontmatter block, register `(<RIOT_ROLE>, "<role>.md")` in `ROLE_PROMPT_FILES` in `pipeline.rs`, and add a matching `RoleSpec` const in `roles.rs`.

### Shared sections (`analysis_prompts/shared/`)

- `rating_rubric.md` — the Good/Average/Poor calibration rubric (win/loss weighting, "lost lane converted into a win is at least Average", etc.). Appended to every composed prompt after the skills referential.
- `response_format.md` — output contract: French, rating as the literal first word (parser contract with `extract_overall_rating`), 150-250 words in 3 paragraphs ending with « Conseil de coach : » + one numeric goal.
- Loaded by `load_shared_sections`; missing files are skipped with a warning (same robustness contract as skills).
- The pipeline appends a final `## Données de la partie (JSON)` section carrying the `{game_data}` placeholder — prompt files must NOT contain `{game_data}` themselves.
- Live calibration harness: `cargo test --test live_llm_calibration -- --ignored --test-threads=1` runs real scenarios against the local LLM server and asserts rating calibration (stomp-win is never Poor, disaster-loss is never Good, etc.).

### Game memory (history.rs)

- Both analysis entry points (auto post-game in `handler::spawn_analysis_task`, manual `/analyze-last-game` in `commands::run`) go through `history::analyze_with_memory` — do not call `pipeline.analyze_game` directly from Discord code.
- History failures degrade to a memory-less analysis (logged warning); they never block the analysis.
- Snapshots are JSON-serialized `AnalysisData` in the `analysis_history.analysis_data` TEXT column; the current match is excluded on read so `/analyze-last-game` reruns don't compare a game to itself.
- `shared/response_format.md` instructs the coach to open with one progression/regression sentence when `recent_games` is non-empty.

### Shared skills (`analysis_prompts/skills/`)

- One file per metric: `cs_per_minute.md`, `damage_per_minute.md`, `kills_assists.md`, `deaths.md`, `vision_score.md`.
- Each file is a generic French markdown block with two placeholders: `{benchmarks}` (role-specific thresholds at Platine/Émeraude) and `{role_notes}` (role-specific tactical notes).
- `roles.rs` declares one `SkillBinding` per (role, skill) with an `SkillImportance` (`Critical` / `High` / `Medium` / `Low` / `NotApplicable`) and the two strings that fill the placeholders.
- `NotApplicable` skips the skill block entirely — e.g. support's `cs_per_minute` binding is `NotApplicable`, so support prompts never lecture about CS targets.
- At startup, `AnalysisPipeline::new` reads the role intro + all skill files, composes the final per-role prompt by appending each applicable skill block (with placeholders substituted) under a "Référentiel par compétence" header, and caches it. `get_prompt_for_role` is a simple lookup.
- Skill files are optional — if a skill file is missing, every role that binds it just won't get that block (logged warning, no failure).
