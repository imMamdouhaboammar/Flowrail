# Flowrail (Project Ariadne): Getting Started & Architecture Guide

Flowrail replaces flat codebase search with a state-machine Model Context Protocol (MCP) server. It slices repositories into isolated **Business Flow Corridors** (for reading) and **Topological Mutation Plans** (for writing), enforcing contract verification and blast-radius fencing at every boundary gate.

---

## 1. Executive Summary & Problem Statement

AI coding agents struggle with large-scale codebase navigation and refactoring due to two distinct failure modes:

1. **Context Pollution:** Flat grep, unranked file globs, and static AST dumps flood the context window with utility boilerplate, logging wrappers, and irrelevant dependencies.
2. **Topological Blindness:** Standard tools traverse dependencies in forward execution order (Controller -> Service -> Repository). Editing in this order leaves intermediate states broken and obscures breaking contract changes until downstream files are modified.

Flowrail enforces a strict state machine over MCP. The agent cannot jump ahead while intermediate gates have broken type contracts or unacknowledged blast-radius contagion.

---

## 2. Core Architectural Pillars

### Pillar 1: Dual-Graph Representation (Read vs. Write Asymmetry)
Reading code and refactoring code move in opposite directions:
* **The Execution Corridor (Read Graph):** Top-down slice starting from an Entry Gate (HTTP route, CLI command, event consumer) down to the Exit Gate (database mutation, external API call), stripping away infrastructure noise.
* **The Mutation Plan (Write Graph):** Bottom-up dependency resolution starting from the modified Contract, Schema, or Type. Files are topologically ordered so upstream definitions are updated and verified before downstream consumers are opened for editing.

### Pillar 2: Hybrid Semantic Extraction
Flowrail builds its graph via a two-tier extraction pipeline:
* **Tier 1 (Deterministic Semantic Index):** Uses SCIP (`scip-typescript` / SCIP Protobuf) to extract cross-file symbol definitions, references, and interface implementations.
* **Tier 2 (Fast AST & Range Extraction):** Uses `oxc` (`oxc_parser`, `oxc_semantic`, `oxc_ast`) for sub-millisecond local AST slicing, export signature hashing, and comment/noise stripping.

### Pillar 3: Blast Radius & Cross-Corridor Contagion Detection
Isolating a corridor hides the rest of the codebase from the agent to conserve context tokens. However, modifying a shared contract inside Corridor A may break Corridor B:
* Every node in a corridor tracks an **External Consumer Count** and a **Contagion Index**.
* When an agent mutates an exported signature at Gate $N$, Flowrail calculates external files depending on this symbol and triggers a **Contagion Barrier**. The agent must acknowledge the blast radius or revert the change.

### Pillar 4: Stateful MCP Enforcement
Flowrail exposes its capabilities as a Model Context Protocol server over `stdio`:
* **State `IDLE`:** No active corridor session.
* **State `MAPPING`:** Flowrail slices the requested entrypoint and computes the topological DAG.
* **State `IN_GATE(n)`:** The agent is granted read/write visibility strictly to Gate $n$ and read-only visibility to immediate contracts.
* **State `VERIFYING(n)`:** Flowrail runs syntax and contract signature checks on Gate $n$.
* **State `BLOCKED(n)`:** Gate $n$ introduced syntax errors or breaking contract changes.
* **State `COMPLETED`:** All gates traversed and verified with zero unresolved contract breaks.

---

## 3. Workspace Layout

```
Flowrail/
├── Cargo.toml                   # Workspace manifest
├── package.json                 # Distribution package
├── install.sh                   # One-line installer
├── crates/
│   ├── flowrail-core/           # Graph algorithms (petgraph), pruning heuristics, contract hashing
│   ├── flowrail-ts-bridge/      # SCIP loader, oxc AST slicer, tsconfig resolver, watcher
│   ├── flowrail-mcp/            # Tokio-based MCP server implementing the 10 tools
│   └── flowrail-cli/            # CLI binary (flowrail init, doctor, demo, search, grep, mcp)
├── fixtures/
│   └── sample-ts-ecommerce/     # Multi-module TypeScript fixture for integration testing
└── docs/
    └── solutions/               # Codified engineering learnings
```

---

## 4. MCP Tools Reference

| Tool | Purpose | Intent |
| :--- | :--- | :--- |
| `flowrail_discover_entrypoints` | Discovers entrypoints ranked by PageRank with query/path constraints. | Read / Survey |
| `flowrail_find_symbols` | Definition-first symbol search with typo tolerance. | Search |
| `flowrail_grep` | In-memory definition grep scoped to the workspace or active corridor. | Search |
| `flowrail_open` | Initializes a corridor session (`READ_FLOW` or `MUTATE_CONTRACT`). | Session Init |
| `flowrail_inspect_gate` | Returns the code slice, contracts, and blast radius for Gate $N$. | Inspection |
| `flowrail_propose_edit` | Stages an edit, checks syntax, and computes contract signature delta. | Mutation |
| `flowrail_advance_gate` | Validates current gate and advances to Gate $N+1$. | Transition |
| `flowrail_abort_or_rollback` | Reverts uncommitted changes back to a prior checkpoint. | Recovery |
| `flowrail_export_artifact` | Exports session state or architecture plans as structured JSON. | Artifact |
| `flowrail_enrich_artifact` | Enriches exported artifacts with verified graph citations and steps. | ADHD Enrichment |

---

## 5. Quickstart

### Installation

Build from source via Cargo or install with the installer script:

```bash
# Clone the repository
git clone https://github.com/imMamdouhaboammar/Flowrail.git
cd Flowrail

# Install Flowrail
./install.sh
```

### Verification

Run the built-in system diagnostics:

```bash
flowrail doctor
```

Run the live interactive demonstration on the included e-commerce fixture:

```bash
flowrail demo
```

### Initialize Your Project

Configure Flowrail MCP for your AI harness inside any workspace:

```bash
flowrail init --target all
```
