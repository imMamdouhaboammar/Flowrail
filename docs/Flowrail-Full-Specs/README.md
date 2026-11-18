# Flowrail · Full Specifications

**Product:** Flowrail  
**Internal project:** Ariadne  
**Owner:** Mamdouh Aboammar  
**Edition:** 1.0 · 15 November 2026  
**Status:** Proposed product and engineering specification with a dated implementation baseline

Flowrail gives a coding agent a task-specific view of a codebase and an evidence-backed plan for changing it. The first use case is a shared TypeScript contract change that starts in one business flow and reaches consumers owned by other modules. Flowrail's analysis is deterministic and local. The external agent authors edits.

Start with the project charter for the product, the architecture for implementation, and the fixture for an example that can be reproduced.

| Document | Contents |
| --- | --- |
| [01 · Product charter](01-product-charter.md) | Owner intent, problem, target users, scope, value and success |
| [02 · Semantic architecture](02-semantic-architecture.md) | Crates, SCIP, AST enrichment, data model, slicing, dependencies, cycles and pruning |
| [03 · Mutation and verification](03-mutation-and-verification.md) | Contract fingerprints, impact evidence, state transitions, checkpoints and recovery |
| [04 · CLI and MCP contracts](04-cli-and-mcp.md) | Commands, six tools, payloads, errors and client integration |
| [05 · Ecommerce benchmark](05-ecommerce-benchmark.md) | Read flow, mutation order, golden inputs, measured diagnostics and reproduction |
| [06 · Evaluation and acceptance](06-evaluation-and-acceptance.md) | Requirement IDs, regression oracles, baseline experiment, performance methodology |
| [07 · Risks and decisions](07-risks-and-decisions.md) | Alternatives, failure modes, mitigations, postponed choices |
| [08 · Delivery and handoff](08-delivery-and-handoff.md) | Milestones, scope, dependencies, release evidence and first implementation unit |
| [09 · Evidence and references](09-evidence-and-references.md) | Repository status, source trail and confidence of historical claims |
| [10 · Viability and release](10-viability-and-release.md) | Working prototype evidence, MVP estimates, comparison and GitHub release criteria |

Machine-readable material includes the [requirement ledger](requirements.json), [plan schema](schemas/plan.schema.json), [session result schema](schemas/session-result.schema.json), [illustrative read plan](examples/read-flow.json), [illustrative mutation plan](examples/mutation-plan.json), [blocked session response](examples/blocked-session.json) and the full [TypeScript fixture](fixtures/ecommerce-trap/README.md) with its [committed golden file](fixtures/ecommerce-trap/expected_flowrail.json).

**Evidence labels.** Owner intent refers to the user's requests and decisions. Observed refers to inspected files or recorded command output. Requirement describes target behavior. Proposal describes a design choice for the target behavior. Experimental target means a threshold that still needs measurement. JSON examples marked illustrative share the proposed interface. The committed fixture golden file records expected behavior in a separate historical format.
