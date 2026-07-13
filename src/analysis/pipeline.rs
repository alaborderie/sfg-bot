use std::collections::HashMap;
use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::analysis::llm::{LlmClient, LlmError};
use crate::analysis::models::{AnalysisData, AnalysisResult};
use crate::analysis::roles::{ROLE_SPECS, RoleSpec, SkillImportance, spec_for};

/// Maps Riot API team_position values to prompt file names.
const ROLE_PROMPT_FILES: &[(&str, &str)] = &[
    ("TOP", "top.md"),
    ("JUNGLE", "jungle.md"),
    ("MIDDLE", "middle.md"),
    ("BOTTOM", "bottom.md"),
    ("UTILITY", "support.md"),
];

const DEFAULT_PROMPT_FILE: &str = "default.md";
const SKILLS_SUBDIR: &str = "skills";
const SHARED_SUBDIR: &str = "shared";
/// Shared prompt sections appended to every composed prompt, in this order.
const SHARED_SECTION_FILES: &[&str] = &["rating_rubric", "response_format"];
/// Trailing section holding the match JSON; `{game_data}` is substituted by
/// `LlmClient::analyze` at request time.
const GAME_DATA_SECTION: &str = "---\n\n## Données de la partie (JSON)\n\n{game_data}";
const SKILL_NAMES: &[&str] = &[
    "cs_per_minute",
    "damage_per_minute",
    "kills_assists",
    "deaths",
    "vision_score",
];

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("LLM error: {0}")]
    LlmError(#[from] LlmError),
    #[error("Prompt file error: {0}")]
    PromptFileError(#[from] std::io::Error),
    #[error("Prompt directory error: {0}")]
    PromptDirError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Clone)]
pub struct AnalysisPipeline {
    llm_client: LlmClient,
    /// Final composed prompts (role intro + skill blocks) keyed by Riot role.
    role_prompts: HashMap<String, String>,
    /// Fallback used when no role-specific composed prompt exists.
    default_prompt: String,
}

impl AnalysisPipeline {
    pub fn new(llm_client: LlmClient, prompts_dir: &str) -> Result<Self, AnalysisError> {
        let dir_path = Path::new(prompts_dir);

        if !dir_path.is_dir() {
            return Err(AnalysisError::PromptDirError(format!(
                "Prompts directory not found: {prompts_dir}"
            )));
        }

        let default_prompt_path = dir_path.join(DEFAULT_PROMPT_FILE);
        let default_intro = fs::read_to_string(&default_prompt_path)
            .map(|raw| strip_frontmatter(&raw).to_string())
            .map_err(|e| {
                AnalysisError::PromptDirError(format!(
                    "Failed to read default prompt {}: {e}",
                    default_prompt_path.display()
                ))
            })?;

        let skills = load_skills(dir_path);
        let shared = load_shared_sections(dir_path);

        let default_prompt = finalize_prompt(compose_base(&default_intro, &shared));

        let mut role_prompts = HashMap::new();
        for (role, filename) in ROLE_PROMPT_FILES {
            let file_path = dir_path.join(filename);
            let intro = match fs::read_to_string(&file_path) {
                Ok(content) => strip_frontmatter(&content).to_string(),
                Err(e) => {
                    tracing::warn!(
                        role = *role,
                        file = %file_path.display(),
                        error = %e,
                        "Role-specific prompt not found, will use default"
                    );
                    continue;
                }
            };

            let composed = match spec_for(role) {
                Some(spec) => compose_prompt(&intro, spec, &skills, &shared),
                None => compose_base(&intro, &shared),
            };
            role_prompts.insert((*role).to_string(), finalize_prompt(composed));
        }

        tracing::info!(
            loaded_roles = role_prompts.len(),
            total_roles = ROLE_PROMPT_FILES.len(),
            loaded_skills = skills.len(),
            specs = ROLE_SPECS.len(),
            "Analysis prompts loaded"
        );

        Ok(Self {
            llm_client,
            role_prompts,
            default_prompt,
        })
    }

