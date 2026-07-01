use crate::{CompletionRequest, CompletionResponse, LlmError, LlmProvider};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Frontier model backend via the Anthropic Messages API.
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    temperature: f32,
    system: Option<&'a str>,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(default)]
    text: String,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> String {
        format!("anthropic:{}", self.model)
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let body = MessagesRequest {
            model: &self.model,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            system: request.system.as_deref(),
            messages: vec![Message {
                role: "user",
                content: &request.prompt,
            }],
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::BackendError {
                status: status.as_u16(),
                body,
            });
        }

        let parsed: MessagesResponse = response
            .json()
            .await
            .map_err(|e| LlmError::Parse(e.to_string()))?;

        let text = parsed
            .content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResponse {
            text,
            model: self.model.clone(),
        })
    }
}
