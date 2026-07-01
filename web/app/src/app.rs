use crate::server_fns::{
    acknowledge_alert, ask_llm, create_watch_rule, delete_watch_rule, extract_case,
    get_trend_report, list_alerts, list_cases, list_watch_rules, CaseFilterQuery,
};
use domain::ComplianceCase;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use uuid::Uuid;

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
            <TrendPanel extract_action/>
            <SearchPanel filter/>
            <CaseList extract_action filter/>
            <AlertsPanel extract_action/>
            <WatchRulesPanel/>
            <ExtractPanel extract_action/>
            <AskPanel/>
        </main>
    }
}

#[component]
fn TrendPanel(
    extract_action: Action<String, Result<Option<ComplianceCase>, ServerFnError>>,
) -> impl IntoView {
    // Deliberately not tied to the search filter — trends summarize the
    // whole tracked dataset, not whatever's currently being searched for.
    let report = Resource::new(
        move || extract_action.version().get(),
        |_| async move { get_trend_report().await },
    );

    view! {
        <section class="trend-panel">
            <h2>"Trends"</h2>
            <Suspense fallback=move || view! { <p>"Loading trends..."</p> }>
                {move || {
                    report
                        .get()
                        .map(|result| match result {
                            Ok(report) => trend_report_view(report).into_any(),
                            Err(err) => {
                                view! { <p class="error">{format!("Failed to load trends: {err}")}</p> }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </section>
    }
}

fn trend_report_view(report: domain::TrendReport) -> impl IntoView {
    if report.total_cases == 0 {
        return view! { <p>"No cases tracked yet."</p> }.into_any();
    }

    view! {
        <div>
            <p class="trend-panel__total">
                {format!("{} tracked case(s)", report.total_cases)}
            </p>
            {bar_section("By industry", report.cases_by_industry)}
            {bar_section("By regulator", report.resolutions_by_regulator)}
            {bar_section("By violation type", report.resolutions_by_violation_type)}
            {bar_section("By resolution kind", report.resolutions_by_kind)}
            {bar_section("By status", report.resolutions_by_status)}
            {rate_section("Monitorship rate by industry", report.monitorship_rate_by_industry)}
            {amount_section("Total sanctions by currency", report.total_sanctions_by_currency)}
        </div>
    }
    .into_any()
}

fn bar_section(title: &'static str, entries: Vec<domain::CountEntry>) -> impl IntoView {
    if entries.is_empty() {
        return ().into_any();
    }
    let max = entries.iter().map(|e| e.count).max().unwrap_or(1).max(1);

    view! {
        <div class="trend-section">
            <h3>{title}</h3>
            <ul class="trend-bars">
                {entries
                    .into_iter()
                    .map(|entry| {
                        let width_pct = (entry.count as f64 / max as f64 * 100.0).round();
                        let value = entry.count.to_string();
                        trend_bar_row(entry.label, width_pct, value)
                    })
                    .collect_view()}
            </ul>
        </div>
    }
    .into_any()
}

fn rate_section(title: &'static str, entries: Vec<domain::RateEntry>) -> impl IntoView {
    if entries.is_empty() {
        return ().into_any();
    }

    view! {
        <div class="trend-section">
            <h3>{title}</h3>
            <ul class="trend-bars">
                {entries
                    .into_iter()
                    .map(|entry| {
                        let pct = (entry.rate * 100.0).round();
                        let value = format!("{pct:.0}% (n={})", entry.sample_size);
                        trend_bar_row(entry.label, pct, value)
                    })
                    .collect_view()}
            </ul>
        </div>
    }
    .into_any()
}

fn amount_section(title: &'static str, entries: Vec<domain::AmountEntry>) -> impl IntoView {
    if entries.is_empty() {
        return ().into_any();
    }
    let max = entries
        .iter()
        .map(|e| e.total)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    view! {
        <div class="trend-section">
            <h3>{title}</h3>
            <ul class="trend-bars">
                {entries
                    .into_iter()
                    .map(|entry| {
                        let width_pct = (entry.total / max * 100.0).round();
                        let value = format!("{:.0} {}", entry.total, entry.currency);
                        trend_bar_row(entry.currency.clone(), width_pct, value)
                    })
                    .collect_view()}
            </ul>
        </div>
    }
    .into_any()
}

fn trend_bar_row(label: String, width_pct: f64, value: String) -> impl IntoView {
    view! {
        <li class="trend-bars__row">
            <span class="trend-bars__label">{label}</span>
            <span class="trend-bars__track">
                <span class="trend-bars__fill" style=format!("width: {width_pct}%")></span>
            </span>
            <span class="trend-bars__value">{value}</span>
        </li>
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
fn AlertsPanel(
    extract_action: Action<String, Result<Option<ComplianceCase>, ServerFnError>>,
) -> impl IntoView {
    let acknowledge_action = Action::new(|id: &Uuid| {
        let id = *id;
        async move { acknowledge_alert(id).await }
    });

    // Refetches when a newly-extracted case might have triggered new alerts,
    // or when one gets acknowledged.
    let alerts = Resource::new(
        move || {
            (
                extract_action.version().get(),
                acknowledge_action.version().get(),
            )
        },
        |_| async move { list_alerts().await },
    );

    view! {
        <section class="alerts-panel">
            <h2>"Alerts"</h2>
            <p class="hint">"Triggered automatically when a new case matches a watch rule below."</p>
            <Suspense fallback=move || view! { <p>"Loading alerts..."</p> }>
                {move || {
                    alerts
                        .get()
                        .map(|result| match result {
                            Ok(alerts) if alerts.is_empty() => {
                                view! { <p>"No alerts yet."</p> }.into_any()
                            }
                            Ok(alerts) => view! {
                                <ul class="alerts-panel__list">
                                    {alerts.into_iter().map(alert_list_item(acknowledge_action)).collect_view()}
                                </ul>
                            }
                                .into_any(),
                            Err(err) => {
                                view! { <p class="error">{format!("Failed to load alerts: {err}")}</p> }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </section>
    }
}

fn alert_list_item(
    acknowledge_action: Action<Uuid, Result<(), ServerFnError>>,
) -> impl Fn(domain::Alert) -> AnyView {
    move |alert: domain::Alert| {
        let id = alert.id;
        let item_class = if alert.acknowledged {
            "alerts-panel__item--acknowledged"
        } else {
            ""
        };

        view! {
            <li class=item_class>
                {alert.message}
                <span class="alerts-panel__timestamp">{alert.created_at.to_string()}</span>
                {(!alert.acknowledged)
                    .then(|| {
                        view! {
                            <button on:click=move |_| {
                                acknowledge_action.dispatch(id);
                            }>"Acknowledge"</button>
                        }
                    })}
            </li>
        }
        .into_any()
    }
}

#[component]
fn WatchRulesPanel() -> impl IntoView {
    let (label, set_label) = signal(String::new());
    let (industry, set_industry) = signal(String::new());
    let (company, set_company) = signal(String::new());

    let create_action = Action::new(move |_: &()| {
        let label = label.get_untracked();
        let industry_value = industry.get_untracked();
        let company_value = company.get_untracked();
        let industry = (!industry_value.is_empty()).then_some(industry_value);
        let company = (!company_value.is_empty()).then_some(company_value);
        async move { create_watch_rule(label, industry, company).await }
    });
    let delete_action = Action::new(|id: &Uuid| {
        let id = *id;
        async move { delete_watch_rule(id).await }
    });

    let rules = Resource::new(
        move || (create_action.version().get(), delete_action.version().get()),
        |_| async move { list_watch_rules().await },
    );

    view! {
        <section class="watch-rules-panel">
            <h2>"Watch rules"</h2>
            <p class="hint">
                "Get alerted when a new case matches an industry and/or a company name (e.g. to track a competitor)."
            </p>
            <div class="watch-rules-panel__form">
                <input
                    type="text"
                    placeholder="Label, e.g. \"Banking watch\""
                    prop:value=move || label.get()
                    on:input=move |ev| set_label.set(event_target_value(&ev))
                />
                <input
                    type="text"
                    placeholder="Industry (optional)"
                    prop:value=move || industry.get()
                    on:input=move |ev| set_industry.set(event_target_value(&ev))
                />
                <input
                    type="text"
                    placeholder="Company name contains (optional)"
                    prop:value=move || company.get()
                    on:input=move |ev| set_company.set(event_target_value(&ev))
                />
                <button on:click=move |_| {
                    create_action.dispatch(());
                    set_label.set(String::new());
                    set_industry.set(String::new());
                    set_company.set(String::new());
                }>"Add rule"</button>
            </div>
            <Suspense fallback=move || view! { <p>"Loading watch rules..."</p> }>
                {move || {
                    rules
                        .get()
                        .map(|result| match result {
                            Ok(rules) if rules.is_empty() => {
                                view! { <p>"No watch rules yet."</p> }.into_any()
                            }
                            Ok(rules) => view! {
                                <ul class="watch-rules-panel__list">
                                    {rules.into_iter().map(watch_rule_list_item(delete_action)).collect_view()}
                                </ul>
                            }
                                .into_any(),
                            Err(err) => {
                                view! { <p class="error">{format!("Failed to load watch rules: {err}")}</p> }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </section>
    }
}

fn watch_rule_list_item(
    delete_action: Action<Uuid, Result<(), ServerFnError>>,
) -> impl Fn(domain::WatchRule) -> AnyView {
    move |rule: domain::WatchRule| {
        let id = rule.id;
        let criteria = [
            rule.industry.as_ref().map(|i| format!("industry: {i}")),
            rule.company_name_contains
                .as_ref()
                .map(|c| format!("company contains: {c}")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");

        view! {
            <li>
                <strong>{rule.label}</strong>
                {(!criteria.is_empty()).then(|| format!(" — {criteria}"))}
                <button on:click=move |_| {
                    delete_action.dispatch(id);
                }>"Remove"</button>
            </li>
        }
        .into_any()
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
