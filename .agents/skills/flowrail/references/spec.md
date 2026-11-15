# Flowrail Technical Specification & Schema Contracts

## 1. Node Roles & Graph Classification

Every symbol in the codebase graph is classified into one of six deterministic roles:
- `EntryGate`: HTTP controllers, route handlers, CLI commands, queue consumers.
- `ContractDefinition`: Exported TypeScript interfaces, type aliases, DTOs, schemas.
- `BusinessLogic`: Domain services, use cases, business entity mutators.
- `DataRepository`: Database queries, ORM adapters, state stores.
- `ExternalBoundary`: Third-party APIs, webhooks, external message emitters.
- `UtilityNoise`: Loggers, telemetry wrappers, pure date/string helpers (pruned automatically).

## 2. Mathematical Noise Scoring Formula

$$\text{Score}_{\text{noise}}(v) = \left( \frac{\text{InDegree}(v)}{\max(1, \text{OutDegree}_{\text{Domain}}(v))} \right) \times (1.0 - W_{\text{mutation}}(v))$$

Where:
- $\text{InDegree}(v)$: Total incoming edges across the workspace.
- $\text{OutDegree}_{\text{Domain}}(v)$: Outgoing edges to non-utility domain nodes.
- $W_{\text{mutation}}(v)$: Weight (0.9 if node mutates persistence state, 0.0 otherwise).
- **Rule**: If $\text{Score} \ge 15.0$ and $W_{\text{mutation}} = 0$, the node is pruned from the corridor.

## 3. Error Codes & State Transitions

| Error Code | Trigger Condition | Remediation |
| :--- | :--- | :--- |
| `VISIBILITY_LOCK_VIOLATION` | Agent calls `flowrail_inspect_gate` on Gate $K > \text{active\_gate}$. | Inspect only the currently unlocked active gate. |
| `SYNTAX_ERROR` | Proposed code fails Tier 1 `oxc_parser` syntax validation. | Fix syntax errors reported in the error message before advancing. |
| `TYPE_ERROR` | Incremental `tsc --noEmit` fails within the active file. | Resolve type contradictions. |
| `CONTAGION_BARRIER` | Exported contract hash changed and external files depend on old signature, but `acknowledge_blast_radius` is false. | Re-propose edit with `acknowledge_blast_radius: true` to inject broken external files as mandatory gates. |
