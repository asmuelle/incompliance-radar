# Roadmap

Current state is a working full-stack scaffold: SSR + WASM hydration, an
in-memory demo case list, and a live query panel against the configured LLM
backend. None of the product-defining features from
[Product Concept](product-concept.md) are built yet.

Rough next steps, roughly in dependency order:

1. **Persistence** — a repository trait + real implementation (likely
   `sqlx` + SQLite or Postgres) behind `web/server`, replacing
   `seed::seed_cases()`. Keep it server-only; don't add it to `web/app`
   directly (see `CLAUDE.md`).
2. **NLP extraction pipeline** — structured extraction of `domain::Resolution`
   fields from raw filing text via the `llm` crate, with a schema-constrained
   prompt and validation.
3. **Crawler** — scheduled fetch jobs against DoJ/SEC/FCA/OFAC sources,
   feeding the extraction pipeline.
4. **Search and filtering UI** — by industry, jurisdiction, violation type,
   law firm.
5. **Alerts** — user-scoped subscriptions notified on new filings/monitor
   appointments/DPA conclusions for tracked industries or competitors.
6. **Trend/benchmark analysis** — aggregate statistics across tracked cases.
7. **Routing** (`leptos_router`) — only once there's a second page to
   justify it.

Contributions and issues: [github.com/asmuelle/incompliance-radar](https://github.com/asmuelle/incompliance-radar).
