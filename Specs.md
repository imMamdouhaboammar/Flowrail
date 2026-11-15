# CORRIDOR: Master Specification & Project Charter

**Document Version:** 1.0.0  
**Status:** Approved Architectural Blueprint  
**Category:** Developer Tooling / Stateful Model Context Protocol (MCP) Server / Semantic Graph Slicer  
**Host Language:** Rust (2024 Edition)  
**Target Language (V1):** TypeScript (Node.js, Next.js, NestJS, React)

---

## 1. Executive Summary & Problem Statement

### 1.1 The Macro Failure Mode of AI Coding Agents
Modern AI coding agents (e.g., Claude Code, Cursor, Windsurf) perform exceptionally well on localized, single-file code generation. However, they consistently degrade or fail when executing **multi-file structural refactoring** across production repositories containing hundreds or thousands of files.

### 1.2 Root Causes of Agent Degradation
The failure during large-scale refactoring stems from how codebase context and execution privileges are exposed to the LLM:

1. **Context Pollution via Infrastructure Noise:**
   When an agent attempts to trace a business feature (such as an order checkout pipeline), standard search tools (`ripgrep`, lexical RAG) and static AST graphs return every connected reference indiscriminately. Utility helpers, loggers, telemetry wrappers, string formatters, and configuration readers flood the context window, diluting the core domain logic.
2. **Topological Blindness (Read vs. Write Asymmetry):**
   Reading a feature follows execution flow from entrypoint to database (`Controller -> Service -> Repository -> Entity`). Refactoring a shared contract requires the exact opposite order (`Contract/Type -> Repository -> Service -> Controller`). Current tools dump flat call graphs without enforcing topological mutation order, causing agents to edit downstream consumers before stabilizing upstream contracts.
3. **Blindness to Cross-Feature Blast Radius (Contagion):**
   While focusing on Feature A, an agent frequently modifies a shared interface or service method without realizing that Features B, C, and D outside its current context window depend on the old signature. The agent fixes the requested feature while silently breaking three unrelated modules.
4. **Unconstrained Navigation Without State Enforcement:**
   Existing graph-based tools act as passive directories: they hand a complex map to a probabilistic model and hope the model follows it. Under long-context pressure, agents skip steps, leave intermediate files in non-compiling states, and jump erratically across directories.

---

## 2. Core Thesis & Solution Overview

**Corridor** replaces unconstrained repository exploration with a **Deterministic Execution and Mutation State-Machine** exposed over the **Model Context Protocol (MCP)**.

Instead of dumping an entire codebase graph into the LLM context window, Corridor enforces four strict architectural primitives without invoking any internal LLMs:

1. **Business Flow Isolation (Corridor Slicing):**
   Starting from a concrete Entry Gate (HTTP route, CLI command, queue consumer, or scheduled job), Corridor computes a clean execution slice down to the Exit Gate (database persistence or external boundary), automatically pruning high-fan-in utility and telemetry noise.
2. **Dual-Graph Directional Planning:**
   * **Read Corridors (Top-Down):** Ordered sequentially along execution and data flow so the agent can comprehend business behavior with minimal token overhead.
   * **Mutation Plans (Bottom-Up):** Ordered in reverse topological order starting from the target TypeScript interface, type alias, or schema definition up to its callers.
3. **Sequential Gate Locking:**
   A Corridor is partitioned into discrete, ordered stations called **Gates** (`Gate 0`, `Gate 1`, ..., `Gate N`). When a session starts, only `Gate 0` is `UNLOCKED`. All subsequent gates remain `LOCKED`—their source code is physically withheld from the agent until `Gate 0` passes syntax and contract verification.
4. **Contract Hashing & Contagion Barriers:**
   At every gate, Corridor computes a canonical `BLAKE3` hash of all exported TypeScript contracts (interfaces, types, function headers). If an edit alters a contract hash and that contract has dependents outside the active corridor, the **Contagion Barrier** blocks progression until the agent either restores backward compatibility or explicitly acknowledges the blast radius—which dynamically appends those external files as mandatory gates at the end of the queue.

---

## 3. Strategic Objectives

1. **60% to 85% Context Token Reduction:** Eliminate peripheral file reads by stripping utility noise and withholding future gate source code until needed.
2. **Zero Silent Regressions Across Module Boundaries:** Guarantee 100% visibility into cross-corridor collateral damage before an agent is permitted to mark a refactoring task complete.
3. **Deterministic, Zero-LLM Core Engine:** Ensure that given an identical git tree and entrypoint query, Corridor produces the exact same graph, gate sequence, and blast radius report in under 50 milliseconds.
4. **Drop-In MCP Integration:** Ship as a single compiled Rust binary that connects to Claude Code, Cursor, or any MCP-compliant client via `stdio` with zero external database dependencies (no Neo4j, no Docker containers).

---

