use std::time::Duration;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::sleep;

const MAX_ATTEMPTS: usize = 3;
// The local Gemma 4 server generates ~30 tokens/s and spends part of the
// budget on reasoning tokens before the visible answer, so both the token
// budget and the HTTP timeout are much larger than a hosted-API setup.
const MAX_TOKENS: u32 = 4096;
const REQUEST_TIMEOUT_SECS: u64 = 300;

#[derive(Clone)]
pub struct LlmClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("LLM API error: {0}")]
    ApiError(String),
    #[error("LLM parse error: {0}")]
    ParseError(String),
    #[error("LLM API rate limited")]
    RateLimited,
    #[error("LLM API request timed out")]
    Timeout,
}

#[derive(Debug, Serialize)]
struct LlmRequest {
    model: String,
    messages: Vec<LlmMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LlmRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct LlmResponse {
    choices: Vec<LlmChoice>,
}

#[derive(Debug, Deserialize)]
struct LlmChoice {
    message: LlmMessage,
}

impl LlmClient {
    pub fn new(api_key: String, base_url: String, model: String) -> Result<Self, LlmError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(LlmError::HttpError)?;

        Ok(Self {
            client,
            api_key,
            model,
            base_url,
        })
    }

    pub async fn analyze(&self, prompt: &str, data_json: &str) -> Result<String, LlmError> {
        let prompt_text = build_prompt_text(prompt, data_json);

        let request_body = LlmRequest {
            model: self.model.clone(),
            messages: vec![LlmMessage {
                role: LlmRole::User,
                content: prompt_text,
            }],
            // Low temperature: ratings must be stable across reruns of
            // similar games; this is evaluation, not creative writing.
            temperature: Some(0.35),
            max_tokens: Some(MAX_TOKENS),
        };

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let mut last_error: Option<LlmError> = None;

        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let parsed: LlmResponse = resp.json().await.map_err(LlmError::HttpError)?;

                        return extract_response_text(parsed);
                    }

                    if status == StatusCode::TOO_MANY_REQUESTS {
                        last_error = Some(LlmError::RateLimited);
                        if attempt + 1 < MAX_ATTEMPTS {
                            sleep(Duration::from_secs(1 << attempt)).await;
                            continue;
                        }
                        return Err(LlmError::RateLimited);
                    }

                    if status.is_client_error() {
                        let body_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "<response body unavailable>".to_string());
                        return Err(LlmError::ApiError(body_text));
                    }

                    let body_text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<response body unavailable>".to_string());

                    if status.is_server_error() {
                        last_error = Some(LlmError::ApiError(body_text.clone()));
                        if attempt + 1 < MAX_ATTEMPTS {
                            sleep(Duration::from_secs(1 << attempt)).await;
                            continue;
                        }
                    }

                    return Err(LlmError::ApiError(body_text));
                }
                Err(error) => {
                    if error.is_timeout() {
                        last_error = Some(LlmError::Timeout);
                    } else {
                        last_error = Some(LlmError::HttpError(error));
                    }
                }
            }

            if attempt + 1 < MAX_ATTEMPTS {
                sleep(Duration::from_secs(1 << attempt)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::ApiError("Unknown LLM API error".to_string())))
    }
}

fn build_prompt_text(prompt: &str, data_json: &str) -> String {
    if prompt.contains("{game_data}") {
        prompt.replace("{game_data}", data_json)
    } else {
        format!("{prompt}\n\nGame Data:\n{data_json}")
    }
}

fn extract_response_text(parsed: LlmResponse) -> Result<String, LlmError> {
    parsed
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| {
            // Reasoning models can exhaust max_tokens before emitting any
            // visible answer, which surfaces as an empty `content` field.
            LlmError::ParseError("Missing or empty content in LLM response".to_string())
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
        let parsed = LlmResponse {
            choices: vec![LlmChoice {
                message: LlmMessage {
                    role: LlmRole::Assistant,
                    content: "Great game".to_string(),
                },
            }],
        };

        let text = extract_response_text(parsed).expect("expected response text");
        assert_eq!(text, "Great game");
    }

    #[test]
    fn extract_response_text_returns_error_when_missing() {
        let parsed = LlmResponse { choices: vec![] };

        let error = extract_response_text(parsed).expect_err("expected parse error");
        assert!(matches!(error, LlmError::ParseError(_)));
    }

    #[test]
    fn extract_response_text_returns_error_when_content_empty() {
        let parsed = LlmResponse {
            choices: vec![LlmChoice {
                message: LlmMessage {
                    role: LlmRole::Assistant,
                    content: "  ".to_string(),
                },
            }],
        };

        let error = extract_response_text(parsed).expect_err("expected parse error");
        assert!(matches!(error, LlmError::ParseError(_)));
    }

    #[test]
    fn llm_request_serializes_correctly() {
        let request = LlmRequest {
            model: "gemma-4".to_string(),
            messages: vec![LlmMessage {
                role: LlmRole::User,
                content: "Hello".to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: Some(1024),
        };

        let value = serde_json::to_value(&request).expect("serialize request");
        assert_eq!(value.get("model").unwrap(), "gemma-4");
        let temperature = value
            .get("temperature")
            .and_then(serde_json::Value::as_f64)
            .expect("temperature missing or not a number");
        assert!((temperature - 0.7).abs() < 1e-6);
        assert_eq!(value.get("max_tokens").unwrap(), &serde_json::json!(1024));
        let role = value["messages"][0]["role"].as_str().unwrap();
        assert_eq!(role, "user");
    }
}
