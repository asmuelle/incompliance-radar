# Getting Started

## Prerequisites

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos
```

`cargo-leptos` shells out to `wasm-bindgen`, and the installed
`wasm-bindgen-cli` version **must exactly match** the `wasm-bindgen` crate
version resolved in `Cargo.lock`:

```bash
cargo install wasm-bindgen-cli --version <version>
```

If `cargo leptos build` fails with a schema-version mismatch, that's the fix
— reinstall the CLI at the version it names in the error.

## Run locally (local model via Ollama)

```bash
ollama pull llama3.1   # or use any model you already have pulled
cp .env.example .env   # defaults to LLM_BACKEND=ollama
cargo leptos watch
```

Open <http://127.0.0.1:3000>.

## Run locally (frontier model via Anthropic)

```bash
cp .env.example .env
```

Then in `.env`:

```bash
LLM_BACKEND=anthropic
ANTHROPIC_API_KEY=sk-...
```

## Common commands

```bash
cargo leptos watch                     # dev server with hot reload
cargo leptos build --release           # production build
cargo test --workspace --exclude frontend
cargo fmt --all
cargo clippy --workspace --exclude frontend -- -D warnings
```
