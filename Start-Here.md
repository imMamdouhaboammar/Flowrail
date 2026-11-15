# CORRIDOR: Deterministic Execution & Mutation State-Machine for AI Coding Agents

## 1. Executive Summary & Problem Statement
AI coding agents fail at large-scale refactoring not because of weak code generation capabilities, but due to **Context Pollution** and **Topological Blindness**. When tasked with modifying a feature across a 1,000+ file repository, standard tools (grep, RAG, or static AST dumps) flood the LLM context window with flat call graphs, utility noise, and unranked dependencies. Consequently, agents lose track of business logic flow, break cross-file type contracts, and hallucinate edits in unrelated modules.

**Corridor** solves this by replacing static codebase exploration with a **Strict State-Machine MCP Server**. Instead of allowing an agent to roam freely across the repository, Corridor slices the codebase into isolated **Business Flow Corridors** (for reading) and **Topological Mutation Plans** (for writing), enforcing contract verification at every boundary gate before unlocking the next file.

---

## 2. Core Architectural Pillars

### Pillar 1: Dual-Graph Representation (Read vs. Write Asymmetry)
Reading code and refactoring code move in opposite directions. Corridor constructs two distinct Directed Acyclic Graphs (DAGs) per feature:
1. **The Execution Corridor (Read Graph):** Top-down slice starting from an Entry Gate (HTTP Route, CLI command, Event Consumer, Cron) down to the Exit Gate (Database mutation, External API call, Response payload), stripping away infrastructure noise (loggers, generic telemetry, standard utils).
2. **The Mutation Plan (Write Graph):** Bottom-up dependency resolution starting from the modified Contract/Schema/Type, ordering target files topologically so that upstream definitions are updated and verified before downstream consumers are opened for editing.

### Pillar 2: Hybrid Semantic Extraction (Static + Runtime Slicing)
Pure static analysis fails in modern architectures utilizing Dependency Injection (DI), Event Buses, and Middleware. Corridor builds its graph via a two-tier pipeline:
1. **Tier 1 (Deterministic Semantic Index):** Leverages `scip-typescript` / LSP (`tsserver`) to extract exact cross-file symbol definitions, references, and interface implementations without reinventing a compiler in Rust.
2. **Tier 2 (Optional Runtime Trace Overlay):** Hooks into existing E2E/Integration tests via V8 Inspector / OpenTelemetry to record actual runtime call stacks during a test run (e.g., `npm test checkout.spec.ts`), bridging dynamic gaps caused by DI containers or event emitters.

### Pillar 3: Blast Radius & Cross-Corridor Contagion Detection
Isolating a corridor hides the rest of the codebase from the agent to save tokens. However, modifying a shared module inside Corridor A might break Corridor B.
* Every Node in a Corridor is tagged with an **External Consumer Count** and a **Contagion Index**.
* If an agent mutates an exported signature or interface at Gate $N$, Corridor calculates the exact external files outside the active corridor that depend on this symbol and injects a mandatory **Contract Checkpoint Warning** before allowing progression.

### Pillar 4: State-Machine Enforcement via MCP
Corridor exposes its capabilities strictly as a Stateful Model Context Protocol (MCP) server. The agent cannot jump to Step 4 while Step 2 has broken type contracts.
* **State `IDLE`:** No active refactoring session.
* **State `MAPPING`:** Corridor slices the requested entrypoint and computes the Mutation DAG.
* **State `IN_GATE(n)`:** Agent is granted read/write visibility strictly to the file(s) in Gate $n$ and read-only visibility to the immediate upstream/downstream Interface Contracts.
* **State `VERIFYING(n)`:** Corridor runs incremental type-checking (`tsc --noEmit` / LSP diagnostics) on Gate $n$.
* **State `BLOCKED(n)`:** Gate $n$ introduced unexpected breaking changes or failed syntax/type validation. Agent must fix Gate $n$ or explicitly declare a Contract Mutation to recalculate downstream gates.
* **State `COMPLETED`:** All gates traversed, zero broken contracts across the blast radius, integration test passes.

