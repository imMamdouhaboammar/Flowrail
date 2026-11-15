# Flowrail

**Experimental Preview · TypeScript · MIT**

Flowrail is a local tool for exploring TypeScript code and planning changes to shared types. It is built in Rust, and the analysis runs without a model API.

## Why I’m building it

Add municipal tax at checkout. The coding agent finds the controller, changes the tax result, and gets checkout working. Then the compiler points to an invoice generator, a subscription renewal worker, and a VAT report. All three used the same result. They were easy to miss while the agent was reading checkout.

That’s the problem I want Flowrail to help with. When I’m reading a feature, I start where the request comes in and follow the work to where it’s saved or returned. When I’m changing a shared type, I start at the type. I want to see who produces it, who stores it, and who reads it in another part of the repo.

Flowrail calls those two views a **read corridor** and a **mutation plan**. The first gives the agent a focused view of the feature. The second is meant to show which changes depend on others and which consumers need a closer look. The agent still writes the patch. A reference tells us who might be affected. Compiler diagnostics and tests tell us what actually broke.

## Try the preview

Use Rust 1.99.1, the toolchain used in CI. From the repository root:

```sh
cargo test --workspace --locked
cargo run --locked -p flowrail-cli -- \
  --workspace fixtures/sample-ts-ecommerce slice \
  --entry 'src/modules/checkout/checkout.controller.ts#CheckoutController' \
  --intent READ_FLOW
```

This command returns a JSON plan from the small fixture using the AST fallback. The TypeScript bridge also ingests SCIP indexes and uses OXC to parse declarations. The real SCIP-backed flow and dependency ordering are still being validated. Use generated plans as leads to check against the source and compiler. Run experiments with MCP edits on a disposable copy of the workspace.

The larger [ecommerce benchmark](docs/Flowrail-Full-Specs/05-ecommerce-benchmark.md) gives us something concrete to check. Checkout compiles with its original type and with an added optional field. Splitting `taxAmountCents` into two required fields produces four TypeScript diagnostics across invoices, renewals, and reporting. You can reproduce those results and generate a real SCIP index with the [fixture commands](docs/Flowrail-Full-Specs/fixtures/ecommerce-trap/README.md).

## Where to look next

| Path | What’s there |
| --- | --- |
| [Story.md](Story.md) | Why I started Flowrail and where I’d use it |
| [Full specifications](docs/Flowrail-Full-Specs/README.md) | Product scope, architecture, tool contracts, and acceptance tests |
| [Viability and release criteria](docs/Flowrail-Full-Specs/10-viability-and-release.md) | What we’ve observed, what still needs proof, and how we’ll judge the MVP |
| [`flowrail-core`](crates/flowrail-core/) | Dependency graph, slicing, contract comparison, and session state |
| [`flowrail-ts-bridge`](crates/flowrail-ts-bridge/) | TypeScript analysis with SCIP and OXC |
| [`flowrail-cli`](crates/flowrail-cli/) and [`flowrail-mcp`](crates/flowrail-mcp/) | Command-line and stdio MCP interfaces |
| [Fixtures](fixtures/) | Small source fixture used by the Rust integration tests |

A useful issue includes a small TypeScript example, the command you ran, and a relationship Flowrail missed or got wrong. See [Contributing](CONTRIBUTING.md) for local checks and [Security](SECURITY.md) for private reports.

[MIT License](LICENSE)
