# Roadmap

Current state is a working full-stack scaffold: SSR + WASM hydration,
SQLite-backed persistence (`crates/db`, seeded with fictional demo data), a
live query panel against the configured LLM backend, an LLM-based extraction
pipeline (`crates/extraction`), a crawler (`crates/crawler`) that pulls real
press releases from the SEC and FCA and feeds them through extraction
automatically, search/filtering by industry, jurisdiction, violation type,
and law firm/monitor, and global watch-rule alerts. Trend analysis from
[Product Concept](product-concept.md) is not built yet.

Rough next steps, roughly in dependency order:

1. ~~**Persistence**~~ — done: `crates/db`'s `CaseRepository` trait +
   `SqliteCaseRepository`, wired into `web/server` via Leptos context.
2. ~~**NLP extraction pipeline**~~ — done: `crates/extraction` extracts
   structured `domain::Resolution` fields from raw filing text via the `llm`
   crate, with a schema-constrained prompt and validation, persisted via
   `CaseRepository::upsert`.
3. ~~**Crawler**~~ — done for SEC and FCA: `crates/crawler`'s `FilingSource`
   trait + `run_crawl` fetch, dedupe, and feed real filings through the same
   `extraction::extract_case` entrypoint the manual-paste UI uses. No DoJ
   connector (bot-blocked, see CLAUDE.md) or OFAC connector yet. Nothing
   schedules the `crawl` binary itself — that's on the operator (cron,
   systemd timer, ...).
4. ~~**Search and filtering UI**~~ — done: `SearchPanel`/`CaseList`
   (`web/app/src/app.rs`) filter by industry, jurisdiction, violation type,
   and law firm/monitor via `CaseRepository::search`, server-side (not
   client-side filtering of a fully-fetched list). No free-text search,
   date-range filtering, or pagination yet — revisit once there's a real
   backlog of cases to justify it.
5. ~~**Alerts**~~ — done as **global** watch rules, not user-scoped (this app
   has no auth/user system — see CLAUDE.md for that tradeoff).
   `domain::WatchRule` (industry and/or company-name-substring criteria) +
   `db::evaluate_case`, checked after every case persisted via manual
   extraction or the crawler. `WatchRulesPanel`/`AlertsPanel` manage rules and
   show/acknowledge triggered alerts. No actual notification delivery
   (email/push) — in-app only. Revisit user-scoping if/when real
   multi-user need emerges.
6. **Trend/benchmark analysis** — aggregate statistics across tracked cases.
7. **Routing** (`leptos_router`) — only once there's a second page to
   justify it.
8. **OFAC connector, and a real DoJ data source** — e.g. an official
   API/data-sharing arrangement rather than scraping around their bot
   protection.

Contributions and issues: [github.com/asmuelle/incompliance-radar](https://github.com/asmuelle/incompliance-radar).
