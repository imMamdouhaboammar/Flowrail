use flowrail_ts_bridge::verifier::TsGateVerifier;
use std::path::Path;

#[test]
fn test_verifier_syntax_validation() {
    let workspace = Path::new(".");
    let valid_ts = "export interface Valid { a: number; }";
    let invalid_ts = "export interface Invalid { a: number "; // missing closing brace

    let clean = TsGateVerifier::verify_gate_edit(workspace, "valid.ts", valid_ts, false).unwrap();
    assert!(
        clean.is_empty(),
        "Valid TypeScript should produce 0 diagnostics"
    );

    let errors =
        TsGateVerifier::verify_gate_edit(workspace, "invalid.ts", invalid_ts, false).unwrap();
    assert!(
        !errors.is_empty(),
        "Broken syntax must produce syntax error diagnostics"
    );
    assert!(errors[0].contains("[Syntax Error]"));
}
