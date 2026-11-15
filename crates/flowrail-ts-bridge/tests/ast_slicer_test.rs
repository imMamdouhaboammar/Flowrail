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
