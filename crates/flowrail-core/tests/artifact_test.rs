use flowrail_core::artifact::{
    AdhdView, AiEnrichment, AntiSlopContract, ArtifactKind, EnrichmentStatus, FlowrailArtifact,
};
use flowrail_core::graph::{NodeRole, SemanticNode};
use flowrail_core::slicer::{
    BlastRadiusReport, BlastRiskLevel, CorridorGate, CorridorPlan, GateStatus, SliceIntent,
};
use flowrail_core::state::CorridorSession;
use std::fs;
use std::path::PathBuf;

fn make_dummy_plan() -> CorridorPlan {
    CorridorPlan {
        corridor_id: "corr-test-123".to_string(),
        intent: SliceIntent::MutateContract,
        entry_gate_symbol: "src/checkout.ts#CheckoutController".to_string(),
        target_mutation_symbol: Some("src/tax.ts#TaxCalculator".to_string()),
        gates: vec![
            CorridorGate {
                gate_index: 0,
                status: GateStatus::Unlocked,
                node: SemanticNode {
                    symbol_id: "src/tax.ts#TaxCalculator".to_string(),
                    display_name: "TaxCalculator".to_string(),
                    file_path: "src/tax.ts".to_string(),
                    byte_range: (10, 50),
                    line_range: (2, 8),
                    role: NodeRole::ContractDefinition,
                    signature_hash: "hash_tax".to_string(),
                    mutates_domain_state: false,
                    is_exported: true,
                },
                upstream_contracts: Vec::new(),
                downstream_in_corridor: vec!["src/checkout.ts#CheckoutController".to_string()],
                blast_radius: BlastRadiusReport {
                    contagion_index: 1,
                    risk_level: BlastRiskLevel::Moderate,
                    external_dependents: vec![flowrail_core::slicer::ExternalDependent {
                        symbol_id: "src/external.ts#ExternalBilling".to_string(),
                        file_path: "src/external.ts".to_string(),
                        call_site_line: 42,
                        edge_kind: flowrail_core::graph::EdgeKind::Calls,
                        confidence: flowrail_core::graph::EdgeConfidence::Extracted,
                    }],
                    has_ambiguous_dependents: false,
                    co_changed_files: Vec::new(),
                },
            },
            CorridorGate {
                gate_index: 1,
                status: GateStatus::Locked,
                node: SemanticNode {
                    symbol_id: "src/checkout.ts#CheckoutController".to_string(),
                    display_name: "CheckoutController".to_string(),
                    file_path: "src/checkout.ts".to_string(),
                    byte_range: (100, 200),
                    line_range: (15, 30),
                    role: NodeRole::EntryGate,
                    signature_hash: "hash_checkout".to_string(),
                    mutates_domain_state: true,
                    is_exported: true,
                },
                upstream_contracts: vec!["src/tax.ts#TaxCalculator".to_string()],
                downstream_in_corridor: Vec::new(),
                blast_radius: BlastRadiusReport {
                    contagion_index: 0,
                    risk_level: BlastRiskLevel::SafeInternal,
                    external_dependents: Vec::new(),
                    has_ambiguous_dependents: false,
                    co_changed_files: Vec::new(),
                },
            },
        ],
        pruned_nodes: Vec::new(),
    }
}

