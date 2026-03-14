use crate::analysis::models::AnalysisResult;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::model::Colour;

const MAX_DESCRIPTION_LEN: usize = 4096;

pub fn format_analysis_embed(result: &AnalysisResult) -> CreateEmbed {
    let (title, description, colour) = if let Some(error) = &result.error {
        (
            format!("📊 Game Analysis — {}", result.champion_name),
            format!("⚠️ Analysis unavailable: {error}"),
            Colour::from_rgb(149, 165, 166),
        )
    } else {
        (
            format!("📊 Game Analysis — {}", result.champion_name),
            result.summary.clone(),
            rating_colour(result.overall_rating.as_deref()),
        )
    };

    CreateEmbed::new()
        .title(title)
        .description(truncate_description(&description))
        .colour(colour)
        .footer(CreateEmbedFooter::new("Powered by Gemini Flash Lite"))
}

pub fn format_analysis_error_embed(summoner_name: &str, error_msg: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("📊 Game Analysis — {summoner_name}"))
        .description(truncate_description(&format!(
            "⚠️ Analysis unavailable: {error_msg}"
        )))
        .colour(Colour::from_rgb(149, 165, 166))
        .footer(CreateEmbedFooter::new("Powered by Gemini Flash Lite"))
}

fn rating_colour(rating: Option<&str>) -> Colour {
    match rating {
        Some(rating) if rating.eq_ignore_ascii_case("good") => Colour::from_rgb(46, 204, 113),
        Some(rating) if rating.eq_ignore_ascii_case("average") => Colour::from_rgb(241, 196, 15),
        Some(rating) if rating.eq_ignore_ascii_case("poor") => Colour::from_rgb(231, 76, 60),
        _ => Colour::from_rgb(149, 165, 166),
    }
}

fn truncate_description(description: &str) -> String {
    if description.len() <= MAX_DESCRIPTION_LEN {
        return description.to_string();
    }

    let mut truncated = description
        .chars()
        .take(MAX_DESCRIPTION_LEN - 3)
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_analysis_embed_sets_good_colour() {
        let result = AnalysisResult {
            summoner_name: "Test".to_string(),
            champion_name: "Ahri".to_string(),
            overall_rating: Some("Good".to_string()),
            summary: "Great game".to_string(),
            error: None,
        };

        let embed = format_analysis_embed(&result);
        let value = serde_json::to_value(embed).expect("serialize embed");
        assert_eq!(
            value.get("color"),
            Some(&serde_json::json!(Colour::from_rgb(46, 204, 113).0))
        );
        assert_eq!(
            value.get("description"),
            Some(&serde_json::json!("Great game"))
        );
    }

    #[test]
    fn format_analysis_embed_sets_error_colour() {
        let result = AnalysisResult {
            summoner_name: "Test".to_string(),
            champion_name: "Ahri".to_string(),
            overall_rating: None,
            summary: "Analysis unavailable: timeout".to_string(),
            error: Some("timeout".to_string()),
        };

        let embed = format_analysis_embed(&result);
        let value = serde_json::to_value(embed).expect("serialize embed");
        assert_eq!(
            value.get("color"),
            Some(&serde_json::json!(Colour::from_rgb(149, 165, 166).0))
        );
        assert!(
            value
                .get("description")
                .and_then(|desc| desc.as_str())
                .unwrap_or_default()
                .contains("Analysis unavailable")
        );
    }

    #[test]
    fn format_analysis_error_embed_sets_title_and_colour() {
        let embed = format_analysis_error_embed("Summoner", "rate limited");
        let value = serde_json::to_value(embed).expect("serialize embed");
        assert_eq!(
            value.get("title"),
            Some(&serde_json::json!("📊 Game Analysis — Summoner"))
        );
        assert_eq!(
            value.get("color"),
            Some(&serde_json::json!(Colour::from_rgb(149, 165, 166).0))
        );
    }

    #[test]
    fn format_analysis_embed_truncates_long_description() {
        let long_summary = "a".repeat(5000);
        let result = AnalysisResult {
            summoner_name: "Test".to_string(),
            champion_name: "Ahri".to_string(),
            overall_rating: Some("Good".to_string()),
            summary: long_summary,
            error: None,
        };

        let embed = format_analysis_embed(&result);
        let value = serde_json::to_value(embed).expect("serialize embed");
        let description = value
            .get("description")
            .and_then(|desc| desc.as_str())
            .unwrap_or_default();
        assert_eq!(description.len(), MAX_DESCRIPTION_LEN);
        assert!(description.ends_with("..."));
    }
}
