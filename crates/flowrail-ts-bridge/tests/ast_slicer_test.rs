use flowrail_ts_bridge::ast_slicer::TsAstSlicer;
use std::path::Path;

#[test]
fn test_ast_slicer_extracts_contracts_and_symbols() {
    let ts_code = r#"
export interface TaxCalculationResult {
    vat: number;
    total: number;
}

export type TaxRate = number;

export function calculateTax(amount: number): TaxCalculationResult {
    const vat = amount * 0.2;
    return { vat, total: amount + vat };
}

export class OrderService {
    processOrder() {
        prisma.order.save();
    }
}
"#;

    let path = Path::new("test.ts");
    let symbols = TsAstSlicer::analyze_file(path, ts_code).expect("Should parse TS symbols");
    assert_eq!(symbols.len(), 4);

    let iface = symbols
        .iter()
        .find(|s| s.name == "TaxCalculationResult")
        .unwrap();
    assert!(iface.is_contract);
    assert!(iface.is_exported);

    let func = symbols.iter().find(|s| s.name == "calculateTax").unwrap();
    assert!(!func.is_contract);
    assert!(func.is_exported);

    let cls = symbols.iter().find(|s| s.name == "OrderService").unwrap();
    assert!(cls.mutates_state);

    let contracts =
        TsAstSlicer::extract_exported_contracts(path, ts_code).expect("Should extract contracts");
    assert_eq!(contracts.len(), 4);

    let hash1 = TsAstSlicer::compute_file_contract_hash(path, ts_code).unwrap();
    assert!(!hash1.is_empty());
}

#[test]
fn test_class_public_surface_mutation_detection() {
    let ts_before = r#"
export class TaxCalculator {
    calculate(amount: number): number {
        return amount * 0.2;
    }
}
"#;
    let ts_after_body_change_only = r#"
export class TaxCalculator {
    calculate(amount: number): number {
        // internal optimization
        const rate = 0.2;
        return amount * rate;
    }
}
"#;
    let ts_after_method_added = r#"
export class TaxCalculator {
    calculate(amount: number): number {
        return amount * 0.2;
    }
    applyDiscount(amount: number, discount: number): number {
        return amount - discount;
    }
}
"#;

    let path = Path::new("test.ts");
    let contracts_before = TsAstSlicer::extract_exported_contracts(path, ts_before).unwrap();
    let contracts_body =
        TsAstSlicer::extract_exported_contracts(path, ts_after_body_change_only).unwrap();
    let contracts_added =
        TsAstSlicer::extract_exported_contracts(path, ts_after_method_added).unwrap();

    let diff_body =
        flowrail_core::contract::ContractHasher::diff_surfaces(&contracts_before, &contracts_body);
    assert!(
        !diff_body.is_mutated,
        "Method body edit should NOT mutate class public surface"
    );

    let diff_added =
        flowrail_core::contract::ContractHasher::diff_surfaces(&contracts_before, &contracts_added);
    assert!(
        diff_added.is_mutated,
        "Adding a method MUST mutate class public surface"
    );
    assert!(
        diff_added
            .modified_symbols
            .contains(&"TaxCalculator".to_string())
    );
}
