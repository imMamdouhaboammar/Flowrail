# Flowrail Rules for OpenCode

When using Flowrail in OpenCode:

1. **Autonomous Tool Routing**: OpenCode delegates codebase navigation to `flowrail_discover_entrypoints` and `flowrail_open`.
2. **Deterministic Flow Reading**: Navigate gates using `flowrail_inspect_gate`.
3. **Safe Reverse Topological Mutation**: Never edit multi-file TypeScript in forward order. Use `intent: "MUTATE_CONTRACT"`.
4. **Contagion Barriers**: When `CONTAGION_BARRIER` triggers, acknowledge blast radius or rollback via `flowrail_abort_or_rollback`.
