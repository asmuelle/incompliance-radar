# Architecture

Full-stack Rust using [Leptos](https://leptos.dev) (server-side rendering +
WASM hydration) on [Axum](https://github.com/tokio-rs/axum), built with
`cargo-leptos`.

```
crates/
  domain/     Wasm-safe core types (Company, ComplianceCase, Resolution,
              Monitor, Sanction, ViolationType, Regulator).
  llm/        Pluggable LLM provider abstraction (Ollama + Anthropic),
              server-only.
  db/         Persistence: CaseRepository + AlertRepository traits,
              SqliteCaseRepository/SqliteAlertRepository (share one pool),
              server-only.
  extraction/ LLM-based structured extraction of a ComplianceCase from raw
              filing text, server-only.
  crawler/    Scheduled fetch jobs (FilingSource trait + SEC/FCA connectors)
              feeding extraction. Standalone `crawl` binary, server-only.
web/
  app/        Shared Leptos UI + server functions + fictional seed data.
  frontend/   Wasm hydration entry point.
  server/     Axum server binary.
  style/      Plain CSS.
```

## Why split `app` / `frontend` / `server`

`cargo-leptos` needs one crate compiled for `wasm32-unknown-unknown` (the
client) and one compiled natively (the server), sharing UI code. Server-only
dependencies (axum, tokio, sqlx, `llm`, `db`, `extraction`) must never leak
into a crate built for wasm32, or the client build breaks.

`crates/domain` has no async runtime or HTTP client dependency, so it
compiles for both targets and is shared everywhere. `web/app`'s `llm`, `db`,
and `extraction` dependencies are optional and gated behind the `ssr` Cargo
feature; the `#[server]` functions that use them reference them via
fully-qualified paths rather than a top-level `use`, since the macro only
compiles the function body under `ssr` — see `CLAUDE.md` in the repository
root for the full rationale and the pattern to follow when adding new
server-only dependencies.

## Persistence

`crates/db` stores each `ComplianceCase` as a JSON blob in SQLite, alongside
indexed `industry`/`jurisdiction`/`company_name` columns used for filtering.
`web/server` connects and runs migrations at startup, seeding the fictional
demo cases only if the database is empty, then makes the repository
available to server functions through Leptos context
(`leptos_routes_with_context` + `provide_context`).

`CaseRepository::search` combines a SQL `WHERE` on the indexed
industry/jurisdiction columns with an in-memory filter (over that
already-narrowed result set) for violation type and monitor firm, which live
inside the JSON blob rather than their own columns. Still server-side search
from the client's perspective — see [Search and filtering](#search-and-filtering)
below.

## NLP extraction

`crates/extraction` sends raw filing text to the configured `llm::LlmProvider`
with a schema-constrained system prompt, then parses and validates the
model's JSON response before converting it into a `domain::ComplianceCase`
(unrecognized enum values fall back to `Other(_)`; malformed dates, negative
sanction amounts, or an unrecognized status are rejected rather than
guessed). The "Extract a case from filing text" panel in the UI calls this
end-to-end and persists the result via `CaseRepository::upsert`.

## Crawler

`crates/crawler` fetches real filings so extraction doesn't rely solely on
manual paste. `FilingSource` is the per-regulator trait; `run_crawl` fetches,
dedupes by URL against sources already recorded on existing cases, and feeds
new filings through `extraction::extract_case` + `CaseRepository::upsert`.
Two connectors exist, both verified against the live sites: SEC (RSS feed +
Drupal body selector, rate-limit aware) and FCA (general news RSS + article
selector). There's deliberately no DoJ connector — `justice.gov` blocks
automated clients with a bot-management challenge, and defeating that isn't
something this project does. The `crawl` binary runs one pass and exits; an
external scheduler (cron, a systemd timer) invokes it periodically.

## Search and filtering

The UI's `SearchPanel` filters by industry, jurisdiction, violation type, and
law firm/monitor, calling `list_cases` with a `CaseFilterQuery` (an all-`None`
filter matches everything). `CaseList`'s data resource refetches whenever the
filter changes or an extraction completes.

## Alerts

Global watch rules (industry and/or company-name-substring criteria), not
per-user — the app has no auth/user system. `db::evaluate_case` checks a
newly-persisted case against every rule and records a `domain::Alert` for
each match; both `extract_case` and the crawler's `ingest_filing` call it
right after `CaseRepository::upsert`, so a case triggers alerts the same way
regardless of how it arrived. The UI's `WatchRulesPanel` manages rules;
`AlertsPanel` shows and acknowledges triggered alerts. There's no actual
notification delivery (email/push) — alerts only show up in-app.

## Data flow (current state)

Two ways a filing reaches `extraction::extract_case`:

```
Browser (WASM) --hydrate--> App component --#[server] fns--> Axum server
SEC/FCA RSS + pages --crawler::run_crawl-------------------> Axum process
```

Both converge on the same pipeline:

```
raw filing text --> extraction::extract_case (schema prompt + validate)
                           |                        |
                  llm::provider_from_env()   db::CaseRepository::upsert
                           |                        |
              Ollama (local) or                 SQLite
              Anthropic (frontier)
```

See [Roadmap](roadmap.md) for what's next.
