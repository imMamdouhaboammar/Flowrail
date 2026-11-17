# Solution: Flowrail (Project Ariadne) Architecture & Implementation

## 1. Problem Context
AI coding agents face severe failure modes when conducting multi-file refactoring on large-scale repositories (1,000+ files):
- **Context Pollution:** Indiscriminate ingestion of peripheral utility noise (loggers, telemetry, helpers) consumes context tokens and dilutes business logic.
- **Topological Blindness (Read vs. Write Asymmetry):** Agents read top-down (Controller -> Service -> Repo) but must mutate bottom-up (Contract -> Service -> Controller). Flat graph dumps allow agents to edit downstream consumers before stabilizing upstream contracts.
- **Contagion Blindness:** Modifying shared type definitions breaks external callers outside the agent's active context window.

## 2. Implemented Architecture
Flowrail solves this via a deterministic, 100% zero-token, zero-LLM state-machine MCP server and CLI:
1. **`flowrail-core`:**
   - `graph.rs`: Semantic graph models (`NodeRole`, `EdgeKind`, `SemanticNode`, `SemanticEdge`, `CodebaseGraph`) using `petgraph::DiGraph` and `IndexMap`.
   - `contract.rs`: Signature normalization and BLAKE3 hashing to detect interface and type drift without sensitivity to whitespace.
   - `pruning.rs`: Deterministic utility noise scoring using $S_{\text{noise}}(v) = \left(\frac{\text{InDegree}(v)}{\max(1, \text{OutDegree}_{\text{Domain}}(v))}\right) \times (1.0 - W_{\text{mutation}}(v))$. Protects `ContractDefinition` nodes and target symbols.
   - `slicer.rs`: Dual-mode slicing (Top-down `ReadFlow` vs Bottom-up `MutateContract` via Tarjan SCC cycle-breaking and reverse topological distance sorting). Calculates external blast radius and `contagion_index`.
   - `state.rs`: `CorridorSession` enforcing `VISIBILITY_LOCK` (locks future gate source code), `CONTAGION_BARRIER` (blocks progression on unacknowledged breaking contract changes), dynamic gate injection on acknowledgment, and hermetic gate rollback.

2. **`flowrail-ts-bridge`:**
   - `ast_slicer.rs`: Sub-millisecond AST range extraction and export signature hashing using `oxc_parser` 0.38.
   - `scip_loader.rs`: Sourcegraph SCIP protobuf loader (`index.scip`) with automatic fallback to fast recursive directory AST extraction when `index.scip` is absent.
   - `verifier.rs`: Two-tier incremental verification (Tier 1 <2ms OXC AST syntax gate + Tier 2 incremental `tsc --noEmit`).

3. **`flowrail-mcp`:**
   - Exposes the 6 stateful Model Context Protocol tools over stdio (`corridor_*` with `flowrail_*` aliases):
     - `corridor_discover_entrypoints` / `flowrail_discover_entrypoints`
     - `corridor_open` / `flowrail_open`
     - `corridor_inspect_gate` / `flowrail_inspect_gate`
     - `corridor_propose_edit` / `flowrail_propose_edit`
     - `corridor_advance_gate` / `flowrail_advance_gate`
     - `corridor_abort_or_rollback` / `flowrail_abort_or_rollback`

4. **`flowrail-cli`:**
   - Commands: `flowrail index`, `flowrail slice`, `flowrail mcp`, `flowrail doctor`.

## 3. Verification & Evidence
- 100% test pass across all 4 workspace crates (14 unit and integration tests).
- Clean clippy pass with 0 warnings.
- Verified on realistic multi-file fixture `fixtures/sample-ts-ecommerce` (checkout, tax, invoicing, renewal).
- Clean-room brand sweep verified via `@plugin:replica-square` (`clean_room_passed: true`, 0 violations).
