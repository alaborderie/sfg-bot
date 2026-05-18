# analysis/

AI-powered post-game analysis using Google Gemini API.

## Files

| File | Purpose |
|---|---|
| `gemini.rs` | `GeminiClient` — HTTP client for Gemini API. Handles request formatting, retry with exponential backoff, rate limit (429) handling |
| `models.rs` | `AnalysisData` (match stats + timeline diffs for prompt context), `AnalysisResult` (rating enum + summary text) |
| `pipeline.rs` | `AnalysisPipeline` — loads role intro + shared skill blocks, composes a per-role prompt at startup, serializes match data as JSON, calls Gemini, extracts rating |
| `roles.rs` | `RoleSpec` table — for each Riot role, lists which shared skills apply with `SkillImportance` and role-specific benchmark / tactical-note strings |
| `discord.rs` | Embed formatters for analysis results — rating-based color coding, description truncation for Discord limits |

## Key Details

### Pipeline Flow

1. `AnalysisPipeline::new(gemini_client, prompts_dir)` loads role-specific prompt files (`top.md`, `jungle.md`, `middle.md`, `bottom.md`, `support.md`) into a `HashMap` + a `default.md` fallback
2. `analyze_game(&self, data: &AnalysisData)` called after game ends
3. Selects prompt by player's role; falls back to `default.md` if role not found
4. Serializes `AnalysisData` as JSON (`data_json`)
5. Calls `gemini_client.analyze(prompt, &data_json)`
6. Extracts rating via `extract_overall_rating()` — parses `Good`/`Average`/`Poor` from response
7. Returns `AnalysisResult` for embed formatting

### Gemini Client (gemini.rs)

- Uses `reqwest` for HTTP
- Endpoint: `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
- Retry logic: exponential backoff on 429 (rate limit) and 5xx errors
- API key passed as query parameter

### Error Handling

- `GeminiError` — manual `impl Error` (not thiserror): `HttpError`, `ApiError`, `ParseError`, `RateLimited`, `Timeout`
- `AnalysisError` — manual `impl Error`: `GeminiError`, `PromptFileError`, `PromptDirError`, `SerializationError`

### Analysis Output

- **Rating**: `Good`, `Average`, or `Poor` — determines embed color (green/yellow/red)
- **Summary**: French-language analysis text, truncated to fit Discord embed limits
- All analysis text generated in French (prompt templates are in French)

### Prompt Templates

- Directory configured via `ANALYSIS_PROMPTS_DIR` env var (default: `analysis_prompts/`)
- Role-specific files at the root: `top.md`, `jungle.md`, `middle.md`, `bottom.md`, `support.md` — these are the **role intro** (identity, tactics, matchup advice).
- Fallback: `default.md` used when role has no specific prompt.
- Each prompt file is a Claude agent definition: YAML frontmatter (`name`, `description`, `model`) followed by the prompt body. `pipeline::strip_frontmatter` removes the frontmatter before the body is sent to Gemini, so the metadata never reaches the model.
- Prompts written in French, instruct Gemini to produce French output.
- When adding a new role: create `{role}.md` with a frontmatter block, register `(<RIOT_ROLE>, "<role>.md")` in `ROLE_PROMPT_FILES` in `pipeline.rs`, and add a matching `RoleSpec` const in `roles.rs`.

### Shared skills (`analysis_prompts/skills/`)

- One file per metric: `cs_per_minute.md`, `damage_per_minute.md`, `kills_assists.md`, `deaths.md`, `vision_score.md`.
- Each file is a generic French markdown block with two placeholders: `{benchmarks}` (role-specific thresholds at Platine/Émeraude) and `{role_notes}` (role-specific tactical notes).
- `roles.rs` declares one `SkillBinding` per (role, skill) with an `SkillImportance` (`Critical` / `High` / `Medium` / `Low` / `NotApplicable`) and the two strings that fill the placeholders.
- `NotApplicable` skips the skill block entirely — e.g. support's `cs_per_minute` binding is `NotApplicable`, so support prompts never lecture about CS targets.
- At startup, `AnalysisPipeline::new` reads the role intro + all skill files, composes the final per-role prompt by appending each applicable skill block (with placeholders substituted) under a "Référentiel par compétence" header, and caches it. `get_prompt_for_role` is a simple lookup.
- Skill files are optional — if a skill file is missing, every role that binds it just won't get that block (logged warning, no failure).
