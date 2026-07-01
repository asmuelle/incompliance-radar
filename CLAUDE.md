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
web/
  app/        Shared UI: the `App` component, `shell()` HTML document,
              `#[server]` functions (server_fns.rs), and in-memory seed data
              (seed.rs). Compiles for BOTH native (ssr feature) and wasm32
              (used by `frontend`). This is the crate you touch for almost
              all UI and server-function changes.
  frontend/   Thin wasm hydration entry point only (`hydrate()` +
              `wasm_bindgen`). Rarely needs edits.
  server/     Axum binary (`main.rs`). Wires up leptos_routes + static file
              serving. Rarely needs edits.
  style/      Plain CSS (main.css) — no Sass toolchain required.
```

### Why the app/frontend/server split

cargo-leptos needs one crate compiled for wasm32 (client) and one compiled
natively (server), from a *shared* UI crate. Server-only dependencies (axum,
tokio, sqlx, `llm`) must never leak into a crate that gets built for
wasm32-unknown-unknown, or the wasm build breaks. Concretely:

- `crates/domain` has zero ssr-only dependencies — it's imported by all of
  `app`, `frontend`, and `server`, native and wasm alike.
- `web/app`'s `llm` dependency is `optional = true`, gated behind the `ssr`
  feature. `server_fns.rs` calls it via **fully-qualified paths**
  (`llm::provider_from_env()`, `llm::LlmProvider::complete(...)`) instead of a
  top-level `use llm::...;`, because the `#[server]` macro only compiles the
  function *body* under the `ssr` feature — a top-level `use` statement is a
  plain module item and would break the wasm build if the crate weren't
  available there.
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

There is no persistence layer yet — `list_cases()` in `web/app/src/seed.rs`
returns hardcoded fictional demo data. Before building the real crawler/NLP
pipeline from `spec.md`, the natural next step is a `crates/db` (or `sqlx`
directly in `web/server`) repository layer behind a trait, following the
Repository Pattern already used for `LlmProvider`. Don't add a database
dependency to `web/app` directly — keep it server-only per the ssr-gating
rules above.

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
