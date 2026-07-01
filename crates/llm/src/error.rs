#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("missing configuration: {0}")]
    Config(String),

    #[error("request to LLM backend failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("LLM backend returned an error response ({status}): {body}")]
    BackendError { status: u16, body: String },

    #[error("failed to parse LLM response: {0}")]
    Parse(String),
}