    fn get_prompt_for_role(&self, role: &str) -> &str {
        self.role_prompts
            .get(role)
            .map(String::as_str)
            .unwrap_or(&self.default_prompt)
    }

    pub async fn analyze_game(&self, data: &AnalysisData) -> AnalysisResult {
        let prompt = self.get_prompt_for_role(&data.role);

        let error_message = match serde_json::to_string_pretty(data) {
            Ok(data_json) => {
                let result = self.llm_client.analyze(prompt, &data_json).await;

                match result {
                    Ok(text) => {
                        let overall_rating = extract_overall_rating(&text);
                        return AnalysisResult {
                            summoner_name: data.summoner_name.clone(),
                            champion_name: data.champion_name.clone(),
                            overall_rating,
                            summary: text,
                            error: None,
                        };
                    }
                    Err(error) => {
                        tracing::warn!(
                            summoner = data.summoner_name.as_str(),
                            role = data.role.as_str(),
                            error = %error,
                            "LLM analysis failed"
                        );
                        error.to_string()
                    }
                }
            }
            Err(error) => {
                tracing::warn!(
                    summoner = data.summoner_name.as_str(),
                    error = %error,
                    "Failed to serialize analysis data"
                );
                error.to_string()
            }
        };

        AnalysisResult {
            summoner_name: data.summoner_name.clone(),
            champion_name: data.champion_name.clone(),
            overall_rating: None,
            summary: format!("Analyse indisponible : {error_message}"),
            error: Some(error_message),
        }
    }
}

/// Loads every shared skill template into a `name -> body` map. Missing
/// files are skipped with a warning rather than failing startup — a role
/// that references a missing skill simply won't get that block.
fn load_skills(prompts_dir: &Path) -> HashMap<&'static str, String> {
    let skills_dir = prompts_dir.join(SKILLS_SUBDIR);
    let mut out = HashMap::new();
    if !skills_dir.is_dir() {
        tracing::warn!(
            dir = %skills_dir.display(),
            "Skills directory not found; role prompts will fall back to intro-only"
        );
        return out;
    }
    for name in SKILL_NAMES {
        let path = skills_dir.join(format!("{name}.md"));
        match fs::read_to_string(&path) {
            Ok(raw) => {
                out.insert(*name, strip_frontmatter(&raw).to_string());
            }
            Err(e) => {
                tracing::warn!(
                    skill = *name,
                    file = %path.display(),
                    error = %e,
                    "Skill file not found, role bindings referencing it will be skipped"
                );
            }
        }
    }
    out
}

/// Loads the shared prompt sections (rating rubric, response format) that are
/// appended to every composed prompt. Missing files are skipped with a warning
/// rather than failing startup, mirroring the skills loading behaviour.
fn load_shared_sections(prompts_dir: &Path) -> Vec<String> {
    let shared_dir = prompts_dir.join(SHARED_SUBDIR);
    let mut out = Vec::new();
    if !shared_dir.is_dir() {
        tracing::warn!(
            dir = %shared_dir.display(),
            "Shared prompts directory not found; composed prompts will omit shared sections"
        );
        return out;
    }
    for name in SHARED_SECTION_FILES {
        let path = shared_dir.join(format!("{name}.md"));
        match fs::read_to_string(&path) {
            Ok(raw) => out.push(strip_frontmatter(&raw).trim().to_string()),
            Err(e) => {
                tracing::warn!(
                    section = *name,
                    file = %path.display(),
                    error = %e,
                    "Shared prompt section not found, composed prompts will omit it"
                );
            }
        }
    }
    out
}

/// Intro + shared sections, for prompts without a role spec (default prompt,
/// unknown roles).
fn compose_base(intro: &str, shared: &[String]) -> String {
    let mut sections: Vec<String> = vec![intro.trim_end().to_string()];
    sections.extend(shared.iter().cloned());
    sections.join("\n\n")
}

