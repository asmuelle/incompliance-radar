# Roadmap

Current state is a working full-stack scaffold: SSR + WASM hydration,
SQLite-backed persistence (`crates/db`, seeded with fictional demo data), a
live query panel against the configured LLM backend, and an LLM-based
extraction pipeline (`crates/extraction`) that turns pasted filing text into
a structured, persisted case. None of the product-defining features from
[Product Concept](product-concept.md) are built yet.

Rough next steps, roughly in dependency order:

1. ~~**Persistence**~~ — done: `crates/db`'s `CaseRepository` trait +
   `SqliteCaseRepository`, wired into `web/server` via Leptos context.
2. ~~**NLP extraction pipeline**~~ — done: `crates/extraction` extracts
   structured `domain::Resolution` fields from raw filing text via the `llm`
   crate, with a schema-constrained prompt and validation, persisted via
   `CaseRepository::upsert`. Reachable today only by pasting text into the
   UI or calling `/api/extract_case` directly — no crawler feeds it yet.
3. **Crawler** — scheduled fetch jobs against DoJ/SEC/FCA/OFAC sources,
   feeding the extraction pipeline via the same `extraction::extract_case`
   entrypoint used by the manual-paste UI today.
4. **Search and filtering UI** — by industry, jurisdiction, violation type,
   law firm. Extend `CaseRepository` with query methods rather than filtering
   `list()` results client-side.
5. **Alerts** — user-scoped subscriptions notified on new filings/monitor
   appointments/DPA conclusions for tracked industries or competitors.
6. **Trend/benchmark analysis** — aggregate statistics across tracked cases.
7. **Routing** (`leptos_router`) — only once there's a second page to
   justify it.

Contributions and issues: [github.com/asmuelle/incompliance-radar](https://github.com/asmuelle/incompliance-radar).
