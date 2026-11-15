# Flowrail

The request sounds small. Add support for municipal tax at checkout.

The coding agent finds the checkout controller, reads the tax service, and updates the result so the checkout screen shows the right total. The diff looks reasonable. Then the compiler points to an invoice generator, a subscription renewal worker, and a quarterly VAT report. All three relied on the old result. None came up while the agent was following the checkout flow.

That example is why I'm building Flowrail. In a codebase I know, I already have a sense of where a change belongs, which contracts are shared, and which parts deserve another look. A coding agent opening the same repository has to reconstruct that context for each task. A broad search gives it a pile of references. A narrow one can hide the very modules that make a change risky.

I want to give the agent the code it needs to understand the job and the evidence it needs to decide what to change.

## Reading a feature and changing it start in different places

To understand checkout, I'd start where the request comes in. I'd read through the orchestrator, see how it gets a tax rule, and follow the order to where it's saved. I'd skip most logging and telemetry. I'd keep the rounding helper because it can change what a customer pays.

To change the tax result, I'd start at `TaxCalculationResult`. I'd look at the service that returns it and the order record that stores it before I touch the orchestration. The controller may only need a review. Invoices, renewals, and reports deserve attention even though they live outside checkout.

Flowrail is designed to produce both views from the code. A **read corridor** shows the parts of a business flow that matter to the task, with source locations and gaps called out. A **mutation plan** starts at the thing that's changing, shows what depends on it, and puts prerequisites before their consumers. A reference tells me who's worth checking. Compiler diagnostics or a failing test tell me where the proposed change actually breaks something.

The agent still writes the patch. In a Flowrail-managed session, the tool is meant to show the current step, track follow-up work, and record the checks against the resulting code. A reviewer should be able to see why a file entered the plan and what was checked before the agent called the work finished.

## Where I'd use it

| Goal | Example | What Flowrail should give me |
| --- | --- | --- |
| Learn an unfamiliar feature | I've joined a team and need to understand how a renewal gets charged. | The worker, pricing and tax decisions, and where the charge is recorded. Each step points back to source I can read. |
| Debug a business rule | The receipt total differs from the checkout total. | A short list of places that calculate, round, store, and format the amount so I can compare them before choosing what to fix. |
| Change a shared TypeScript type | Split a single tax amount into national and municipal amounts. | The producer, stored shape, checkout consumer, and other affected features, with the order and evidence for each step. |
| Migrate an API or service signature | Move callers from a positional argument list to an options object. | A plan for the signature and its callers, with the exact use sites and the checks I can run as the migration progresses. |
| Plan a staged rollout | Add a field now and remove the old one after consumers move. | A way to compare both changes against the same consumers and see which usages still depend on the old field. |
| Estimate the scope of a ticket | Decide whether a checkout change needs the billing or reporting team. | Affected files and paths to their dependencies, including areas where the available analysis leaves the answer open. |
| Review an agent-authored change | A PR updates the order record and checkout service. | The shared contract, relevant downstream consumers, and the evidence behind the review. I can see whether the scope needs to include renewals or invoices. |
| Choose integration tests | Decide what to run after a pricing refactor. | The affected business flows to check, including exempt checkout, invoice generation, renewals, and reporting when the evidence points there. |
| Hand off or recover interrupted work | Another engineer picks up a refactor halfway through. | In a managed session, a record of which steps passed, which repairs are expected, and what checkpoint can be restored. |

## What I'm proving first

I'm starting with TypeScript because shared interfaces and return types make the problem concrete and the compiler gives me a way to check the result. Ariadne is the internal name for the Rust project. The analysis is designed to run locally, using source code and semantic evidence. An agent can use that output without Flowrail calling a model of its own.

The first benchmark is [the ecommerce fixture](docs/Flowrail-Full-Specs/fixtures/ecommerce-trap/README.md). The fixture has a working checkout and three consumers outside it. With the original contract, the typecheck and checkout tests pass. Adding an optional field still typechecks. Replacing `taxAmountCents` with two required fields produces four compiler diagnostics across invoicing, renewals, and reporting. Those are the known results Flowrail has to account for when it builds its plan.

After that, I want to put it in front of a real refactor and measure what changes. Did the agent find the consumers a person would have checked? Did it spend less time and context reading irrelevant code? Did the resulting change still pass the compiler and integration tests? That's what will tell me whether Flowrail is worth using on the next refactor.

[Read the full project specifications](docs/Flowrail-Full-Specs/README.md)
