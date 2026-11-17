# Flowrail (Project Ariadne) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and verify Flowrail (Project Ariadne), a deterministic execution and mutation state-machine MCP server and CLI for AI coding agents that slices TypeScript codebases into noise-pruned Business Flow Corridors and topological Mutation Plans.

**Architecture:** A high-performance 4-crate Rust workspace comprising `flowrail-core` (graph models, petgraph DAG algorithms, fan-in pruning, and state-machine transitions), `flowrail-ts-bridge` (oxc AST range extraction, BLAKE3 contract signature hashing, and SCIP protobuf ingestion), `flowrail-mcp` (stdio Model Context Protocol JSON-RPC 2.0 server providing the 6 deterministic gatekeeping tools), and `flowrail-cli` (CLI binary).

**Tech Stack:** Rust 2024 Edition, `petgraph`, `oxc_parser` 0.38, `oxc_ast`, `oxc_span`, `scip` 0.4, `protobuf` 3.2.0, `blake3`, `tokio`, `serde_json`, `clap` 4.5.

**Spec:** `Specs.md`, `Pruning_Slicing_Algorithm.json`, `corridor-ts-bridge.json`.

## Global Constraints

- Crate names: `flowrail-core`, `flowrail-ts-bridge`, `flowrail-mcp`, `flowrail-cli`.
- CLI binary name: `flowrail`.
- Host language: Rust 2024 Edition, with 100% deterministic, zero-token, zero-LLM core execution.
- MCP Server supports both canonical `corridor_*` and `flowrail_*` tool aliases for backwards compatibility.
- Never prune nodes classified as `ContractDefinition` or matching `target_mutation_symbol`.
- Enforce strict fail-closed state transitions: Gates cannot advance if contract is mutated without acknowledging external blast radius.

## Review Focus

1. **Cycle Detection in Dependency Graph:** Cyclic imports in TypeScript must not cause infinite loops in forward/backward slicing (Tarjan SCC / cycle breaking).
2. **Contract Hash Sensitivity:** Modifying private implementation details or whitespace must not trigger a contract mutation; altering exported parameter types, return types, or interface fields must trigger it.
3. **Contagion Barrier Enforcement:** Advancing a gate with broken external dependents without `acknowledge_blast_radius: true` must fail with an informative contagion error.
4. **Hermetic Rollback:** Restoring to a previous gate must accurately rewrite disk files to pre-edit state without leaving orphaned mutations.
5. **OXC AST API Stability:** Compatible traversal of `oxc_parser` 0.38 AST declarations and spans without panics on unexpected syntax.

---

### Task 1: `flowrail-core` Graph Models and Indices

**Files:**
- Create: `crates/flowrail-core/src/graph.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Test: `crates/flowrail-core/tests/graph_test.rs`

- [x] Step 1: Write failing unit test in `crates/flowrail-core/tests/graph_test.rs` testing `CodebaseGraph::new()`, node addition, edge addition, role queries, and symbol resolution.
- [x] Step 2: Run `cargo test -p flowrail-core --test graph_test` to verify failure.
- [x] Step 3: Implement `crates/flowrail-core/src/graph.rs` with `NodeRole`, `EdgeKind`, `SemanticNode`, `SemanticEdge`, and `CodebaseGraph` backed by `petgraph::DiGraph` and `IndexMap`.
- [x] Step 4: Export module in `crates/flowrail-core/src/lib.rs`.
- [x] Step 5: Run `cargo test -p flowrail-core --test graph_test` and confirm pass.

---

### Task 2: `flowrail-core` Contract Hashing and Signature Diffing

**Files:**
- Create: `crates/flowrail-core/src/contract.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Test: `crates/flowrail-core/tests/contract_test.rs`

- [x] Step 1: Write failing test in `crates/flowrail-core/tests/contract_test.rs` verifying BLAKE3 signature normalization, whitespace invariance, and `diff_surfaces`.
- [x] Step 2: Run `cargo test -p flowrail-core --test contract_test` to verify failure.
- [x] Step 3: Implement `ExportedSymbolContract`, `ContractKind`, `ContractDiff`, and `ContractHasher` in `crates/flowrail-core/src/contract.rs`.
- [x] Step 4: Run `cargo test -p flowrail-core --test contract_test` and confirm pass.

