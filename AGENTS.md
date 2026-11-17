# Flowrail (Project Ariadne)

Deterministic Execution & Mutation State-Machine for AI Coding Agents.
Flowrail replaces naive flat codebase searches with an incremental Model Context Protocol (MCP) server that slices codebases into isolated **Business Flow Corridors** (for reading) and **Topological Mutation Plans** (for writing), enforcing contract verification and blast-radius fencing at every boundary gate.

## Workspace Layout

- `crates/flowrail-core`: Graph models, DAG algorithms (`petgraph`), fan-in pruning heuristics, signature hashing (`blake3`), and blast radius calculation.
- `crates/flowrail-ts-bridge`: TypeScript semantic extraction via SCIP Protobuf (`scip`), fast AST range extraction and export signature hashing (`oxc`).
- `crates/flowrail-mcp`: Stateful Model Context Protocol (MCP) server over `tokio` implementing the 6 gatekeeping tools.
- `crates/flowrail-cli`: Command-line interface (`flowrail`).
- `docs/solutions/`: Codified engineering learnings and architectural resolutions.

## Core Commands

```bash
cargo check                     # Fast type-check and dependency verification across workspace
cargo test                      # Run all workspace unit and integration test suites
cargo run --bin flowrail -- ...  # Run Flowrail CLI binary
cargo clippy                    # Run Rust linter
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
