---
name: add-sfg-bot-feature
description: Use when extending the sfg-bot Discord bot — adding a new slash command, a new database table or migration, a new analysis prompt for a LoL role, or a new Riot API integration. Covers the full path from code change to deployed binary, including the cross-repo coupling with k3s-ovh-config.
---

# Add an sfg-bot feature

You are editing the sfg-bot repository (Rust edition 2024, serenity 0.12, riven 2, sqlx 0.8, tokio).
Read the root `CLAUDE.md` and the module `CLAUDE.md` next to the files being changed
(`src/discord/`, `src/riot/`, `src/db/`, `src/notification/`, `src/analysis/`).
User-facing Discord text and analysis prompt bodies are French.

Establish the requested scope before acting. A diagnosis or review remains
read-only. Local branches or isolated worktrees are routine preparation for
authorized work unless the user forbids them. Commit, publication, merge,
deployment, workload restarts, and live Discord actions need scope that covers
them. Reuse authority already supplied rather than reconfirming each step.

This skill covers the four most common change shapes. Pick the section matching your task; some changes combine two (e.g. a new command that needs a new table).

---

## A) Add a new slash command

1. **Pick a name** (kebab-case for the slash interface, snake_case for the Rust handler). Use the requested name or existing conventions; ask only if an unresolved product choice matters.
2. In `src/discord/commands.rs`:
   - Add `fn register_{name}() -> CreateCommand` and include it in the `Vec` returned by `register_all()`. Descriptions are French, matching `/init-sfg-bot`, `/add-summoner`, `/remove-summoner`, `/list-summoners`, `/analyze-last-game`.
   - Add `pub async fn run_{name}(...)`. Existing handlers return `()` and answer Discord themselves — there is no `CommandError`. Signature shape, dropping any dependency the command does not use:
     ```rust
     pub async fn run_{name}(
         cache_http: &impl serenity::http::CacheHttp,
         command: &serenity::model::application::CommandInteraction,
         repository: &Arc<dyn Repository>,
         riot_client: &Arc<dyn RiotApiClient>,
         default_region: &str,
     )
     ```
   - Read arguments with `command.data.options()`, then `find_map` on `opt.name` and `ResolvedValue`. Match by option name.
   - The first parameter is `&impl CacheHttp` (the handler passes `&ctx`). Verification drives the same functions without a gateway shard. Fast commands call `command.create_response` with `CreateInteractionResponse::Message`. Commands that hit Riot or the LLM defer first (`CreateInteractionResponse::Defer`), then follow up — see `run` (`/analyze-last-game`). Errors are ephemeral French strings.
3. Add a match arm in `Bot::interaction_create` in `src/discord/handler.rs` on `command.data.name`, passing the same `Arc`s the other arms pass.
4. **User-facing strings are French.** Match the tone of the existing commands.
5. If the command persists state, add a Repository method (see C below). SQL stays in `PgRepository`.
6. Verify: `cargo fmt && cargo clippy -- -D warnings && cargo test --features test-mocks`.

## B) Add a new analysis prompt for a LoL role

Role files are the intro only. `AnalysisPipeline::new` composes the prompt that the LLM sees: role intro + applicable skill blocks (`analysis_prompts/skills/`) + shared sections (`analysis_prompts/shared/rating_rubric.md`, `response_format.md`) + a trailing `{game_data}` section. `strip_frontmatter` drops the YAML block before any of that is sent. The API model is `LLM_MODEL` (default `deepseek-v4.1-flash` on the OpenCode Go OpenAI-compatible API). The frontmatter `model:` field is metadata; existing files set it to `sonnet`.

1. Create `analysis_prompts/{role}.md` in the same shape as `top.md`:
   ```markdown
   ---
   name: lol-coach-{role}
   description: {one-line French description of the role focus}
   model: sonnet
   ---

   Tu es un coach pro de League of Legends spécialisé dans le rôle de {rôle}. ...
   ```
   The body holds the role intro, analysis axes, coaching tips, and role-specific response priorities, in French, and tells the model to answer in French. Leave the rating rubric, the response contract, and `{game_data}` out of the file — the pipeline appends those.
