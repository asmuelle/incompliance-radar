//! Pluggable LLM provider abstraction used for the NLP extraction pipeline
//! (parsing DPAs/NPAs/monitor appointments out of regulatory text).
//!
//! Two backends ship out of the box:
//! - [`providers::ollama::OllamaProvider`] — local models served by Ollama.
//! - [`providers::anthropic::AnthropicProvider`] — Anthropic's frontier models.
//!
//! Both implement the same [`LlmProvider`] trait so the rest of the app (and
//! future backends) never need to know which one is active. Selection happens
//! once at startup via [`LlmConfig::from_env`].

mod config;
mod error;
pub mod providers;

pub use config::{LlmBackend, LlmConfig};
pub use error::LlmError;

use async_trait::async_trait;

/// A single request to complete/extract against an LLM.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// System prompt / instructions (extraction schema, role, constraints).
    pub system: Option<String>,
    /// User content, e.g. the raw regulatory document text.
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl CompletionRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            system: None,
            prompt: prompt.into(),
            max_tokens: 1024,
            temperature: 0.0,
        }
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
}

/// Common interface every LLM backend (local or frontier) implements.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Human-readable name, e.g. "ollama:llama3.1" or "anthropic:claude-sonnet-5".
    fn name(&self) -> String;

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;
}

/// Build the configured provider from environment variables. See [`LlmConfig::from_env`]
/// for the variables it reads (`LLM_BACKEND`, `OLLAMA_BASE_URL`, `OLLAMA_MODEL`,
/// `ANTHROPIC_API_KEY`, `ANTHROPIC_MODEL`).
pub fn provider_from_env() -> Result<Box<dyn LlmProvider>, LlmError> {
    match LlmConfig::from_env()?.backend {
        LlmBackend::Ollama { base_url, model } => Ok(Box::new(
            providers::ollama::OllamaProvider::new(base_url, model),
        )),
        LlmBackend::Anthropic { api_key, model } => Ok(Box::new(
            providers::anthropic::AnthropicProvider::new(api_key, model),
        )),
    }
}