/// Appends the game-data section unless the intro already carries its own
/// `{game_data}` placeholder (kept for custom prompt directories that predate
/// shared-section composition).
fn finalize_prompt(composed: String) -> String {
    if composed.contains("{game_data}") {
        composed
    } else {
        format!("{composed}\n\n{GAME_DATA_SECTION}")
    }
}

/// Composes the final per-role prompt: role intro + each applicable skill
/// block with role-specific thresholds substituted in, followed by the shared
/// sections.
fn compose_prompt(
    intro: &str,
    spec: &RoleSpec,
    skills: &HashMap<&'static str, String>,
    shared: &[String],
) -> String {
    let mut sections: Vec<String> = vec![intro.trim_end().to_string()];

    sections.push(String::from(
        "\n---\n\n## Référentiel par compétence (Platine / Émeraude)\n",
    ));

    for binding in spec.bindings {
        if binding.importance == SkillImportance::NotApplicable {
            continue;
        }
        let Some(template) = skills.get(binding.skill) else {
            continue;
        };
        let body = template
            .replace("{benchmarks}", binding.benchmarks)
            .replace("{role_notes}", binding.role_notes);

        if let Some(label) = binding.importance.label_fr() {
            sections.push(format!("{label}\n\n{body}"));
        } else {
            sections.push(body);
        }
    }

    sections.extend(shared.iter().cloned());

    sections.join("\n\n")
}

/// Strips YAML frontmatter from a prompt file so Claude agent metadata
/// (`---\nname: ...\n---`) is not sent to the LLM as part of the system prompt.
fn strip_frontmatter(raw: &str) -> &str {
    let trimmed = raw.trim_start_matches('\u{feff}');
    let Some(rest) = trimmed.strip_prefix("---") else {
        return raw;
    };
    let after_open = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"));
    let Some(after_open) = after_open else {
        return raw;
    };
    let mut search_from = 0;
    while let Some(idx) = after_open[search_from..].find("---") {
        let abs = search_from + idx;
        let starts_at_line_start = abs == 0 || after_open.as_bytes()[abs - 1] == b'\n';
        if starts_at_line_start {
            let after_close = &after_open[abs + 3..];
            let trimmed = after_close
                .strip_prefix("\r\n")
                .or_else(|| after_close.strip_prefix('\n'))
                .unwrap_or(after_close);
            return trimmed.trim_start();
        }
        search_from = abs + 3;
    }
    raw
}