2. Register `("<RIOT_ROLE>", "{role}.md")` in `ROLE_PROMPT_FILES` in `src/analysis/pipeline.rs`. Keys are Riot `team_position` values: `TOP`, `JUNGLE`, `MIDDLE`, `BOTTOM`, `UTILITY`. The support file is `support.md` and its key is `UTILITY`.
3. Add a `RoleSpec` const in `src/analysis/roles.rs` and include it in the `ROLE_SPECS` slice. Each binding is a `SkillBinding` (`skill`, `SkillImportance`, `benchmarks`, `role_notes`). `NotApplicable` omits that skill block.
4. A new metric needs three more edits: `analysis_prompts/skills/{metric}.md` with `{benchmarks}` and `{role_notes}` placeholders, the name added to `SKILL_NAMES` in `pipeline.rs`, and a `SkillBinding` on every `RoleSpec` (use `NotApplicable` where the metric does not apply).
5. Discord entry points (`handler::spawn_analysis_task` and `commands::run`) call `history::analyze_with_memory`. Keep that path so the per-player snapshot is stored.
6. Verify with `cargo test --features test-mocks` (pipeline construction). The live calibration harness (`cargo test --test live_llm_calibration -- --ignored --test-threads=1`) calls the real LLM and needs `LLM_API_KEY`; run it only when the task is to recalibrate ratings.
7. **Dockerfile:** `COPY analysis_prompts ./analysis_prompts` already ships the whole tree, and the builder stage copies `migrations/` before `cargo build`. A new file inside either directory needs no Dockerfile edit. Pipeline init returns an error when the prompts directory or `default.md` is missing; `Bot::new` logs that and starts with analysis disabled. A missing role file logs a warning and that role uses `default.md`.

## C) Add a database migration / model

1. Create `migrations/{YYYYMMDDHHMMSS}_{name}.sql`. Use a real UTC timestamp — files run in lexicographic order via `sqlx::migrate!()` at startup. Follow the existing files: `uuid-ossp` is already enabled, primary keys are `UUID PRIMARY KEY DEFAULT uuid_generate_v4()`, timestamps are `TIMESTAMPTZ`.
2. Write idempotent DDL (`CREATE TABLE IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`, `ADD COLUMN IF NOT EXISTS`) where it does not conflict with the intent of the migration. See `20260722000000_add_end_retry_count_to_active_games.sql`.
3. If the change touches an existing table that already has data in production, prefer expand-then-contract (add the new column nullable → backfill via a separate migration → switch reads → drop the old).
4. Add the row struct in `src/db/models.rs`. Match the columns you created — several tables have no `updated_at`. Insert inputs are a separate `New*` struct with `Debug, Clone` only:
   ```rust
   #[derive(Debug, Clone, FromRow)]
   pub struct Name {
       pub id: Uuid,
       pub created_at: DateTime<Utc>,
   }

   #[derive(Debug, Clone)]
   pub struct NewName {
       // insert columns
   }
   ```
   Store JSON payloads as `TEXT` and parse them with `serde_json` (`analysis_history.analysis_data` does this). The sqlx `json` feature is not enabled. The notification table is `notification_queue`.
5. Add the method on the `Repository` trait and on `PgRepository` in `src/db/repository.rs`. The trait already has `#[async_trait]` and returns `Result<..., RepositoryError>`. Queries are runtime strings with bound parameters:
   ```rust
   sqlx::query_as::<_, Name>("SELECT * FROM name WHERE id = $1")
       .bind(id)
       .fetch_optional(&self.pool)
       .await?;
   ```
   Use `sqlx::query_as::<_, T>`, `sqlx::query`, or `sqlx::query_scalar`. There is no `.sqlx` offline cache and no `cargo sqlx prepare` step — `SQLX_OFFLINE=true` is set in CI, the Docker build, and `.claude/settings.json` because these queries do not need a database at compile time.
6. Tests that mock the trait (`MockRepository`, feature `test-mocks`) need an expectation for every new method they hit.
7. Migrations are embedded by `sqlx::migrate!()` and the image build copies `migrations/`. No manual migrate step in the deploy.

## D) Add a Riot API integration

