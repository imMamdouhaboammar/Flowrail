# Flowrail

**Experimental Preview · TypeScript · MIT**

Flowrail is a local Rust tool that gives an AI coding agent a focused view of a TypeScript codebase before it reads or changes code. The analysis runs without a model API.

## The problem it solves

Add municipal tax at checkout. The agent finds the controller, changes the tax result, and gets checkout working. Then the compiler points to an invoice generator, a subscription renewal worker, and a VAT report. All three used the same result. They were easy to miss while the agent was following the checkout flow.

Flowrail produces two views from source code and a TypeScript semantic index:

- **Read corridor.** Start at a controller or handler. Follow the execution path to where data is saved or returned. Skip logging and utilities. Surface the rounding helper that actually affects what a customer pays.
- **Mutation plan.** Start at the type being changed. Walk backward through its producer, the records that store it, and the consumers in other features. Order those steps so a producer is verified before its callers are touched. Report cross-feature consumers before the first file is edited.

The agent still writes the patch. Flowrail controls what the agent sees and when, so it cannot modify Gate 2 before Gate 1 compiles.

---

## When to use it

| Situation | What Flowrail does |
| --- | --- |
| You are changing a shared `interface` or `type` and do not know all its consumers | Builds a mutation plan ordered from the type outward; reports every file that imports it, inside and outside the active feature |
| An agent is making a multi-file refactor and you have seen it break intermediate states before | Locks future files until the current file passes syntax verification; rolls back to a named checkpoint on failure |
| You are reading an unfamiliar feature and want only the code that matters to the business logic | Slices a read corridor from the entry point; prunes loggers, telemetry, and high-fan-in utilities automatically |
| You need to know the blast radius of a type change before touching anything | Runs `flowrail_open` with `intent: MUTATE_CONTRACT`; returns a contagion index and a list of external callers before any edit is proposed |
| An agent completed a refactor and you want to verify no caller was silently skipped | Session state records which gates were verified and which repairs were acknowledged |

Flowrail is **not** useful for single-file edits, bug fixes that do not touch shared contracts, or projects not written in TypeScript.

---

## Install

### Option A — one command from the repository

```bash
./install.sh
```

Builds a release binary with Cargo and links it to `~/.cargo/bin/flowrail`. Requires Rust 1.75 or later. The script installs Rust via `rustup` if it is not found.

### Option B — Cargo directly

```bash
cargo install --path crates/flowrail-cli --force
```

### Option C — remote install (no local clone required)

```bash
curl -fsSL https://raw.githubusercontent.com/imMamdouhaboammar/Flowrail/main/install.sh | bash
```

Verify the install:

```bash
flowrail doctor
```

---

## Initialize a project

Run once inside any TypeScript workspace:

```bash
flowrail init
```

This creates three files:

| File | Used by |
| --- | --- |
| `.flowrail/mcp.json` | Flowrail internal config |
| `.cursor/mcp.json` | Cursor |
| `.mcp.json` | Claude Code, Windsurf, and any MCP-compatible client |

All three contain:

```json
{
  "mcpServers": {
    "flowrail": {
      "command": "flowrail",
      "args": ["--workspace", ".", "mcp"]
    }
  }
}
```

After `init`, restart your editor. Flowrail's eight MCP tools become available to any agent running in that workspace.

To apply to every project without running `init` per repo, add the global config to your editor's user MCP settings pointing `--workspace` at `${workspaceFolder}`.

---

## CLI reference

```bash
flowrail demo                      # Run the built-in ecommerce fixture end-to-end
flowrail doctor                    # Verify the semantic graph, AST engine, and Git cache
flowrail search "checkout"         # Fuzzy symbol search with Git status and frecency ranking
flowrail grep "TaxResult"          # In-memory grep, corridor-scoped or workspace-wide
flowrail slice -e "<symbol>"       # Print a JSON corridor plan to stdout
flowrail mcp                       # Start the stdio MCP server (used by editors automatically)
```

---

## MCP tools

The MCP server speaks JSON-RPC 2.0 over stdio. Both `flowrail_*` and `corridor_*` prefixes are accepted.

