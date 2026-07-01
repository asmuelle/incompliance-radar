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
web/
  app/        Shared Leptos UI + server functions + in-memory seed data.
  frontend/   Wasm hydration entry point.
  server/     Axum server binary.
  style/      Plain CSS.
```

## Why split `app` / `frontend` / `server`

`cargo-leptos` needs one crate compiled for `wasm32-unknown-unknown` (the
client) and one compiled natively (the server), sharing UI code. Server-only
dependencies (axum, tokio, a future database layer, the `llm` crate) must
never leak into a crate built for wasm32, or the client build breaks.

`crates/domain` has no async runtime or HTTP client dependency, so it
compiles for both targets and is shared everywhere. `web/app`'s `llm`
dependency is optional and gated behind the `ssr` Cargo feature; the
`#[server]` functions that use it reference it via fully-qualified paths
rather than a top-level `use`, since the macro only compiles the function
body under `ssr` — see `CLAUDE.md` in the repository root for the full
rationale and the pattern to follow when adding new server-only
dependencies.

## Data flow (current state)

```
Browser (WASM) --hydrate--> App component
                                |
                        #[server] functions
                                |
                    Axum server (native binary)
                          /            \
              seed.rs (in-memory)   llm::provider_from_env()
                                            |
                                  Ollama (local) or Anthropic (frontier)
```

There is no persistence layer yet. See [Roadmap](roadmap.md).
