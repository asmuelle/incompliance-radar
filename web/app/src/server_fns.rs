use domain::ComplianceCase;
use leptos::prelude::*;

/// Lists tracked compliance cases from the repository provided via Leptos
/// context (see `web/server/src/main.rs`). Fully-qualified path keeps the
/// client (wasm) build free of the `db` crate, which is only pulled in under
/// the `ssr` feature.
#[server(endpoint = "/list_cases")]
pub async fn list_cases() -> Result<Vec<ComplianceCase>, ServerFnError> {
    let repo = use_context::<std::sync::Arc<dyn db::CaseRepository>>()
        .ok_or_else(|| ServerFnError::new("case repository not available"))?;
    repo.list()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Runs a free-form prompt against whichever LLM backend is configured via
/// `LLM_BACKEND` (local Ollama model or the Anthropic frontier API) — see
/// `crates/llm`. Fully-qualified paths here keep the client (wasm) build free
/// of the `llm` crate, which is only pulled in under the `ssr` feature.
#[server(endpoint = "/ask_llm")]
pub async fn ask_llm(prompt: String) -> Result<String, ServerFnError> {
    let provider = llm::provider_from_env().map_err(|e| ServerFnError::new(e.to_string()))?;
    let response = llm::LlmProvider::complete(&*provider, llm::CompletionRequest::new(prompt))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(response.text)
}

/// Extracts a structured compliance case from raw filing text via the
/// configured LLM (`crates/extraction`) and persists it. Fully-qualified
/// paths keep the client (wasm) build free of `extraction`/`llm`/`db`, which
/// are only pulled in under the `ssr` feature.
#[server(endpoint = "/extract_case")]
pub async fn extract_case(raw_text: String) -> Result<ComplianceCase, ServerFnError> {
    let provider = llm::provider_from_env().map_err(|e| ServerFnError::new(e.to_string()))?;
    let case = extraction::extract_case(&*provider, &raw_text)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let repo = use_context::<std::sync::Arc<dyn db::CaseRepository>>()
        .ok_or_else(|| ServerFnError::new("case repository not available"))?;
    repo.upsert(&case)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(case)
}
