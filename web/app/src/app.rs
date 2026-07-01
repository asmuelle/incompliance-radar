use crate::server_fns::{ask_llm, extract_case, list_cases, CaseFilterQuery};
use domain::ComplianceCase;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};

/// Known `ViolationType` display labels for the search filter dropdown.
/// `Other(_)` variants (whatever a filing didn't map to a known type) aren't
/// listed since they're arbitrary strings — filtering by exact text match on
/// `Other(_)` content isn't useful as a fixed dropdown option.
const VIOLATION_TYPE_OPTIONS: &[&str] = &[
    "Bribery",
    "Money Laundering",
    "Sanctions Violation",
    "Antitrust Fraud",
    "Securities Fraud",
    "Tax Evasion",
    "Export Control",
];

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let extract_action = Action::new(|raw_text: &String| {
        let raw_text = raw_text.to_owned();
        async move { extract_case(raw_text).await }
    });
    let filter = RwSignal::new(CaseFilterQuery::default());

    view! {
        <Stylesheet id="leptos" href="/pkg/incompliance-radar.css"/>
        <Title text="incomplianceRadar"/>
        <main class="page">
            <header class="page__header">
                <h1>"incomplianceRadar"</h1>
                <p>"Tracking global compliance monitorships, DPAs and NPAs."</p>
            </header>
            <SearchPanel filter/>
            <CaseList extract_action filter/>
            <ExtractPanel extract_action/>
            <AskPanel/>
        </main>
    }
}

#[component]
fn SearchPanel(filter: RwSignal<CaseFilterQuery>) -> impl IntoView {
    let non_empty = |s: String| (!s.is_empty()).then_some(s);

    view! {
        <section class="search-panel">
            <h2>"Search cases"</h2>
            <div class="search-panel__fields">
                <label>
                    "Industry"
                    <input
                        type="text"
                        placeholder="e.g. Banking"
                        prop:value=move || filter.get().industry.unwrap_or_default()
                        on:input=move |ev| {
                            filter.update(|f| f.industry = non_empty(event_target_value(&ev)));
                        }
                    />
                </label>
                <label>
                    "Jurisdiction"
                    <input
                        type="text"
                        placeholder="e.g. US"
                        prop:value=move || filter.get().jurisdiction.unwrap_or_default()
                        on:input=move |ev| {
                            filter.update(|f| f.jurisdiction = non_empty(event_target_value(&ev)));
                        }
                    />
                </label>
                <label>
                    "Violation type"
                    <select
                        on:change=move |ev| {
                            filter.update(|f| f.violation_type = non_empty(event_target_value(&ev)));
                        }
                    >
                        <option value="">"Any"</option>
                        {VIOLATION_TYPE_OPTIONS
                            .iter()
                            .map(|option| view! { <option value=*option>{*option}</option> })
                            .collect_view()}
                    </select>
                </label>
                <label>
                    "Law firm / monitor"
                    <input
                        type="text"
                        placeholder="e.g. Kroll"
                        prop:value=move || filter.get().monitor_firm.unwrap_or_default()
                        on:input=move |ev| {
                            filter.update(|f| f.monitor_firm = non_empty(event_target_value(&ev)));
                        }
                    />
                </label>
            </div>
        </section>
    }
}

#[component]
fn CaseList(
    extract_action: Action<String, Result<Option<ComplianceCase>, ServerFnError>>,
    filter: RwSignal<CaseFilterQuery>,
) -> impl IntoView {
    // Refetches whenever `extract_action` completes (a newly-extracted case
    // should show up without a manual page reload) or the filter changes.
    let cases = Resource::new(
        move || (extract_action.version().get(), filter.get()),
        |(_, filter)| async move { list_cases(filter).await },
    );

    view! {
        <section class="case-list">
            <h2>"Tracked cases"</h2>
            <Suspense fallback=move || view! { <p>"Loading cases..."</p> }>
                {move || {
                    cases
                        .get()
                        .map(|result| match result {
                            Ok(cases) if cases.is_empty() => {
                                view! { <p>"No cases match this search."</p> }.into_any()
                            }
                            Ok(cases) => view! {
                                <ul>
                                    {cases.into_iter().map(case_list_item).collect_view()}
                                </ul>
                            }
                                .into_any(),
                            Err(err) => {
                                view! { <p class="error">{format!("Failed to load cases: {err}")}</p> }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </section>
    }
}

fn case_list_item(case: ComplianceCase) -> impl IntoView {
    view! {
        <li>
            <strong>{case.company.name}</strong>
            " — "
            {case.company.industry}
            " ("
            {case.company.jurisdiction}
            ")"
            <ul class="resolution-list">
                {case.resolutions.into_iter().map(resolution_list_item).collect_view()}
            </ul>
        </li>
    }
}

fn resolution_list_item(resolution: domain::Resolution) -> impl IntoView {
    let violations = resolution
        .violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let monitor_firm = resolution.monitor.and_then(|m| m.firm);

    view! {
        <li>
            {resolution.regulator.to_string()} " " {resolution.kind.to_string()} " — "
            {resolution.status.to_string()}
            {(!violations.is_empty()).then(|| view! { <span>" · " {violations}</span> })}
            {monitor_firm.map(|firm| view! { <span>" · Monitor: " {firm}</span> })}
        </li>
    }
}

#[component]
fn ExtractPanel(
    extract_action: Action<String, Result<Option<ComplianceCase>, ServerFnError>>,
) -> impl IntoView {
    let (raw_text, set_raw_text) = signal(String::new());

    view! {
        <section class="extract-panel">
            <h2>"Extract a case from filing text"</h2>
            <p class="hint">
                "Paste a press release or filing excerpt; the configured LLM extracts structured fields and saves the result to the case list above."
            </p>
            <textarea
                prop:value=move || raw_text.get()
                on:input=move |ev| set_raw_text.set(event_target_value(&ev))
                placeholder="e.g. paste a DoJ press release announcing a deferred prosecution agreement..."
            />
            <button on:click=move |_| {
                extract_action.dispatch(raw_text.get());
            }>"Extract & Save"</button>
            <div class="extract-panel__result">
                {move || match extract_action.value().get() {
                    Some(Ok(Some(case))) => {
                        format!("Saved \"{}\" with {} resolution(s).", case.company.name, case.resolutions.len())
                    }
                    Some(Ok(None)) => {
                        "That text doesn't look like an enforcement action, DPA/NPA, or monitorship — nothing saved."
                            .to_string()
                    }
                    Some(Err(err)) => format!("Error: {err}"),
                    None => String::new(),
                }}
            </div>
        </section>
    }
}

#[component]
fn AskPanel() -> impl IntoView {
    let (prompt, set_prompt) = signal(String::new());
    let ask = Action::new(|prompt: &String| {
        let prompt = prompt.to_owned();
        async move { ask_llm(prompt).await }
    });

    view! {
        <section class="ask-panel">
            <h2>"Ask the configured LLM"</h2>
            <p class="hint">
                "Uses whichever backend is set via LLM_BACKEND: a local Ollama model or the Anthropic frontier API."
            </p>
            <textarea
                prop:value=move || prompt.get()
                on:input=move |ev| set_prompt.set(event_target_value(&ev))
                placeholder="e.g. Summarize the typical obligations in an FCPA DPA"
            />
            <button on:click=move |_| {
                ask.dispatch(prompt.get());
            }>"Ask"</button>
            <div class="ask-panel__result">
                {move || match ask.value().get() {
                    Some(Ok(text)) => text,
                    Some(Err(err)) => format!("Error: {err}"),
                    None => String::new(),
                }}
            </div>
        </section>
    }
}