## 4. Functional Specifications

### 4.1 Semantic Indexing & Graph Construction (`corridor-ts-bridge`)
* **REQ-F1.1 (Cross-File Semantic Resolution):** The system must ingest Protobuf indexes generated by `@sourcegraph/scip-typescript` (`index.scip`) to resolve cross-file symbol definitions, references, interface implementations, and type aliases accurately.
* **REQ-F1.2 (Sub-Millisecond AST Slicing):** The system must integrate `oxc_parser` to extract exact byte offsets (`byte_range`) and 1-based line numbers (`line_range`) for top-level declarations, classes, methods, functions, interfaces, and type aliases.
* **REQ-F1.3 (Body-Agnostic Contract Hashing):** When computing the signature hash of a function or method via `oxc_parser`, the system must hash strictly the exported declaration header (name, generic parameters, parameter types, and return type), excluding the internal function body. Modifying internal implementation logic without altering the signature must not trigger a contract mutation alert.
* **REQ-F1.4 (Automated Node Role Classification):** Every node in the semantic graph must be deterministically classified into one of six roles:
  * `EntryGate`: HTTP controllers, route handlers, CLI commands, queue consumers, event listeners.
  * `ContractDefinition`: Exported TypeScript interfaces, type aliases, DTO classes, Zod/validation schemas.
  * `BusinessLogic`: Domain services, use-case orchestrators, domain entities, calculators.
  * `DataRepository`: Database repositories, ORM callers (Prisma/TypeORM/Drizzle), external API adapters, state mutators.
  * `ExternalBoundary`: Third-party SDK boundaries or system exit points.
  * `UtilityNoise`: Loggers, metrics collectors, tracers, generic formatters, pure date/string helpers.

### 4.2 Heuristic Noise Pruning (`corridor-core::pruning`)
* **REQ-F2.1 (Utility Noise Score Calculation):** During forward corridor slicing, each candidate node $v$ must be evaluated using the deterministic noise formula:
  $$S_{\text{noise}}(v) = \left( \frac{\text{InDegree}(v)}{\max(1, \text{OutDegree}_{\text{Domain}}(v))} \right) \times (1.0 - W_{\text{mutation}}(v))$$
  Where $\text{InDegree}(v)$ is the total incoming reference count across the repository, $\text{OutDegree}_{\text{Domain}}(v)$ is the count of outgoing calls to non-utility domain nodes, and $W_{\text{mutation}}(v) \in \{0.0, 0.9\}$ reflects whether $v$ mutates domain or persistence state.
* **REQ-F2.2 (Automatic Pruning Rules):** A node must be automatically pruned from the corridor (and its outgoing edges ignored) if:
  1. Its file path matches configured ignore prefixes (`node_modules/`, `src/core/logger`, `src/core/telemetry`, `src/shared/utils`), OR
  2. Its `InDegree` meets or exceeds `max_fan_in_utility_threshold` (default: `15`), its `OutDegree_Domain` equals `0`, and `mutates_domain_state` is `false`.
* **REQ-F2.3 (Contract Protection Invariant):** A node classified as `ContractDefinition` or explicitly targeted by `target_mutation_symbol` must never be pruned, regardless of its `InDegree`.

### 4.3 Dual-Mode Slicing & Blast Radius Calculation (`corridor-core::slicer`)
* **REQ-F3.1 (Forward Read Slice):** When `intent == READ_FLOW`, the slicer must perform a breadth-first traversal from `entry_symbol` up to `max_corridor_depth`, condense any cyclic dependencies using Tarjan's/Kosaraju's Strongly Connected Components (SCC) algorithm, and output gates in top-down execution order.
* **REQ-F3.2 (Backward Mutation Slice):** When `intent == MUTATE_CONTRACT`, the slicer must establish the active corridor boundary from `entry_symbol`, then traverse incoming edges backwards from `target_mutation_symbol` to order impacted corridor nodes by dependency distance (Gate 0 = Contract Definition, Gate 1 = Immediate Service/Implementer, Gate N = Entry Controller).
* **REQ-F3.3 (Contagion Index & External Dependents):** For every gate $g$ in a corridor, the slicer must inspect all incoming edges across the global repository graph. Any caller or type consumer $u \notin \text{ActiveCorridor}$ must be recorded in `blast_radius.external_dependents` with its exact file path, line number, and edge kind. The total count of such external references defines the gate's `contagion_index`.

### 4.4 Stateful Gatekeeper & Enforcement Barriers (`corridor-core::state`)
* **REQ-F4.1 (Visibility Lock Enforcement):** Inspecting a gate with status `LOCKED` must return an explicit `VISIBILITY_LOCK_VIOLATION` error. Only summary metadata (symbol ID, file path, role, and blast radius count) may be visible in the corridor roadmap for locked gates.
* **REQ-F4.2 (Pre-Advance Verification Barrier):** Calling `corridor_advance_gate` on active gate $k$ must fail and transition the session to `BlockedAtGate(k)` if:
  1. `oxc_parser` detects any TypeScript syntax errors in the modified file, OR
  2. Optional incremental `tsc --noEmit` diagnostics report unresolved type errors within the current gate file.
