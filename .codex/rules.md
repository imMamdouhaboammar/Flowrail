# Flowrail Guidelines for OpenAI Codex & Operators

When interacting with codebases equipped with Flowrail:

## 1. Zero Blind Grepping
- Do not perform broad, recursive scans of the workspace.
- Identify the canonical entrypoints using `flowrail_discover_entrypoints`.
- Find exact symbol definitions using `flowrail_find_symbols({ definitions_only: true })`.

## 2. Business Flow Traversal
- Read execution paths end-to-end with `flowrail_open({ entry_symbol: "<symbol>", intent: "READ_FLOW" })`.
- Step through gates with `flowrail_inspect_gate({ gate_index: N })`.

## 3. Contract Mutation & Blast-Radius Protection
- For refactoring or contract mutations, call `flowrail_open({ entry_symbol: "<entry>", intent: "MUTATE_CONTRACT", target_mutation_symbol: "<target>" })`.
- Modify gates in reverse topological order.
- Propose edits via `flowrail_propose_edit` and verify contract deltas before advancing.
- Respect `CONTAGION_BARRIER` triggers when external consumers are impacted.
