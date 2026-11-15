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
