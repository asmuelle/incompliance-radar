//! Turns raw regulatory filing text into a structured `domain::ComplianceCase`
//! via a configured LLM (`crates/llm`), with a schema-constrained prompt
//! (`prompt::SYSTEM_PROMPT`) and validation of the model's JSON output
//! (`parsed::ParsedCase::try_into_domain`) before any of it is trusted.
//!
//! Server-only, like `llm` and `db` — never a wasm target.

mod error;
mod parsed;
mod prompt;

pub use error::ExtractionError;

use domain::ComplianceCase;
use llm::{CompletionRequest, LlmProvider};
use parsed::ParsedCase;

/// Extracts a `ComplianceCase` from raw filing text using the given provider.
/// Assigns fresh ids — callers persist the result via `db::CaseRepository::upsert`.
pub async fn extract_case(
    provider: &dyn LlmProvider,
    raw_text: &str,
) -> Result<ComplianceCase, ExtractionError> {
    let request = CompletionRequest::new(raw_text).with_system(prompt::SYSTEM_PROMPT);
    let response = provider.complete(request).await?;
    let parsed = parse_json_response(&response.text)?;
    parsed.try_into_domain()
}

fn parse_json_response(text: &str) -> Result<ParsedCase, ExtractionError> {
    let json = extract_json_object(text).ok_or_else(|| {
        ExtractionError::Validation("model response did not contain a JSON object".into())
    })?;
    Ok(serde_json::from_str(json)?)
}

/// Models sometimes wrap JSON in prose or markdown code fences despite
/// instructions not to; take the outermost `{...}` block defensively.
fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end >= start).then(|| &text[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_object_strips_markdown_fences() {
        let text = "Here you go:\n```json\n{\"a\": 1}\n```\nHope that helps!";
        assert_eq!(extract_json_object(text), Some("{\"a\": 1}"));
    }

    #[test]
    fn extract_json_object_returns_none_without_braces() {
        assert_eq!(extract_json_object("no json here"), None);
    }
}
