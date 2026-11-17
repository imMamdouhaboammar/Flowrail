use flowrail_core::graph::{EdgeKind, NodeRole, SemanticNode};
use flowrail_core::slicer::{
    BlastRadiusReport, BlastRiskLevel, CorridorGate, CorridorPlan, ExternalDependent, GateStatus,
    SliceIntent,
};
use flowrail_core::state::{CorridorSession, SessionPhase};
use std::fs;
use std::path::PathBuf;

struct TempDirGuard(PathBuf);

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_state_machine_contagion_barrier_and_rollback() {
    let guard = TempDirGuard::new("flowrail_state_test");
    let temp_dir = guard.path().to_path_buf();
    fs::create_dir_all(temp_dir.join("src/tax")).unwrap();
    fs::create_dir_all(temp_dir.join("src/invoicing")).unwrap();

    let tax_file = temp_dir.join("src/tax/types.ts");
    let original_tax_code = "export interface TaxCalculationResult { vat: number; }\n";
    fs::write(&tax_file, original_tax_code).unwrap();

    let gate0_node = SemanticNode {
        symbol_id: "src/tax/types.ts#TaxCalculationResult".to_string(),
        display_name: "TaxCalculationResult".to_string(),
        file_path: "src/tax/types.ts".to_string(),
        byte_range: (0, 50),
        line_range: (1, 3),
        role: NodeRole::ContractDefinition,
        signature_hash: "hash_orig".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    let ext_dep = ExternalDependent {
        symbol_id: "src/invoicing/invoice.ts#generateInvoice".to_string(),
        file_path: "src/invoicing/invoice.ts".to_string(),
        call_site_line: 10,
        edge_kind: EdgeKind::ReferencesType,
    };

    let gate0 = CorridorGate {
        gate_index: 0,
        status: GateStatus::Unlocked,
        node: gate0_node,
        upstream_contracts: Vec::new(),
        downstream_in_corridor: Vec::new(),
        blast_radius: BlastRadiusReport {
            contagion_index: 1,
            risk_level: BlastRiskLevel::Moderate,
            external_dependents: vec![ext_dep],
        },
    };

    let plan = CorridorPlan {
        corridor_id: "cor_test_1".to_string(),
        intent: SliceIntent::MutateContract,
        entry_gate_symbol: "entry".to_string(),
        target_mutation_symbol: Some("src/tax/types.ts#TaxCalculationResult".to_string()),
        gates: vec![gate0],
        pruned_nodes: Vec::new(),
    };

    let mut session = CorridorSession::new(plan, temp_dir.clone());
    assert_eq!(session.phase, SessionPhase::ActiveGate(0));

    // 1. Propose edit changing contract WITHOUT acknowledging blast radius
    let mutated_code = "export interface TaxCalculationResult { vat: number; total: number; }\n";
    session
        .propose_gate_edit(
            0,
            "src/tax/types.ts",
            mutated_code,
            "hash_mutated",
            false, // acknowledge_blast_radius = false
        )
        .expect("Propose edit should succeed");

    // Advancing gate MUST fail due to contagion barrier
    let advance_result = session.advance_gate();
    assert!(
        advance_result.is_err(),
        "Advancing without acknowledging blast radius must trigger contagion barrier"
    );
    let err_msg = advance_result.unwrap_err().to_string();
    assert!(
        err_msg.contains("CONTAGION_BARRIER"),
        "Error must cite CONTAGION_BARRIER: {}",
        err_msg
    );

    // 2. Propose edit WITH acknowledging blast radius
    session
        .propose_gate_edit(
            0,
            "src/tax/types.ts",
            mutated_code,
            "hash_mutated",
            true, // acknowledge_blast_radius = true
        )
        .expect("Propose edit should succeed");

    let advance_success = session.advance_gate();
    assert!(
        advance_success.is_ok(),
        "Advancing with acknowledgment should succeed"
    );

    // Dynamic gate expansion: Gate 1 should be dynamically injected for the external dependent!
    assert_eq!(
        session.plan.gates.len(),
        2,
        "External dependent must be dynamically injected as Gate 1"
    );
    assert_eq!(session.plan.gates[0].status, GateStatus::Verified);
    assert_eq!(session.plan.gates[1].status, GateStatus::Unlocked);
    assert_eq!(session.phase, SessionPhase::ActiveGate(1));

    // 3. Rollback to Gate 0
    let restored = session
        .rollback_to_gate(0)
        .expect("Rollback should succeed");
    assert!(
        !restored.is_empty(),
        "Restored files list should not be empty"
    );
    let content_on_disk = fs::read_to_string(&tax_file).unwrap();
    assert_eq!(
        content_on_disk, original_tax_code,
        "Disk content must be restored on rollback"
    );
    assert_eq!(session.plan.gates[0].status, GateStatus::Unlocked);
    assert_eq!(session.phase, SessionPhase::ActiveGate(0));
}