---

## 3. System Data Models & Schemas

### 3.1 The Corridor Graph Schema (Internal & Serialization Format)
```json
{
  "corridor_id": "cor_checkout_tax_v1",
  "target_language": "typescript",
  "entry_gate": {
    "symbol": "CheckoutController.processCheckout",
    "file_path": "src/modules/checkout/checkout.controller.ts",
    "line_range": [42, 89],
    "trigger_type": "HTTP_POST",
    "route": "/api/v1/checkout"
  },
  "nodes": [
    {
      "gate_index": 0,
      "node_id": "node_tax_types",
      "role": "CONTRACT_DEFINITION",
      "file_path": "src/modules/tax/tax.types.ts",
      "symbols": ["TaxCalculationInput", "TaxCalculationResult"],
      "blast_radius": {
        "internal_dependents": ["node_tax_service", "node_checkout_service"],
        "external_dependents_count": 3,
        "external_files": [
          "src/modules/invoicing/invoice.generator.ts",
          "src/modules/reporting/tax-report.job.ts",
          "src/modules/subscriptions/renewal.service.ts"
        ]
      }
    },
    {
      "gate_index": 1,
      "node_id": "node_tax_service",
      "role": "BUSINESS_LOGIC",
      "file_path": "src/modules/tax/tax.service.ts",
      "symbols": ["TaxService.calculateVat"],
      "upstream_contracts": ["node_tax_types"],
      "downstream_Consumers": ["node_checkout_service"]
    }
  ],
  "exit_gate": {
    "symbol": "OrderRepository.saveOrder",
    "file_path": "src/modules/orders/order.repository.ts",
    "verification_command": "npx vitest run src/modules/checkout/__tests__/checkout.int.spec.ts"
  }
}


اسم المشروع الكودي هو Ariadne واسمه التجاري Flowrail>
# CORRIDOR: Deterministic Execution & Mutation State-Machine for AI Coding Agents

## 1. Executive Summary & Problem Statement
AI coding agents fail at large-scale refactoring not because of weak code generation capabilities, but due to **Context Pollution** and **Topological Blindness**. When tasked with modifying a feature across a 1,000+ file repository, standard tools (grep, RAG, or static AST dumps) flood the LLM context window with flat call graphs, utility noise, and unranked dependencies. Consequently, agents lose track of business logic flow, break cross-file type contracts, and hallucinate edits in unrelated modules.

**Corridor** solves this by replacing static codebase exploration with a **Strict State-Machine MCP Server**. Instead of allowing an agent to roam freely across the repository, Corridor slices the codebase into isolated **Business Flow Corridors** (for reading) and **Topological Mutation Plans** (for writing), enforcing contract verification at every boundary gate before unlocking the next file.

---

## 2. Core Architectural Pillars

### Pillar 1: Dual-Graph Representation (Read vs. Write Asymmetry)
Reading code and refactoring code move in opposite directions. Corridor constructs two distinct Directed Acyclic Graphs (DAGs) per feature:
1. **The Execution Corridor (Read Graph):** Top-down slice starting from an Entry Gate (HTTP Route, CLI command, Event Consumer, Cron) down to the Exit Gate (Database mutation, External API call, Response payload), stripping away infrastructure noise (loggers, generic telemetry, standard utils).
2. **The Mutation Plan (Write Graph):** Bottom-up dependency resolution starting from the modified Contract/Schema/Type, ordering target files topologically so that upstream definitions are updated and verified before downstream consumers are opened for editing.

### Pillar 2: Hybrid Semantic Extraction (Static + Runtime Slicing)
Pure static analysis fails in modern architectures utilizing Dependency Injection (DI), Event Buses, and Middleware. Corridor builds its graph via a two-tier pipeline:
1. **Tier 1 (Deterministic Semantic Index):** Leverages `scip-typescript` / LSP (`tsserver`) to extract exact cross-file symbol definitions, references, and interface implementations without reinventing a compiler in Rust.
2. **Tier 2 (Optional Runtime Trace Overlay):** Hooks into existing E2E/Integration tests via V8 Inspector / OpenTelemetry to record actual runtime call stacks during a test run (e.g., `npm test checkout.spec.ts`), bridging dynamic gaps caused by DI containers or event emitters.

### Pillar 3: Blast Radius & Cross-Corridor Contagion Detection
Isolating a corridor hides the rest of the codebase from the agent to save tokens. However, modifying a shared module inside Corridor A might break Corridor B.
* Every Node in a Corridor is tagged with an **External Consumer Count** and a **Contagion Index**.
* If an agent mutates an exported signature or interface at Gate $N$, Corridor calculates the exact external files outside the active corridor that depend on this symbol and injects a mandatory **Contract Checkpoint Warning** before allowing progression.

### Pillar 4: State-Machine Enforcement via MCP
Corridor exposes its capabilities strictly as a Stateful Model Context Protocol (MCP) server. The agent cannot jump to Step 4 while Step 2 has broken type contracts.
* **State `IDLE`:** No active refactoring session.
* **State `MAPPING`:** Corridor slices the requested entrypoint and computes the Mutation DAG.
* **State `IN_GATE(n)`:** Agent is granted read/write visibility strictly to the file(s) in Gate $n$ and read-only visibility to the immediate upstream/downstream Interface Contracts.
* **State `VERIFYING(n)`:** Corridor runs incremental type-checking (`tsc --noEmit` / LSP diagnostics) on Gate $n$.
* **State `BLOCKED(n)`:** Gate $n$ introduced unexpected breaking changes or failed syntax/type validation. Agent must fix Gate $n$ or explicitly declare a Contract Mutation to recalculate downstream gates.
* **State `COMPLETED`:** All gates traversed, zero broken contracts across the blast radius, integration test passes.

---

## 3. System Data Models & Schemas

### 3.1 The Corridor Graph Schema (Internal & Serialization Format)
```json
{
  "corridor_id": "cor_checkout_tax_v1",
  "target_language": "typescript",
  "entry_gate": {
    "symbol": "CheckoutController.processCheckout",
    "file_path": "src/modules/checkout/checkout.controller.ts",
    "line_range": [42, 89],
    "trigger_type": "HTTP_POST",
    "route": "/api/v1/checkout"
  },
  "nodes": [
    {
      "gate_index": 0,
      "node_id": "node_tax_types",
      "role": "CONTRACT_DEFINITION",
      "file_path": "src/modules/tax/tax.types.ts",
      "symbols": ["TaxCalculationInput", "TaxCalculationResult"],
      "blast_radius": {
        "internal_dependents": ["node_tax_service", "node_checkout_service"],
        "external_dependents_count": 3,
        "external_files": [
          "src/modules/invoicing/invoice.generator.ts",
          "src/modules/reporting/tax-report.job.ts",
          "src/modules/subscriptions/renewal.service.ts"
        ]
      }
    },
    {
      "gate_index": 1,
      "node_id": "node_tax_service",
      "role": "BUSINESS_LOGIC",
      "file_path": "src/modules/tax/tax.service.ts",
      "symbols": ["TaxService.calculateVat"],
      "upstream_contracts": ["node_tax_types"],
      "downstream_Consumers": ["node_checkout_service"]
    }
  ],
  "exit_gate": {
    "symbol": "OrderRepository.saveOrder",
    "file_path": "src/modules/orders/order.repository.ts",
    "verification_command": "npx vitest run src/modules/checkout/__tests__/checkout.int.spec.ts"
  }
}


