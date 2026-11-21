Open and sequentially traverse a business flow corridor with Flowrail.

Usage:
```bash
/corridor <entry_symbol>
```

Initializes an isolated reading corridor via `flowrail_open({ entry_symbol: "<entry>", intent: "READ_FLOW" })`. Inspect gates sequentially with `flowrail_inspect_gate`.