* **REQ-F4.3 (Dynamic Contagion Expansion):** If an edit at gate $k$ alters the file's exported contract hash (`pre_edit_contract_hash != post_edit_contract_hash`) AND `blast_radius.contagion_index > 0`:
  * If `acknowledge_blast_radius == false`, `corridor_advance_gate` must reject advancement with a `CONTAGION_BARRIER` error listing every external file and line number that will break.
  * If `acknowledge_blast_radius == true`, `corridor_advance_gate` must dynamically append each external dependent as a new `LOCKED` gate at the end of the session plan before unlocking gate $k+1$.
* **REQ-F4.4 (Deterministic Rollback):** Calling `corridor_abort_or_rollback(to_gate_index)` must restore the exact pre-edit file contents on disk for all gates $\ge \text{to\_gate\_index}$, relock gates $> \text{to\_gate\_index}$, and set `ActiveGate(to_gate_index)`.

### 4.5 MCP Server Interface (`corridor-mcp`)
The server must communicate via JSON-RPC 2.0 over `stdio` and expose exactly six deterministic tools:
1. `corridor_discover_entrypoints`: Queries the indexed graph for `EntryGate` nodes matching an optional domain or path filter.
2. `corridor_open`: Initializes a `READ_FLOW` or `MUTATE_CONTRACT` session, persists state to `.corridor/active_session.json`, returns the high-level gate roadmap, and unlocks `Gate 0`.
3. `corridor_inspect_gate`: Returns the line-numbered code slice, upstream contracts, downstream corridor consumers, and blast radius report for an `UNLOCKED` or `VERIFIED` gate.
4. `corridor_propose_edit`: Writes proposed file content for the active gate, computes pre/post contract hashes, runs syntax/type verification, and records a checkpoint.
5. `corridor_advance_gate`: Executes the enforcement barrier, injects blast-radius gates if applicable, locks the current gate as `VERIFIED`, and unlocks the next gate.
6. `corridor_abort_or_rollback`: Reverts working tree changes back to a prior gate checkpoint.

---

## 5. Non-Functional Requirements & Architectural Constraints

| Category | Requirement | Target Specification |
| :--- | :--- | :--- |
| **Determinism** | Zero stochastic operations | 100% deterministic graph algorithms; zero LLM API calls or vector embeddings inside Corridor. |
| **Indexing Latency** | SCIP Protobuf ingestion + AST enrichment | `< 1.5 seconds` for a 100,000 LOC TypeScript repository. |
| **Slicing Latency** | Forward/Backward slice + Blast Radius | `< 30 milliseconds` per `corridor_open` invocation. |
| **Verification Latency** | Gate AST parse + Contract BLAKE3 diff | `< 10 milliseconds` per `corridor_propose_edit` call (Tier 1 OXC check). |
| **Memory Footprint** | Resident Set Size (RSS) during MCP loop | `< 80 MB` RAM for a 1,000-file TypeScript graph held in `petgraph`. |
| **Deployment** | Binary packaging & state storage | Single native binary (`corridor`) storing local state in `.corridor/` directory. |

---

## 6. Explicit Non-Goals (Out of Scope for V1)

To prevent scope creep and ensure rapid delivery of a production-grade MVP, the following are strictly excluded from V1:

1. **No Graphical User Interface (GUI) or Web Dashboards:** Humans do not browse visual node graphs in V1; AI Coding Agents consume structured JSON/Markdown over MCP.
2. **No Autonomous Code Generation or Codemod Execution:** Unlike `OpenRewrite`, Corridor does not rewrite code itself. It acts strictly as the navigation corridor, context filter, and compiler gatekeeper for an external coding agent.
3. **No Polyglot Target Support in V1:** Python, Go, Rust, and Java target repositories will not be supported until TypeScript cross-file contract gating achieves benchmark targets on complex Node.js/Next.js/NestJS codebases.
4. **No Cloud SaaS or Remote Storage:** All indexing, slicing, and checkpointing happen 100% locally on the developer's machine.

---

## 7. Success Criteria & Quantitative Evaluation Benchmarks

Corridor V1 will be evaluated against a standardized benchmark suite consisting of 10 real-world multi-file refactoring tasks on a medium-to-large TypeScript codebase (300+ files, modular architecture with shared DTOs, services, controllers, and background workers). Each task will be executed twice by the same coding agent (e.g., Claude Code): once using standard baseline tools (`ripgrep` + file read/edit) and once constrained by the `Corridor` MCP server.

### 7.1 Primary Success Metrics

