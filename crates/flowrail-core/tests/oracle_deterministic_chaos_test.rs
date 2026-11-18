use flowrail_core::graph::{CodebaseGraph, NodeRole, SemanticNode};
use flowrail_core::pruning::PruningConfig;
use flowrail_core::slicer::{CorridorSlicer, SliceIntent};
use flowrail_core::state::{CorridorSession, SessionPhase};
use std::fs;

/// Deterministic 32-bit LCG pseudorandom generator for reproducible chaos injection.
struct ChaosRng {
    state: u32,
}

impl ChaosRng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    fn next_bool(&mut self) -> bool {
        (self.next_u32() & 1) == 1
    }
}

#[test]
fn test_oracle_seeded_deterministic_chaos_rollback() {
    let mut chaos = ChaosRng::new(0x5EED1234);

    let tmp_dir = std::env::temp_dir().join(format!("flowrail_chaos_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(tmp_dir.join("src")).unwrap();

    let initial_content = "export class PaymentService {\n    charge() { return true; }\n}\n";
    let file_path = "src/payment.service.ts";
    fs::write(tmp_dir.join(file_path), initial_content).unwrap();

    let mut graph = CodebaseGraph::new();
    let node = SemanticNode {
        symbol_id: "src/payment.service.ts#PaymentService".to_string(),
        display_name: "PaymentService".to_string(),
        file_path: file_path.to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::BusinessLogic,
        signature_hash: "hash_init".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };
    graph.add_node(node);

    let plan = CorridorSlicer::build_corridor(
        &graph,
        "PaymentService",
        SliceIntent::ReadFlow,
        None,
        &PruningConfig::default(),
    )
    .expect("Should slice corridor");

    let mut session = CorridorSession::new(plan, tmp_dir.clone());

    // Chaos simulation loop: 25 randomized fault injections
    for cycle in 0..25 {
        let is_corrupted_syntax = chaos.next_bool();
        let proposed = if is_corrupted_syntax {
            format!(
                "// CHAOS FAULT {}\nexport class PaymentService {{{{\n",
                cycle
            )
        } else {
            format!(
                "// VALID EDIT {}\nexport class PaymentService {{ charge() {{ return false; }} }}\n",
                cycle
            )
        };

        let diagnostics = if is_corrupted_syntax {
            vec![format!("Syntax error injected at cycle {}", cycle)]
        } else {
            vec![]
        };

        // Propose edit
        fs::write(tmp_dir.join(file_path), &proposed).unwrap();
        session
            .record_proposed_edit(
                0,
                initial_content.to_string(),
                true,
                proposed.clone(),
                "pre_hash".to_string(),
                "post_hash".to_string(),
                diagnostics,
                false,
            )
            .unwrap();

        if is_corrupted_syntax {
            assert!(
                matches!(session.phase, SessionPhase::BlockedAtGate { .. }),
                "Cycle {}: Corrupted edit must transition session into BlockedAtGate",
                cycle
            );
            // Verify advance is blocked
            assert!(session.advance_gate().is_err());

            // Chaos Recovery Action: Rollback gate
            session
                .rollback_to_gate(0)
                .expect("Rollback must succeed cleanly");

            // Verify bit-level state invariant
            let restored = fs::read_to_string(tmp_dir.join(file_path)).unwrap();
            assert_eq!(
                restored, initial_content,
                "Cycle {}: Rollback must restore disk content bit-for-bit to original state",
                cycle
            );
            assert!(matches!(session.phase, SessionPhase::ActiveGate(0)));
        } else {
            assert!(matches!(session.phase, SessionPhase::ActiveGate(0)));
            // Clean up back to baseline for next cycle
            fs::write(tmp_dir.join(file_path), initial_content).unwrap();
        }
    }

    let _ = fs::remove_dir_all(&tmp_dir);
}
