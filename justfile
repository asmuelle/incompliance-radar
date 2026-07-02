# Run `just` with no arguments to list available recipes.
default:
    @just --list

# Must match the wasm-bindgen version resolved in Cargo.lock (see CLAUDE.md
# and .github/workflows/ci.yml) — bump both together.
wasm_bindgen_version := "0.2.126"

# One-time setup for a fresh checkout.
install-tools:
    rustup target add wasm32-unknown-unknown
    cargo install cargo-leptos --locked
    cargo install wasm-bindgen-cli --version {{ wasm_bindgen_version }} --locked

# Dev server with hot reload, http://127.0.0.1:3000
watch:
    cargo leptos watch

# Production build (native server + wasm client).
build:
    cargo leptos build

build-release:
    cargo leptos build --release

test:
    cargo test --workspace --exclude frontend

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

# frontend needs --target wasm32-unknown-unknown for clippy, so it's checked
# separately from the rest of the workspace.
clippy: clippy-native clippy-wasm

clippy-native:
    cargo clippy --workspace --exclude frontend --all-targets -- -D warnings

clippy-wasm:
    cargo clippy -p domain -p frontend --target wasm32-unknown-unknown -- -D warnings

check:
    cargo check -p app --features ssr
    cargo check -p frontend --target wasm32-unknown-unknown

# One crawl pass across configured regulator sources (SEC, FCA) — see
# CLAUDE.md's Crawler section for env vars and connector details.
crawl:
    cargo run -p crawler --bin crawl

# Download the Corporate Prosecution Registry bulk export and import the
# historical DPA/NPA/declination corpus into the local database. Idempotent —
# re-running refreshes imported cases instead of duplicating them. See
# CLAUDE.md's Importer section, including the data-licensing caveat, before
# shipping this data in a commercial deployment.
import-registry:
    mkdir -p data
    curl -sSL -o data/corp-crime.csv https://corporate-prosecution-registry.com/media/corp-crime.csv
    cargo run -p importer --bin import-registry -- data/corp-crime.csv

# Everything CI runs, in the same order (.github/workflows/ci.yml), so you
# can catch failures locally before pushing.
ci: fmt-check clippy-native clippy-wasm test build-release

clean:
    cargo clean