---

### Task 3: `flowrail-core` Deterministic Noise Pruning Engine

**Files:**
- Create: `crates/flowrail-core/src/pruning.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Test: `crates/flowrail-core/tests/pruning_test.rs`

- [x] Step 1: Write failing test in `crates/flowrail-core/tests/pruning_test.rs` verifying utility noise score formula, hard-pruned regex patterns, and contract exception rule.
- [x] Step 2: Run `cargo test -p flowrail-core --test pruning_test` to verify failure.
- [x] Step 3: Implement `PruningConfig`, `PruneEvaluator`, and `PrunedNodeRecord` in `crates/flowrail-core/src/pruning.rs`.
- [x] Step 4: Run `cargo test -p flowrail-core --test pruning_test` and confirm pass.

---

### Task 4: `flowrail-core` Slicing Engine and Blast Radius Calculator

**Files:**
- Create: `crates/flowrail-core/src/slicer.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Test: `crates/flowrail-core/tests/slicer_test.rs`

- [x] Step 1: Write failing test in `crates/flowrail-core/tests/slicer_test.rs` verifying Forward Read Corridor slicing, Backward Mutation DAG slicing, topological sorting, and external dependent contagion indexing.
- [x] Step 2: Run `cargo test -p flowrail-core --test slicer_test` to verify failure.
- [x] Step 3: Implement `SliceIntent`, `GateStatus`, `BlastRiskLevel`, `CorridorGate`, `CorridorPlan`, and `CorridorSlicer` in `crates/flowrail-core/src/slicer.rs`.
- [x] Step 4: Run `cargo test -p flowrail-core --test slicer_test` and confirm pass.

---

### Task 5: `flowrail-core` State-Machine Session and Checkpointing

