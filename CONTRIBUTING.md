# Contributing to Flowrail

For analysis bugs, include a minimal TypeScript fixture, the entry and target IDs, the command, and the expected relationship or ordering. Say whether you used a SCIP index or the AST fallback. Use synthetic inputs that can be shared publicly.

Run these checks from the repository root using Rust 1.99.1:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

For changes to indexing, symbol resolution, or impact reporting, also run the [ecommerce fixture checks](docs/Flowrail-Full-Specs/fixtures/ecommerce-trap/README.md). If a change claims improved SCIP behavior, include an assertion over the real SCIP path. For larger changes, use the [specifications](docs/Flowrail-Full-Specs/README.md) to identify the behavior and acceptance criteria.
