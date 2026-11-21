# Flowrail Rules & Directives for Claude Code

When working in this codebase or with the Flowrail MCP server:

## 1. Zero Blind Crawling & Context Hygiene
- Never run unconstrained recursive directory traversals or multi-thousand-line greps.
- Discover entrypoints via `flowrail_discover_entrypoints` using PageRank centrality.
- Locate declarations with `flowrail_find_symbols({ definitions_only: true })`.
- Restrict in-memory searches with `flowrail_grep({ scope: "corridor" })`.

## 2. Reading Corridors (`READ_FLOW`)
- Open flow corridors with `flowrail_open({ entry_symbol: "<symbol>", intent: "READ_FLOW" })`.
- Inspect gates sequentially: `flowrail_inspect_gate({ gate_index: N })`.
- Respect `co_changed_files_warning` for non-import Git co-change coupling.

## 3. Safe Multi-File Mutation (`MUTATE_CONTRACT`)
- Never modify multi-file dependencies in forward execution order.
- Initialize a mutation corridor with `flowrail_open({ entry_symbol: "<entry>", intent: "MUTATE_CONTRACT", target_mutation_symbol: "<target>" })`.
- Follow reverse topological ordering (Contract -> Repository -> Service -> Controller).
- Advance gates only after verifying contract deltas (`Cosmetic`, `CompatibleAdditive`, `Breaking`).
- If a `CONTAGION_BARRIER` triggers, acknowledge blast radius or rollback.
