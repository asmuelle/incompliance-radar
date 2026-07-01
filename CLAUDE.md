# CLAUDE.md

Guidance for Claude Code (and other agentic coding tools) working in this repository.

## What this is

incomplianceRadar is a platform for tracking global compliance monitorships,
Deferred Prosecution Agreements (DPAs) and Non-Prosecution Agreements (NPAs) —
see `spec.md` for the full product concept (in German). The long-term vision
includes a crawler, NLP extraction pipeline, search/filtering, alerting and
trend analysis. **What exists today is the foundational full-stack Rust
scaffold**, not the full product — treat `spec.md` as the north star and this
file as the current state of the implementation.

## Architecture

Full-stack Rust using [Leptos](https://leptos.dev) (SSR + WASM) on
[Axum](https://github.com/tokio-rs/axum), built with `cargo-leptos`.

```
crates/
  domain/     Wasm-safe core types (Company, ComplianceCase, Resolution, Monitor,
              Sanction, ViolationType, Regulator). No tokio/reqwest/sqlx — must
              compile for both native and wasm32-unknown-unknown.
  llm/        Pluggable LLM provider abstraction. `LlmProvider` trait +
              `OllamaProvider` (local models) + `AnthropicProvider` (frontier).
              Server-only (uses reqwest/tokio); never a wasm target.
  db/         Persistence. `CaseRepository` trait + `SqliteCaseRepository`
              (sqlx). Server-only; never a wasm target.
  extraction/ LLM-based structured extraction of a `ComplianceCase` from raw
              filing text (schema-constrained prompt + JSON parse +
              validation). Depends on `llm` and `domain`; server-only.
  crawler/    Scheduled fetch jobs (`FilingSource` trait + SEC/FCA
              connectors) feeding `extraction`. Standalone `crawl` binary,
              not part of the web app. Server-only.
web/
  app/        Shared UI: the `App` component, `shell()` HTML document,
              `#[server]` functions (server_fns.rs), and fictional seed data
              (seed.rs, used to populate an empty database on first run).
              Compiles for BOTH native (ssr feature) and wasm32 (used by
              `frontend`). This is the crate you touch for almost all UI and
              server-function changes.
  frontend/   Thin wasm hydration entry point only (`hydrate()` +
              `wasm_bindgen`). Rarely needs edits.
  server/     Axum binary (`main.rs`). Connects/migrates/seeds the database,
              wires up leptos_routes + static file serving.
  style/      Plain CSS (main.css) — no Sass toolchain required.
```

### Why the app/frontend/server split

cargo-leptos needs one crate compiled for wasm32 (client) and one compiled
natively (server), from a *shared* UI crate. Server-only dependencies (axum,
tokio, sqlx, `llm`, `db`, `extraction`) must never leak into a crate that
gets built for wasm32-unknown-unknown, or the wasm build breaks. Concretely:

- `crates/domain` has zero ssr-only dependencies — it's imported by all of
  `app`, `frontend`, and `server`, native and wasm alike.
- `web/app`'s `llm`, `db`, and `extraction` dependencies are `optional =
  true`, gated behind the `ssr` feature. `server_fns.rs` calls them via
  **fully-qualified paths** (`llm::provider_from_env()`, `db::CaseRepository`,
  `extraction::extract_case`) instead of a top-level `use llm::...;` /
  `use db::...;` / `use extraction::...;`, because the `#[server]` macro
  only compiles the function *body* under the `ssr` feature — a top-level
  `use` statement is a plain module item and would break the wasm build if
  the crate weren't available there.
- Don't add axum/tokio/sqlx/reqwest as unconditional dependencies of
  `web/app` — always gate them behind `ssr` the same way.

## LLM backend

Selected at runtime via `LLM_BACKEND` env var (see `.env.example`):

- `ollama` (default) — talks to a local Ollama server, no API key. Requires
  `ollama serve` running and the model pulled (`ollama pull llama3.1` or set
  `OLLAMA_MODEL` to whatever's already pulled, e.g. `ollama list`).
- `anthropic` — frontier model via the Anthropic Messages API. Requires
  `ANTHROPIC_API_KEY`.

Both implement `llm::LlmProvider` (`crates/llm/src/lib.rs`). To add a new
backend (e.g. OpenAI, a candle-based local backend): implement the trait in
`crates/llm/src/providers/`, add a variant to `LlmBackend` in
`crates/llm/src/config.rs`, and wire it into `provider_from_env()`.

The `ask_llm` server function (`web/app/src/server_fns.rs`) demonstrates the
end-to-end wiring: UI → server fn → `llm::provider_from_env()` → whichever
backend is configured.

## Persistence

`crates/db` defines `CaseRepository` (`list`/`get`/`upsert`/`delete`) and the
only implementation, `SqliteCaseRepository`. Each case is stored as a JSON
blob (the full `domain::ComplianceCase`, including nested resolutions/
monitors/sanctions) alongside a few indexed columns — see
`crates/db/migrations/0001_create_compliance_cases.sql` for why the schema
isn't normalized further yet.

`web/server/src/main.rs` connects (creating the SQLite file and running
migrations if needed), seeds the fictional demo cases from `app::seed` only
if the database is empty, wraps the repository in `Arc<dyn CaseRepository>`,
and makes it available to server functions via
`leptos_axum::LeptosRoutes::leptos_routes_with_context` +
`provide_context(repo.clone())`. `list_cases` in `web/app/src/server_fns.rs`
retrieves it with `use_context::<Arc<dyn db::CaseRepository>>()`.

Configured via `DATABASE_URL` (see `.env.example`), default
`sqlite://incompliance-radar.db?mode=rwc` (created relative to the working
directory the server binary is run from).

To add a write path (crawler ingestion, manual curation UI, etc.), call
`CaseRepository::upsert` from a new server function the same way — the trait
already supports it, only a caller is missing.

## NLP extraction

`crates/extraction::extract_case(provider, raw_text) -> Result<Option<ComplianceCase>, ExtractionError>`
turns raw filing text into a `domain::ComplianceCase` using any
`llm::LlmProvider`:

1. `prompt::SYSTEM_PROMPT` first asks the model to decide whether the text is
   actually about an enforcement action/DPA/NPA/monitorship at all; if not,
   it returns `{"not_applicable": true}` and `extract_case` returns `Ok(None)`
   — a normal, expected outcome (most items in a general regulator news feed
   aren't enforcement actions), not an error. This exists because the naive
   version — requiring every field, always — broke the first time it was fed
   real (non-enforcement) FCA news items: the model correctly followed
   "use null for unclear fields", which failed to deserialize into
   `ParsedCase`'s required `String` fields. Don't remove the escape hatch
   without re-testing against a real, mixed-content feed.

   Also seen live: the model sometimes ignores the standalone-sentinel
   instruction and instead returns a **full** case-shaped object with the
   literal string `"not_applicable"` stuffed into `company_name`. Because
   that's a non-empty string, it silently passed validation and produced a
   garbage row (`company_name: "not_applicable"`) until
   `parsed::looks_like_not_applicable_marker` was added to catch it — see
   `parsed.rs`'s `company_name_literally_not_applicable_is_treated_as_not_a_case`
   test. If you see garbage rows again, check the actual model response
   (`response.text` before parsing) rather than assuming the schema/validation
   is wrong; models don't reliably follow "always respond in shape A or B,
   never a hybrid."
2. Otherwise, it returns a single JSON object with an exact field shape (see
   the prompt itself for the schema).
3. `extract_json_object` defensively strips prose/markdown fences models
   sometimes add despite instructions not to, taking the outermost `{...}`.
4. `parsed::ParsedCase` (a plain-string DTO, deliberately looser than
   `domain`'s types) deserializes the JSON, then `try_into_domain` validates
   it at the boundary: known enum values (regulator, resolution kind,
   violation type) map to their `domain` variant with an `Other(_)` fallback
   for anything unrecognized; `status` has **no** fallback (`domain::
   ResolutionStatus` has no `Other` variant) and is rejected if it isn't
   exactly one of active/completed/terminated/breached; dates must parse as
   `YYYY-MM-DD`; sanction amounts must be non-negative. See
   `crates/extraction/src/parsed.rs` tests for the exact validation matrix.

The `extract_case` server function (`web/app/src/server_fns.rs`) wires it
end-to-end: raw text → `extraction::extract_case` → `CaseRepository::upsert`
(skipped on `None`) → returned to the UI, which bumps `Action::version()` to
refetch the case list (see `ExtractPanel`/`CaseList` in
`web/app/src/app.rs`).

`MAX_RESPONSE_TOKENS` (4096) in `lib.rs` is deliberately generous — a real
FCA press release truncated at the previous default of 1024 (some local
models emit reasoning before the JSON), producing a response with no closing
brace at all. Don't lower it without re-testing against real filing text.

## Crawler

`crates/crawler` feeds real filings into `extraction::extract_case`
automatically instead of requiring manual paste. `FilingSource` is the
per-regulator trait (`fetch_recent() -> Vec<RawFiling>`); `run_crawl(source,
provider, repo)` fetches, dedupes against URLs already recorded as a
resolution's `source` on an existing case (an O(n) full-table scan via
`repo.list()` — fine at today's scale, revisit with a dedicated indexed query
if the case count grows large), and calls `extraction::extract_case` +
`CaseRepository::upsert` for the rest. One filing failing extraction or
persistence is logged and counted, not fatal to the run.

Two connectors exist today, both verified against the live sites (not just
written from guessed HTML structure):

- `sources::sec::SecPressReleases` — SEC's press release RSS feed
  (`sec.gov/news/pressreleases.rss`) + per-page `div.field--name-body`
  (Drupal). SEC's fair-access policy requires a descriptive `User-Agent`,
  which `SecPressReleases::new` takes as a parameter — **customize it for
  your deployment**, don't ship the default verbatim. SEC also actively rate
  limits (~1 req/sec is safe; going faster gets you a 403 with `Request Rate
  Threshold Exceeded`, which the crawler detects via `is_rate_limited` and
  stops that source's fetch early rather than burning through the rest of
  the batch on failures).
- `sources::fca::FcaNews` — FCA's general news feed (there's no
  press-releases-only feed; `/news/press-releases/rss.xml` 404s) + per-page
  `article` selector. Expect a lot of `Ok(None)` from non-enforcement items
  (speeches, consultations, blog posts) mixed into this feed.

**The DoJ has no connector** — `justice.gov` sits behind an Akamai
bot-management interstitial (a JS proof-of-work challenge) that blocks plain
HTTP clients. Don't try to solve/bypass that challenge; if DoJ coverage is
needed, look for an official API/data-sharing arrangement instead of
defeating their anti-automation controls.

Both connectors cap fetches at `MAX_ITEMS` (10) per run, both to bound LLM
call volume and to avoid re-fetching a regulator's entire feed history every
run — there's no `since` cursor, `run_crawl`'s URL-based dedup is what makes
re-fetching the same window safe.

The `crawl` binary (`crates/crawler/src/bin/crawl.rs`) runs one pass across
all configured sources and exits — it is **not** a scheduler. Invoke it
periodically yourself, e.g. via cron:

```cron
# every 6 hours
0 */6 * * * cd /path/to/incompliance-radar && DATABASE_URL=sqlite://incompliance-radar.db LLM_BACKEND=ollama ./target/release/crawl >> crawl.log 2>&1
```

## Commands

```bash
cargo leptos watch          # dev server with hot reload, http://127.0.0.1:3000
cargo leptos build          # production build (native server + wasm client)
cargo leptos build --release
cargo test --workspace      # unit tests (native crates only)
cargo check -p app --features ssr           # check server-side app compile
cargo check -p frontend --target wasm32-unknown-unknown  # check wasm compile
cargo fmt --all
cargo clippy --workspace --exclude frontend -- -D warnings   # frontend needs --target wasm32-unknown-unknown for clippy
cargo build -p crawler --bin crawl && ./target/debug/crawl   # one crawl pass, see Crawler section
```

`cargo-leptos` and the `wasm32-unknown-unknown` target must be installed:
`cargo install cargo-leptos`, `rustup target add wasm32-unknown-unknown`.
The `wasm-bindgen-cli` version installed **must exactly match** the
`wasm-bindgen` crate version resolved in `Cargo.lock`, or `cargo leptos build`
fails with a schema-version mismatch — `cargo install wasm-bindgen-cli
--version <X>` to fix.

## Current known gaps (documented, not silently missing)

The crawler covers SEC and FCA only — no DoJ (bot-blocked, see Crawler
section above) or OFAC connector yet. Nothing is actually scheduled: the
`crawl` binary must be invoked periodically by something external (cron,
systemd timer, ...).

There is no search/filtering UI yet, even though the schema indexes
`industry`/`jurisdiction` for it. Add query methods to `CaseRepository` as
real filter requirements emerge, rather than fetching everything via `list()`
and filtering client-side.

There is no routing (`leptos_router`) yet — the app is a single page. Add it
when there's a second page to justify it (YAGNI).

## Conventions

- Keep `crates/domain` free of async runtimes and HTTP clients — it must
  compile to wasm32.
- New server-only crates/deps go through the same `optional = true` +
  feature-gate treatment as `llm`.
- Seed/demo data must stay obviously fictional (see naming in `seed.rs`) —
  this product deals with real enforcement actions against real companies;
  never fabricate demo data that could look like a factual claim about a real
  company.