#[test]
fn test_artifact_generation_from_plan() {
    let plan = make_dummy_plan();
    let workspace = PathBuf::from("/fake/workspace");
    let artifact = FlowrailArtifact::from_plan(&plan, &workspace, None);

    assert_eq!(artifact.schema_version, "flowrail.artifact.v1");
    assert_eq!(artifact.kind, ArtifactKind::CorridorPlan);
    assert!(artifact.artifact_id.starts_with("art-plan-"));
    assert_eq!(artifact.deterministic.total_gates, 2);
    assert_eq!(artifact.deterministic.active_gate_index, 0);
    assert_eq!(artifact.deterministic.blast_summary.contagion_index, 1);
    assert_eq!(
        artifact.deterministic.blast_summary.risk_level,
        BlastRiskLevel::Moderate
    );

    // Baseline grounded ADHD enrichment checks
    let adhd = &artifact.ai_enrichment.adhd_view;
    assert!(
        adhd.headline_action
            .contains("Inspect Gate 0 (`TaxCalculator`) in `src/tax.ts`")
    );
    assert!(adhd.state_anchor.contains("Gate 0 of 2"));
    assert!(!adhd.estimated_duration.is_empty());
    assert!(!adhd.bounded_steps.is_empty());
    assert!(adhd.bounded_steps.len() <= 5);
    assert!(
        adhd.terminal_action
            .contains("Inspect Gate 0 source slice in `src/tax.ts`")
    );

    // Anti-slop grounding
    let anti_slop = &artifact.ai_enrichment.anti_slop_contract;
    assert!(
        anti_slop
            .grounded_symbols
            .contains(&"src/tax.ts#TaxCalculator".to_string())
    );
    assert!(anti_slop.grounded_files.contains(&"src/tax.ts".to_string()));
    assert!(anti_slop.tangent_free);
    assert!(anti_slop.zero_conversational_fluff);
    assert!(anti_slop.direct_payload);
    assert!(anti_slop.validation_passed);

    // Clean JSON formatting
    let json_str = artifact
        .to_clean_json()
        .expect("Must serialize to clean JSON");
    assert!(json_str.contains("\"schema_version\": \"flowrail.artifact.v1\""));
    assert!(json_str.contains("\"headline_action\""));
}

#[test]
fn test_artifact_enrichment_validation_accepts_valid() {
    let plan = make_dummy_plan();
    let workspace = PathBuf::from("/fake/workspace");
    let mut artifact = FlowrailArtifact::from_plan(&plan, &workspace, None);

    let custom_enrichment = AiEnrichment {
        status: EnrichmentStatus::Enriched,
        human_readable_summary:
            "Tax calculator mutation plan with 1 external dependency in external.ts".to_string(),
        adhd_view: AdhdView {
            headline_action: "Edit `src/tax.ts` lines 2-8 to add optional tax rate parameter."
                .to_string(),
            state_anchor: "Gate 0 of 2 unlocked. Awaiting contract modification.".to_string(),
            estimated_duration: "8 minutes".to_string(),
            bounded_steps: vec![
                "1. Update TaxCalculator in `src/tax.ts`".to_string(),
                "2. Call corridor_propose_edit with acknowledge_blast_radius: true".to_string(),
                "3. Verify compiler diagnostics at Gate 0".to_string(),
            ],
            visible_wins: vec!["Blast radius bounded to external.ts".to_string()],
            terminal_action: "Open `src/tax.ts` and inspect TaxCalculator.".to_string(),
            error_diagnostic: None,
        },
        anti_slop_contract: AntiSlopContract {
            grounded_symbols: vec!["src/tax.ts#TaxCalculator".to_string()],
            grounded_files: vec!["src/tax.ts".to_string(), "src/external.ts".to_string()],
            tangent_free: true,
            zero_conversational_fluff: true,
            direct_payload: true,
            validation_passed: false,
        },
        enrichment_timestamp: None,
    };

    let result = artifact.apply_and_validate_enrichment(custom_enrichment);
    assert!(
        result.is_ok(),
        "Valid grounded enrichment must pass validation"
    );
    assert_eq!(artifact.ai_enrichment.status, EnrichmentStatus::Enriched);
    assert!(artifact.ai_enrichment.anti_slop_contract.validation_passed);
}

#[test]
fn test_artifact_rejects_hallucinated_symbol() {
    let plan = make_dummy_plan();
    let workspace = PathBuf::from("/fake/workspace");
    let mut artifact = FlowrailArtifact::from_plan(&plan, &workspace, None);

    let hallucinated = AiEnrichment {
        status: EnrichmentStatus::Enriched,
        human_readable_summary: "Some summary".to_string(),
        adhd_view: AdhdView {
            headline_action: "Edit some code.".to_string(),
            state_anchor: "Gate 0 of 2".to_string(),
            estimated_duration: "5 mins".to_string(),
            bounded_steps: vec!["1. Do something".to_string()],
            visible_wins: vec![],
            terminal_action: "Run test".to_string(),
            error_diagnostic: None,
        },
        anti_slop_contract: AntiSlopContract {
            grounded_symbols: vec!["src/imaginary.ts#GhostFunction".to_string()],
            grounded_files: vec!["src/tax.ts".to_string()],
            tangent_free: true,
            zero_conversational_fluff: true,
            direct_payload: true,
            validation_passed: false,
        },
        enrichment_timestamp: None,
    };

    let err = artifact
        .apply_and_validate_enrichment(hallucinated)
        .unwrap_err();
    assert!(
        err.to_string().contains("HALLUCINATED_SYMBOL_REJECTED"),
        "Expected hallucination error, got: {}",
        err
    );
}

