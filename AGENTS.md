# Flowrail (Project Ariadne)

Deterministic Execution & Mutation State-Machine for AI Coding Agents.
Flowrail replaces naive flat codebase searches with an incremental Model Context Protocol (MCP) server that slices codebases into isolated **Business Flow Corridors** (for reading) and **Topological Mutation Plans** (for writing), enforcing contract verification and blast-radius fencing at every boundary gate.

## Workspace Layout

- `crates/flowrail-core`: Graph models, DAG algorithms (`petgraph`), fan-in pruning heuristics, signature hashing (`blake3`), and blast radius calculation.
- `crates/flowrail-ts-bridge`: TypeScript semantic extraction via SCIP Protobuf (`scip`), fast AST range extraction and export signature hashing (`oxc`).
- `crates/flowrail-mcp`: Stateful Model Context Protocol (MCP) server over `tokio` implementing the 10 MCP tools.
- `crates/flowrail-cli`: Command-line interface (`flowrail`).
- `docs/solutions/`: Codified engineering learnings and architectural resolutions.

## Core Commands

```bash
cargo check                     # Fast type-check and dependency verification across workspace
cargo test                      # Run all workspace unit and integration test suites
cargo run --bin flowrail -- ...  # Run Flowrail CLI binary
cargo clippy                    # Run Rust linter
```

## Stack Architecture & Context7 Documentation Directory

