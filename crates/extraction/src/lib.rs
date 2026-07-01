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

/// Generous budget: some local models "think out loud" before emitting JSON,
/// and real filing text can produce long obligations/sanctions lists. Seen
/// truncated (no closing brace at all) at the previous default of 1024
/// against a verbose real FCA press release — don't lower this without
/// re-testing against real filing text.
const MAX_RESPONSE_TOKENS: u32 = 4096;

/// Extracts a `ComplianceCase` from raw filing text using the given provider.
/// Returns `Ok(None)` when the model determines the text isn't actually about
/// an enforcement action, DPA/NPA, or monitorship (see `prompt::SYSTEM_PROMPT`'s
/// `not_applicable` escape hatch) — callers such as the crawler feed in plenty
/// of unrelated press releases and news items, and that's a normal outcome,
/// not a failure. Assigns fresh ids on `Some` — callers persist the result via
/// `db::CaseRepository::upsert`.
pub async fn extract_case(
    provider: &dyn LlmProvider,
    raw_text: &str,
) -> Result<Option<ComplianceCase>, ExtractionError> {
    let request = CompletionRequest::new(raw_text)
        .with_system(prompt::SYSTEM_PROMPT)
        .with_max_tokens(MAX_RESPONSE_TOKENS);
    let response = provider.complete(request).await?;

    let json = extract_json_object(&response.text).ok_or_else(|| {
        ExtractionError::Validation("model response did not contain a JSON object".into())
    })?;

    if is_not_applicable(json) {
        return Ok(None);
    }

    let parsed: ParsedCase = serde_json::from_str(json)?;
    parsed.try_into_domain()
}

/// A real `ParsedCase` object also deserializes fine into this (its extra
/// fields are simply ignored), so this only ever returns `true` for the
/// `{ "not_applicable": true }` sentinel the prompt asks for.
fn is_not_applicable(json: &str) -> bool {
    #[derive(serde::Deserialize)]
    struct NotApplicable {
        #[serde(default)]
        not_applicable: bool,
    }
    serde_json::from_str::<NotApplicable>(json)
        .map(|v| v.not_applicable)
        .unwrap_or(false)
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

    #[test]
    fn is_not_applicable_recognizes_the_sentinel() {
        assert!(is_not_applicable(r#"{"not_applicable": true}"#));
    }

    #[test]
    fn is_not_applicable_is_false_for_a_real_case_object() {
        let json = r#"{"company_name": "Acme", "not_applicable": false}"#;
        assert!(!is_not_applicable(json));
    }

    #[test]
    fn is_not_applicable_is_false_when_key_is_absent() {
        assert!(!is_not_applicable(r#"{"company_name": "Acme"}"#));
    }
}
