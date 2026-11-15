# Evaluation and acceptance

The [machine-readable ledger](requirements.json) tracks requirements and first delivery stages. The acceptance IDs below identify their independent checks. Use unit tests for algorithm behavior and a real index plus compiler and execution for integration claims. Capture commands, exit status, artifact IDs and exact configuration. A fixture expected result stays separate from the produced result.

| Acceptance | Requirement | Evidence required |
| --- | --- | --- |
| AC-01 | FR-01, FR-02 · Index identity and typed evidence | Decode real SCIP. Resolve a known cross-file symbol and prove call vs reference classifications at exact sites. Reject mismatched source, invalid paths and invalid offsets. Cover Unicode, encoding and stale dirty bytes. |
| AC-02 | FR-03, FR-04 · Read projection | Golden invocation returns four required static call edges, scope and meaningful boundaries. Hidden utilities remain available in the underlying evidence. Contracts and monetary logic remain visible. |
| AC-03 | FR-05 · Mutation prerequisites | On the same fixture and a renamed variant, assert every required predecessor before its dependent. Direct plus indirect paths and cycles retain constraints. Reordering insertion and rerunning the command yield the same canonical output. |
| AC-04 | FR-06, FR-07 · Impact and contract decisions | Predict the three golden consumer symbols. Compare baseline, optional addition and breaking split in the real checker. Require exact new break sites, count unique symbols separately and carry unresolved edges as unknown. Formatting and explicit-body-only changes preserve applicable fingerprints. |
| AC-05 | FR-08, FR-09 · Visibility and writes | Verify locked source is absent and out-of-scope diffs do not change bytes. Replays are idempotent. Reject a stale gate/revision or wrong preimage. Verify claimed access boundary through a real stdio client. |
| AC-06 | FR-10 · Acknowledgement and re-plan | Block on unacknowledged enumerated impacts. Accept only the exact proposal, revision and impact set. Introduce each required external gate once while preserving prerequisites. Reject unrelated diagnostics while allowing attributed deferred repair. |
| AC-07 | FR-11 · Completion | Deliberately leave an external compiler error and then a failing integration test. Both keep the session open. A fully repaired snapshot reaches COMPLETED only after fresh project verification. |
| AC-08 | FR-12 · Recovery | Exercise two gates in the same file, dirty preimages, external changes, failed writes and process interruption at each journal transition. Restore exact managed bytes in reverse order or report a conflict. |
| AC-09 | FR-13, FR-14 · Delivery interfaces | Run CLI end to end from a clean checkout, validate JSON and exits, and negotiate MCP over stdio on documented client/OS versions. Confirm stdout and stderr remain separate. |
| AC-10 | FR-15 · Product experiment | Publish task definitions, truth sets, configurations, outcomes, denominators, limitations and raw machine-readable observations. |

## Comparative design

- Use ten independently reviewed multi-file tasks in one or more representative TypeScript repositories of at least 300 source files. Include API shape changes, transitive type uses, compatible changes, shared state effects, an unresolved dynamic path and tasks with no external impact. Keep a held-out setup that the implementation did not target.
- Pair baseline and assisted runs for each task. Reset the same source/dependency snapshot per run. Keep the model version, agent/harness, prompt, time budget, available verifier and stop criteria fixed. The baseline uses ordinary local search, read and edit tools. Record the added Flowrail tools and which host tools are available in assisted runs. Randomize run order and repeat to expose model variance.
- Count failures and timeouts, including tasks where the agent claimed completion early. Report all-task completion and costs, then cost comparisons on mutually successful pairs. Attribute success to final compiler, tests and expected changes, with a review for unnecessary edits. Report blinded scoring where practical.
- Pin the evaluation inputs and metrics. Store project characteristics, comparator, decimal precision, CPU/OS/tool versions, cold/warm policy and sample count. Keep source code and prompt contents local unless authorized. Re-run after material changes to analysis or interfaces.

| Metric | Proposed target and denominator |
| --- | --- |
| Relevant-utility suppression | Original target ≥80% of independently labelled, connected and safe-to-hide symbols. Denominator is the eligible set. Also report false suppressions and block fixture release on any skipped required contract/effect. |
| Agent input-token reduction | Test ≥60% vs paired baseline. Sum measured input and context tokens per run including tool payloads. Record cache treatment separately. Report distribution and completion. Original 60–85% is an ambition, with 70–85% present in an earlier draft. |
| Confirmed external impact | Detect every labelled external break in the golden case and acceptance tasks. Report recall (detected actual breaks / actual breaks) and precision (confirmed detections / all confirmed predictions) with raw counts. Report potential and unknown separately. No repo-wide guarantee inferred from this score. |
| Dependency-order compliance | Zero violations of the frozen prerequisite edge set by tool-managed transitions. Count rework and writes/file as measurements. Host edits outside session are a separate observation when instrumentation is available. |
| Verified completion quality | Every session claiming completion goes green on the declared project compiler and integration checks against its verified bytes. Track pass rate across all attempts and behavior regressions revealed by additional tests. |
| Performance | Historical budget proposals: less than 1.5 s warm ingest + AST enrichment at 100k LOC, less than 30 ms warm slice, less than 10 ms fast gate syntax + signature check, less than 80 MB sustained RSS at 1,000 files. Treat all as experimental until measured. |

Measure index generation separately from ingest/enrichment, warm query, AST-only comparison, full TypeScript diagnostics and integration execution. Measure p50/p95, peak RSS, document sizes and coverage. A fast cached query is independent of compiler or index startup. Record regressions on a fixed reference machine before deciding release thresholds. The repository-specific example using a 111 KB index establishes artifact production only.
