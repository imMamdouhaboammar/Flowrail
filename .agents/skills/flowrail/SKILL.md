---
name: flowrail
description: >
  Deterministic execution and mutation state-machine MCP server and CLI for AI coding agents. Slices TypeScript codebases into isolated business flow corridors (for reading) and topological mutation plans (for writing), enforcing sequential visibility locks, contract BLAKE3 hashing, and cross-feature blast-radius contagion barriers.
author: Mamdouh Aboammar <mamdouhfces1997@gmail.com>
repository: https://github.com/imMamdouhaboammar/Flowrail
metadata:
  version: "0.1.0"
  engine: "flowrail-core"
  target: "TypeScript/JavaScript"
  author: Mamdouh Aboammar <mamdouhfces1997@gmail.com>
  repository: https://github.com/imMamdouhaboammar/Flowrail
---

# Flowrail: Deterministic Execution & Mutation State-Machine

> A zero-LLM, high-performance Rust Model Context Protocol (MCP) server and CLI that guarantees safe, regression-free multi-file refactoring and business flow comprehension on TypeScript repositories.

## Why This Skill Is Needed & Target State

**Why this skill is needed:** Modern AI coding agents excel at single-file edits but consistently fail on multi-file structural refactoring across production codebases:
1. **Context Pollution:** Blindly ingesting loggers, telemetry, and pure utilities consumes context tokens (60-85% waste) and dilutes core business logic.
2. **Topological Blindness:** Reading code follows execution flow (`Controller -> Service -> Repository`), but refactoring shared types requires the exact reverse topological order (`Contract -> Repository -> Service -> Controller`). Agents edit callers before contracts stabilize.
3. **Contagion Blindness:** Modifying a shared interface or DTO in Feature A silently breaks Features B, C, and D outside the agent's active context.
4. **Erratic Multi-File Hopping:** Without mechanical state enforcement, agents jump randomly across files, leaving intermediate files broken.

**Target state:** Flowrail replaces unconstrained repository reading/writing with a deterministic state machine exposed via standard MCP tools, ensuring zero regressions, zero out-of-order edits, and verified step transitions (syntax checking via Tier 1 OXC parser by default; full type-checking when `--full-typecheck` is enabled).

---

## 2. Core Architectural Invariants

| Invariant | Mechanism | Guarantee |
| :--- | :--- | :--- |
| **Noise Pruning** | $S_{\text{noise}}(v) = \left(\frac{\text{InDegree}(v)}{\max(1, \text{OutDegree}_{\text{Domain}}(v))}\right) \times (1.0 - W_{\text{mutation}}(v))$ | Strips 80%+ of connected utility/logger noise; protects contract definitions. |
| **Visibility Lock** | Future gates are marked `LOCKED`; source code is withheld. | Prevents model distraction; cuts prompt tokens by 60–85%. |
| **Contract Hashing** | Normalized `BLAKE3` hash of exported declaration headers. | Implementation changes pass; signature changes trigger contract mutation checks. |
| **Contagion Barrier** | Global incoming reference scan across files outside active corridor. | Blocks `flowrail_advance_gate` if external callers break without explicit acknowledgement. |
| **Dynamic Gate Injection**| When blast radius is acknowledged, external callers are appended to the queue. | Zero silent cross-module breakages left unresolved. |
| **Hermetic Rollback** | File snapshots stored per-gate in `.flowrail/active_session.json`. | Instant byte-exact restore to any prior checkpoint if build or types fail. |

---

## 3. Tool Reference (MCP Server Interface)

The Flowrail MCP server communicates over `stdio` via JSON-RPC 2.0. Both `flowrail_*` and `corridor_*` prefixes are supported:

### 3.1 Discovery & Search
- `flowrail_discover_entrypoints({ query, filter, git_only, definitions_only, max_results })`:
  Fuzzy searches entrypoint gates (HTTP controllers, CLI commands, queue consumers) with Git status badges (`[GIT:MODIFIED]`, `[GIT:STAGED]`, `[GIT:UNTRACKED]`), path constraints (`*.ts !tests/ auth`), and frecency weighting.
