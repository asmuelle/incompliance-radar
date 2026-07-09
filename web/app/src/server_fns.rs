use domain::{Alert, ComplianceCase, TrendReport, WatchRule};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Search criteria for [`list_cases`] — a plain-string DTO so it stays
/// wasm-safe (no dependency on the `db` crate); converted to `db::CaseFilter`
/// inside the server-only body. An all-`None` filter matches every case.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CaseFilterQuery {
    pub industry: Option<String>,
    pub jurisdiction: Option<String>,
    pub violation_type: Option<String>,
    pub monitor_firm: Option<String>,
}

/// Lists tracked compliance cases matching `filter` (an empty filter matches
/// everything), from the repository provided via Leptos context (see
/// `web/server/src/main.rs`). Fully-qualified path keeps the client (wasm)
/// build free of the `db` crate, which is only pulled in under the `ssr`
/// feature.
#[server(endpoint = "/list_cases")]
pub async fn list_cases(filter: CaseFilterQuery) -> Result<Vec<ComplianceCase>, ServerFnError> {
    let repo = use_context::<std::sync::Arc<dyn db::CaseRepository>>()
        .ok_or_else(|| ServerFnError::new("case repository not available"))?;
    let db_filter = db::CaseFilter {
        industry: filter.industry,
        jurisdiction: filter.jurisdiction,
        violation_type: filter.violation_type,
        monitor_firm: filter.monitor_firm,
    };
    repo.search(&db_filter)
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
/// configured LLM (`crates/extraction`) and persists it. Returns `None` if
/// the model determines the text isn't actually about an enforcement
/// action/DPA/NPA/monitorship. Fully-qualified paths keep the client (wasm)
/// build free of `extraction`/`llm`/`db`, which are only pulled in under the
/// `ssr` feature.
#[server(endpoint = "/extract_case")]
pub async fn extract_case(raw_text: String) -> Result<Option<ComplianceCase>, ServerFnError> {
    let provider = llm::provider_from_env().map_err(|e| ServerFnError::new(e.to_string()))?;
    let Some(case) = extraction::extract_case(&*provider, &raw_text)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
    else {
        return Ok(None);
    };

    let repo = use_context::<std::sync::Arc<dyn db::CaseRepository>>()
        .ok_or_else(|| ServerFnError::new("case repository not available"))?;
    repo.upsert(&case)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if let Err(err) = db::evaluate_case(&case, &*alert_repo_context()?).await {
        // A watch-rule match failing to record shouldn't fail the extraction
        // itself — the case is already safely persisted at this point.
        leptos::logging::error!("failed to evaluate watch rules for extracted case: {err}");
    }

    Ok(Some(case))
}

// Plain top-level fn, not a `#[server]` body — the macro only strips
// ssr-only *bodies* for the client build, so this needs its own gate or the
// wasm build fails trying to resolve `db` (only an ssr-feature dependency).
#[cfg(feature = "ssr")]
fn alert_repo_context() -> Result<std::sync::Arc<dyn db::AlertRepository>, ServerFnError> {
    use_context::<std::sync::Arc<dyn db::AlertRepository>>()
        .ok_or_else(|| ServerFnError::new("alert repository not available"))
}

/// Watch rules the operator has configured — see `domain::WatchRule` for
/// match semantics. Global, not per-user: this app has no auth/user system.
#[server(endpoint = "/list_watch_rules")]
pub async fn list_watch_rules() -> Result<Vec<WatchRule>, ServerFnError> {
    alert_repo_context()?
        .list_rules()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// The three resolution-level criteria arrive as plain strings (wasm-safe
/// DTO style, like `CaseFilterQuery`): `regime`/`violation_type` are display
/// labels parsed via `Regime::parse`/`ViolationType::parse` (unknown text
/// becomes `Other(..)` rather than being dropped), and `regulator` is free
/// text resolved to a canonical slug via `Regulator::normalize`.
#[server(endpoint = "/create_watch_rule")]
pub async fn create_watch_rule(
    label: String,
    industry: Option<String>,
    company_name_contains: Option<String>,
    regime: Option<String>,
    regulator: Option<String>,
    violation_type: Option<String>,
) -> Result<WatchRule, ServerFnError> {
    let rule = WatchRule::new(
        label,
        industry,
        company_name_contains,
        chrono::Utc::now().naive_utc(),
    )
    .with_regime(regime.as_deref().map(domain::Regime::parse))
    .with_regulator_slug(
        regulator
            .as_deref()
            .map(|text| domain::Regulator::normalize(text).slug),
    )
    .with_violation_type(violation_type.as_deref().map(domain::ViolationType::parse));
    alert_repo_context()?
        .create_rule(&rule)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(rule)
}

#[server(endpoint = "/delete_watch_rule")]
pub async fn delete_watch_rule(id: uuid::Uuid) -> Result<(), ServerFnError> {
    alert_repo_context()?
        .delete_rule(id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Alerts already triggered by past matches, newest first.
#[server(endpoint = "/list_alerts")]
pub async fn list_alerts() -> Result<Vec<Alert>, ServerFnError> {
    alert_repo_context()?
        .list_alerts()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(endpoint = "/acknowledge_alert")]
pub async fn acknowledge_alert(id: uuid::Uuid) -> Result<(), ServerFnError> {
    alert_repo_context()?
        .acknowledge_alert(id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Aggregate statistics across every tracked case (not filtered by the
/// search panel — trends should reflect the whole dataset). See
/// `domain::compute_trend_report` for what's computed.
///
/// Named `get_trend_report`, not `trend_report`: the `#[server]` macro
/// PascalCases the function name for its generated request struct, which
/// would collide with the imported `domain::TrendReport` return type and
/// fail with a confusing orphan-rule error (`only traits defined in the
/// current crate can be implemented for types defined outside of the
/// crate`) — the macro ends up targeting the imported type instead of a
/// struct of its own.
#[server(endpoint = "/trend_report")]
pub async fn get_trend_report() -> Result<TrendReport, ServerFnError> {
    let repo = use_context::<std::sync::Arc<dyn db::CaseRepository>>()
        .ok_or_else(|| ServerFnError::new("case repository not available"))?;
    let cases = repo
        .list()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain::compute_trend_report(&cases))
}
