# Contributing to Flowrail

Thank you for your interest in contributing to **Flowrail (Project Ariadne)**!

Flowrail is an open-source, deterministic state-machine and Model Context Protocol (MCP) server that empowers AI coding agents to navigate, understand, and refactor complex codebases without context pollution or breaking contract changes.

We welcome contributions from compiler engineers, graph theorists, systems developers, AI harness builders, and anyone testing Flowrail in production environments.

---

## Areas Where We Need Your Help

### 1. Multi-Language Semantic Bridges (Python, Rust, Go, Java, C#)
Flowrail's core architecture (`flowrail-core`) is completely stack-agnostic. While our initial bridge (`flowrail-ts-bridge`) targets TypeScript via SCIP and `oxc`, we are actively expanding Flowrail to support other ecosystems:

- **Python Bridge (`flowrail-python-bridge`):**
  - Integrate `scip-python` or LSP (`pyright` / `basedpyright` / `jedi`) for cross-file symbol resolution.
  - Fast local AST slicing and type-hint contract hashing via `ruff` / Rust-based Python AST parsers.
  - Slicing through common Python frameworks (FastAPI, Django, Flask, Celery).

- **Rust Bridge (`flowrail-rust-bridge`):**
  - Extract call graphs and trait definitions via `rust-analyzer` / SCIP.
  - Fast AST inspection using `syn` / `proc_macro2` for struct, enum, and trait signature hashing.
  - Slicing through async runtimes (`tokio`, `actix-web`, `axum`).

- **Go Bridge (`flowrail-go-bridge`):**
  - Extract definitions and call graphs via `scip-go` or `golang.org/x/tools/gopls`.
  - Interface implementation matching and struct field contract hashing.
  - Slicing through Gin, Echo, Fiber, and gRPC microservices.

- **Other Languages:**
  - Java / Kotlin (via `scip-java` or tree-sitter), C# / .NET, and Ruby.

> **How to create a language bridge:**
> Create a new crate under `crates/flowrail-<lang>-bridge/`. Implement graph population into `flowrail_core::graph::CodebaseGraph` and contract signature hashing conforming to `flowrail_core::contract::ContractHasher`.

---

### 2. Testing, Real-World Benchmarks & Bug Reports
We need developers to test Flowrail across diverse, real-world repositories:

- **Try Flowrail on your project:**
  Run `flowrail doctor` and `flowrail init --target all` in your repository and test corridor navigation (`flowrail_open` with `READ_FLOW` and `MUTATE_CONTRACT`) with your AI coding agent (Claude Code, Cursor, Codex, Windsurf, OpenCode, Antigravity).
- **Report bugs and edge cases:**
  If Flowrail prunes a symbol it shouldn't have, fails to resolve a path alias, miscalculates a blast radius, or encounters a circular dependency panic, please [open an issue](https://github.com/imMamdouhaboammar/Flowrail/issues) with:
  1. A minimal code fixture reproducing the issue.
  2. The entrypoint symbol and target mutation symbol.
  3. The exact CLI command or MCP tool invocation.
  4. The observed vs. expected gate sequence or blast radius report.

---

### 3. Core Engine Performance & Algorithm Optimization
We welcome improvements to `flowrail-core` and `flowrail-mcp`:

- **Graph Algorithms:** Sub-millisecond dominator tree computation, incremental PageRank updates, and multi-threaded Tarjan SCC cycle resolution.
- **Frecency & Search:** Enhancing typo-tolerant symbol search and hybrid BM25/Frecency ranking.
- **MCP Protocol & Transports:** SSE (Server-Sent Events) transport, HTTP streaming, and support for emerging MCP protocol specifications.
- **Agent Harnesses:** Improving rules, prompt templates, and slash command integrations for new agent harnesses.

---

## Development Workflow

### Prerequisites
- **Rust Toolchain:** Version `1.99.1` or later (Edition 2024).
- **Bun:** `>= 1.0.0` (for testing fixture packages and scripts).

### Building the Project
```bash
# Build the workspace in debug mode
cargo build

# Build release binary
cargo build --release
```

### Running Test Suites
All tests must pass cleanly before opening a pull request:

```bash
# Run all workspace unit and integration tests
cargo test --workspace

# Run Clippy linter with zero warnings
cargo clippy --workspace --all-targets --locked -- -D warnings

# Check code formatting
cargo fmt --all -- --check
```

### Testing Fixtures
To verify TypeScript SCIP extraction and benchmark slicing against the included e-commerce fixture:

```bash
cargo test --test e2e_fixture_test
cargo test --test cli_test
```

---

## Submitting Pull Requests

1. **Fork and branch:** Create a feature branch with a descriptive name (e.g., `feat/python-bridge-scip`, `fix/pagerank-damping-factor`).
2. **Atomic commits:** Make discrete, atomic commits with clear commit messages following Conventional Commits (`feat:`, `fix:`, `perf:`, `docs:`, `test:`).
3. **Keep PRs focused:** Multiple small, reviewable PRs are preferred over massive monolithic diffs.
4. **Zero warnings:** Ensure `cargo clippy`, `cargo test`, and `cargo fmt` pass with zero warnings.
5. **No AI slop or boilerplate narration:** Write clear code and precise technical explanations. Focus comments on *why* non-obvious architecture decisions were made.