| Metric ID | Metric Name | Baseline (Unconstrained Agent) | Target Success Criterion (With Corridor) | Measurement Methodology |
| :--- | :--- | :--- | :--- | :--- |
| **SC-01** | **Noise Pruning Ratio** | `0%` (Agent manually filters or ingests utility imports) | **$\ge 80\%$** of connected logger, telemetry, and pure utility nodes stripped automatically | $\frac{\vert{}\text{PrunedNodes}\vert{}}{\vert{}\text{PrunedNodes}\vert{} + \vert{}\text{CorridorGates}\vert{}}$ across benchmark entrypoints |
| **SC-02** | **Cumulative Input Token Consumption** | `100%` baseline token spend per refactoring task | **$\ge 60\%$ reduction** in total input tokens consumed to complete the refactoring task | Compare total prompt tokens logged by the agent harness across all 10 tasks |
| **SC-03** | **External Blast-Radius Detection Rate** | Unpredictable; frequently leaves external callers broken | **$100\%$ pre-exit detection** of broken external dependents outside the primary feature flow | Introduce breaking changes to shared interfaces used by 3+ external modules; verify `CONTAGION_BARRIER` triggers every time |
| **SC-04** | **Topological Edit Compliance** | Frequent back-and-forth re-edits of the same file (`2.5+` edits/file) | **$0$ out-of-order file edits**; $\le 1.2$ average edit passes per unlocked gate | Audit `.corridor/active_session.json` checkpoint logs to verify strictly monotonic gate progression |
| **SC-05** | **Post-Refactoring Build Pass Rate** | `~55% - 70%` clean `tsc --noEmit` pass on first completion claim | **$100\%$ clean `tsc --noEmit` pass** when session reaches `SessionPhase::Completed` | Run full repository `tsc --noEmit` and integration test suite immediately after session completion |

---

## 8. End-to-End Operational Workflow (Agent Interaction Protocol)

The following sequence defines the mandatory operational loop when an AI agent executes a refactoring request using Corridor:

```text
User Prompt: "Update TaxCalculationResult to support compound regional taxes and refactor the checkout flow."
 │
 ├─► Step 1: Agent calls `corridor_discover_entrypoints({ filter: "checkout" })`
 │     └─◄ Corridor returns `CheckoutController.processCheckout` (no file contents dumped).
 │
 ├─► Step 2: Agent calls `corridor_open({
 │             entry_symbol: "src/checkout/checkout.controller.ts#CheckoutController.processCheckout",
 │             intent: "MUTATE_CONTRACT",
 │             target_mutation_symbol: "src/tax/tax.types.ts#TaxCalculationResult"
 │           })`
 │     └─◄ Corridor prunes 42 logger/utility nodes, builds a 4-gate bottom-up Mutation Plan,
 │         unlocks Gate 0 (`tax.types.ts`), locks Gates 1..3, and warns that 2 external files
 │         (`invoice.generator.ts`, `renewal.worker.ts`) depend on `TaxCalculationResult`.
 │
 ├─► Step 3: Agent edits Gate 0 and calls `corridor_propose_edit({ gate_index: 0, new_file_content: "..." })`
 │     └─◄ Corridor runs `oxc_parser`, detects that `TaxCalculationResult` contract hash changed,
 │         and verifies syntax is clean.
 │
 ├─► Step 4: Agent calls `corridor_advance_gate()`
 │     └─◄ Corridor triggers `CONTAGION_BARRIER` because 2 external dependents were not acknowledged.
 │
 ├─► Step 5: Agent re-submits `corridor_propose_edit` with `acknowledge_blast_radius: true` and calls `corridor_advance_gate()`
 │     └─◄ Corridor dynamically appends `invoice.generator.ts` (Gate 4) and `renewal.worker.ts` (Gate 5)
 │         to the corridor queue, locks Gate 0 as `VERIFIED`, and unlocks Gate 1 (`tax.service.ts`).
 │
 └─► Step 6: Agent iterates sequentially through Gates 1, 2, 3, 4, and 5.
       └─◄ Upon advancing past Gate 5, Corridor transitions to `SessionPhase::Completed`.
           Zero unrelated files were read, and zero external dependents were left broken.

---

## 9. Release Milestones & Verification Gates
Milestone M1 (Core Slicer & SCIP Ingestion): corridor index and corridor slice CLI commands operational on real TypeScript repositories; verified 
≥
80
%
≥80% utility noise pruning.

Milestone M2 (Contract Hasher & Blast Radius Engine): oxc_parser signature hashing and cross-corridor contagion detection passing 100% of unit test fixtures.

Milestone M3 (Stateful MCP Server): Full 6-tool JSON-RPC stdio server integrated with Claude Code and Cursor; passing all 5 quantitative success criteria (SC-01 through SC-05) on the multi-module TypeScript benchmark repository.