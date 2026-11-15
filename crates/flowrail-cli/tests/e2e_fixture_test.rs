use flowrail_core::pruning::PruningConfig;
use flowrail_core::slicer::{CorridorSlicer, GateStatus, SliceIntent};
use flowrail_mcp::tools::CorridorMcpEngine;
use flowrail_ts_bridge::scip_loader::ScipGraphBuilder;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_e2e_fixture_full_corridor_lifecycle() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/sample-ts-ecommerce")
        .canonicalize()
        .expect("Fixture directory must exist");

    // 1. Build graph directly from fixture files
    let graph = ScipGraphBuilder::build_from_directory(&workspace)
        .expect("Should build semantic graph from fixture");

    assert!(
        graph.node_count() >= 5,
        "Expected at least 5 nodes in fixture graph"
    );

    // 2. Discover Entrypoints
    let mut engine = CorridorMcpEngine::new(workspace.clone(), graph.clone(), false);
    let disc_res = engine
        .dispatch_tool(
            "corridor_discover_entrypoints",
            &json!({ "filter": "checkout" }),
        )
        .unwrap();
    assert!(disc_res.contains("CheckoutController"));

    // 3. Test Read Corridor Slicing (Top-Down: Controller -> Service -> Types)
    let entry_symbol = "src/modules/checkout/checkout.controller.ts#CheckoutController";
    let read_plan = CorridorSlicer::build_corridor(
        &graph,
        entry_symbol,
        SliceIntent::ReadFlow,
        None,
        &PruningConfig::default(),
    )
    .expect("Should slice read corridor");

    assert!(read_plan.gates.len() >= 2);
    assert_eq!(read_plan.gates[0].node.symbol_id, entry_symbol);
    assert_eq!(read_plan.gates[0].status, GateStatus::Unlocked);

    // 4. Test Mutation DAG Slicing (Bottom-Up: Types -> Service -> Controller)
    let target_symbol = "src/modules/tax/tax.types.ts#TaxCalculationResult";
    let mutate_plan = CorridorSlicer::build_corridor(
        &graph,
        entry_symbol,
        SliceIntent::MutateContract,
        Some(target_symbol),
        &PruningConfig::default(),
    )
    .expect("Should slice mutation DAG");

    assert_eq!(
        mutate_plan.gates[0].node.symbol_id, target_symbol,
        "Gate 0 must be the target mutated type in bottom-up mutation plan"
    );

    // Check Contagion / Blast Radius on Gate 0
    let gate0_blast = &mutate_plan.gates[0].blast_radius;
    assert!(
        gate0_blast.contagion_index >= 1,
        "Contagion index must detect external dependents (InvoiceGenerator / RenewalService)"
    );

    // 5. Test State Machine & Contagion Barrier via MCP Engine
    let open_res = engine
        .dispatch_tool(
            "corridor_open",
            &json!({
                "entry_symbol": entry_symbol,
                "intent": "MUTATE_CONTRACT",
                "target_mutation_symbol": target_symbol
            }),
        )
        .unwrap();
    assert!(open_res.contains("Gate 0 is now UNLOCKED"));

    // Read Gate 0 code slice
    let inspect_gate0 = engine
        .dispatch_tool("corridor_inspect_gate", &json!({ "gate_index": 0 }))
        .unwrap();
    assert!(inspect_gate0.contains("TaxCalculationResult"));

    // Verify Visibility Lock: Reading Gate 1 before completing Gate 0 must be rejected
    let inspect_locked = engine.dispatch_tool("corridor_inspect_gate", &json!({ "gate_index": 1 }));
    assert!(
        inspect_locked.is_err(),
        "Reading a locked gate must fail with VISIBILITY_LOCK_VIOLATION"
    );

    // Clean up any session file created during test
    let _ = fs::remove_file(workspace.join(".corridor/active_session.json"));
}
