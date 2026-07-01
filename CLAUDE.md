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
tokio, sqlx, `llm`, `db`) must never leak into a crate that gets built for
wasm32-unknown-unknown, or the wasm build breaks. Concretely:

- `crates/domain` has zero ssr-only dependencies — it's imported by all of
  `app`, `frontend`, and `server`, native and wasm alike.
- `web/app`'s `llm` and `db` dependencies are `optional = true`, gated behind
  the `ssr` feature. `server_fns.rs` calls them via **fully-qualified paths**
  (`llm::provider_from_env()`, `db::CaseRepository`) instead of a top-level
  `use llm::...;` / `use db::...;`, because the `#[server]` macro only
  compiles the function *body* under the `ssr` feature — a top-level `use`
  statement is a plain module item and would break the wasm build if the
  crate weren't available there.
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
```

`cargo-leptos` and the `wasm32-unknown-unknown` target must be installed:
`cargo install cargo-leptos`, `rustup target add wasm32-unknown-unknown`.
The `wasm-bindgen-cli` version installed **must exactly match** the
`wasm-bindgen` crate version resolved in `Cargo.lock`, or `cargo leptos build`
fails with a schema-version mismatch — `cargo install wasm-bindgen-cli
--version <X>` to fix.

## Current known gaps (documented, not silently missing)

There is no crawler or NLP extraction pipeline yet — the database only ever
contains the fictional demo seed data (or whatever's manually inserted via
`CaseRepository::upsert`). Building the real ingestion pipeline from
`spec.md` is the natural next step; feed it through the same
`CaseRepository` trait rather than a new storage path.

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
