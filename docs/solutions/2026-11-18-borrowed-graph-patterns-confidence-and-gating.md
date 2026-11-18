# Borrowed Graph Patterns: Edge Confidence, Detail Level Gating & Git Freshness

**Date:** 2026-11-18  
**Status:** Implemented & Verified  
**Methodology:** `/ponytail` (smallest complete change) & `/anti-slop-engineering` (zero bloat, clean contracts)

---

## Context & Motivation

Rather than reinventing graph abstractions, Flowrail adopted four high-leverage architectural patterns from mature production systems (`gsd:graphify`, `code-review-graph`, and `xerj`):

1. **Edge Confidence Tiers (from `gsd:graphify`)**: Discriminate between strictly extracted AST edges and inferred/ambiguous dynamic dependencies to sharpen blast radius calculations.
2. **Context Gating (`detail_level: minimal | full`) (from `code-review-graph`)**: Prevent token explosion in coding agents by providing token-optimized summaries on demand.
3. **Git HEAD Invalidation (from `gsd:graphify`)**: Bind corridor sessions to the exact commit OID at creation time to prevent executing mutations against stale repository states.
4. **Definition-First Quick Location (from `xerj`)**: Provide single-call `file:line` location and definition flags without requiring full AST re-traversals.

---

## Technical Implementation

### 1. Edge Confidence Tiers (`crates/flowrail-core/src/graph.rs` & `slicer.rs`)
- Added `EdgeConfidence` enum: `Extracted` (default), `Inferred`, `Ambiguous`.
- Annotated `SemanticEdge.confidence` with `#[serde(default)]` for zero breaking changes.
- Updated `ExternalDependent` and `BlastRadiusReport` to record `has_ambiguous_dependents`. When ambiguous or dynamic callers are detected, the blast radius flag signals strict gate vigilance.
- In `flowrail-ts-bridge`:
  - SCIP and explicit AST imports generate `EdgeConfidence::Extracted`.
  - Fallback text matches without explicit imports generate `EdgeConfidence::Inferred`.

### 2. Token-Optimized Context Gating (`crates/flowrail-mcp/src/tools.rs`)
- Added `detail_level: "minimal" | "full"` to schemas and handlers for `flowrail_open` / `corridor_open` and `flowrail_inspect_gate` / `corridor_inspect_gate`.
- In `"minimal"` mode:
  - `open`: emits concise gate roadmaps without verbose metadata.
  - `inspect_gate`: returns gate status, upstream contracts, and blast radius without reading raw multi-line file slices into the LLM context.

### 3. Git Freshness Invalidation (`crates/flowrail-core/src/git.rs` & `state.rs`)
- Added `GitStatusResolver::get_head_commit` using `git2` to compute a deterministic composite head fingerprint.
- Supports both standard repositories and repositories with submodules:
  - Discovers root HEAD commit.
  - Recursively enumerates submodules sorted deterministically by path.
  - Attaches each submodule's checked-out workdir/head OID (`parent_commit|submodule_path:submodule_oid`).
  - Detects internal commits within submodules even before they are committed into the parent git tree.
- `CorridorSession` records `built_at_commit: Option<String>`.
- `check_git_freshness(&self)` checks whether the workspace HEAD has advanced.
- `tool_inspect_gate` surfaces `"git_freshness_warning"` if the repository HEAD moved since the corridor was opened.

### 4. Definition-First Quick Location (`crates/flowrail-mcp/src/tools.rs`)
- In `flowrail_find_symbols`, results include `def_location` (`"file:line"`) and `is_definition: bool` to allow instant symbol jumps without re-parsing.

---

## Verification & Compliance

- **Unit & Integration Tests**: Added `test_mcp_borrowed_patterns_confidence_detail_level_and_def_first` in `crates/flowrail-mcp/tests/mcp_test.rs`.
- **Full Suite**: 49 tests passing across all workspace crates with zero failures.
- **Linter**: `cargo clippy` passes with zero warnings.
