# Flowrail Operational Procedure (Runbook)

## Multi-File Refactoring Walkthrough

### Scenario: Refactoring Tax Calculation Result in Checkout Flow

1. **Step 1: Discover Entrypoint**
   ```json
   {
     "tool": "flowrail_discover_entrypoints",
     "arguments": { "query": "checkout" }
   }
   ```
   **Output:** Identifies `src/modules/checkout/checkout.controller.ts#CheckoutController`.

2. **Step 2: Initialize Mutation Corridor**
   ```json
   {
     "tool": "flowrail_open",
     "arguments": {
       "entry_symbol": "src/modules/checkout/checkout.controller.ts#CheckoutController",
       "intent": "MUTATE_CONTRACT",
       "target_mutation_symbol": "src/modules/tax/tax.types.ts#TaxCalculationResult"
     }
   }
   ```
   **Output:** Returns a 3-gate mutation plan. Gate 0 is `TaxCalculationResult` (Unlocked).
   Alerts that `invoice.generator.ts` and `renewal.service.ts` depend on `TaxCalculationResult` (`contagion_index: 2`).

3. **Step 3: Inspect Gate 0**
   ```json
   {
     "tool": "flowrail_inspect_gate",
     "arguments": { "gate_index": 0 }
   }
   ```
   Returns the code slice for `tax.types.ts` with line numbers and dependent contracts.

4. **Step 4: Propose Refactored Contract**
   ```json
   {
     "tool": "flowrail_propose_edit",
     "arguments": {
       "gate_index": 0,
       "new_file_content": "export interface TaxCalculationResult {\n  amount: number;\n  taxAmount: number;\n  currency: string;\n  breakdown?: Record<string, number>;\n}\n",
       "acknowledge_blast_radius": true
     }
   }
   ```
   Flowrail verifies syntax via `oxc_parser` and detects the modified BLAKE3 hash.

5. **Step 5: Advance Gate**
   ```json
   {
     "tool": "flowrail_advance_gate",
     "arguments": {}
   }
   ```
   Because `acknowledge_blast_radius` was set to `true`, Flowrail dynamically appends `invoice.generator.ts` and `renewal.service.ts` to the queue, marks Gate 0 as `VERIFIED`, and unlocks Gate 1 (`TaxService`).

6. **Step 6: Sequential Resolution**
   Proceed through all remaining gates until all files compile and `SessionPhase::Completed` is reached.
