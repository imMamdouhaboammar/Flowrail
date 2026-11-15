# Product charter

## Origin and purpose

Mamdouh's initial question about OpenRewrite concerned large-scale changes in Java, web, Go, Rust and Python projects. The project idea grew from a different bottleneck he sees during large refactors: a coding agent arrives in an unfamiliar repository, sees many plausible files, and has to work out where a business operation begins, what matters, and where a change should start. He wants the repository's business logic presented as a navigable map with entry, transition and exit points, and guidance an agent can follow while changing code.

The commercial name is **Flowrail**. **Ariadne** is the project's internal codename. Earlier drafts used **Corridor** for the product and tool prefix. A **corridor** remains the term for the relevant part of a business flow.

## Job to be done

| Actor | Situation | What Flowrail should let them do |
| --- | --- | --- |
| Coding agent | Asked to change a shared return type in a project it has recently opened | Identify the right contract, inspect the relevant behavior, see prerequisite order and find consumers outside the feature |
| Engineer or reviewer | Wants to review a multi-file refactor before accepting it | Verify the proposed scope against source evidence, see unresolved edges and confirm checks ran against the resulting snapshot |
| Maintainer | Wants to retrace or undo an interrupted, tool-managed edit | Identify what the session changed and restore its checkpoint without losing independent work |
| Developer tooling team | Needs reproducible evidence before adding another integration | Compare results against a compiler oracle and a baseline agent using ordinary repository tools |

### Reference scenario

A developer asks the agent to support national and municipal tax in Checkout. Reading starts at the checkout controller and follows validation, orchestration, tax rules, order persistence and response. Changing starts at the tax result contract, reaches the tax producer and embedded order type, then the checkout consumer. Invoicing, recurring charges and VAT reporting live outside the checkout flow and depend on the same contract. Flowrail should surface the risk while the agent can still change its approach.

### Why these failures happen

- Search returns text matches across the whole repository. Generic utilities, barrels and repeated identifiers obscure the decision a developer needs to make.
- A structural graph can show adjacency. A refactor needs an entry point, scope, a direction for dependencies, and the evidence behind a proposed step.
- A forward read from Checkout can miss incoming uses from other modules. A local change can leave code outside the active context broken.
- Agents can report success while downstream work is unfinished. A managed session needs explicit verification and an honest record of everything still open.
- Some repositories contain dynamic dispatch and runtime registration. The answer needs to expose where evidence is incomplete so a human or agent can investigate.

These are product hypotheses. The magnitude and frequency of each failure must be measured across real tasks.

## Product shape

1. **Read a business flow.** Start from an explicit symbol. Return ordered source anchors, callable dependencies, contracts, known boundaries, branches and unresolved parts. Distinguish static relationships from observed trace events.
2. **Plan a contract change.** Start from the target. Identify producer and consumer dependencies, report review and edit candidates, and show evidence for external impact before execution.
3. **Guide an edit session.** Serve the active gate, stage a caller-proposed patch, check it, carry forward documented repair obligations and protect checkpoint history. Finish only when the stated checks pass.
4. **Make the result auditable.** Every relevant relation carries provenance. Coverage, diagnostics and final checks identify the project scope and snapshot.

OpenRewrite informed the initial discussion through its search and rewrite recipes over Lossless Semantic Trees. Flowrail's first product centers on agent navigation, reviewable sequencing and verification. Code graphs, knowledge graphs and search tools are possible inputs or baselines. Claims about competitive advantage require measured comparisons. [OpenRewrite recipes](https://docs.openrewrite.org/concepts-and-explanations/recipes)

## Boundaries and ambitions

| Horizon | Scope |
| --- | --- |
| First proof | Strict TypeScript project, real SCIP, explicit entry and target, read-only CLI, truthful relationship evidence, one golden fixture |
| Pilot | Real TypeScript projects, known compiler scopes, contract comparisons, project diagnostics, deterministic impact reports and documented unsupported patterns |
| V1 | Local CLI and stdio MCP, six tools, bounded managed edit sessions, dynamic impact review, restart/rollback and reproducible evaluation |
| After V1 | Targeted framework entry discovery, measured incremental indexing, optional trace overlay, project references and package layouts according to demand, then other language adapters if evidence justifies them |

Initial analysis uses Rust for the host and TypeScript for the first language under analysis. The Rust executable is a distribution goal. Generating SCIP and running TypeScript verification still require the target toolchain. Initial operation requires a local checkout. No hosted service, graph database, vector index, graphical UI, general-purpose codemods, automated source generation, full runtime equivalence proof, or polyglot guarantee is required for V1.

## Success

The first success is a read-only plan that finds the three known external checkout-contract consumers in a real SCIP-backed fixture, preserves required semantic nodes, survives the direct-and-indirect dependency case, and gives stable evidence on repeated runs. Product success requires measuring whether an agent completes changes with comparable correctness and less input context across representative tasks. Original targets for tokens, coverage and latency are preserved in the [evaluation spec](06-evaluation-and-acceptance.md) as hypotheses.
