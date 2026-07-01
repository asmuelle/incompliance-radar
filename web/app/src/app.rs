use crate::server_fns::{ask_llm, extract_case, list_cases};
use domain::ComplianceCase;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};

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

    view! {
        <Stylesheet id="leptos" href="/pkg/incompliance-radar.css"/>
        <Title text="incomplianceRadar"/>
        <main class="page">
            <header class="page__header">
                <h1>"incomplianceRadar"</h1>
                <p>"Tracking global compliance monitorships, DPAs and NPAs."</p>
            </header>
            <CaseList extract_action/>
            <ExtractPanel extract_action/>
            <AskPanel/>
        </main>
    }
}

#[component]
fn CaseList(
    extract_action: Action<String, Result<Option<ComplianceCase>, ServerFnError>>,
) -> impl IntoView {
    // Refetches whenever `extract_action` completes, so a newly-extracted
    // case shows up without a manual page reload.
    let cases = Resource::new(
        move || extract_action.version().get(),
        |_| async move { list_cases().await },
    );

    view! {
        <section class="case-list">
            <h2>"Tracked cases"</h2>
            <Suspense fallback=move || view! { <p>"Loading cases..."</p> }>
                {move || {
                    cases
                        .get()
                        .map(|result| match result {
                            Ok(cases) => view! {
                                <ul>
                                    {cases
                                        .into_iter()
                                        .map(|case| {
                                            view! {
                                                <li>
                                                    <strong>{case.company.name.clone()}</strong>
                                                    " — "
                                                    {case.company.industry.clone()}
                                                    " ("
                                                    {case.company.jurisdiction.clone()}
                                                    ") — "
                                                    {case.resolutions.len()}
                                                    " resolution(s)"
                                                </li>
                                            }
                                        })
                                        .collect_view()}
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
