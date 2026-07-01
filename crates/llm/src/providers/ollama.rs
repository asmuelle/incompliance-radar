use crate::{CompletionRequest, CompletionResponse, LlmError, LlmProvider};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Talks to a locally running Ollama server (`ollama serve`, default port 11434).
/// No API key needed; the user is responsible for `ollama pull <model>` first.
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.into(),
        }
    }
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    system: Option<&'a str>,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Serialize)]
struct GenerateOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> String {
        format!("ollama:{}", self.model)
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let body = GenerateRequest {
            model: &self.model,
            prompt: &request.prompt,
            system: request.system.as_deref(),
            stream: false,
            options: GenerateOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
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

        let parsed: GenerateResponse = response
            .json()
            .await
            .map_err(|e| LlmError::Parse(e.to_string()))?;

        Ok(CompletionResponse {
            text: parsed.response,
            model: self.model.clone(),
        })
    }
}