1. Add the method to `RiotApiClient` in `src/riot/client.rs`. The trait already has `#[async_trait]` and `#[cfg_attr(feature = "test-mocks", mockall::automock)]`, so `MockRiotApiClient` grows the method. Tests that call it still need expectations. Return `Result<..., RiotClientError>`.
2. Implement it on `RiotClient`, delegating to `riven` when the endpoint is a Riot API:
   - Account-v1 and match-v5 take `RegionalRoute` from `RiotClient::regional_for_region`.
   - Spectator (`spectator_v5().get_current_game_info_by_puuid`) takes `PlatformRoute` from `RiotClient::platform_for_region`.
   - `get_all_champions` is a Data Dragon HTTP fetch and takes no route.
   - `RiotClient::new` sets riven retries to 0. Match-lookup retry already lives in `GameTracker`; do not add a second retry loop on the client.
3. Call the method from the existing consumer. The per-summoner loop uses `GameTracker::check_summoner_game_state`. Keep `GameTracker<R: RiotApiClient + ?Sized, D: Repository + ?Sized>` generic — callers hold `Arc<dyn RiotApiClient>` and `Arc<dyn Repository>`.

---

## Cross-repo coupling — applies to all four shapes

Cluster manifests and secrets live in the sibling repo `../k3s-ovh-config` (GitHub: `alaborderie/k3s-ovh-config`). This repository's deploy workflow does not apply those manifests. See the Deployment section of the root `CLAUDE.md`.

If the feature reads a new env var, adds a new port, or needs more CPU/memory:

1. Prepare the required `../k3s-ovh-config` change. Open a PR only when publication is authorized.
2. Update the manifests there:
   - New env var → `env:` entry in `sfg-bot.yaml` with `valueFrom.secretKeyRef`.
   - New secret value → `stringData` entry in `sfg-bot-secrets.yaml`. Secret values stay in that file. `.env.example` in this repo documents the variable name only.
   - New port → `containerPort`, the matching `Service.ports`, and the IngressRoute when the port is HTTP-routable. An HTTP health probe already exists: `HEALTH_CHECK_PORT` binds `0.0.0.0` and answers `200 OK`.
   - New CPU/memory floor → bump `resources.requests` / `resources.limits` in `sfg-bot.yaml`, staying consistent with the values already there.
   - A brand-new database (not an application migration) → `sfg-bot-postgres-init.yaml`. Tables and columns belong in `migrations/` here.
3. If merge is authorized, merge the k3s-ovh-config PR first and wait until that change is applied on the cluster.
4. If merging the application PR is also authorized, merge it next. `.github/workflows/deploy.yml` (push to `main`) pushes the image to `ghcr.io` and, over SSH, runs `sudo kubectl rollout restart deployment sfg-bot`. The rollout restarts the deployment; it does not apply manifest edits.

Skipping step 1 means the new pod starts without the env var, port, or resources it needs.

## Commit & PR

- Conventional Commits with scope: `feat(discord):`, `feat(db):`, `fix(analysis):`, `refactor(notification):`.
- One logical change per PR. Squash on merge is the default — keep the PR title in the same Conventional Commits shape, since it becomes the squashed subject.
- CI (`.github/workflows/ci.yml`, `SQLX_OFFLINE=true`) runs `cargo test --features test-mocks` and lint (`cargo fmt --check`, `cargo clippy -- -D warnings`) in parallel, then `cargo build --release` which needs both. `cargo audit` is advisory and exits 0. Every job except audit must pass.
- Create commits, push, open or merge PRs only within the user's authorized publication scope.

## Verify after deploy

Run live checks only after an authorized deployment. For local-only or read-only work, report the commands and expected evidence without connecting to the cluster or Discord.

```sh
ssh {VPS} 'sudo kubectl logs deployment/sfg-bot --tail=100'
```

Expect `Starting SFG Bot...`, `Database migrations complete`, `Bot ready as {name}`, `Registered {n} global slash command(s)`, `Cache ready, starting background tasks`, `Starting {n} independent polling tasks`, then `Polling task started for {Name}#{Tag}` and `Current game for {Name}#{Tag}: In Game` or `Not In Game`. With `LLM_API_KEY` set, also `Analysis prompts loaded`; without it, `LLM API key not configured, analysis disabled`. With `HEALTH_CHECK_PORT` set, `Health check listener started`.

For analysis changes, trigger `/analyze-last-game` in the channel set by `/init-sfg-bot`. That command does not run the LLM when `LLM_API_KEY` is unset.
