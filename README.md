# Flowrail

**Deterministic Execution & Mutation State-Machine for AI Coding Agents**

Flowrail is a local Rust engine and Model Context Protocol (MCP) server that gives AI coding agents a focused, topologically verified slice of a codebase before they read or edit files. Slicing and contract verification run completely offline without model API calls or token burn.

---

## The Problem It Solves

Ask an AI agent to update municipal tax calculations at checkout. The agent navigates to the checkout controller, tweaks the tax structure, and gets checkout compiling. Minutes later, CI fails across an invoice generator, a subscription renewal worker, and a VAT audit job: all three depended on the same calculation result. They were invisible while the agent was focused on checkout.

Standard agent tools use flat greps or raw AST dumps. These dump thousands of lines of irrelevant logger calls, telemetry spans, and utilities into the context window, causing agents to lose track of execution flow and break cross-file type contracts.

Flowrail builds two complementary Directed Acyclic Graphs (DAGs) from source code and semantic indexes:

- **Read Corridor:** Starts at an entry point (HTTP route, CLI command, event consumer) and traces execution down to data mutations or external calls. Automatically prunes logger, telemetry, and high-fan-in utility noise so the agent only sees domain logic.
- **Mutation Plan:** Starts at the modified type or contract, traverses backward through producers and consumers, and orders edits in reverse topological order. Producers are verified before consumers are opened for editing, and cross-feature blast radius is surfaced before the first file is touched.

---

## When to Use Flowrail

| Scenario | What Flowrail Does |
| :--- | :--- |
| Changing a shared interface, schema, or type with unknown consumers | Computes a topological mutation plan; reports every caller inside and outside the active feature before editing begins. |
| Large refactor where agents break intermediate compilation states | Locks future gates until the active gate compiles; reverts to atomic checkpoints if an approach fails. |
| Reading an unfamiliar feature without context pollution | Slices a top-down read corridor from the entrypoint, stripping away loggers, metrics, and boilerplate utilities. |
| Calculating blast radius before touching production code | Runs `flowrail_open` with `intent: MUTATE_CONTRACT`; returns a contagion index and list of external callers upfront. |
| Verifying complete refactoring coverage | Tracks gate transitions and ensures every impacted caller is addressed before completing the session. |

---

## Multi-Harness Support

Flowrail integrates out of the box with major AI coding assistants and IDEs:

| Agent Harness | Configuration | Supported Surfaces |
| :--- | :--- | :--- |
| **Claude Code & Desktop** | `.mcp.json`, `.claude/` | MCP server, slash commands (`/flowrail`, `/corridor`, `/mutate`, `/doctor`) |
| **OpenAI Codex** | `.codex/config.toml` | Native Codex MCP, `CODEX.md` rules |
| **OpenCode** | `.opencode/opencode.json` | Plugin manifest, MCP server, slash commands |
| **Cursor** | `.cursor/mcp.json` | Cursor rules (`.cursor/rules/flowrail.mdc`), MCP tools |
| **Windsurf** | `.windsurf/mcp_config.json` | MCP configuration, `.windsurfrules` |
| **Zed** | `.zed/settings.json` | Context server integration |
| **VS Code / Cline** | `.vscode/mcp.json` | `cline_mcp_settings.json` |
| **Antigravity** | `mcp_config.json`, `plugin.json` | Specialized subagents, autonomous lifecycle hooks |

Initialize all harnesses in one command:

```bash
flowrail init --target all
```

Or target a specific environment: `flowrail init --target claude`, `cursor`, `codex`, `opencode`, `windsurf`, `zed`, `vscode`, or `antigravity`.

---

## Installation

### Option A — One-line script

```bash
./install.sh
```

Builds the release binary and links it to `~/.cargo/bin/flowrail`. Requires Rust 1.75+ (or installs it via `rustup` if missing).

### Option B — Cargo directly

```bash
cargo install --path crates/flowrail-cli --force
```

### Option C — Remote install

```bash
curl -fsSL https://raw.githubusercontent.com/imMamdouhaboammar/Flowrail/main/install.sh | bash
```

Verify your installation:

```bash
flowrail doctor
```

---

## Quickstart

Run the built-in demonstration on the included e-commerce fixture:

```bash
flowrail demo
```

Run an in-memory symbol search with Git status badges and frecency ranking:

```bash
flowrail search "checkout"
```

Run in-memory definition grep:

