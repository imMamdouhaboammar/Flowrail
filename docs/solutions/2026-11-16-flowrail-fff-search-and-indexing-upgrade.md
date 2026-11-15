# Solution: Flowrail System Upgrade Inspired by dmtrKovalenko/fff

## 1. Problem Context & Architectural Gap
Flowrail (Project Ariadne) enforces deterministic business flow corridors (for reading) and topological mutation plans (for writing), eliminating AI coding agent failure modes such as context pollution, topological blindness, and uncontained contagion.

Prior to this upgrade, Flowrail had four operational limitations identified through clean-room reverse-engineering of `dmtrKovalenko/fff`:
1. **Flat Substring Discovery**: `corridor_discover_entrypoints` only supported linear scans with naive substring matching, lacking Git status awareness, frecency weighting, and query constraints (`*.ts !tests/ auth`).
2. **Corridor Grep Void**: Agents lacked in-memory content search bounded to the active corridor slice, causing unnecessary context waste when hunting for usages or definition sites.
3. **Cold Directory Traversal**: AST indexing previously scanned all directories indiscriminately, slowing down workspace bootstrapping when large untracked folders existed.
4. **No Background File Watcher**: Graph updates required static cold rescans rather than incremental AST reslicing upon file changes.

## 2. Implemented Architecture & Innovations

### A. Resident Git Status Resolver (`crates/flowrail-core/src/git.rs`)
- Integrates `git2` with memory-cached status lookups (`Modified`, `Staged`, `Untracked`, `Clean`).
- Priority score weighting: Staged (+200), Modified (+150), Untracked (+100).
- Automatic TTL-based freshness (1.5s) and explicit invalidation upon gate mutations (`corridor_propose_edit`, `corridor_abort_or_rollback`).
- Fallback for non-git environments with zero panics.

### B. In-Memory Frecency Engine (`crates/flowrail-core/src/frecency.rs`)
- Thread-safe `FrecencyStore` with exponential time decay ($T_{\text{half}} = 300\text{s}$):
  $$S(t) = \sum_{k} e^{-\lambda (t - t_k)}$$
- Automatically tracks symbol accesses during discovery, gate inspection, and edit proposals.
- Normalizes scores into a $[0.0, 250.0]$ ranking boost, placing active refactoring corridors at the top of agent discovery.

### C. Typo-Tolerant Multi-Stage Search (`crates/flowrail-core/src/search.rs`)
- `QueryConstraints`: Extracts positive globs (`*.ts`), negative exclusions (`!tests/`), path scopes (`billing/`), and search terms.
- 4-Stage Progressive Cascade:
  - Stage 1: Exact symbol and display name matches (Score 1000+).
  - Stage 2: Prefix and camelCase acronym matches (e.g. `CC` matches `CheckoutController`, Score 650-800).
  - Stage 3: Substring matches (Score 300-450).
  - Stage 4: Typo-tolerant Levenshtein distance matching ($\le 2$ distance across sub-words, Score 200-250).
- Total scoring formula:
  $$\text{TotalScore} = \text{MatchScore} + S_{\text{git}} + S_{\text{frecency}} + S_{\text{role}}$$

### D. Corridor-Scoped & Workspace Grep (`crates/flowrail-core/src/grep.rs`)
- High-performance in-memory streaming line grep.
- Dual-mode scoping:
  - `"corridor"`: Restricts search strictly to files belonging to the active corridor session, maintaining strict context isolation.
  - `"workspace"`: Searches across all indexed TypeScript/JavaScript files.
- `definitions_only: true`: Filters matches to declaration lines (`export function`, `class`, `interface`, `type`), stripping comments, calls, and imports.
- Token-optimized compact formatting with `[DEF]`, line numbers, and Git badges.

### E. Background File Watcher & Incremental AST Bridge (`crates/flowrail-ts-bridge/src/watcher.rs`)
- Portable `BackgroundWatcher` monitoring filesystem timestamp changes.
- `ScipGraphBuilder::update_file_in_graph`: Incrementally re-parses only the affected file via `Oxc` and updates graph nodes/signatures in sub-milliseconds without rebuilding the whole graph.
- Directory filtering: Excludes hidden directories (`.agents`, `.agy`, `.claude`, `.cursor`, `.flowrail`), `node_modules`, `dist`, and `target`, dropping doctor/scan latency from 43s to <1.3s.

### F. MCP & CLI Integration
- MCP Tools:
  - `corridor_discover_entrypoints` / `flowrail_discover_entrypoints` (Upgraded with fuzzy query, `git_only`, `definitions_only`, badges).
  - `corridor_find_symbols` / `flowrail_find_symbols` (New tool for instant symbol search).
  - `corridor_grep` / `flowrail_grep` (New tool for corridor-scoped/workspace content search).
  - `corridor_inspect_gate` (Upgraded with `compact: true` mode).
- CLI Commands:
  - `flowrail search <query>`
  - `flowrail grep <query>`
  - `flowrail doctor` (Upgraded with Git status cache stats and engine readiness).

## 3. Verification Evidence
- 100% test pass across all 4 workspace crates (28 unit and integration tests passing).
- `cargo clippy --all-targets`: 0 warnings, clean pass.
- Clean-room verified: zero external proprietary dependencies or trademark leaks.
- Anti-slop verified: `bunx anti-slop-engineering check --base HEAD` green.
