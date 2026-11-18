# Viability, MVP scope, and release criteria

**Assessment date** · 15 November 2026  
**Status** · Working prototype. Read-only MVP criteria pending.  
**Scope** · Local Rust tooling for TypeScript projects

## Feasibility and product value

Flowrail is technically feasible within a defined TypeScript project scope. The working tree has Rust implementations for the CLI, graph, SCIP loader and MCP surface. The strongest initial use case is **planning a change to a shared TypeScript contract before the first edit**. A useful plan identifies who produces the type, who stores it, who consumes it across feature boundaries, and which steps depend on others. Each conclusion should point to source evidence and distinguish a possible impact from a compiler- or test-confirmed break.

TypeScript already exposes symbol references, and Code Review Graph already offers impact-radius and affected-flow queries. OpenRewrite automates recipe-defined transformations. Flowrail's product hypothesis is that a prerequisite-ordered plan and explicit coverage gaps improve the agent's refactoring outcome. We still need a comparison to establish that benefit. Broad business-flow discovery, large token savings, and reliable managed edits are unverified. [TypeScript navigation](https://code.visualstudio.com/docs/languages/typescript) · [Code Review Graph releases](https://github.com/tirth8205/code-review-graph/releases) · [OpenRewrite recipes](https://docs.openrewrite.org/concepts-and-explanations/recipes)

## Observed on the Mac

| Check | Result |
| --- | --- |
| Executable fixture at `1dbbacc` | A fresh Git export passed TypeScript typechecking and all three checkout tests, then generated a real SCIP index. The optional-field addition typechecked. Replacing `taxAmountCents` produced four TS2339 diagnostics in the three expected external consumers. |
| Index loading with the existing `flowrail 0.1.0` debug binary | The `doctor` command reported 167 nodes and 374 edges against the local fixture index. |
| Slicing with the documented entry ID | Both READ_FLOW and MUTATE_CONTRACT exited with an entry-symbol lookup error. |
| Slicing with raw SCIP IDs | READ_FLOW returned only `CheckoutController.processCheckout`. MUTATE_CONTRACT returned only `TaxCalculationResult` as a gate. Its impact output had 12 rows across six distinct files. It reached invoicing, renewals and reporting, and also included the tax producer, order type and checkout orchestrator as external. |
| Source review | The existing CLI can fall back to an AST scan when no SCIP index is present. The Rust fixture lifecycle test inspected uses that scan. The mutation slicer orders by shortest distance, which cannot establish precedence when a consumer also references the target directly. |

The CLI check used the debug binary already present on disk. A fresh build was outside this assessment, so the run does not certify a specific source revision or the full Rust/MCP test suite. The fixture results establish the expected break sites. They leave Flowrail's correctness and the benefit over an agent using standard tools open. See the [fixture and commands](05-ecommerce-benchmark.md) and [implementation evidence](09-evidence-and-references.md).

## MVP and effort estimates

The proposed MVP is a **read-only CLI** for a defined TypeScript project. It accepts an explicit entry and target, reads a real SCIP index, and returns a flow and a change plan with source locations, deduplicated external consumers, and visible coverage gaps. Validate it on the fixture and a TypeScript project we did not design around the benchmark.

| Stage | Estimated elapsed effort from this baseline | Acceptance |
| --- | --- | --- |
| Technical proof | 2–5 working days | The documented symbol resolves. The real SCIP-backed run finds the checkout call edges and the three external consumers. The tax producer and persisted order contract precede the orchestrator. |
| Read-only MVP | 2–4 weeks total | The same workflow passes on the fixture and an independent TypeScript project, with reproducible setup and clear errors or coverage gaps. |
| Managed MCP edits | 6–10 weeks total | Stage and verify scoped edits, enforce prerequisites, run final project checks and recover when a session stops or the workspace changes. |

These are planning ranges for one full-time developer working with agent assistance and reusing the current Rust workspace. Cross-file symbol resolution, ambiguous relationships and behavior on an independent project are the largest unknowns.

The working-tree count was **2,265 physical lines of Rust implementation** and **783 physical lines of Rust tests** across the product crates. That count includes comments and blank lines. A rough final size for the read-only MVP is **3,000–5,000 implementation lines plus 1,000–2,000 test lines**, excluding dependencies, generated output, documentation and fixtures. Use those numbers for planning only.

## One-week decision

1. Make the existing CLI reproduce the [golden fixture](05-ecommerce-benchmark.md) from a real SCIP index. Check the documented alias lookup, invocation versus type-reference classification, prerequisite ordering and three unique external consumers with source sites. Use the compiler oracle to separate possible impact from confirmed breakage.
2. Give the same coding agent the same contract-change task in two fresh copies of the same snapshot. In the baseline, let it use repository search, file reads and TypeScript references. In the other run, add Flowrail. Hold the model, task, verifier and time budget constant. Score missed consumers, files read or input tokens, and final compiler and integration results.
3. Continue if Flowrail reduces missed consumers or reading effort with comparable final correctness. If the outcome and effort are effectively the same, narrow the investment to contract-change planning and defer managed MCP edits. Treat this as an early investment decision. [Full evaluation method](06-evaluation-and-acceptance.md)

## GitHub release criteria

| Publication | Earliest practical point |
| --- | --- |
| Private development repository | Immediately after reviewing the files and history that would be pushed. At assessment time there was no configured remote, the working tree had uncommitted work, and the initial commit tracked 4,197 files, mostly agent tooling and configuration. Review the tracked history as well as the intended project files before setting up the remote. |
| Public experimental preview | Roughly 1–2 weeks if the existing analysis can be repaired without a redesign. Publish when a second person can reproduce the real SCIP-backed fixture from a clean clone using the documented commands. Include the supported scope and current limits. |
| Public MVP | After the independent TypeScript project also passes the read-only workflow. Use the 2–4 week estimate above for planning. |
