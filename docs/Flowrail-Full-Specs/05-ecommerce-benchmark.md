# Ecommerce benchmark

## Reference checkout and reproduction

Fixture commit: `1dbbacc` on `test/ecommerce-trap-fixture` in the local Flowrail repository. Treat the committed fixture as the reproducible baseline. The complete committed fixture is included under [fixtures/ecommerce-trap](fixtures/ecommerce-trap/README.md). The original source bundle in the root checkout and the nested copied workspace observed there have different paths and provenance. Use the Git object to locate the baseline when the old worktree path is unavailable.

From a checkout containing the fixture commit, in `fixtures/ecommerce-trap`:

```sh
bun install --frozen-lockfile --ignore-scripts
bun run typecheck
bun run test
bun run test:mutations
bun run index:scip
```

The baseline pins TypeScript 5.6.3 and scip-typescript 0.3.14. Generated dependencies, build output and `.flowrail/index.scip` are ignored. The [committed golden file](fixtures/ecommerce-trap/expected_flowrail.json) is preserved as a historical benchmark contract. Its ranges use one-based inclusive lines. The [read example](examples/read-flow.json) and [mutation example](examples/mutation-plan.json) illustrate proposed API output.

## Read behavior

| Step or branch | Symbol | Why it belongs |
| --- | --- | --- |
| Entry | CheckoutController.processCheckout | Accept checkout request, reject empty carts, invoke orchestration |
| Coordination | CheckoutOrchestrator.executeCheckout | Validate subtotal, request VAT, construct and save a record, return a response |
| Tax | TaxService.calculateVat | Handle exempt checkout, obtain the tax rule for other checkouts, calculate tax |
| Policy lookup | TaxRepository.findActiveRuleByJurisdiction | Supply the expected rate for the fixture's jurisdiction |
| Persistence boundary | OrderRepository.saveOrder | Write the fixture's in-memory order store and return the persisted record |

There are four required static call edges in the golden file. The tax rule lookup is conditional. The save call is a sibling of the tax call in the static graph. The response is a separate outcome of orchestration. Loggers and telemetry can be suppressed in this view. A time utility may be reduced to a summary, but the timestamp contribution and provenance remain visible if relevant to the task. Currency rounding and monetary validation remain visible. The currency format function can be hidden in this checkout view.

## Mutation behavior

| Gate | Symbol | Reason and expected action |
| --- | --- | --- |
| 0 | TaxCalculationResult | Target. Compare an optional field addition with a replace-and-split change. |
| 1 | TaxService.calculateVat | Producer. Both exempt and ordinary return paths need coherent output. |
| 2 | PersistedOrderRecord | Embedded shared contract. Review persisted shape and compatibility obligations. |
| 3 | CheckoutOrchestrator.executeCheckout | Direct annotation and uses of producer and stored order contract. Change consumers after both prerequisites. |
| 4 | CheckoutController.processCheckout | Review the outward API and run its behavior checks. Edit only when the new boundary requires it. |

Gate 1 and Gate 2 are independent after Gate 0. The golden file presents one deterministic ordering. A shortest-distance heuristic alone can place Gate 3 in the same layer as its prerequisites because it also names the target directly. Validate the prerequisite relation separately from display order. In the reference mutation, acknowledgement adds invoice, renewal and audit review/edit gates when evidence confirms their obligations and no additional prerequisite prevents appending them.

### External consumers and compiler oracle

| Consumer | Dependency chain | Confirmed lines for replacing the property |
| --- | --- | --- |
| InvoiceGenerator.buildComplianceInvoice | Typed access through `PersistedOrderRecord.taxBreakdown` | `src/modules/invoicing/invoice.generator.ts:22` |
| RenewalWorker.processRecurringCharge | Calls tax producer and reads the typed result | `src/modules/subscriptions/renewal.worker.ts:42,43` |
| VatAuditJob.summarizeQuarterlyLiability | Reduces a collection typed with the tax result | `src/modules/reporting/vat-audit.job.ts:18` |

Recorded baseline evidence from the fixture delivery: TypeScript typecheck passed, 3 of 3 checkout tests passed, and a real SCIP index of 111,254 bytes was produced from a fresh Git export on 15 November 2026. An optional `municipalTaxCents?: number` produced no diagnostics. Replacing `taxAmountCents` with required `nationalVatCents` and `municipalTaxCents` reported four TS2339 sites across the three expected external consumers. The fixture oracle checks these properties with in-memory source overlays using the real TypeScript compiler. It preserves the baseline files. It establishes the expected external break sites for that mutation. The recorded run leaves unverified Flowrail's search recall, gate operation, tax-split behavior after repair, index accuracy, build performance or general readiness.

### Acceptance fixtures beyond the golden

- Rename and relocate modules while preserving semantics. Assert preserved dependency invariants and changed source anchors. This detects checkout-specific hardcoding.
- Add a type-only import, a value reference with no call, and aliases across files. Compare with actual invocation sites.
- Add a checkout-reachable helper that looks generic but affects tax, a same-name unrelated type, and a branch with unresolved dynamic dispatch. Check retention, symbol resolution and coverage warnings.
- Add mutual recursion, star re-exports, overloaded functions, Unicode and CRLF. Verify grouped cycles, identity, precise spans and stable sorting.
- End-to-end mutation closes on a fully repaired snapshot with checks proving totals and persistence behavior. That repaired integration case is a future acceptance test.
