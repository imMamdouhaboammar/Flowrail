# Flowrail Rules for OpenCode

<!-- Include shared Flowrail directives -->
See `rules/AGENTS.md` for complete deterministic execution & mutation state-machine specifications.

## Quick Reference
- **Entrypoints**: `flowrail_discover_entrypoints`
- **Symbols**: `flowrail_find_symbols`
- **Flow Reading**: `flowrail_open` (intent: `READ_FLOW`), `flowrail_inspect_gate`
- **Safe Mutation**: `flowrail_open` (intent: `MUTATE_CONTRACT`), `flowrail_propose_edit`, `flowrail_advance_gate`
- **Rollback**: `flowrail_abort_or_rollback`
- **Health**: `flowrail doctor`
