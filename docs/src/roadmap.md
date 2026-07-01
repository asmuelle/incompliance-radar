# Roadmap

Current state is a working full-stack scaffold: SSR + WASM hydration,
SQLite-backed persistence (`crates/db`, seeded with fictional demo data), and
a live query panel against the configured LLM backend. None of the
product-defining features from [Product Concept](product-concept.md) are
built yet.

Rough next steps, roughly in dependency order:

1. ~~**Persistence**~~ — done: `crates/db`'s `CaseRepository` trait +
   `SqliteCaseRepository`, wired into `web/server` via Leptos context.
2. **NLP extraction pipeline** — structured extraction of `domain::Resolution`
   fields from raw filing text via the `llm` crate, with a schema-constrained
   prompt and validation, persisted via `CaseRepository::upsert`.
3. **Crawler** — scheduled fetch jobs against DoJ/SEC/FCA/OFAC sources,
   feeding the extraction pipeline.
4. **Search and filtering UI** — by industry, jurisdiction, violation type,
   law firm. Extend `CaseRepository` with query methods rather than filtering
   `list()` results client-side.
5. **Alerts** — user-scoped subscriptions notified on new filings/monitor
   appointments/DPA conclusions for tracked industries or competitors.
6. **Trend/benchmark analysis** — aggregate statistics across tracked cases.
7. **Routing** (`leptos_router`) — only once there's a second page to
   justify it.

Contributions and issues: [github.com/asmuelle/incompliance-radar](https://github.com/asmuelle/incompliance-radar).
