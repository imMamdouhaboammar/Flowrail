# Risks and decisions

## Decision record

| Decision | Direction and trade-off | Revisit when |
| --- | --- | --- |
| D-01 · Identity | Owner chose Flowrail commercially and Ariadne internally. Keep current `flowrail-*` crate names. Earlier `corridor_*` examples are historical. | An external compatibility commitment requires aliases. |
| D-02 · Host and target | Keep the Rust workspace for graph, CLI and session work. TypeScript is the first supported target. A TypeScript-only tool would simplify compiler integration. A Go host would replace existing work without removing the need for TypeScript semantic tooling. | A small proof finds the Rust/TypeScript bridge costs more than it saves on actual supported projects. |
| D-03 · Index evidence | Adopt SCIP plus AST join for the initial proof. Use the checker for distinctions SCIP cannot establish. A raw parser or flat text search is useful for discovery; neither establishes cross-file identity by itself. | Pinned indexer misses necessary evidence and a checker-backed extractor demonstrably closes the gap. |
| D-04 · Read versus edit | Share one typed evidence graph. Derive two distinct views with different edge directions and constraints. Keep presentation pruning outside impact traversal. | Shared model prevents an evidence type that the target workflow genuinely needs. |
| D-05 · Storage | Start with in-memory analysis and versioned local artifacts. Mutation adds a durable per-session journal. Earlier SQLite/WAL and Git stash concepts are deferred. | Artifact size, recovery needs or measured I/O cost justifies an embedded store. |
| D-06 · Runtime | Keep runtime tracing as an optional later overlay. Avoid requiring instrumentation for the first TypeScript result. | A supported workflow is materially blocked by unresolved dispatch and has representative instrumented tests. |
| D-07 · Session authority | Use patched candidate bytes, scoped writes, final compiler/tests and exact rollback. Start read-only, then test the managed write path on an isolated workspace. | The target host can supply stronger filesystem controls or requires coordinated multi-file gates. |
| D-08 · Verification scope | Require a clean baseline for verified managed completion. Name selected projects and checks. A reported partial plan can still be useful in read-only mode. | Existing repository debt materially blocks adoption and a sound diagnostic comparison can be independently verified. |
| D-09 · Utility judgment | Preserve semantics on uncertain cases. Start with explicit opt-in metadata and conservative role rules. Treat fan-in as a signal. | Independent labeled evaluations justify more automatic suppression without lost critical evidence. |
| D-10 · Delivery order | Prove the read-only fixture and transitive impact ahead of MCP. Package complexity, additional targets and assisted editing depend on this proof. | Evidence shows the core product question is answered and the next bottleneck is integration. |

## Failure modes

| Risk | What breaks | Required response and proof |
| --- | --- | --- |
| SCIP occurrence looks like a call | Type imports and reference-only uses inflate the flow | Join with enclosing syntax and symbol identity. Prove call and reference at separate sites with a real index. |
| Dynamic dispatch, DI, events or middleware | Static paths can omit a critical runtime edge | Show unresolved boundary. Add an adapter or trace only for a named, validated case. Track skipped scope. |
| Shortest path substitutes for topology | A direct consumer can be released before its producer or nested contract | Construct prerequisite edges independently. Use the fixture's direct-plus-indirect case and a renamed variant. |
| Full graph treated as a DAG | Recursion yields false sequential gates | Condense relevant cycles and represent coordinated groups. Verify cyclic fixtures and stable order. |
| File-level utility pruning | Monetary math, state or contract dependencies disappear | Suppress presentation at symbol level with reasons. Preserve graph connectivity, effects and contracts. |
| Inferred export, aliases or declaration merging | Header hash misses real API changes | Version canonical forms and report partial coverage. Exercise checker-derived signatures, re-export and body-change cases. |
| Changed hash equals breakage | Backward-compatible changes trigger avoidable expansion | Use pre/post diagnostic evidence and maintain potential vs confirmed vs checked clean. |
| Broken intermediate sources | The verifier would stall at the first intended break, or later code could hide it | Attribute temporary failures to named repair obligations tied to baseline and candidate. Final clean scope is required. |
| A stale or partial index looks current | Wrong source spans, skipped consumers or unreliable edits | Bind bytes, index, options and scope. Refuse managed writes on unverified provenance. |
| Multiple gates touch one file | Incorrect rollback order restores intermediate content | Store preimages by ordered transaction. Restore with digests in reverse order and test conflicts. |
| Out-of-band edits, path escapes, crashes | Unrelated code is overwritten or session state diverges | Isolate writer, validate path and scope before writing, journal, recheck bytes and exercise interruption. |
| Prompt or command injection in repository content | A source comment or config-like string misdirects the agent | Treat repository data as evidence. Flowrail executes only configured checks in the chosen workspace. Keep content out of autonomous command construction. |
| External readers bypass MCP | Locked source is visible through host tools | State the trust boundary. Demonstrate MCP behavior and use host isolation only when stronger enforcement is required. |
| Overfitting to the golden file | The demo passes while ordinary projects fail | Use renamed fixtures, held-out tasks, real indexes and independent compiler/tests. Do not fall back to fabricated edges to rescue integration results. |
| Performance claims based on toy input | Users expect sub-50 ms operations that include compiler work | Report cold and warm operations separately on real repositories, including memory and coverage. |
| Ambiguous baseline checkout | Developers use an old JSON or nested clone with divergent expectations | Pin commit and fixture-relative path. Normalize repository layout in a separate authorized change. |
| Prototype used as a write API | Path and authority checks can run after disk mutation, or stale verification can complete | Keep integration read-only until write sequencing and final evidence pass AC-05 through AC-08. |

### Open questions

- Which first held-out TypeScript codebase and tasks represent the user's actual agent workflow?
- What level of automatic entry discovery is needed after explicit symbol selection works? Which frameworks justify an adapter?
- When an initial implementation can verify only a single tsconfig, which real workspace layouts are intentionally supported in the pilot?
- What acknowledgement UX and actor authority does each MCP host support?
- Should the final package ship a managed indexer, invoke an explicitly configured local indexer, or consume only prebuilt indexes? Decide after validating installation and supported tool versions.
- Which contract shapes and runtime behavior are critical enough for V1 to block completion when their coverage is unresolved?