- `flowrail_find_symbols({ query, definitions_only, git_only, max_results })`:
  Instant typo-tolerant symbol search across all AST nodes using progressive 4-stage matching (Exact -> Acronym -> Substring -> Levenshtein).
- `flowrail_grep({ query, scope, definitions_only, case_sensitive, context_lines, max_matches })`:
  Fast in-memory grep. `scope: "corridor"` restricts search strictly to files in the active session to prevent context leakage; `definitions_only: true` isolates declaration lines (`[DEF]`).

### 3.2 State Machine & Mutation Gating
- `flowrail_open({ entry_symbol, intent, target_mutation_symbol })`:
  Initializes a session. `intent: "READ_FLOW"` produces a top-down execution corridor; `intent: "MUTATE_CONTRACT"` produces a bottom-up reverse topological mutation plan with blast radius calculations. Unlocks `Gate 0`.
- `flowrail_inspect_gate({ gate_index, compact })`:
  Returns line-numbered code slice, contract definitions, and blast radius report. Fails with `VISIBILITY_LOCK_VIOLATION` if the gate is locked.
- `flowrail_propose_edit({ gate_index, new_file_content, acknowledge_blast_radius })`:
  Applies proposed content, hashes exported contracts, runs Tier 1 OXC AST syntax verification, and checks for contagion.
- `flowrail_advance_gate()`:
  Verifies syntax with the Tier 1 OXC parser (or runs incremental `tsc --noEmit` on the active file if `--full-typecheck` is enabled). If contract hash changed and external dependents exist without `acknowledge_blast_radius: true`, halts with `CONTAGION_BARRIER`. If acknowledged, injects external files as new locked gates, locks active gate as `VERIFIED`, and unlocks Gate N+1.
- `flowrail_abort_or_rollback({ to_gate_index })`:
  Rolls back working tree files to checkpoint `to_gate_index` and relocks downstream gates.

---

## 4. Standard Operational Refactoring Protocol

When an agent performs multi-file refactoring on a TypeScript repository, it MUST follow this 6-step loop:

```text
[Step 1: Discover Entrypoint]
   └─► Call flowrail_discover_entrypoints({ query: "<feature_keyword>" })
         Select target EntryGate symbol.

[Step 2: Initialize Corridor]
   └─► Call flowrail_open({
         entry_symbol: "<selected_entry_symbol>",
         intent: "MUTATE_CONTRACT",
         target_mutation_symbol: "<target_interface_or_dto>"
       })
         Review gate roadmap and contagion warning for Gate 0.

[Step 3: Inspect Active Gate]
   └─► Call flowrail_inspect_gate({ gate_index: 0 })
         Study upstream contracts and isolated code slice.

[Step 4: Propose Edit]
   └─► Call flowrail_propose_edit({
         gate_index: 0,
         new_file_content: "<clean_refactored_code>",
         acknowledge_blast_radius: true (if external callers must be updated)
       })

[Step 5: Advance Gate]
   └─► Call flowrail_advance_gate()
         If CONTAGION_BARRIER triggers, acknowledge external blast radius.
         Flowrail unlocks Gate 1 and appends external callers to the end.

[Step 6: Iterate & Complete]
   └─► Repeat Steps 3-5 for Gate 1..N until all gates reach VERIFIED.
         Session automatically completes with zero broken callers.
```

---

## 5. CLI Drivers & Quick Commands

```bash
flowrail doctor             # Verify workspace semantic graph, AST engine, and Git cache
flowrail init               # Scaffold .flowrail/ and generate MCP configs for Cursor/Claude
flowrail demo               # Run live end-to-end deterministic demonstration on sample fixtures
flowrail search "checkout"  # Typo-tolerant symbol discovery with Git status badges
flowrail grep "TaxResult"   # Corridor-scoped or workspace in-memory grep
flowrail slice -e "<sym>"   # Output JSON corridor plan from CLI
flowrail mcp                # Launch stdio MCP server for Claude Code / Cursor / Windsurf
```

## 6. Usecase & Domain References

- **Runbook**: [`usecases/run.md`](usecases/run.md)
- **Technical Specification**: [`references/spec.md`](references/spec.md)
