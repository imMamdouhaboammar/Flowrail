---
name: flowrail-mutation
description: >
  Use when planning or executing multi-file refactoring, interface changes, or contract migrations in TypeScript. Enforces reverse topological order, contract signature BLAKE3 hashing, blast-radius fencing, and gate rollback protection.
---

# Flowrail Mutation: Safe Multi-File Refactoring Protocol

Use this skill whenever code changes cross file boundaries or modify exported interfaces, types, or function signatures. Flowrail prevents erratic multi-file hopping, broken callers, and out-of-order edits.

## The 6-Step Mutation Loop

### Step 1: Initialize Mutation Corridor
Determine the entry gate and the target contract to mutate:

```typescript
flowrail_open({
  entry_symbol: "packages/api/src/routes/order.ts::handleCheckout",
  intent: "MUTATE_CONTRACT",
  target_mutation_symbol: "packages/core/src/types/order.ts::OrderPayload"
});
```

Flowrail computes a bottom-up reverse topological dependency plan:
`Contract / DTO (Gate 0) -> Repository (Gate 1) -> Service (Gate 2) -> Controller (Gate 3)`

Gate 0 is UNLOCKED; all subsequent gates are LOCKED.

### Step 2: Inspect Active Unlocked Gate
Examine the isolated gate slice, upstream contracts, and blast radius:

```typescript
flowrail_inspect_gate({
  gate_index: 0,
  compact: false
});
```

### Step 3: Propose Edits
Apply modifications to the unlocked file:

```typescript
flowrail_propose_edit({
  gate_index: 0,
  new_file_content: "// updated file content with refactored interface\n...",
  acknowledge_blast_radius: false
});
```

Flowrail verifies:
- Syntax check via Tier 1 OXC parser.
- Contract BLAKE3 hash diff detection.
- Delta classification: `Cosmetic`, `CompatibleAdditive`, or `Breaking`.

### Step 4: Handle Contagion Barrier & Advance Gate
Attempt to advance to the next gate:

```typescript
flowrail_advance_gate();
```

- **If no breaking external changes exist**: Gate 0 is locked as `VERIFIED`, and Gate 1 is unlocked.
- **If external callers outside the corridor break**: Halts with `CONTAGION_BARRIER`.
  - Review the reported external callers.
  - If intentional, acknowledge blast radius:
    ```typescript
    flowrail_propose_edit({
      gate_index: 0,
      new_file_content: "...",
      acknowledge_blast_radius: true
    });
    flowrail_advance_gate();
    ```
  - Flowrail automatically queues the external callers as new gates in the mutation plan.

### Step 5: Advance Through All Gates
Repeat Steps 2 to 4 sequentially for each gate until all gates are VERIFIED.

### Step 6: Rollback on Failure
If any gate produces unresolvable syntax, type, or test errors, immediately restore the workspace:

```typescript
flowrail_abort_or_rollback({
  to_gate_index: 0 // restores workspace to pristine state before mutation
});
```
