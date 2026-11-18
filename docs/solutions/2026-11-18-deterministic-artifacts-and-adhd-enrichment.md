# Solution: Deterministic Artifacts with Grounded AI Enrichment (ADHD & Anti-Slop Discipline)

## Context
Flowrail operates as a deterministic state machine and codebase slicing engine. Previously, session inspection and plan execution outputs were serialized directly as internal graph models without a standardized, human-readable, and AI-enrichable artifact lifecycle.

## Problem Statement
When human developers and AI agents collaborate on corridor slicing and mutation:
1. Raw internal graph dumps overload working memory.
2. AI agents frequently hallucinate ungrounded files or symbols when summarizing codebases.
3. Readers with ADHD require structured cognitive formatting: leading with the next action, bounded multi-step paths (≤5 items), visible wins, concrete time estimates, and zero conversational pleasantries.

## Architectural Solution

### 1. Two-Tier Artifact Architecture (`FlowrailArtifact`)
- **Tier 1: Deterministic Engine Ground Truth:** Produced exclusively by Flowrail's Rust core (`flowrail-core::artifact`). Contains verified gate nodes, topological ordering, signature hashes, contagion index, external dependents, git status, and compiler diagnostics. Zero-hallucination baseline.
- **Tier 2: AI Enrichment Container (`AiEnrichment`):** Structured slots for human-readable translation governed by strict machine-checked invariants:
  - `headline_action`: Rule 1 of `/i-have-adhd` (Lead with the next action).
  - `state_anchor`: Rule 5 of `/i-have-adhd` (Restate state every turn).
  - `estimated_duration`: Rule 6 of `/i-have-adhd` (Concrete time estimates).
  - `bounded_steps`: Rules 2 & 9 of `/i-have-adhd` (Numbered tasks capped strictly at ≤5 items).
  - `visible_wins`: Rule 7 of `/i-have-adhd` (Show what now works/compiles).
  - `terminal_action`: Rule 3 of `/i-have-adhd` (Single action doable in <2 minutes).
  - `error_diagnostic`: Rule 8 of `/i-have-adhd` (Matter-of-fact `{ cause, fix }` without conversational hedging).

### 2. Anti-Hallucination & Anti-Slop Enforcement Gate
The deterministic engine exposes `apply_and_validate_enrichment`:
- **Symbol & File Allowlist:** Any symbol or file path cited in `grounded_symbols` or `grounded_files` must physically exist in the deterministic payload. Hallucinated tokens trigger immediate rejection (`HALLUCINATED_SYMBOL_REJECTED`, `HALLUCINATED_FILE_REJECTED`).
- **Cap Enforcement:** Bounded steps and visible wins exceeding 5 items are rejected (`ADHD_VIOLATION_RULE_9`).
- **Conversational Fluff Elimination:** Rejects conversational filler ("great question", "hope this helps", "let me know", "game-changer", etc.) upholding the Direct Payload Invariant.

### 3. Surface Integrations
- **CLI Subcommand:** `flowrail artifact show`, `flowrail artifact list`, `flowrail artifact generate`, and `flowrail artifact validate --file <path>`.
- **CLI Flag:** `flowrail slice --entry <symbol> --artifact` automatically persists to `.flowrail/artifacts/latest.json`.
- **MCP Server Tools:** `corridor_export_artifact` / `flowrail_export_artifact` and `corridor_enrich_artifact` / `flowrail_enrich_artifact`.
- **Automatic Lifecycle Snapshotting:** Every `corridor_open`, `corridor_propose_edit`, `corridor_advance_gate`, and `corridor_abort_or_rollback` updates `.flowrail/artifacts/latest.json`.

## Verification Evidence
- `crates/flowrail-core/tests/artifact_test.rs`: 9 unit tests verifying plan/session/doctor artifact generation, deterministic baseline generation, disk persistence, and rejection of hallucinated symbols, hallucinated files, conversational fluff, and >5 items.
- `crates/flowrail-mcp/tests/mcp_test.rs`: Integration test verifying `corridor_export_artifact` and `corridor_enrich_artifact` roundtrip.
- `crates/flowrail-cli/tests/cli_test.rs`: CLI test verifying `slice --artifact`, `artifact show`, `artifact list`, and `artifact validate`.
