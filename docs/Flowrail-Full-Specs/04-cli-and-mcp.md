# CLI and MCP contracts

All shapes here are proposed v1 contracts. The CLI and MCP crates showed scaffold behavior at inspection. Use explicit paths and JSON for automation, and an optional compact Markdown renderer for humans. The CLI and MCP tools share validation, planning and verification behavior.

## CLI

| Command | Meaning |
| --- | --- |
| `flowrail index --workspace <dir> --project <tsconfig> [--scip <path>]` | Build a snapshot from a provided SCIP file or generate one with the configured indexer. Return scope, digest, indexer metadata and coverage. |
| `flowrail slice --workspace <dir> --entry <symbol> --intent READ_FLOW` | Resolve the entry and emit a read plan from the active indexed snapshot. |
| `flowrail slice --workspace <dir> --entry <symbol> --intent MUTATE_CONTRACT --target <symbol>` | Emit a read-only mutation plan. A plan request never edits source. |
| `flowrail mcp --workspace <dir>` | Serve the same indexed workspace and session operations over stdio. |

Use one machine-readable result per request with an explicit schema version. Emit JSON on stdout and human logs on stderr. Reserve exit 0 for a returned successful command result (which can carry incomplete coverage), 2 for invalid usage, 3 for missing/stale input or missing toolchain, 4 for analysis failure and 1 for other internal failure. Keep useful suggestions in errors. Stable sort candidate lists and evidence. Update output format only with a compatible schema revision or an explicit major version.

## MCP tools

Proposed canonical names use the Flowrail prefix. The six-tool count refers to these product operations. MCP protocol setup and capability discovery remain protocol concerns. Tool responses expose structured content and a compact text representation as needed by the selected client and protocol version. MCP stdout is reserved for protocol messages. See [MCP specifications](https://modelcontextprotocol.io/specification/).

| Tool | Required inputs | Success and side effects |
| --- | --- | --- |
| `flowrail_discover_entrypoints` | None. Optional query, path prefix, cursor and limit. | Bounded entries with stable IDs, source anchors, kind and evidence. |
| `flowrail_open` | `entry`, `intent`. `target` required for mutation. | Session ID, revision, snapshot and roadmap. Claims mutation ownership where applicable. |
| `flowrail_inspect_gate` | `session_id`, `gate_id` | Exact source excerpt for authorized gate, contract context, prerequisites, evidence, impact and current revision. |
| `flowrail_propose_edit` | `session_id`, `gate_id`, `expected_revision`, `request_id`, `file_path`, `preimage_digest`, `patch` | Stages candidate and returns proposal ID, surface diff, diagnostics and impact revision. V1 accepts one validated file patch at a time. |
| `flowrail_advance_gate` | `session_id`, `gate_id`, `expected_revision`, `request_id`. Proposal ID for edit, review disposition for review-only. | Applies accepted candidate, updates gates and returns next action or final evidence. Optional `acknowledgement: { proposal_id, impact_revision }` accepts the enumerated obligation set. The server resolves the set from the stored revision and rejects stale acknowledgement. |
| `flowrail_rollback` | `session_id`, `mode` (ROLLBACK or ABORT), `expected_revision`, `request_id`. ROLLBACK also needs a checkpoint ID. | Restores managed bytes, invalidates later evidence and returns revised session or conflict. |

The plan schema describes read-only CLI results. The session result schema covers advance and rollback transitions and illustrates the application error shape. Other MCP responses carry the fields in the tool table. Gate and session IDs are opaque. Revisions advance monotonically for changes, including any durably recorded blocked transition. Duplicate calls and client disconnection resolve against durable state. For transport-level parse or method errors follow JSON-RPC/MCP conventions. For valid tool invocations with application errors return a structured tool failure with `isError` as supported and the applicable fields from the [session result schema](schemas/session-result.schema.json). Never put candidate source text in error or audit logs unless explicitly requested and scoped.

| Error code | Trigger | Retry guidance |
| --- | --- | --- |
| SYMBOL_NOT_FOUND / SYMBOL_AMBIGUOUS | No exact symbol or multiple matches | Supply a qualified ID from discovery or candidate list. |
| TARGET_OUTSIDE_CORRIDOR | No supported relationship from entry to target | Check target, scope and indexed project. |
| INDEX_STALE / INDEX_INCOMPLETE | Snapshot mismatch or missing required evidence | Refresh a valid index, expand scope or inspect gaps. |
| VISIBILITY_LOCK_VIOLATION | Future gate source requested | Inspect the active gate or its roadmap summary. |
| INVALID_GATE / WRONG_MODE | Gate mismatch or write sent to a read session | Refresh and act on the active gate of a mutation session. |
| REVISION_CONFLICT / PREIMAGE_MISMATCH | Plan advanced or workspace drifted | Reinspect and rebase the proposal on current workspace evidence. |
| PATCH_OUT_OF_SCOPE / PATH_OUTSIDE_WORKSPACE | Hunk, file or path is outside the authorized gate | Use the allowed declaration and import ranges. |
| SYNTAX_CHECK_FAILED / TYPE_CHECK_FAILED | Compiler evidence blocks progression | Repair the listed sites or reduce the scope of the proposed change. |
| CONTAGION_BARRIER | Impact obligation needs a decision | Submit a compatible proposal or acknowledge the current enumerated obligations. |
| FINAL_VERIFICATION_FAILED / VERIFIER_TIMEOUT | Final checks failed or did not finish | Resolve the failure and retry on the same or refreshed snapshot. |
| WORKSPACE_BUSY / ROLLBACK_CONFLICT / RECOVERY_REQUIRED | Writer contention, changed path or interrupted checkpoint | Reinspect ownership and restore only after resolving the conflict. |

### Compatibility and distribution

Start with a Rust executable and local state under a configured `.flowrail/` directory. SCIP generation and compilation are explicit toolchain dependencies with version metadata. Keep index and managed state out of source commits by default. Separate index artifact format from CLI/MCP plan serialization. For a crashed or disconnected client, allow it to reopen a matching durable session by explicitly passing its session ID. Reclaim the writer only after recovery and revision validation. Validate stdio setup independently for each documented client and platform. Claude Code and Cursor are first integration targets named in the original discussion. Compatibility is claimed after an actual stdio round trip and a caught failure on the tested versions.
