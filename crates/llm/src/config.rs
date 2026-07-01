use crate::error::LlmError;

const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_OLLAMA_MODEL: &str = "llama3.1";
const DEFAULT_ANTHROPIC_MODEL: &str = "claude-sonnet-5";

#[derive(Debug, Clone)]
pub enum LlmBackend {
    /// Local model served by Ollama (default — no API key required).
    Ollama { base_url: String, model: String },
    /// Frontier model via the Anthropic API.
    Anthropic { api_key: String, model: String },
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub backend: LlmBackend,
}

impl LlmConfig {
    /// Reads:
    /// - `LLM_BACKEND` = `ollama` (default) | `anthropic`
    /// - `OLLAMA_BASE_URL` (default `http://localhost:11434`), `OLLAMA_MODEL` (default `llama3.1`)
    /// - `ANTHROPIC_API_KEY` (required for the anthropic backend), `ANTHROPIC_MODEL`
    ///   (default `claude-sonnet-5`)
    pub fn from_env() -> Result<Self, LlmError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Same as [`Self::from_env`] but takes an arbitrary variable lookup, so the
    /// selection logic can be unit-tested without mutating process-wide env vars.
    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, LlmError> {
        let backend_name = lookup("LLM_BACKEND").unwrap_or_else(|| "ollama".to_string());
        let backend = match backend_name.to_lowercase().as_str() {
            "ollama" => LlmBackend::Ollama {
                base_url: lookup("OLLAMA_BASE_URL")
                    .unwrap_or_else(|| DEFAULT_OLLAMA_BASE_URL.to_string()),
                model: lookup("OLLAMA_MODEL").unwrap_or_else(|| DEFAULT_OLLAMA_MODEL.to_string()),
            },
            "anthropic" => LlmBackend::Anthropic {
                api_key: lookup("ANTHROPIC_API_KEY")
                    .ok_or_else(|| LlmError::Config("ANTHROPIC_API_KEY is not set".into()))?,
                model: lookup("ANTHROPIC_MODEL")
                    .unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.to_string()),
            },
            other => {
                return Err(LlmError::Config(format!(
                    "unknown LLM_BACKEND '{other}', expected 'ollama' or 'anthropic'"
                )))
            }
        };
        Ok(Self { backend })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_ollama_with_default_url_and_model_when_unset() {
        let config = LlmConfig::from_lookup(|_| None).unwrap();
        match config.backend {
            LlmBackend::Ollama { base_url, model } => {
                assert_eq!(base_url, DEFAULT_OLLAMA_BASE_URL);
                assert_eq!(model, DEFAULT_OLLAMA_MODEL);
            }
            LlmBackend::Anthropic { .. } => panic!("expected ollama backend"),
        }
    }

    #[test]
    fn anthropic_backend_requires_api_key() {
        let err =
            LlmConfig::from_lookup(|key| (key == "LLM_BACKEND").then(|| "anthropic".to_string()))
                .unwrap_err();

        assert!(matches!(err, LlmError::Config(_)));
    }

    #[test]
    fn anthropic_backend_uses_provided_api_key_and_model() {
        let config = LlmConfig::from_lookup(|key| match key {
            "LLM_BACKEND" => Some("anthropic".to_string()),
            "ANTHROPIC_API_KEY" => Some("sk-test".to_string()),
            "ANTHROPIC_MODEL" => Some("claude-custom".to_string()),
            _ => None,
        })
        .unwrap();

        match config.backend {
            LlmBackend::Anthropic { api_key, model } => {
                assert_eq!(api_key, "sk-test");
                assert_eq!(model, "claude-custom");
            }
            LlmBackend::Ollama { .. } => panic!("expected anthropic backend"),
        }
    }

    #[test]
    fn unknown_backend_is_rejected() {
        let err =
            LlmConfig::from_lookup(|key| (key == "LLM_BACKEND").then(|| "unknown".to_string()))
                .unwrap_err();

        assert!(matches!(err, LlmError::Config(_)));
    }
}