/// Extracts the overall rating from the LLM's response.
///
/// The prompt instructs the LLM to start the response with one of `Good`,
/// `Average`, or `Poor`. We pick the earliest **word-boundary** occurrence of
/// any of those three terms (case-insensitive). This is more robust than a
/// substring scan, which would:
/// - Return `Good` if the response merely *mentioned* the word later on,
///   even when the actual rating up-front was `Poor`.
/// - Prefer `Good` over `Poor` regardless of which appears first.
fn extract_overall_rating(text: &str) -> Option<String> {
    const CANDIDATES: &[(&str, &str)] =
        &[("good", "Good"), ("average", "Average"), ("poor", "Poor")];

    let lowered = text.to_lowercase();
    let bytes = lowered.as_bytes();
    let mut best: Option<(usize, &str)> = None;

    for (needle, label) in CANDIDATES {
        let mut start = 0;
        while let Some(rel) = lowered[start..].find(needle) {
            let abs = start + rel;
            let before_ok = abs == 0 || !bytes[abs - 1].is_ascii_alphabetic();
            let end = abs + needle.len();
            let after_ok = end >= bytes.len() || !bytes[end].is_ascii_alphabetic();
            if before_ok && after_ok {
                match best {
                    Some((existing, _)) if existing <= abs => {}
                    _ => best = Some((abs, label)),
                }
                break;
            }
            start = abs + 1;
        }
    }

    best.map(|(_, label)| label.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::models::AnalysisData;
    use std::fs;
    use tempfile::TempDir;

    fn sample_analysis_data() -> AnalysisData {
        AnalysisData {
            summoner_name: "TestSummoner".to_string(),
            champion_name: "Ahri".to_string(),
            win: true,
            kills: 10,
            deaths: 2,
            assists: 8,
            kda: Some(9.0),
            kill_participation: Some(0.65),
            gold_per_minute: Some(450.0),
            damage_per_minute: Some(600.0),
            vision_score_per_minute: Some(1.2),
            team_damage_percentage: Some(0.28),
            max_cs_advantage_on_lane_opponent: Some(12.0),
            early_laning_phase_gold_exp_advantage: Some(150.0),
            laning_phase_gold_exp_advantage: Some(220.0),
            turret_kills: 2,
            inhibitor_kills: 0,
            objectives_stolen: 0,
            damage_dealt_to_objectives: 3200,
            total_damage_dealt_to_champions: 24000,
            gold_earned: 15000,
            total_cs: 210,
            enemy_champion_name: Some("Orianna".to_string()),
            enemy_cs: Some(190),
            enemy_gold: Some(14000),
            enemy_damage: Some(20000),
            gold_diff_at_10: Some(300),
            gold_diff_at_15: Some(450),
            gold_diff_at_20: Some(600),
            cs_diff_at_10: Some(10),
            cs_diff_at_15: Some(15),
            cs_diff_at_20: Some(20),
            game_duration_secs: 2100,
            role: "MIDDLE".to_string(),
            game_mode: "CLASSIC".to_string(),
        }
    }

    fn write_role_files(dir: &TempDir) {
        fs::write(dir.path().join("default.md"), "Default prompt: {game_data}").unwrap();
        fs::write(dir.path().join("top.md"), "Top lane prompt: {game_data}").unwrap();
        fs::write(dir.path().join("jungle.md"), "Jungle prompt: {game_data}").unwrap();
        fs::write(dir.path().join("middle.md"), "Mid lane prompt: {game_data}").unwrap();
        fs::write(dir.path().join("bottom.md"), "ADC prompt: {game_data}").unwrap();
        fs::write(dir.path().join("support.md"), "Support prompt: {game_data}").unwrap();
    }

    fn write_skill_files(dir: &TempDir) {
        let skills = dir.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        for name in SKILL_NAMES {
            fs::write(
                skills.join(format!("{name}.md")),
                format!("### {name} skill\n\n{{benchmarks}}\n\n{{role_notes}}\n"),
            )
            .unwrap();
        }
    }

    fn make_client() -> LlmClient {
        LlmClient::new(
            "fake-key".to_string(),
            "http://localhost:8080/v1".to_string(),
            "gemma-4-26b".to_string(),
        )
        .unwrap()
    }

    fn make_pipeline(dir: &TempDir) -> AnalysisPipeline {
        let client = make_client();
        AnalysisPipeline::new(client, dir.path().to_str().unwrap()).unwrap()
    }

    #[test]
    fn extract_overall_rating_detects_good() {
        let result = extract_overall_rating("Good performance overall");
        assert_eq!(result.as_deref(), Some("Good"));
    }

    #[test]
    fn extract_overall_rating_detects_average() {
        let result = extract_overall_rating("An average showing");
        assert_eq!(result.as_deref(), Some("Average"));
    }

    #[test]
    fn extract_overall_rating_detects_poor() {
        let result = extract_overall_rating("Poor lane control");
        assert_eq!(result.as_deref(), Some("Poor"));
    }

    #[test]
    fn extract_overall_rating_returns_none_when_missing() {
        let result = extract_overall_rating("Solid play with no rating keyword");
        assert!(result.is_none());
    }

    #[test]
    fn extract_overall_rating_picks_first_word_when_multiple_present() {
        let result = extract_overall_rating("Poor laning but a Good late game");
        assert_eq!(result.as_deref(), Some("Poor"));
    }

    #[test]
    fn extract_overall_rating_ignores_substring_in_other_word() {
        let result = extract_overall_rating("Misunderstood positioning");
        assert!(result.is_none());
    }

    #[test]
    fn extract_overall_rating_word_boundaries_for_poor() {
        let result = extract_overall_rating("poorly executed teamfights, Average overall");
        assert_eq!(result.as_deref(), Some("Average"));
    }

    #[test]
    fn strip_frontmatter_removes_yaml_block() {
        let input = "---\nname: lol-coach-top\ndescription: foo\n---\n\nTu es un coach.";
        assert_eq!(strip_frontmatter(input), "Tu es un coach.");
    }

    #[test]
    fn strip_frontmatter_passthrough_when_absent() {
        let input = "Tu es un coach.\nLigne 2.";
        assert_eq!(strip_frontmatter(input), input);
    }

    #[test]
    fn strip_frontmatter_passthrough_when_unterminated() {
        let input = "---\nname: foo\nno closing fence";
        assert_eq!(strip_frontmatter(input), input);
    }

    #[test]
    fn pipeline_strips_frontmatter_from_prompts() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("default.md"),
            "---\nname: lol-coach-default\ndescription: d\n---\n\nDefault prompt: {game_data}",
        )
        .unwrap();
        fs::write(
            dir.path().join("top.md"),
            "---\nname: lol-coach-top\n---\n\nTop lane prompt: {game_data}",
        )
        .unwrap();

        let pipeline = make_pipeline(&dir);

        let top = pipeline.get_prompt_for_role("TOP");
        assert!(!top.contains("name: lol-coach-top"));
        assert!(top.starts_with("Top lane prompt:"));

        let fallback = pipeline.get_prompt_for_role("UNKNOWN");
        assert!(!fallback.contains("name: lol-coach-default"));
        assert!(fallback.starts_with("Default prompt:"));
    }

    #[test]
    fn analysis_data_serializes_to_json() {
        let data = sample_analysis_data();
        let json = serde_json::to_string_pretty(&data).expect("serialize analysis data");
        assert!(json.contains("TestSummoner"));
        assert!(json.contains("Ahri"));
        assert!(json.contains("gold_per_minute"));
    }

    #[test]
    fn get_prompt_for_role_returns_role_specific_prompt() {
        let dir = TempDir::new().unwrap();
        write_role_files(&dir);
        let pipeline = make_pipeline(&dir);

        assert!(pipeline.get_prompt_for_role("TOP").contains("Top lane"));
        assert!(pipeline.get_prompt_for_role("JUNGLE").contains("Jungle"));
        assert!(pipeline.get_prompt_for_role("MIDDLE").contains("Mid lane"));
        assert!(pipeline.get_prompt_for_role("BOTTOM").contains("ADC"));
        assert!(pipeline.get_prompt_for_role("UTILITY").contains("Support"));
    }

    #[test]
    fn get_prompt_for_role_falls_back_to_default() {
        let dir = TempDir::new().unwrap();
        write_role_files(&dir);
        let pipeline = make_pipeline(&dir);

        assert!(pipeline.get_prompt_for_role("").contains("Default"));
        assert!(pipeline.get_prompt_for_role("UNKNOWN").contains("Default"));
    }

    #[test]
    fn new_fails_when_directory_missing() {
        let client = make_client();
        let result = AnalysisPipeline::new(client, "/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn new_fails_when_default_prompt_missing() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("top.md"), "Top prompt").unwrap();

        let client = make_client();
        let result = AnalysisPipeline::new(client, dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn repo_prompts_compose_with_shared_sections() {
        let pipeline = AnalysisPipeline::new(make_client(), "analysis_prompts").unwrap();

        for role in [
            "TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY", "", "UNKNOWN",
        ] {
            let prompt = pipeline.get_prompt_for_role(role);
            assert!(
                prompt.contains("## Barème de notation"),
                "rating rubric missing for role {role:?}"
            );
            assert!(
                prompt.contains("## Format de réponse"),
                "response format missing for role {role:?}"
            );
            assert_eq!(
                prompt.matches("{game_data}").count(),
                1,
                "expected exactly one game_data placeholder for role {role:?}"
            );
            assert!(
                prompt.trim_end().ends_with("{game_data}"),
                "game data section must be last for role {role:?}"
            );
        }

        let top = pipeline.get_prompt_for_role("TOP");
        assert!(top.contains("Top Lane"));
        assert!(top.contains("Référentiel par compétence"));
        let referential = top.find("Référentiel par compétence").unwrap();
        let rubric = top.find("## Barème de notation").unwrap();
        let format = top.find("## Format de réponse").unwrap();
        assert!(
            referential < rubric && rubric < format,
            "sections out of order: referential={referential} rubric={rubric} format={format}"
        );

        assert!(pipeline.get_prompt_for_role("UTILITY").contains("Support"));
    }

    #[test]
    fn new_succeeds_with_only_default_prompt() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("default.md"), "Default only: {game_data}").unwrap();
        let pipeline = make_pipeline(&dir);

        assert!(pipeline.get_prompt_for_role("TOP").contains("Default"));
        assert!(pipeline.get_prompt_for_role("MIDDLE").contains("Default"));
    }

    #[test]
    fn composed_role_prompt_includes_applicable_skills() {
        let dir = TempDir::new().unwrap();
        write_role_files(&dir);
        write_skill_files(&dir);
        let pipeline = make_pipeline(&dir);

        let top = pipeline.get_prompt_for_role("TOP");
        assert!(top.contains("Top lane prompt"));
        assert!(top.contains("Référentiel par compétence"));
        assert!(top.contains("cs_per_minute skill"));
        assert!(top.contains("damage_per_minute skill"));
        assert!(top.contains("kills_assists skill"));
        assert!(top.contains("deaths skill"));
        assert!(top.contains("vision_score skill"));
        // Top spec marks CS as Critical, so the critical label should appear.
        assert!(top.contains("Métrique critique"));
    }

    #[test]
    fn composed_support_prompt_skips_cs_per_minute() {
        let dir = TempDir::new().unwrap();
        write_role_files(&dir);
        write_skill_files(&dir);
        let pipeline = make_pipeline(&dir);

        let support = pipeline.get_prompt_for_role("UTILITY");
        assert!(support.contains("Support prompt"));
        // Support config marks CS as NotApplicable → its block must NOT appear.
        assert!(!support.contains("cs_per_minute skill"));
        // ...but vision and KP are critical and must appear.
        assert!(support.contains("vision_score skill"));
        assert!(support.contains("kills_assists skill"));
    }

    #[test]
    fn composed_prompt_substitutes_benchmarks_and_notes() {
        let dir = TempDir::new().unwrap();
        write_role_files(&dir);
        write_skill_files(&dir);
        let pipeline = make_pipeline(&dir);

        let adc = pipeline.get_prompt_for_role("BOTTOM");
        // BOTTOM spec has concrete CS benchmarks like "8-9 CS/min".
        assert!(adc.contains("8-9 CS/min"));
        // It also has concrete DPM benchmarks like "800-1100 DPM".
        assert!(adc.contains("800-1100 DPM"));
    }

    #[test]
    fn pipeline_works_without_skills_directory() {
        let dir = TempDir::new().unwrap();
        write_role_files(&dir);
        // No skills dir written.
        let pipeline = make_pipeline(&dir);

        let top = pipeline.get_prompt_for_role("TOP");
        // Still composes the header but no skill bodies.
        assert!(top.contains("Top lane prompt"));
        assert!(!top.contains("cs_per_minute skill"));
    }
}
