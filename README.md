# Flowrail

[![Experimental Preview](https://img.shields.io/badge/EXPERIMENTAL-PREVIEW-252525?style=for-the-badge&labelColor=111111)](docs/Flowrail-Full-Specs/10-viability-and-release.md)
[![CI](https://img.shields.io/github/actions/workflow/status/imMamdouhaboammar/Flowrail/ci.yml?branch=main&style=for-the-badge&labelColor=111111)](https://github.com/imMamdouhaboammar/Flowrail/actions/workflows/ci.yml)
[![MIT License](https://img.shields.io/badge/LICENSE-MIT-252525?style=for-the-badge&labelColor=111111)](LICENSE)

Flowrail is a local CLI for exploring TypeScript code and planning changes to shared types. Give it an entry symbol to see where a feature goes, or add a target type to see what depends on it. The CLI returns a JSON plan for you or a coding agent to inspect.

This is an **experimental preview**. The quickstart uses a bundled fixture and the AST fallback. The TypeScript bridge can also read SCIP indexes. Symbol lookup and dependency ordering on that path are still being validated. [Current limits and release criteria](docs/Flowrail-Full-Specs/10-viability-and-release.md)

## Build from source

Use Rust 1.99.1, the version tested in CI.

```sh
git clone https://github.com/imMamdouhaboammar/Flowrail.git
cd Flowrail
rustup toolchain install 1.99.1 --profile minimal
cargo +1.99.1 build --locked -p flowrail-cli
```

## Follow a checkout

Start at the controller in the [small ecommerce fixture](fixtures/sample-ts-ecommerce/). The JSON plan lists the symbols Flowrail found and the code it filtered from the view.

```sh
cargo +1.99.1 run --locked -p flowrail-cli -- \
  --workspace fixtures/sample-ts-ecommerce slice \
  --entry 'src/modules/checkout/checkout.controller.ts#CheckoutController' \
  --intent READ_FLOW
```

## Plan a tax change

Say checkout needs to report national and municipal tax separately. To understand checkout, I’d start at the controller. To plan that change, I’d start at `TaxCalculationResult` and check who produces it and who reads it, including callers outside checkout.

```sh
cargo +1.99.1 run --locked -p flowrail-cli -- \
  --workspace fixtures/sample-ts-ecommerce slice \
  --entry 'src/modules/checkout/checkout.controller.ts#CheckoutController' \
  --intent MUTATE_CONTRACT \
  --target 'src/modules/tax/tax.types.ts#TaxCalculationResult'
```

This command prints a plan without editing files. Its first gate is the target type. Use the reported dependencies as leads to check against source and compiler output.

## Check the impact with TypeScript

The [full ecommerce benchmark](docs/Flowrail-Full-Specs/05-ecommerce-benchmark.md) adds a separate compiler check. Adding an optional field typechecks. Replacing `taxAmountCents` with two required fields produces four diagnostics across invoicing, renewals, and VAT reporting. The benchmark uses those known results to evaluate the plan.

```sh
cd docs/Flowrail-Full-Specs/fixtures/ecommerce-trap
bun install --frozen-lockfile --ignore-scripts
bun run test:mutations
```

The fixture uses Bun and Node.js. Its [README](docs/Flowrail-Full-Specs/fixtures/ecommerce-trap/README.md) shows how to run the checkout tests and generate the SCIP index.

## Documentation

- [Story](Story.md) for why I started Flowrail and where I’d use it
- [Full specifications](docs/Flowrail-Full-Specs/README.md) for scope, architecture, and acceptance criteria
- [CLI and MCP](docs/Flowrail-Full-Specs/04-cli-and-mcp.md) for the proposed tool contracts
- [Fixtures](fixtures/) for the TypeScript sources used by the Rust integration tests
- [Rust workspace](crates/) for the CLI, dependency analysis, TypeScript bridge, and MCP server

## Contributing

A useful issue includes a small TypeScript example, the command you ran, and a dependency Flowrail missed or got wrong. See [CONTRIBUTING.md](CONTRIBUTING.md) for local checks and [SECURITY.md](SECURITY.md) for private reports. Try experimental MCP edits in a disposable checkout.
