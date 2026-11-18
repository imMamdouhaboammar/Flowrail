use flowrail_core::graph::NodeRole;
use flowrail_ts_bridge::scip_loader::ScipGraphBuilder;
use std::fs;

#[test]
fn test_build_from_directory_ast_fallback() {
    let temp_dir = std::env::temp_dir().join(format!("flowrail_scip_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("src/tax")).unwrap();
    fs::create_dir_all(temp_dir.join("src/checkout")).unwrap();

    let types_code = r#"
export interface TaxCalculationResult {
    vat: number;
    total: number;
}
"#;
    fs::write(temp_dir.join("src/tax/tax.types.ts"), types_code).unwrap();

    let controller_code = r#"
import { TaxCalculationResult } from "../tax/tax.types";

export class CheckoutController {
    processCheckout(): TaxCalculationResult {
        return { vat: 10, total: 110 };
    }
}
"#;
    fs::write(
        temp_dir.join("src/checkout/checkout.controller.ts"),
        controller_code,
    )
    .unwrap();

    let graph = ScipGraphBuilder::build_from_directory(&temp_dir)
        .expect("Should build graph from directory");
    assert!(graph.node_count() >= 2);

    let types_node = graph.get_node_by_symbol("src/tax/tax.types.ts#TaxCalculationResult");
    assert!(
        types_node.is_some(),
        "TaxCalculationResult symbol must exist"
    );
    assert_eq!(types_node.unwrap().role, NodeRole::ContractDefinition);

    let ctrl_node =
        graph.get_node_by_symbol("src/checkout/checkout.controller.ts#CheckoutController");
    assert!(ctrl_node.is_some(), "CheckoutController symbol must exist");
    assert_eq!(ctrl_node.unwrap().role, NodeRole::EntryGate);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_update_file_in_graph_removes_deleted_symbols() {
    let temp_dir =
        std::env::temp_dir().join(format!("flowrail_update_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("src")).unwrap();

    let file_path = temp_dir.join("src/service.ts");
    let code_v1 = r#"
export function calculateA(): number { return 1; }
export function calculateB(): number { return 2; }
"#;
    fs::write(&file_path, code_v1).unwrap();

    let mut graph = ScipGraphBuilder::build_from_directory(&temp_dir).unwrap();
    assert!(graph.get_node_by_symbol("src/service.ts#calculateA").is_some());
    assert!(graph.get_node_by_symbol("src/service.ts#calculateB").is_some());

    // Version 2: calculateB is deleted, calculateC is added
    let code_v2 = r#"
export function calculateA(): number { return 10; }
export function calculateC(): number { return 30; }
"#;
    fs::write(&file_path, code_v2).unwrap();

    ScipGraphBuilder::update_file_in_graph(&mut graph, &temp_dir, &file_path).unwrap();

    // calculateA must still exist
    assert!(graph.get_node_by_symbol("src/service.ts#calculateA").is_some());
    // calculateC must now exist
    assert!(graph.get_node_by_symbol("src/service.ts#calculateC").is_some());
    // calculateB must have been pruned!
    assert!(
        graph.get_node_by_symbol("src/service.ts#calculateB").is_none(),
        "Deleted symbol must be pruned from graph on incremental file update"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
