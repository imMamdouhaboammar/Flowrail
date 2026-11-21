---
name: flowrail-corridor
description: >
  Use when exploring unfamiliar features, tracing end-to-end execution paths, or discovering entrypoints in TypeScript codebases. Slices code into isolated Business Flow Corridors, pruning utility and logger noise while keeping context focused.
---

# Flowrail Corridor: Business Flow Navigation & Slicing

Use this skill when you need to understand how a feature works end-to-end (e.g. from an HTTP route or controller down to services and repositories) without loading hundreds of irrelevant files into context.

## Workflow

### 1. Discover Entrypoints
Find where the business flow starts using fuzzy search with path constraints and PageRank scoring:

```typescript
// Call MCP tool
flowrail_discover_entrypoints({
  query: "checkout *.ts !tests/",
  definitions_only: true,
  max_results: 10
});
```

Returns entrypoints ranked by graph importance (`pagerank_score`) and annotated with Git status (`[GIT:MODIFIED]`, `[GIT:STAGED]`).

### 2. Locate Symbols
When looking for a specific interface, class, or function:

```typescript
flowrail_find_symbols({
  query: "OrderProcessor",
  definitions_only: true,
  max_results: 15
});
```

Employs 4-stage matching (Exact -> Acronym -> Substring -> Levenshtein) to locate definitions instantly.

### 3. Open a Reading Corridor
Initialize an isolated business flow slice:

```typescript
flowrail_open({
  entry_symbol: "<symbol_id_from_discovery>",
  intent: "READ_FLOW",
  detail_level: "minimal" // or "full"
});
```

Flowrail computes a top-down execution corridor (`Controller -> Service -> Repository`), automatically pruning fan-in logger and telemetry noise ($S_{\text{noise}} > 0.85$).

### 4. Inspect Gates Sequentially
Inspect each gate in the corridor:

```typescript
flowrail_inspect_gate({
  gate_index: 0,
  compact: true // set false to view line-numbered source code
});
```

### 5. Corridor-Scoped Grep
Search for references or keywords strictly within the files of the active corridor:

```typescript
flowrail_grep({
  query: "status",
  scope: "corridor",
  definitions_only: false
});
```