3.2 Pruned Noise Rules (Heuristic & Semantic Filter)
To keep corridors purely focused on business logic, the Slicer automatically prunes edges leading to:

Standard library primitives and node_modules (unless explicitly whitelisted as boundary adapters).

High-fan-in / Zero-business-state utilities (e.g., Logger.*, Metrics.*, DateUtils.*, ConfigService.get).

Pure UI presentation primitives (when slicing backend/domain flows) or pure infrastructure boilerplate (middlewares that do not mutate the domain payload).

4. MCP Tool Interface Specification (What the AI Sees)
Corridor exposes exactly 6 deterministic tools to the AI Agent via MCP:

corridor_discover_entrypoints(query?: string, domain_filter?: string)

Scans the repo for HTTP routes, CLI commands, Queue workers, and GraphQL resolvers. Returns a clean list of entry gates without dumping file contents.

corridor_open(entrypoint_id: string, intent: "READ_FLOW" | "MUTATE_CONTRACT", target_symbol?: string)

Initializes the state machine. Returns the ordered list of Gate summaries (titles and roles only, hiding raw code of future gates) and opens Gate 0 with its full code slice and upstream/downstream type contracts.

corridor_inspect_gate(gate_index: number)

Returns the exact code slice of the current active gate, its imported contract definitions, and its Blast Radius report. Refuses to return full code for future locked gates to prevent context pollution.