```bash
flowrail grep "TaxResult"
```

Start the stdio Model Context Protocol (MCP) server:

```bash
flowrail mcp
```

---

## MCP Tools Reference

Flowrail implements 10 stateful MCP tools over JSON-RPC 2.0 stdio (accepts both `flowrail_*` and `corridor_*` prefixes):

| Tool | Purpose |
| :--- | :--- |
| `flowrail_discover_entrypoints` | Discovers HTTP routes, CLI handlers, and queue consumers with PageRank ranking. |
| `flowrail_find_symbols` | Definition-first symbol search with typo-tolerant subword matching. |
| `flowrail_grep` | In-memory definition grep; scope can be restricted to the active corridor. |
| `flowrail_open` | Opens a session with `READ_FLOW` or `MUTATE_CONTRACT` intent; unlocks Gate 0. |
| `flowrail_inspect_gate` | Returns the source slice, imported contracts, and blast radius for Gate $N$. |
| `flowrail_propose_edit` | Stages edits, verifies syntax with OXC, and computes contract signature deltas. |
| `flowrail_advance_gate` | Verifies the active gate, locks it, and unlocks Gate $N+1$. |
| `flowrail_abort_or_rollback` | Restores files to any prior gate checkpoint. |
| `flowrail_export_artifact` | Exports session state or architecture plans as structured JSON. |
| `flowrail_enrich_artifact` | Applies validated AI enrichment (actionable steps, visible wins, diagnostics) with zero hallucination. |

---

## Expanding Beyond TypeScript: Multi-Language Roadmap

Flowrail's core graph engine (`flowrail-core`) is completely language-agnostic. While our first bridge (`flowrail-ts-bridge`) targets TypeScript via SCIP and OXC, we are actively expanding Flowrail to support other programming ecosystems.

We are calling for community contributors to build and maintain language bridges:

- **Python (`flowrail-python-bridge`):** Cross-file symbol extraction via `scip-python` / LSP (`basedpyright` / `pyright`), fast AST slicing and type signature hashing via Rust-based Python parsers (Ruff/AST), slicing through FastAPI, Django, Flask, and Celery flows.
- **Rust (`flowrail-rust-bridge`):** Call graphs and trait definitions via `rust-analyzer` SCIP, fast AST slicing with `syn`, and slicing through `tokio`, `actix-web`, and `axum`.
- **Go (`flowrail-go-bridge`):** Slicing via `scip-go` or `gopls`, interface matching, and struct contract hashing for Gin, Echo, Fiber, and gRPC services.
- **Java / Kotlin / C#:** Enterprise graph slicing via SCIP language indexers.

If you are interested in building or testing a language bridge, check out our [Contributing Guide](CONTRIBUTING.md) to get started.

---

## Community, Testing & Bug Reports

We want developers to run Flowrail on real repositories and stress-test it against diverse code patterns:

- **Try it on your codebase:** Run `flowrail doctor` and `flowrail init --target all` on your projects. Test corridor navigation and refactoring loops with your AI agent.
- **Report bugs and edge cases:** Found a missed dependency, an inaccurate blast radius, a path alias resolution bug, or a circular dependency issue? Please [open an issue](https://github.com/imMamdouhaboammar/Flowrail/issues) with a minimal reproducing fixture.
- **System performance & optimization:** Help us optimize graph traversal times, dominator tree calculations, memory footprints, and MCP responsiveness.

---

## Repository Layout

| Path | Purpose |
| :--- | :--- |
| [`crates/flowrail-core`](crates/flowrail-core/) | Graph models, DAG algorithms (petgraph), pruning heuristics, contract hashing (blake3) |
| [`crates/flowrail-ts-bridge`](crates/flowrail-ts-bridge/) | TypeScript semantic extraction via SCIP and OXC AST slicing |
| [`crates/flowrail-mcp`](crates/flowrail-mcp/) | Stateful Model Context Protocol server over tokio |
| [`crates/flowrail-cli`](crates/flowrail-cli/) | CLI binary driver (`flowrail init`, `doctor`, `demo`, `search`, `grep`, `mcp`) |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Contribution guide, multi-language bridge specs, and development workflow |
| [`docs/solutions/`](docs/solutions/) | Codified engineering learnings and architectural decisions |
| [`fixtures/`](fixtures/) | Multi-module test fixtures for integration benchmarks |

---

## License

[MIT](LICENSE) © Mamdouh Aboammar
