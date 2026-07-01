use crate::server_fns::{ask_llm, list_cases};
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

    view! {
        <Stylesheet id="leptos" href="/pkg/incompliance-radar.css"/>
        <Title text="incomplianceRadar"/>
        <main class="page">
            <header class="page__header">
                <h1>"incomplianceRadar"</h1>
                <p>"Tracking global compliance monitorships, DPAs and NPAs."</p>
            </header>
            <CaseList/>
            <AskPanel/>
        </main>
    }
}

#[component]
fn CaseList() -> impl IntoView {
    let cases = Resource::new(|| (), |_| async move { list_cases().await });

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
