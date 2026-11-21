---
name: flowrail-doctor
description: >
  Use when assessing codebase health, diagnosing semantic graph resolution issues, checking Git co-change coupling, or generating clean ADHD/Anti-Slop JSON architecture artifacts using Flowrail.
---

# Flowrail Doctor: System Health & Artifact Governance

Use this skill to inspect workspace readiness, analyze graph connectivity, verify SCIP/TypeScript indexing, and produce clean JSON artifacts.

## CLI & MCP Procedures

### 1. Run Flowrail Doctor
Check workspace status and graph statistics:

```bash
flowrail doctor
```

Output reports:
- Total Semantic Nodes and Dependency Edges
- Git Status Cache (staged, modified, untracked)
- Symbol resolution engine status
- Saves an audit artifact to `.flowrail/artifacts/art-doctor-*.json`

### 2. Export Artifacts
Capture deterministic corridor plans or doctor results:

```typescript
flowrail_export_artifact({
  save_to_disk: true
});
```

Saves to `.flowrail/artifacts/latest.json`.

### 3. AI Enrichment & Anti-Slop Validation
Apply structured summary and action items to the exported artifact:

```typescript
flowrail_enrich_artifact({
  enrichment: {
    adhd_view: {
      headline_action: "Refactor OrderPayload interface in packages/core",
      state_anchor: "ACTIVE_SESSION_GATE_0",
      estimated_duration: "10m",
      bounded_steps: [
        "Inspect packages/core/src/types/order.ts",
        "Add currency field to OrderPayload",
        "Advance gate to update OrderRepository",
        "Update OrderService caller",
        "Verify OrderController handler"
      ],
      visible_wins: [
        "Normalized BLAKE3 contract hash",
        "Zero cross-feature contagion"
      ],
      terminal_action: "Run cargo test and commit changes"
    },
    anti_slop_contract: {
      grounded_symbols: [
        "packages/core/src/types/order.ts::OrderPayload"
      ],
      grounded_files: [
        "packages/core/src/types/order.ts"
      ],
      conversational_filler_count: 0
    },
    human_readable_summary: "Topological refactoring plan for OrderPayload across 4 gates."
  }
});
```

Validation guarantees:
- **Zero Hallucination**: Every symbol and file path is validated against the deterministic graph.
- **ADHD Rule 9**: Steps are bounded to $\le 5$ items.
- **Anti-Slop**: Conversational fluff is rejected.
