use std::error::Error;
use std::fmt;
use std::time::Duration;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

const DEFAULT_MODEL: &str = "gemini-2.5-flash-lite";
const MAX_ATTEMPTS: usize = 3;
const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Clone)]
pub struct GeminiClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

#[derive(Debug)]
pub enum GeminiError {
    HttpError(reqwest::Error),
    ApiError(String),
    ParseError(String),
    RateLimited,
    Timeout,
}

impl fmt::Display for GeminiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeminiError::HttpError(error) => write!(f, "HTTP error: {error}"),
            GeminiError::ApiError(message) => write!(f, "Gemini API error: {message}"),
            GeminiError::ParseError(message) => write!(f, "Gemini parse error: {message}"),
            GeminiError::RateLimited => write!(f, "Gemini API rate limited"),
            GeminiError::Timeout => write!(f, "Gemini API request timed out"),
        }
    }
}

impl Error for GeminiError {}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
}

#[derive(Debug, Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Debug, Deserialize)]
struct GeminiPartResponse {
    text: Option<String>,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Result<Self, GeminiError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(GeminiError::HttpError)?;

        Ok(Self {
            client,
            api_key,
            model: DEFAULT_MODEL.to_string(),
        })
    }

    pub async fn analyze(&self, prompt: &str, data_json: &str) -> Result<String, GeminiError> {
        let prompt_text = build_prompt_text(prompt, data_json);

        let request_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt_text }],
            }],
            generation_config: GenerationConfig {
                temperature: 0.7,
                max_output_tokens: 1024,
            },
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        );

        let mut last_error: Option<GeminiError> = None;

        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .client
                .post(&url)
                .header("x-goog-api-key", &self.api_key)
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let parsed: GeminiResponse =
                            resp.json().await.map_err(GeminiError::HttpError)?;

                        return extract_response_text(parsed);
                    }

                    if status == StatusCode::TOO_MANY_REQUESTS {
                        last_error = Some(GeminiError::RateLimited);
                        if attempt + 1 < MAX_ATTEMPTS {
                            sleep(Duration::from_secs(1 << attempt)).await;
                            continue;
                        }
                        return Err(GeminiError::RateLimited);
                    }

                    if status.is_client_error() {
                        let body_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "<response body unavailable>".to_string());
                        return Err(GeminiError::ApiError(body_text));
                    }

                    let body_text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<response body unavailable>".to_string());

                    if status.is_server_error() {
                        last_error = Some(GeminiError::ApiError(body_text.clone()));
                        if attempt + 1 < MAX_ATTEMPTS {
                            sleep(Duration::from_secs(1 << attempt)).await;
                            continue;
                        }
                    }

                    return Err(GeminiError::ApiError(body_text));
                }
                Err(error) => {
                    if error.is_timeout() {
                        last_error = Some(GeminiError::Timeout);
                    } else {
                        last_error = Some(GeminiError::HttpError(error));
                    }
                }
            }

            if attempt + 1 < MAX_ATTEMPTS {
                sleep(Duration::from_secs(1 << attempt)).await;
            }
        }

        Err(last_error
            .unwrap_or_else(|| GeminiError::ApiError("Unknown Gemini API error".to_string())))
    }
}

fn build_prompt_text(prompt: &str, data_json: &str) -> String {
    if prompt.contains("{game_data}") {
        prompt.replace("{game_data}", data_json)
    } else {
        format!("{prompt}\n\nGame Data:\n{data_json}")
    }
}

fn extract_response_text(parsed: GeminiResponse) -> Result<String, GeminiError> {
    parsed
        .candidates
        .first()
        .and_then(|candidate| candidate.content.parts.first())
        .and_then(|part| part.text.clone())
        .ok_or_else(|| {
            GeminiError::ParseError("Missing candidates content in Gemini response".to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_text_replaces_placeholder() {
        let prompt = "Analyze: {game_data}";
        let data_json = "{\"foo\":1}";
        let result = build_prompt_text(prompt, data_json);
        assert_eq!(result, "Analyze: {\"foo\":1}");
    }

    #[test]
    fn build_prompt_text_falls_back_when_missing_placeholder() {
        let prompt = "Analyze this game";
        let data_json = "{\"bar\":2}";
        let result = build_prompt_text(prompt, data_json);
        assert!(result.contains("Analyze this game"));
        assert!(result.contains("Game Data:"));
        assert!(result.contains(data_json));
    }

    #[test]
    fn extract_response_text_returns_text_when_present() {
        let parsed = GeminiResponse {
            candidates: vec![GeminiCandidate {
                content: GeminiContentResponse {
                    parts: vec![GeminiPartResponse {
                        text: Some("Great game".to_string()),
                    }],
                },
            }],
        };

        let text = extract_response_text(parsed).expect("expected response text");
        assert_eq!(text, "Great game");
    }

    #[test]
    fn extract_response_text_returns_error_when_missing() {
        let parsed = GeminiResponse {
            candidates: vec![GeminiCandidate {
                content: GeminiContentResponse { parts: vec![] },
            }],
        };

        let error = extract_response_text(parsed).expect_err("expected parse error");
        assert!(matches!(error, GeminiError::ParseError(_)));
    }

    #[test]
    fn gemini_request_serializes_with_generation_config() {
        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: "Hello".to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: 0.7,
                max_output_tokens: 1024,
            },
        };

        let value = serde_json::to_value(&request).expect("serialize request");
        assert!(value.get("contents").is_some());
        let config = value
            .get("generationConfig")
            .expect("generationConfig missing");
        let temperature = config
            .get("temperature")
            .and_then(serde_json::Value::as_f64)
            .expect("temperature missing or not a number");
        assert!((temperature - 0.7).abs() < 1e-6);
        assert_eq!(
            config.get("maxOutputTokens"),
            Some(&serde_json::json!(1024))
        );
    }
}
