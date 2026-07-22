pub mod discord;
pub mod history;
pub mod llm;
pub mod models;
pub mod pipeline;
pub mod roles;

/// Game modes the coach can meaningfully analyze. Arena ("CHERRY") has no
/// lanes, roles, or CS economy, so the role-based prompts produce nonsense
/// there — skip the analysis entirely.
pub fn is_analyzable_mode(game_mode: &str) -> bool {
    !game_mode.eq_ignore_ascii_case("CHERRY")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_modes_are_analyzable() {
        assert!(is_analyzable_mode("CLASSIC"));
        assert!(is_analyzable_mode("ARAM"));
    }

    #[test]
    fn arena_is_not_analyzable() {
        assert!(!is_analyzable_mode("CHERRY"));
        assert!(!is_analyzable_mode("cherry"));
    }
}
