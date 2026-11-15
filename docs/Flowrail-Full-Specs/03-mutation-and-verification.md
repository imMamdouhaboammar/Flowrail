# Mutation and verification

## Contract comparison (FR-07)

A contract is an exported compile-time shape or a configured boundary with explicit extraction support. Start with interfaces, type aliases and explicitly typed callable signatures. Capture qualified export names, kinds, members, modifiers, optionality, generic constraints and defaults, overload sets, parameter and return types, bases and re-exports as supported. Encode versioned structured data and hash its canonical bytes with BLAKE3. Preserve ordering wherever it can alter behavior, such as overload resolution or tuple elements. Remove only irrelevant formatting and comments. Compare per symbol, with a file-level aggregate for drift detection. A changed public member yields a field-level diff. A formatting edit should preserve the fingerprint.

An implementation-only edit to an explicitly annotated function should preserve its signature fingerprint. Body changes may alter an inferred export, computed type, observable behavior or a runtime schema. Use checker evidence for inferred or imported changes that affect the effective surface, or mark coverage unknown. Runtime validation schemas, decorators, schema migrations and compatibility across previously persisted values require dedicated extractors and tests. Never convert an unchanged hash into a claim that behavior was preserved.

### Impact states

| State | Evidence | Verification action |
| --- | --- | --- |
| Potential | Typed dependency or use site on the baseline snapshot | Inspect sites and scope. A changed fingerprint alone belongs here. |
| Confirmed breaking | New relevant compiler diagnostic or a failing registered test tied to this change | Record obligation, exact site and check identity. |
| Checked clean | Scoped relevant checks passed on the candidate snapshot | Record what was checked, including skipped areas. |
| Unknown | Index gap, unresolved dispatch, unsupported surface, stale evidence or verifier failure | Preserve the gap and resolve it for verified completion if safety depends on it. |

The comparison is relative to a pinned baseline. A repository already failing verification can still be analyzed in read-only mode. For V1 managed completion, require a known green baseline in the configured project scope and a green final check. Baseline-relative completion for existing debt is a possible later extension requiring an explicit policy.

## State model (FR-08 to FR-12)

A session owns a workspace scope and a sequence of stable gate IDs. The current plan has a revision. Gate states are LOCKED, UNLOCKED and VERIFIED. VERIFIED means the gate was processed and its transition checks passed, with any provisional repair obligations recorded separately. Session phases are ACTIVE, BLOCKED, VERIFYING, COMPLETED, ABORTED and RECOVERY_REQUIRED. Use reason codes for blocked phases. Mutating requests carry expected session revision and request ID. Replay returns the earlier outcome only if the request contents match. A stale revision requests a refresh.

| State and event | Preconditions | Observable result |
| --- | --- | --- |
| Open | Resolved inputs, supported scope and a current snapshot. Write session ownership acquired for mutation. | ACTIVE with at most one unlocked gate. Read-only sessions serve navigation and reject writes. |
| Inspect | Gate is unlocked or already verified. Source digest and span match. | Render contextual slices and evidence. Locked gates expose roadmap metadata only. |
| Propose edit | Active mutation gate, matching revision and preimage, permitted file and hunks. | Stage candidate, recompute fingerprint and diagnostics. Rejected syntax or ownership leaves original workspace bytes intact. |
| Advance with outstanding external impact | Candidate validated, plan and impact statement bound to current revision. | BLOCKED pending compatibility evidence or acknowledgement of enumerated repair work. |
| Accept acknowledged impact | Acknowledgement matches proposal, snapshot and impact revision. Obligations fit supported scope. | Commit the managed patch and checkpoint together, add deduplicated gates and unlock next actionable gate. |
| Advance with review only | Required review reason and evidence recorded. | Verify step without a source edit. |
| Advance last gate | Gate-local checks pass. | VERIFYING. Complete only after fresh project checks, completed obligations and verified final bytes. |
| Final verification fails | New errors, failed checks, stale files or required coverage gap. | BLOCKED with actionable evidence and a repair/resume path. |
| Rollback or abort | Revision and file preimages match session records. | Restore affected files to checkpoint or pre-session bytes, clear later evidence, relock future gates or mark ABORTED. |
| Restart after interrupted write | A transaction journal exists. | Finish or restore according to verified pre/post digests, or enter RECOVERY_REQUIRED with a precise conflict. |

### Staged verification and compiler debt

- Parse candidate AST and validate allowed scope before writing. Derive hashes and check results on the server side from candidate bytes and tool outputs.
- Run the target's configured TypeScript project check. A checked type error in the active code blocks advancement unless it is explicitly attributable to a later repair obligation supported by the plan. External impact can be confirmed after comparing baseline and candidate diagnostics.
- Breaking an interface intentionally can create errors at both producer and consumers. Record each accepted temporary obligation against specific evidence and a future gate. Avoid a blanket allowance for compiler errors. If the verifier cannot attribute the failures, block and show the limitation. Refresh semantic facts once possible.
- The final verification reruns a full project typecheck and the declared integration commands on the resulting snapshot. Compare final bytes with the input verified by each check. A timeout, canceled subprocess, stale verification or nonzero exit blocks completion. A typecheck proves only the selected TypeScript scope. A test proves the exercised behavior.
- When a supposedly compatible change creates no new checked breaks, record compatibility evidence with compiler scope. Distinguish that from waiving external changes. Known unresolved relevant cases remain open.

## Ownership, persistence and recovery

The local stdio server can restrict what its tools show and which patches those tools apply. Filesystem tools granted by a separate host can bypass Flowrail. Use an isolated checkout and host-level access controls if host enforcement is required. Treat repository text, comments, tool output and indexes as untrusted inputs to the workflow.

Use one managed writer per workspace. Validate paths against the session root and gate, reject traversal and symlink escapes, and recheck identity before mutation. A declaration edit often requires an import hunk. Bind edits to the active declaration plus explicitly permitted same-file import hunks. Multiple declarations in one file must share the latest file revision. Treat unexpected changes made by an IDE or another process as conflicts. A digest compare cannot eliminate a race with an uncooperative external writer. Managed mutation requires an isolated or otherwise controlled checkout.

Use a local per-session ledger and bounded checkpoint data. Record pre- and postimage digests, source scope, baseline and current snapshot IDs, plan revisions, proposed and committed edits, diagnostic IDs, rollback state and verification outcomes. Restrict state permissions to the local user where supported. Provide documented cleanup and defect-report redaction. Keep source text out of telemetry by default. Do not use repository text as a command. Run only user-configured checks with bounded process time and captured outcomes.

Write a durable transaction marker before replacing managed files. Write temporary files in the same filesystem and rename atomically where supported. Journal enough to recover if a process dies between source replacement and session persistence, including when a later operation must undo several changes to the same file. Restore in reverse transaction order, with digest checks. A pre-existing dirty file's true preimage is the saved local bytes. Never use broad reset, checkout, stash or clean as rollback. When recovery detects an external edit, present known hashes, affected path and safe manual resolution. Preserve independent work.