| Category | Component / Library | Resolved / Declared Version | Context7 Library ID | Official Documentation & API Reference |
| :--- | :--- | :--- | :--- | :--- |
| **Protocol** | Model Context Protocol (MCP) | `2026-07-28` / `2025-11-25` | `/modelcontextprotocol/modelcontextprotocol` | [MCP Specification](https://modelcontextprotocol.io) · [GitHub Protocol Schema](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/2026-07-28/schema.ts) |
| **Runtime & Toolchain** | Rust / Cargo | `1.99.1` (Edition `2024`) | `/websites/rust_lang` | [The Rust Reference](https://doc.rust-lang.org/reference/) · [Standard Library Docs](https://doc.rust-lang.org/std/) |
| **Async Runtime** | `tokio` | `1.54.1` (`^1.40`, `features = ["full"]`) | `/tokio-rs/tokio` or `/websites/rs_tokio` | [Tokio Docs.rs](https://docs.rs/tokio/1.54.1/tokio/) · [Tokio Tutorial](https://tokio.rs) |
| **AST & Semantics** | `oxc` (`parser`, `ast`, `semantic`, `allocator`, `span`) | `0.38.0` | `/oxc-project/oxc` or `/websites/oxc_rs` | [Oxc Architecture & Docs](https://oxc.rs) · [GitHub Repository](https://github.com/oxc-project/oxc) |
| **Graph & DAG** | `petgraph` | `0.8.3` (`features = ["serde-1"]`) | `/websites/docs_rs_petgraph` | [Petgraph Docs.rs](https://docs.rs/petgraph/0.8.3/petgraph/) · [GitHub](https://github.com/petgraph/petgraph) |
| **Index Map** | `indexmap` | `2.14.2` (`features = ["serde"]`) | `/websites/docs_rs_indexmap` | [IndexMap Docs.rs](https://docs.rs/indexmap/2.14.2/indexmap/) |
| **Code Intelligence** | `scip` & `protobuf` | `scip 0.10.0` / `protobuf 3.7.2` | `/sourcegraph/scip` | [SCIP Specification](https://github.com/sourcegraph/scip) · [scip-typescript](https://github.com/sourcegraph/scip-typescript) |
| **Fast Hashing** | `blake3` | `1.8.7` | `/websites/docs_rs_blake3` | [BLAKE3 Docs.rs](https://docs.rs/blake3/1.8.7/blake3/) · [GitHub](https://github.com/BLAKE3-team/BLAKE3) |
| **Persistence** | `rusqlite` | `0.43.0` (`features = ["bundled"]`) | `/websites/docs_rs_rusqlite` | [Rusqlite Docs.rs](https://docs.rs/rusqlite/0.43.0/rusqlite/) |
| **Version Control** | `git2` | `0.20.4` | `/websites/docs_rs_git2` | [Git2 Docs.rs](https://docs.rs/git2/0.20.4/git2/) · [libgit2](https://libgit2.org/) |
| **CLI & Flags** | `clap` | `4.6.7` (`features = ["derive"]`) | `/websites/docs_rs_clap` | [Clap Docs.rs](https://docs.rs/clap/4.6.7/clap/) · [Clap Guide](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html) |
| **Serialization** | `serde` & `serde_json` | `1.0.229` | `/websites/docs_rs_serde` | [Serde.rs](https://serde.rs) · [Serde JSON Docs.rs](https://docs.rs/serde_json/1.0.229/serde_json/) |
| **Diagnostics & Tracing** | `tracing` & `tracing-subscriber` | `0.1.44` / `0.3.23` | `/websites/docs_rs_tracing` | [Tracing Docs.rs](https://docs.rs/tracing/0.1.44/tracing/) · [Tracing Subscriber](https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/) |
| **Error Handling** | `thiserror` & `anyhow` | `thiserror 2.0.21` / `anyhow 1.0.104` | `/websites/docs_rs_thiserror` | [Thiserror Docs.rs](https://docs.rs/thiserror/2.0.21/thiserror/) · [Anyhow Docs.rs](https://docs.rs/anyhow/1.0.104/anyhow/) |
| **JS/TS Engine** | `bun` | `>=1.0.0` | `/oven-sh/bun` | [Bun Documentation](https://bun.sh/docs) |

### Context7 Quick-Fetch Queries

To pull up-to-date documentation on demand, invoke `query-docs` with the relevant `libraryId`:

```markdown
# Tokio runtime, concurrency primitives & channels
query-docs libraryId="/tokio-rs/tokio" query="tokio sync mpsc task spawn runtime" language="Rust"

# Oxc parser, AST allocator, and semantic analysis
query-docs libraryId="/oxc-project/oxc" query="oxc parser ast allocator semantic visitor" language="Rust"

# Model Context Protocol wire schemas & tool calling
query-docs libraryId="/modelcontextprotocol/modelcontextprotocol" query="tools list call request result schema json-rpc" language="TypeScript"
```

<!-- compound-engineering:start -->
## Compound Engineering Loop

All coding and refactoring sessions must adhere to the Compound Engineering 80/20 loop:
- **80% Planning & Review:** Frame requirements and evaluate architecture via `ce-brainstorm`, create execution plans via `ce-plan`, conduct adversarial multi-angle review via `ce-code-review`, and consolidate durable learnings via `ce-compound`.
- **20% Execution:** Minimal, focused, high-precision implementation via `ce-work`, AST simplification via `ce-simplify-code`, or rapid execution via `lfg`.
- **Systematic Debugging:** Trace failing behaviors and root causes via `ce-debug`.
- **Durable Knowledge:** Codify non-obvious engineering solutions and post-mortem resolutions into `docs/solutions/`.
<!-- compound-engineering:end -->

<!-- Waza:start -->
## Waza Engineering Habits & Discipline

Follow Waza engineering habits and discipline:
- `think`: Turn rough ideas into approved, decision-complete plans before coding.
- `hunt`: Identify and verify root cause before touching implementation.
- `check`: Review diffs, PRs, release readiness, and publishing follow-through.
- `ui`: Produce distinctive production UI and screenshot-grounded visual polish.
- `write`: Rewrite and polish technical documentation and copy.
- `learn`: Systematic research workflow from source bundle to publish-ready output.
- `read`: URL/doc summarization and analysis.
- `health`: Audit agent config, instruction drift, and maintainability.
- Strictly adhere to `anti-patterns.md`, `clarity.md`, and `durable-context.md`.
<!-- Waza:end -->

<!-- anti-slop-engineering:start -->
## anti-slop-engineering
Follow Anti-Slop Engineering standards, Google SRE Discipline (KISS, DRY, YAGNI, Fix Issues at Scale), and clean-room gates:
- `anti-slop-engineering`: `.agents/skills/anti-slop-engineering/SKILL.md`
- `api-ergonomics-and-contract-governor`: `.agents/skills/api-ergonomics-and-contract-governor/SKILL.md`
- `apple-craft-and-frame-budget`: `.agents/skills/apple-craft-and-frame-budget/SKILL.md`
- `apple-craft-and-interaction-physics`: `.agents/skills/apple-craft-and-interaction-physics/SKILL.md`
- `asset-provenance-guard`: `.agents/skills/asset-provenance-guard/SKILL.md`
- `backprop-guard`: `.agents/skills/backprop-guard/SKILL.md`
- `chromatic-drift-guard`: `.agents/skills/chromatic-drift-guard/SKILL.md`
- `cognitive-load-and-attention-governor`: `.agents/skills/cognitive-load-and-attention-governor/SKILL.md`
- `comment-budget-guard`: `.agents/skills/comment-budget-guard/SKILL.md`
- `decision-guard`: `.agents/skills/decision-guard/SKILL.md`
- `demock-guard`: `.agents/skills/demock-guard/SKILL.md`
- `design-system-drift-guard`: `.agents/skills/design-system-drift-guard/SKILL.md`
- `deterministic-simulation-and-property-testing`: `.agents/skills/deterministic-simulation-and-property-testing/SKILL.md`
- `future-scale-gate`: `.agents/skills/future-scale-gate/SKILL.md`
- `grumpy-senior-staff-engineer`: `.agents/skills/grumpy-senior-staff-engineer/SKILL.md`
- `honeypot-trap-guard`: `.agents/skills/honeypot-trap-guard/SKILL.md`
- `house-tell-guard`: `.agents/skills/house-tell-guard/SKILL.md`
- `install-anti-slop`: `.agents/skills/install-anti-slop/SKILL.md`
- `mandela-guard`: `.agents/skills/mandela-guard/SKILL.md`
- `mechanical-sympathy-and-perf-gate`: `.agents/skills/mechanical-sympathy-and-perf-gate/SKILL.md`
- `mechanical-sympathy-profiler`: `.agents/skills/mechanical-sympathy-profiler/SKILL.md`
- `motion-runtime-guard`: `.agents/skills/motion-runtime-guard/SKILL.md`
- `observability-and-slo-architect`: `.agents/skills/observability-and-slo-architect/SKILL.md`
- `observability-and-slo-gate`: `.agents/skills/observability-and-slo-gate/SKILL.md`
- `paranoid-principal-architect`: `.agents/skills/paranoid-principal-architect/SKILL.md`
- `pr-boundary-guard`: `.agents/skills/pr-boundary-guard/SKILL.md`
- `privacy-and-data-lineage-gate`: `.agents/skills/privacy-and-data-lineage-gate/SKILL.md`
- `re0-handoff-guard`: `.agents/skills/re0-handoff-guard/SKILL.md`
- `reduction-guard`: `.agents/skills/reduction-guard/SKILL.md`
- `repo-memory-and-invariant-steward`: `.agents/skills/repo-memory-and-invariant-steward/SKILL.md`
- `root-fix-gate`: `.agents/skills/root-fix-gate/SKILL.md`
- `safe-evolution-and-rollout-choreographer`: `.agents/skills/safe-evolution-and-rollout-choreographer/SKILL.md`
- `scientific-fault-isolation`: `.agents/skills/scientific-fault-isolation/SKILL.md`
- `styleseed-guard`: `.agents/skills/styleseed-guard/SKILL.md`
- `tagore-guard`: `.agents/skills/tagore-guard/SKILL.md`
- `type-level-and-ast-slop-gate`: `.agents/skills/type-level-and-ast-slop-gate/SKILL.md`
- `ui-and-copy-discipline`: `.agents/skills/ui-and-copy-discipline/SKILL.md`
- `verification-oracle-architect`: `.agents/skills/verification-oracle-architect/SKILL.md`
- `zero-downtime-migration-gate`: `.agents/skills/zero-downtime-migration-gate/SKILL.md`
Before concluding tasks, verify against automated guards: `bunx anti-slop-engineering check --base HEAD`.
Run audit dossier on demand: `bunx anti-slop-engineering audit`.
To update anti-slop-engineering across all agents: `bunx anti-slop-engineering update`.
<!-- anti-slop-engineering:end -->
