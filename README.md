# incomplianceRadar

AI-powered platform for tracking global compliance monitorships, Deferred
Prosecution Agreements (DPAs) and Non-Prosecution Agreements (NPAs). See
[`spec.md`](spec.md) for the full product concept.

Full-stack Rust: [Leptos](https://leptos.dev) (SSR + WASM) on
[Axum](https://github.com/tokio-rs/axum). LLM access is pluggable — run
entirely against a local model via [Ollama](https://ollama.com), or point it
at a frontier model (Anthropic).

Project docs: **[asmuelle.github.io/incompliance-radar](https://asmuelle.github.io/incompliance-radar/)**

## Status

Early-stage scaffold: a working full-stack app with SQLite-backed persistence
(seeded with fictional demo data on first run) and a live LLM query panel.
The crawler, NLP extraction pipeline, search/alerting described in `spec.md`
are not built yet. See [`CLAUDE.md`](CLAUDE.md) for the current architecture
and what's next.

## Getting started

Prerequisites:

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos
cargo install wasm-bindgen-cli --version <version matching Cargo.lock's wasm-bindgen>
```

Run with a local model (default, requires [Ollama](https://ollama.com) running):

```bash
ollama pull llama3.1   # or any model you have; set OLLAMA_MODEL to match
cp .env.example .env
cargo leptos watch
```

Open http://127.0.0.1:3000.

To use a frontier model instead, set in `.env`:

```bash
LLM_BACKEND=anthropic
ANTHROPIC_API_KEY=sk-...
```

## Workspace layout

```
crates/domain/   Core compliance domain types (wasm-safe)
crates/llm/      LLM provider abstraction (Ollama + Anthropic)
crates/db/       Persistence (CaseRepository trait + SQLite)
web/app/         Shared Leptos UI + server functions
web/frontend/    Wasm hydration entry point
web/server/      Axum server binary
docs/            GitHub Pages source (mdBook)
```

## Development

```bash
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --exclude frontend -- -D warnings
```

## License

[MIT](LICENSE)