#[test]
fn test_artifact_rejects_hallucinated_file() {
    let plan = make_dummy_plan();
    let workspace = PathBuf::from("/fake/workspace");
    let mut artifact = FlowrailArtifact::from_plan(&plan, &workspace, None);

    let hallucinated = AiEnrichment {
        status: EnrichmentStatus::Enriched,
        human_readable_summary: "Some summary".to_string(),
        adhd_view: AdhdView {
            headline_action: "Edit some code.".to_string(),
            state_anchor: "Gate 0 of 2".to_string(),
            estimated_duration: "5 mins".to_string(),
            bounded_steps: vec!["1. Do something".to_string()],
            visible_wins: vec![],
            terminal_action: "Run test".to_string(),
            error_diagnostic: None,
        },
        anti_slop_contract: AntiSlopContract {
            grounded_symbols: vec!["src/tax.ts#TaxCalculator".to_string()],
            grounded_files: vec!["src/nonexistent/secret_database.ts".to_string()],
            tangent_free: true,
            zero_conversational_fluff: true,
            direct_payload: true,
            validation_passed: false,
        },
        enrichment_timestamp: None,
    };

    let err = artifact
        .apply_and_validate_enrichment(hallucinated)
        .unwrap_err();
    assert!(
        err.to_string().contains("HALLUCINATED_FILE_REJECTED"),
        "Expected hallucinated file error, got: {}",
        err
    );
}

#[test]
fn test_artifact_rejects_conversational_fluff() {
    let plan = make_dummy_plan();
    let workspace = PathBuf::from("/fake/workspace");
    let mut artifact = FlowrailArtifact::from_plan(&plan, &workspace, None);

    let fluff = AiEnrichment {
        status: EnrichmentStatus::Enriched,
        human_readable_summary: "Hope this helps with your corridor.".to_string(),
        adhd_view: AdhdView {
            headline_action: "Great question! Let's edit `src/tax.ts`.".to_string(),
            state_anchor: "Gate 0 of 2".to_string(),
            estimated_duration: "5 mins".to_string(),
            bounded_steps: vec!["1. Do something".to_string()],
            visible_wins: vec![],
            terminal_action: "Run test".to_string(),
            error_diagnostic: None,
        },
        anti_slop_contract: AntiSlopContract {
            grounded_symbols: vec!["src/tax.ts#TaxCalculator".to_string()],
            grounded_files: vec!["src/tax.ts".to_string()],
            tangent_free: true,
            zero_conversational_fluff: true,
            direct_payload: true,
            validation_passed: false,
        },
        enrichment_timestamp: None,
    };

    let err = artifact.apply_and_validate_enrichment(fluff).unwrap_err();
    assert!(
        err.to_string().contains("ANTI_SLOP_VIOLATION"),
        "Expected anti-slop conversational fluff rejection, got: {}",
        err
    );
}

