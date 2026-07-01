#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("LLM request failed: {0}")]
    Llm(#[from] llm::LlmError),

    #[error("failed to parse model output as JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("model output failed validation: {0}")]
    Validation(String),
}