| Tool | What it does |
| --- | --- |
| `flowrail_discover_entrypoints` | Fuzzy search for HTTP controllers, CLI handlers, and queue consumers with Git status badges |
| `flowrail_find_symbols` | Typo-tolerant symbol search across all AST nodes |
| `flowrail_grep` | In-memory grep; `scope: "corridor"` restricts results to files in the active session |
| `flowrail_open` | Opens a session with `READ_FLOW` or `MUTATE_CONTRACT` intent; unlocks Gate 0 |
| `flowrail_inspect_gate` | Returns the source slice and blast radius report for the unlocked gate |
| `flowrail_propose_edit` | Applies a proposed file content, checks syntax with OXC, hashes exported contracts |
| `flowrail_advance_gate` | Verifies the active gate, locks it, and unlocks Gate N+1 |
| `flowrail_abort_or_rollback` | Reverts files to any prior gate checkpoint |

Full tool contracts: [`.agents/skills/flowrail/SKILL.md`](.agents/skills/flowrail/SKILL.md)

---

## Agent skill

The skill file at `.agents/skills/flowrail/SKILL.md` tells an agent when to invoke Flowrail and how to follow the six-step refactoring protocol. Copy it to your global skills directory so agents pick it up without project-specific configuration:

```bash
mkdir -p ~/.agents/skills/flowrail
cp .agents/skills/flowrail/SKILL.md ~/.agents/skills/flowrail/SKILL.md
```

---

## Agent setup prompt

Paste this into any AI coding agent to have it install Flowrail, initialize it in the current project, and register the skill — using the latest published version.

```
You are setting up Flowrail in this project. Follow these steps exactly.

1. Check whether the `flowrail` binary is on PATH:
   - Run: `flowrail --version`
   - If not found, install it:
     curl -fsSL https://raw.githubusercontent.com/imMamdouhaboammar/Flowrail/main/install.sh | bash
     Then verify: `flowrail doctor`

2. Initialize Flowrail in this workspace:
   - Run: `flowrail init`
   - Confirm that `.flowrail/mcp.json`, `.cursor/mcp.json`, and `.mcp.json` were created.

3. Install the agent skill globally:
   - Run:
     mkdir -p ~/.agents/skills/flowrail
     curl -fsSL https://raw.githubusercontent.com/imMamdouhaboammar/Flowrail/main/.agents/skills/flowrail/SKILL.md \
       -o ~/.agents/skills/flowrail/SKILL.md
   - Confirm the file exists at `~/.agents/skills/flowrail/SKILL.md`.

4. Run the built-in demo to confirm everything works:
   - Run: `flowrail demo`
   - Expected: the demo prints a read corridor and a mutation plan with contagion warnings
     for the sample fixture. No errors.

5. Report back:
   - The output of `flowrail doctor`
   - Which MCP config files were created and their paths
   - The last line of `flowrail demo` output
```

---

## Try the preview

```sh
cargo test --workspace --locked
cargo run --locked -p flowrail-cli -- \
  --workspace fixtures/sample-ts-ecommerce slice \
  --entry 'src/modules/checkout/checkout.controller.ts#CheckoutController' \
  --intent READ_FLOW
```

The command returns a JSON corridor plan from the sample fixture using the AST fallback. The [ecommerce benchmark](docs/Flowrail-Full-Specs/05-ecommerce-benchmark.md) describes the known results: adding an optional field typechecks cleanly; splitting `taxAmountCents` into two required fields produces four diagnostics across invoicing, renewals, and reporting.

---

## Where to look next

| Path | What's there |
| --- | --- |
| [Story.md](Story.md) | Why I started Flowrail and where I'd use it |
| [Full specifications](docs/Flowrail-Full-Specs/README.md) | Product scope, architecture, tool contracts, and acceptance tests |
| [Viability and release criteria](docs/Flowrail-Full-Specs/10-viability-and-release.md) | What we've observed, what still needs proof, and how we'll judge the MVP |
| [`flowrail-core`](crates/flowrail-core/) | Dependency graph, slicing, contract comparison, and session state |
| [`flowrail-ts-bridge`](crates/flowrail-ts-bridge/) | TypeScript analysis with SCIP and OXC |
| [`flowrail-cli`](crates/flowrail-cli/) and [`flowrail-mcp`](crates/flowrail-mcp/) | Command-line and stdio MCP interfaces |
| [Fixtures](fixtures/) | Small source fixture used by the Rust integration tests |

A useful issue includes a small TypeScript example, the command you ran, and a relationship Flowrail missed or got wrong. See [Contributing](CONTRIBUTING.md) for local checks and [Security](SECURITY.md) for private reports.

[MIT License](LICENSE)
