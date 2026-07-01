use domain::ComplianceCase;
use leptos::prelude::*;

/// Lists tracked compliance cases. Backed by in-memory demo data today;
/// swap the body for a repository call once persistence is added.
#[server(endpoint = "/list_cases")]
pub async fn list_cases() -> Result<Vec<ComplianceCase>, ServerFnError> {
    Ok(crate::seed::seed_cases())
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
