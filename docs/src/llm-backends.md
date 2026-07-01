# LLM Backends

incomplianceRadar's NLP extraction and Q&A features run against a single
`LlmProvider` trait (`crates/llm/src/lib.rs`), selected at runtime via the
`LLM_BACKEND` environment variable — no code changes needed to switch.

| Backend | `LLM_BACKEND` | Requires | Notes |
|---|---|---|---|
| Ollama (local) | `ollama` (default) | `ollama serve` running locally | No API key, no data leaves the machine. Set `OLLAMA_MODEL` to any model you've pulled. |
| Anthropic (frontier) | `anthropic` | `ANTHROPIC_API_KEY` | Uses the Messages API. Set `ANTHROPIC_MODEL` to override the default. |

See `.env.example` in the repository root for all variables.

## Adding a new backend

1. Implement `LlmProvider` in `crates/llm/src/providers/<name>.rs`.
2. Add a variant to `LlmBackend` in `crates/llm/src/config.rs` and wire it
   into `LlmConfig::from_env`.
3. `provider_from_env()` in `crates/llm/src/lib.rs` picks it up automatically.

This is the same Repository/Strategy pattern used elsewhere in the codebase —
callers depend on the trait, never on a concrete provider.