**Files:**
- Create: `crates/flowrail-core/src/state.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Test: `crates/flowrail-core/tests/state_test.rs`

- [x] Step 1: Write failing test in `crates/flowrail-core/tests/state_test.rs` testing `CorridorSession`: unlocking Gate 0, proposing edits, enforcing Contagion Barrier on unacknowledged breaking changes, dynamic gate injection on acknowledgement, and rollback.
- [x] Step 2: Run `cargo test -p flowrail-core --test state_test` to verify failure.
- [x] Step 3: Implement `SessionPhase`, `GateCheckpoint`, and `CorridorSession` in `crates/flowrail-core/src/state.rs`.
- [x] Step 4: Run `cargo test -p flowrail-core --test state_test` and confirm pass.

---

### Task 6: `flowrail-ts-bridge` AST Slicing and Contract Extraction via OXC

**Files:**
- Create: `crates/flowrail-ts-bridge/src/ast_slicer.rs`
- Modify: `crates/flowrail-ts-bridge/src/lib.rs`
- Test: `crates/flowrail-ts-bridge/tests/ast_slicer_test.rs`

- [x] Step 1: Write failing test in `crates/flowrail-ts-bridge/tests/ast_slicer_test.rs` with sample TypeScript source containing interfaces, functions, classes, and types.
- [x] Step 2: Run `cargo test -p flowrail-ts-bridge --test ast_slicer_test` to verify failure.
- [x] Step 3: Implement `TsAstSlicer` in `crates/flowrail-ts-bridge/src/ast_slicer.rs` using `oxc_parser` 0.38, correctly extracting symbol ranges, contracts, and state mutations.
- [x] Step 4: Run `cargo test -p flowrail-ts-bridge --test ast_slicer_test` and confirm pass.

---

### Task 7: `flowrail-ts-bridge` SCIP Protobuf Ingestion and Synthetic Fallback

**Files:**
- Create: `crates/flowrail-ts-bridge/src/scip_loader.rs`
- Modify: `crates/flowrail-ts-bridge/src/lib.rs`
- Test: `crates/flowrail-ts-bridge/tests/scip_loader_test.rs`

- [x] Step 1: Write failing test in `crates/flowrail-ts-bridge/tests/scip_loader_test.rs` loading a sample SCIP index or parsing TypeScript files into a `CodebaseGraph`.
- [x] Step 2: Run `cargo test -p flowrail-ts-bridge --test scip_loader_test` to verify failure.
- [x] Step 3: Implement `ScipGraphBuilder` in `crates/flowrail-ts-bridge/src/scip_loader.rs` supporting SCIP protobuf decoding and AST-based graph construction.
- [x] Step 4: Run `cargo test -p flowrail-ts-bridge --test scip_loader_test` and confirm pass.

---

### Task 8: `flowrail-ts-bridge` Incremental Verification Gate

**Files:**
- Create: `crates/flowrail-ts-bridge/src/verifier.rs`
- Modify: `crates/flowrail-ts-bridge/src/lib.rs`
- Test: `crates/flowrail-ts-bridge/tests/verifier_test.rs`

- [x] Step 1: Write failing test in `crates/flowrail-ts-bridge/tests/verifier_test.rs` testing valid vs syntactically invalid TypeScript edits.
- [x] Step 2: Run `cargo test -p flowrail-ts-bridge --test verifier_test` to verify failure.
- [x] Step 3: Implement `TsGateVerifier` in `crates/flowrail-ts-bridge/src/verifier.rs`.
- [x] Step 4: Run `cargo test -p flowrail-ts-bridge --test verifier_test` and confirm pass.

---

### Task 9: `flowrail-mcp` Stateful Server and 6 Deterministic Tools

**Files:**
- Create: `crates/flowrail-mcp/src/tools.rs`
- Create: `crates/flowrail-mcp/src/server.rs`
- Modify: `crates/flowrail-mcp/src/lib.rs`
- Test: `crates/flowrail-mcp/tests/mcp_test.rs`

- [x] Step 1: Write failing test in `crates/flowrail-mcp/tests/mcp_test.rs` simulating JSON-RPC stdio protocol calls: `initialize`, `tools/list`, and calls to each of the 6 tools.
- [x] Step 2: Run `cargo test -p flowrail-mcp --test mcp_test` to verify failure.
- [x] Step 3: Implement `CorridorMcpEngine` (with tool aliases for `corridor_*` and `flowrail_*`) in `crates/flowrail-mcp/src/tools.rs` and `McpStdioServer` in `crates/flowrail-mcp/src/server.rs`.
- [x] Step 4: Run `cargo test -p flowrail-mcp --test mcp_test` and confirm pass.

---

### Task 10: `flowrail-cli` Binary Commands

**Files:**
- Modify: `crates/flowrail-cli/src/main.rs`
- Test: `crates/flowrail-cli/tests/cli_test.rs`

- [x] Step 1: Write failing test in `crates/flowrail-cli/tests/cli_test.rs` testing CLI argument parsing and commands (`slice`, `mcp`, `doctor`).
- [x] Step 2: Run `cargo test -p flowrail-cli --test cli_test` to verify failure.
- [x] Step 3: Implement CLI commands in `crates/flowrail-cli/src/main.rs` with clap v4.
- [x] Step 4: Run `cargo test -p flowrail-cli --test cli_test` and confirm pass.

---

### Task 11: End-to-End Verification on Fixture E-Commerce Repo

**Files:**
- Create: `fixtures/sample-ts-ecommerce` (multi-module TS project with checkout, tax, invoicing, and subscription renewal)
- Test: `tests/e2e_flowrail_test.rs`

- [x] Step 1: Create realistic multi-file TypeScript fixtures in `fixtures/sample-ts-ecommerce/`.
- [x] Step 2: Write end-to-end integration test validating noise pruning, read corridor slicing, mutation DAG slicing, contagion barrier triggering, and complete gate progression.
- [x] Step 3: Run full workspace test suite `cargo test` and verify 100% green.