#[test]
fn test_artifact_rejects_excessive_bounded_steps_adhd_rule_9() {
    let plan = make_dummy_plan();
    let workspace = PathBuf::from("/fake/workspace");
    let mut artifact = FlowrailArtifact::from_plan(&plan, &workspace, None);

    let excessive = AiEnrichment {
        status: EnrichmentStatus::Enriched,
        human_readable_summary: "Summary".to_string(),
        adhd_view: AdhdView {
            headline_action: "Edit `src/tax.ts`.".to_string(),
            state_anchor: "Gate 0 of 2".to_string(),
            estimated_duration: "5 mins".to_string(),
            bounded_steps: vec![
                "Step 1".to_string(),
                "Step 2".to_string(),
                "Step 3".to_string(),
                "Step 4".to_string(),
                "Step 5".to_string(),
                "Step 6".to_string(), // exceeds cap of 5
            ],
            visible_wins: vec![],
            terminal_action: "Run test".to_string(),
            error_diagnostic: None,
        },
        anti_slop_contract: AntiSlopContract {
            grounded_symbols: vec!["src/tax.ts#TaxCalculator".to_string()],
            grounded_files: vec!["src/tax.ts".to_string()],
            tangent_free: true,
            zero_conversational_fluff: true,
            direct_payload: true,
            validation_passed: false,
        },
        enrichment_timestamp: None,
    };

    let err = artifact
        .apply_and_validate_enrichment(excessive)
        .unwrap_err();
    assert!(
        err.to_string().contains("ADHD_VIOLATION_RULE_9"),
        "Expected ADHD Rule 9 violation for >5 items, got: {}",
        err
    );
}

#[test]
fn test_artifact_disk_persistence() {
    let plan = make_dummy_plan();
    let temp_dir = std::env::temp_dir().join(format!("flowrail_art_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Create temp dir");

    let artifact = FlowrailArtifact::from_plan(&plan, &temp_dir, None);

    let saved_path = artifact
        .save_to_workspace(&temp_dir)
        .expect("Save to workspace must succeed");

    assert!(saved_path.exists());
    let latest_path = temp_dir.join(".flowrail/artifacts/latest.json");
    assert!(latest_path.exists());

    let content = std::fs::read_to_string(&saved_path).expect("Read saved artifact");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("Parse saved artifact JSON");
    assert_eq!(parsed["schema_version"], "flowrail.artifact.v1");
    assert_eq!(
        parsed["deterministic"]["entry_symbol"],
        "src/checkout.ts#CheckoutController"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_artifact_generation_from_session_active_and_blocked() {
    let plan = make_dummy_plan();
    let temp_dir = std::env::temp_dir().join(format!("flowrail_sess_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Create temp dir");

    let mut session = CorridorSession::new(plan, temp_dir.clone());
    let artifact_active = FlowrailArtifact::from_session(&session, None);
    assert_eq!(artifact_active.kind, ArtifactKind::CorridorExecution);
    assert_eq!(artifact_active.deterministic.active_gate_index, 0);
    assert!(
        artifact_active
            .ai_enrichment
            .adhd_view
            .state_anchor
            .contains("Gate 0 of 2")
    );

    // Now simulate blocked gate with diagnostic
    session
        .record_proposed_edit(
            0,
            "old content".to_string(),
            true,
            "new content with type error".to_string(),
            "pre_hash".to_string(),
            "post_hash".to_string(),
            vec![
                "TS2322: Type 'string' is not assignable to type 'number' in src/tax.ts:4"
                    .to_string(),
            ],
            false,
        )
        .expect("Record edit");

    let artifact_blocked = FlowrailArtifact::from_session(&session, None);
    assert!(
        !artifact_blocked
            .deterministic
            .verification_summary
            .diagnostics_clean
    );
    let adhd = &artifact_blocked.ai_enrichment.adhd_view;
    assert!(
        adhd.headline_action
            .contains("Fix TypeScript compiler diagnostic")
    );
    assert!(adhd.state_anchor.contains("Blocked at Gate 0 of 2"));
    assert!(adhd.error_diagnostic.is_some());
    let err_diag = adhd.error_diagnostic.as_ref().unwrap();
    assert!(err_diag.cause.contains("TS2322"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_artifact_generation_from_doctor() {
    let temp_dir = std::env::temp_dir().join(format!("flowrail_doc_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Create temp dir");

    let artifact = FlowrailArtifact::from_doctor(&temp_dir, 42, 108, None);
    assert_eq!(artifact.kind, ArtifactKind::WorkspaceAudit);
    assert!(artifact.artifact_id.starts_with("art-doctor-"));
    assert!(
        artifact
            .ai_enrichment
            .adhd_view
            .headline_action
            .contains("semantic graph verified clean")
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
