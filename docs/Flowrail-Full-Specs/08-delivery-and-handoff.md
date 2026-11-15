# Delivery and handoff

| Milestone | Deliverable | Dependencies and exit check |
| --- | --- | --- |
| M0 · Fixture | Executable source, pinned dependencies, compiler oracle and baseline SCIP artifact | Checkout tests run, three expected external consumers confirmed with four diagnostics, optional change clean. Verified at the fixture commit. |
| M1 · Read-only proof | CLI imports real SCIP, enriches AST, shows read and mutation plans with evidence | M0. Pass AC-01 to AC-03 and the impact projection portion of AC-04. No source changes required to index and slice. |
| M2 · Contract and impact | Fingerprints, scope and compatibility evidence, baseline-relative diagnostics with explanations | M1. Pass complete AC-04. Demonstrate on the golden fixture and one held-out TypeScript project before designing further adapters. |
| M3 · Managed mutation | Six stdio tools, staged patches, recovery and final checks | M2. Pass AC-05 through AC-09. CLI portions of AC-09 start in M1. Test terminal failure and restart, not only the happy path. |
| M4 · Pilot | Two documented client setups and comparative evaluation | M3. Publish AC-10 results and exact tested scope. User decides whether measured results justify release and next language work. |

## First implementation unit

**Goal:** Starting from a golden checkout containing `1dbbacc`, run one CLI proof against a real SCIP artifact and verify prerequisite, relationship and impact evidence. Work in an isolated checkout that includes the relevant current core changes deliberately; the fixture branch contains the fixture commit and does not automatically carry uncommitted main-checkout code.

1. Reproduce M0. Confirm the known-good fixture and which sources are included in the TypeScript project. Build the first negative check with a real index reference that must not be classified as a call.
2. Extend the existing SCIP loader and AST slicer in the TypeScript bridge to join ranges and identities. Attach exact evidence for the four golden call edges. Surface missing and unsupported evidence explicitly. Verify the currency and hidden-utility cases without path-dependent core logic.
3. Correct the mutation ordering to preserve prerequisites, independent of shortest distance. Compute external impact from the full typed graph before view pruning. Verify known dependency paths and deduplicated counts.
4. Provide minimal CLI index import and slice commands. Validate both public examples against the contract, comparing invariant sets and edge order requirements. Compare stable output across two equivalent runs and a relocated or renamed test project.
5. Record exact revision, workspace source digest, indexer/compiler versions, commands, passed assertions and scope gaps. Cross-review ordering, edge evidence and claims against the independent fixture. Move to M2 when the evidence meets its prerequisites.

### Release evidence and working rules

- Target one behavior at a time. Start with an observable failing acceptance case, implement the smallest coherent change and rerun the focused check before the broader suite.
- Reuse the existing crate boundaries. Avoid compatibility aliases, databases, runtime tracing and extra frameworks until requirements justify their cost.
- Record current implementation, failures, recovery and test commands. A review comment, green unit test over a synthetic graph or a schema-valid response does not prove the CLI works against real TypeScript.
- Preserve current workspace changes. Keep generated `.flowrail` and dependency artifacts out of source commits. Before publishing, inspect the staged files and reference exact revisions. Any source or PR review service that needs repository upload requires an authorized target and permitted scope.
- Require reproducible release artifacts, locally reviewable failure output, documented supported configurations and real client verification. Keep version-specific setup in the release docs after testing. No deadline is inferred from the chat's early week estimates.
