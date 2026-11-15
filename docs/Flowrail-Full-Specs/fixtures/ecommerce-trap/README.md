# Ecommerce trap

Standalone strict TypeScript fixture for the Flowrail/Ariadne semantic slice. Run from this directory with Bun and Node.js:

```sh
bun install --frozen-lockfile --ignore-scripts
bun run typecheck
bun run test
bun run test:mutations
bun run index:scip
```

The last command creates a real `.flowrail/index.scip` from the checked-in sources. Build output, dependencies and the index are not committed.

The checkout controller calls the orchestrator, which calculates tax and saves the order through an in-memory repository. The tax service reads the jurisdiction's rule before rounding the amount. Logging and telemetry are presentation noise for this task; do not discard the whole currency utility module, which also owns rounding and validation.

`expected_flowrail.json` is a hand-authored benchmark target, not Flowrail output. The read flow describes static call edges, not a runtime trace. For the mutation plan, the tax contract comes before the producer and persisted-order contract, and both come before the orchestrator. The orchestrator also references the tax contract directly, so sorting by shortest distance can produce an invalid order. The numbered gate sequence is one valid order; independent gates need not have a forced dependency. The controller is an outer review step, not necessarily a required edit.

The mutation check runs the real TypeScript compiler over three in-memory versions of the tax contract without modifying source files. An optional field must compile cleanly; replacing `taxAmountCents` must produce the expected property diagnostics in invoicing, renewals and reporting. These diagnostics certify the fixture's external consumers for this change, not Flowrail's detection or MCP gate behavior.