corridor_propose_edit(gate_index: number, file_path: string, diff_or_content: string)

Applies the edit in a shadowed/working state, runs incremental LSP/Type diagnostics, and computes whether the exported contract of this gate changed.

corridor_advance_gate()

The Enforcement Barrier: Checks if the current gate compiles cleanly or if its downstream contract mutations match the expected next gates. If valid, locks the current gate and unlocks gate_index + 1. If external blast radius files were broken, dynamically injects them into the corridor queue!

corridor_abort_or_rollback(to_gate_index?: number)

Restores working tree state to a specific checkpoint gate if the agent realizes its architectural approach failed.

5. Technical Stack & Crate Architecture (Rust + TypeScript Target)
Host Language: Rust (2024 Edition)

Graph Engine: petgraph (DAG construction, Tarjan's strongly connected components for cycle detection, topological sorting, Dominator tree analysis for blast radius).

Semantic Ingestion:

Primary: scip crate (Protobuf parser for Sourcegraph SCIP indexes generated via scip-typescript) OR headless typescript-language-server JSON-RPC client.

Fast AST Slicing & Pruning: oxc_parser & oxc_semantic (for ultra-fast local AST range extraction, export signature hashing, and comment/noise stripping).

State & Storage: rusqlite (Embedded SQLite with WAL mode stored in .corridor/state.db for sub-millisecond graph queries and session state persistence).

MCP Transport: tokio + serde_json (Stdio & SSE transport implementing the Model Context Protocol specification).

Git Safety Layer: git2 (libgit2 bindings for creating lightweight in-memory/stash checkpoints at every passed Gate).

6. Repository Directory Structure
Plaintext
corridor/
├── Cargo.toml
├── PROJECT_MANIFEST.md
├── crates/
│   ├── corridor-core/           # Graph models, DAG algorithms, Blast Radius calculator
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── graph.rs         # petgraph wrappers, Forward/Backward slicing
│   │   │   ├── pruning.rs       # Fan-in heuristics to filter out loggers/utils
│   │   │   ├── contract.rs      # Interface signature hashing & diffing
│   │   │   └── state.rs         # State machine transitions & gate locking logic
│   ├── corridor-ts-bridge/      # TypeScript semantic extractor & verifier
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── scip_loader.rs   # Parses index.scip into Corridor Core Graph
│   │   │   ├── lsp_client.rs    # Persistent tsserver daemon for <50ms diagnostics
│   │   │   └── ast_slicer.rs    # Uses oxc to extract exact function/type byte ranges
│   ├── corridor-mcp/            # Stateful MCP Server implementation
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── tools.rs         # The 6 MCP tool handlers
│   │   │   └── prompt_fmt.rs    # Formats Gate views into token-efficient Markdown
│   └── corridor-cli/            # Binary entrypoint (`corridor init`, `corridor mcp`)
│       └── src/
│           └── main.rs
└── fixtures/
    └── sample-ts-ecommerce/     # Realistic multi-module TS project for testing slices
7. Phased Execution Roadmap (MVP to V1)
Phase 1: The Semantic Graph & Slicer CLI (Weeks 1-2)
Goal: Prove that given a TypeScript repo and a function name, Corridor outputs a clean, noise-free JSON corridor without starting an LLM.

Deliverables:

Integrate scip-typescript invocation and parse the resulting Protobuf into petgraph.

Implement oxc range extractor to pull exact code blocks for nodes.

Build the Fan-in/Utility Pruning algorithm (automatically drop nodes with >50 incoming edges and 0 domain state mutations, like Logger).

CLI command: corridor slice --entry "src/checkout/controller.ts:processCheckout".

Phase 2: Mutation DAG & Blast Radius Engine (Week 3)
Goal: Prove that modifying a shared Type calculates the exact topological order of files to fix, plus external collateral damage.

Deliverables:

Implement corridor-core::contract using oxc to hash exported TypeScript interface, type, and function signatures.

Build reverse-topological sorting from a mutated node up to corridor boundaries.

Build the Contagion Detector (flagging external dependents outside the active corridor).

Phase 3: The Stateful MCP Server & LSP Gatekeeper (Week 4)
Goal: Connect Claude Code / Cursor to Corridor and run a real multi-file refactoring task locked inside the state machine.

Deliverables:

Implement the 6 MCP tools over stdio.

Integrate incremental tsc --noEmit / tsserver diagnostics check on corridor_advance_gate().

Automatic Git stash/tree checkpointing per gate for instant rollback.

Phase 4: Runtime Trace Overlay (V1.1 - Post-MVP)
Goal: Solve Dynamic Dispatch, DI (tsyringe, NestJS), and EventEmitter blind spots.

Deliverables:

Inject a lightweight Node --import hook during test execution (vitest / jest) to log actual runtime call edges.

Merge runtime edges into the static petgraph with [DYNAMIC_EDGE] metadata tags.

8. Non-Goals & Strict Anti-Patterns (What We Will NOT Build)
No LLM Inside the Core Engine: Corridor itself must be 100% deterministic and zero-token. It does not call OpenAI/Anthropic APIs to "summarize" code. It builds deterministic graphs and serves them to the user's coding agent.

No Visual Dashboards in MVP: Humans are not the primary consumer; AI Agents over MCP are. Visual UI is a distraction until the MCP state machine is proven.

No Multi-Language Support in MVP: We do not touch Python, Go, or Rust target repos until TypeScript cross-file contract gating achieves >95% accuracy on complex Next.js/NestJS codebases.


---

## 2. ملف إعداد مساحة العمل (`Cargo.toml` للـ Workspace)

عشان لما تيجي تبدأ الكود فعلياً، دي المكتبات الحقيقية اللي هتحتاجها في Rust من غير تخريف:

```toml
[workspace]
resolver = "2"
members = [
    "crates/corridor-core",
    "crates/corridor-ts-bridge",
    "crates/corridor-mcp",
    "crates/corridor-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
authors = ["Mamdouh Aboammar"]
description = "Deterministic Execution & Mutation State-Machine for AI Coding Agents"

[workspace.dependencies]
# Async & Runtime
tokio = { version = "1.40", features = ["full"] }
anyhow = "1.0"
thiserror = "2.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Serialization & MCP
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Graph & Algorithms
petgraph = { version = "0.6", features = ["serde-1"] }
indexmap = { version = "2.6", features = ["serde"] }

# Fast AST Slicing & Signature Hashing for TypeScript
oxc_allocator = "0.38"
oxc_parser = "0.38"
oxc_ast = "0.38"
oxc_span = "0.38"
oxc_semantic = "0.38"

# SCIP Protobuf Ingestion (Cross-file Type Resolution)
scip = "0.4"
protobuf = "3.5"

# State Persistence & Git Checkpoints
rusqlite = { version = "0.32", features = ["bundled"] }
git2 = "0.19"
blake3 = "1.5"

# CLI
clap = { version = "4.5", features = ["derive"] }




