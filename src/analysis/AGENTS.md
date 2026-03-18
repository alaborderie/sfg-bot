# analysis/

AI-powered post-game analysis using Google Gemini API.

## Files

| File | Purpose |
|---|---|
| `gemini.rs` | `GeminiClient` — HTTP client for Gemini API. Handles request formatting, retry with exponential backoff, rate limit (429) handling |
| `models.rs` | `AnalysisData` (match stats + timeline diffs for prompt context), `AnalysisResult` (rating enum + summary text) |
| `pipeline.rs` | `AnalysisPipeline` — loads prompt template from `ANALYSIS_PROMPT_PATH`, interpolates match data, calls Gemini, parses response to extract rating (Good/Average/Poor) |
| `discord.rs` | Embed formatters for analysis results — rating-based color coding, description truncation for Discord limits |

## Key Details

### Pipeline Flow

1. `AnalysisPipeline::analyze(match_data)` called after game ends
2. Loads prompt template from file (default: `ANALYSIS_PROMPT.md`)
3. Builds `AnalysisData` with rich context: player stats, team stats, timeline events, gold/xp diffs
4. Interpolates data into prompt template
5. Sends to Gemini API (`gemini-3.1-flash-lite-preview` model)
6. Parses response: extracts rating line (Good/Average/Poor) + summary text
7. Returns `AnalysisResult` for embed formatting

### Gemini Client (gemini.rs)

- Uses `reqwest` for HTTP
- Endpoint: `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
- Retry logic: exponential backoff on 429 (rate limit) and 5xx errors
- API key passed as query parameter

### Analysis Output

- **Rating**: `Good`, `Average`, or `Poor` — determines embed color (green/yellow/red)
- **Summary**: French-language analysis text, truncated to fit Discord embed limits
- All analysis text generated in French (prompt template is in French)

### Prompt Template

- Located at `ANALYSIS_PROMPT.md` (configurable via `ANALYSIS_PROMPT_PATH`)
- Template uses placeholder variables for match data interpolation
- Written in French, instructs Gemini to produce French output
