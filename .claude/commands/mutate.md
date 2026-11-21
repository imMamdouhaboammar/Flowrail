Plan a safe reverse topological mutation corridor and fence blast radius before editing.

Usage:
```bash
/mutate <entry_symbol> <target_mutation_symbol>
```

This initializes a mutation corridor via `flowrail_open({ entry_symbol: "<entry>", intent: "MUTATE_CONTRACT", target_mutation_symbol: "<target>" })`. Edits are gated in bottom-up topological order with BLAKE3 contract hashing and contagion barriers.
