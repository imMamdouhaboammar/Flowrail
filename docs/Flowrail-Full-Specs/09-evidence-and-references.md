# Evidence and references

## Source record

| Evidence | What it establishes |
| --- | --- |
| Owner's conversation, `chat-with-7alawa.txt`, lines 1–55 | Idea begins with large-scale refactoring and agent navigation through business logic. Source of the Rust-versus-Go and first-target discussion. |
| Same conversation, lines 57–292 and 294–563 | Initial Corridor architecture, algorithms, tool surface, options and draft scaffolding. These were design proposals. |
| Same conversation, lines 564–866 | Request for a charter with problem, idea, goal and success criteria, followed by the explicitly requested English Markdown specification. Source of original targets and six-tool flow. |
| Same conversation, lines 867–1127 | Owner chose Ariadne and Flowrail names and requested the ecommerce fixture. The proposal contains an older benchmark, superseded for reproduction by the executable fixture commit. |
| Main checkout, `Specs.md` and `docs/superpowers/plans/2026-11-15-flowrail-ariadne-core-engine.md` | Existing charter and implementation sequence. The charter retains the old working name. The plan proposes graph, contract, pruning, slicer, state, bridge, verification, MCP and CLI work. |
| Local checkout inspection, 15 November 2026 | The original workspace had uncommitted Rust changes. The standalone ecommerce fixture was exported from local commit `1dbbacc`. Use the dated viability assessment for the results of the later CLI checks. |
| Fresh Git export of the fixture commit and local execution | Bun 1.4.4 and Node v26.13.0. Frozen install, typecheck, 3 checkout tests, breaking and compatible compiler oracle and SCIP indexing all exited successfully. Generated index size was 111,254 bytes. The earlier observed size of 111,284 bytes came from a different directory. SCIP metadata can encode a project root, so file size and raw bytes are not the determinism criterion. |

The implementation inventory reflects observations made during a changing working tree. This packet contains an exact copy of the committed standalone fixture. The full conversation and the uncommitted Rust implementation remain at their source. The architecture, versioned schemas and roadmap in this packet are proposals.

## Static review of the implementation at inspection

| Location | Observed behavior | Acceptance work |
| --- | --- | --- |
| `crates/flowrail-core/src/slicer.rs:233–250` | Reverse traversal records shortest distance and sorts by distance alone. Tied order is drawn from a HashMap. | AC-03. Use stable prerequisite order and assert it with the real fixture graph. |
| `crates/flowrail-core/src/slicer.rs:106–122` | Suppression is applied during forward traversal and a suppressed node is not queued. | AC-02. Separate semantic reachability from presentation suppression. |
| `crates/flowrail-ts-bridge/src/scip_loader.rs:101–136` | For mapped references, fallback edge type is Calls after the role and write checks. Enclosing owner is selected by line containment in this branch. | AC-01. Confirm callee syntax and owner span. Preserve references and unresolved edges with separate kinds. |
| `crates/flowrail-core/src/state.rs:129–168` | Proposal writes to the joined path and then records the proposal; its method supplies no compiler diagnostic before that write. | AC-05/08. Stage edits and enforce gate/path/revision/diagnostic checks before replacing managed bytes. |
| `crates/flowrail-core/src/state.rs:247–257` | Stepping past the last gate sets Completed. | AC-07. Bind final verification to the actual project and snapshot. |
| `crates/flowrail-core/src/state.rs:261–303` | Rollback writes checkpoint content and persistence uses a direct file write. | AC-08. Verify deterministic reverse restore, conflict handling and crash recovery. |

These observations establish gaps in the examined paths. End-to-end Rust, CLI and MCP behavior remains unverified in this delivery. The Markdown, examples and schemas document the target contracts. CodeRabbit CLI availability was checked, but its status command failed with ENOTDIR before any external review. Public documentation research used Context7 and direct source search when the Parallel endpoint returned rate limiting. No PR or external review claim is included.

## Technical references

- [SCIP protocol definition](https://github.com/scip-code/scip/blob/main/scip.proto). Position encodings, definition and reference roles, relationship messages, ranges and project metadata.
- [scip-typescript documentation](https://github.com/sourcegraph/scip-typescript). Local TypeScript project indexing. Pin and verify the actual supported indexer version and project format.
- [TypeScript Compiler API](https://github.com/microsoft/TypeScript/wiki/Using-the-Compiler-API). Program, checker and diagnostics. The fixture’s compiler API oracle uses a host overlay.
- [OXC parser](https://oxc.rs/docs/guide/usage/parser). Rust TypeScript AST parsing and source spans. Semantic classification and compiler checks require explicit supporting evidence.
- [petgraph algorithms](https://docs.rs/petgraph/latest/petgraph/algo/index.html). Strongly connected components and topological ordering primitives.
- [MCP specifications](https://modelcontextprotocol.io/specification/). Tools, messages, transport and protocol version negotiation.
- [OpenRewrite recipes](https://docs.openrewrite.org/concepts-and-explanations/recipes) and [Lossless Semantic Trees](https://docs.openrewrite.org/concepts-and-explanations/lossless-semantic-trees). Primary sources for the starting point of the product discussion.
- [OpenTelemetry tracing concepts](https://opentelemetry.io/docs/concepts/signals/traces/). Optional later source for evidence from instrumented workloads.
