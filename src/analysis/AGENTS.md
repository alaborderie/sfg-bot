# analysis/

AI-powered post-game analysis using Google Gemini API.

## Files

| File | Purpose |
|---|---|
| `gemini.rs` | `GeminiClient` — HTTP client for Gemini API. Handles request formatting, retry with exponential backoff, rate limit (429) handling |
| `models.rs` | `AnalysisData` (match stats + timeline diffs for prompt context), `AnalysisResult` (rating enum + summary text) |
| `pipeline.rs` | `AnalysisPipeline` — loads role-specific prompts from a directory, selects prompt by player role, serializes match data as JSON, calls Gemini, extracts rating |
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
- Role-specific files: `top.md`, `jungle.md`, `middle.md`, `bottom.md`, `support.md`
- Fallback: `default.md` used when role has no specific prompt
- Prompts written in French, instruct Gemini to produce French output
