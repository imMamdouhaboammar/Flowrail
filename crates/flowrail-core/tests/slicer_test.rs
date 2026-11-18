use flowrail_core::graph::{CodebaseGraph, EdgeKind, NodeRole, SemanticEdge, SemanticNode};
use flowrail_core::pruning::PruningConfig;
use flowrail_core::slicer::{CorridorSlicer, GateStatus, SliceIntent};

#[test]
fn test_forward_slice_read_flow() {
    let mut graph = CodebaseGraph::new();

    let controller = SemanticNode {
        symbol_id: "src/checkout/controller.ts#CheckoutController.processCheckout".to_string(),
        display_name: "CheckoutController.processCheckout".to_string(),
        file_path: "src/checkout/controller.ts".to_string(),
        byte_range: (10, 200),
        line_range: (5, 20),
        role: NodeRole::EntryGate,
        signature_hash: "h_ctrl".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    let service = SemanticNode {
        symbol_id: "src/checkout/service.ts#CheckoutService.execute".to_string(),
        display_name: "CheckoutService.execute".to_string(),
        file_path: "src/checkout/service.ts".to_string(),
        byte_range: (10, 300),
        line_range: (10, 40),
        role: NodeRole::BusinessLogic,
        signature_hash: "h_srv".to_string(),
        mutates_domain_state: true,
        is_exported: true,
    };

    let logger = SemanticNode {
        symbol_id: "src/utils/logger.ts#Logger.info".to_string(),
        display_name: "Logger.info".to_string(),
        file_path: "src/utils/logger.ts".to_string(),
        byte_range: (0, 50),
        line_range: (1, 5),
        role: NodeRole::UtilityNoise,
        signature_hash: "h_log".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    let c_idx = graph.add_node(controller.clone());
    let s_idx = graph.add_node(service.clone());
    let l_idx = graph.add_node(logger);

    graph.add_edge(
        c_idx,
        s_idx,
        SemanticEdge {
            kind: EdgeKind::Calls,
            call_site_line: 12,
            is_dynamic_trace: false,
        },
    );
    graph.add_edge(
        s_idx,
        l_idx,
        SemanticEdge {
            kind: EdgeKind::Calls,
            call_site_line: 25,
            is_dynamic_trace: false,
        },
    );

    let config = PruningConfig::default();
    let plan = CorridorSlicer::slice(
        &graph,
        &controller.symbol_id,
        SliceIntent::ReadFlow,
        None,
        &config,
    )
    .expect("Slice should succeed");

    assert_eq!(
        plan.gates.len(),
        2,
        "Logger should be pruned, leaving controller and service"
    );
    assert_eq!(plan.gates[0].node.symbol_id, controller.symbol_id);
    assert_eq!(plan.gates[0].status, GateStatus::Unlocked);
    assert_eq!(plan.gates[1].node.symbol_id, service.symbol_id);
    assert_eq!(plan.gates[1].status, GateStatus::Locked);
}

#[test]
fn test_backward_slice_mutation_dag_and_contagion() {
    let mut graph = CodebaseGraph::new();

    let tax_types = SemanticNode {
        symbol_id: "src/tax/tax.types.ts#TaxCalculationResult".to_string(),
        display_name: "TaxCalculationResult".to_string(),
        file_path: "src/tax/tax.types.ts".to_string(),
        byte_range: (10, 100),
        line_range: (1, 10),
        role: NodeRole::ContractDefinition,
        signature_hash: "h_types".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    let tax_service = SemanticNode {
        symbol_id: "src/tax/tax.service.ts#TaxService.calculateVat".to_string(),
        display_name: "TaxService.calculateVat".to_string(),
        file_path: "src/tax/tax.service.ts".to_string(),
        byte_range: (20, 200),
        line_range: (5, 25),
        role: NodeRole::BusinessLogic,
        signature_hash: "h_tax_srv".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    let checkout_ctrl = SemanticNode {
        symbol_id: "src/checkout/checkout.controller.ts#CheckoutController.processCheckout"
            .to_string(),
        display_name: "CheckoutController.processCheckout".to_string(),
        file_path: "src/checkout/checkout.controller.ts".to_string(),
        byte_range: (10, 150),
        line_range: (5, 30),
        role: NodeRole::EntryGate,
        signature_hash: "h_chk_ctrl".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    // External caller outside checkout corridor
    let invoice_gen = SemanticNode {
        symbol_id: "src/invoicing/invoice.generator.ts#InvoiceGenerator.create".to_string(),
        display_name: "InvoiceGenerator.create".to_string(),
        file_path: "src/invoicing/invoice.generator.ts".to_string(),
        byte_range: (10, 100),
        line_range: (5, 20),
        role: NodeRole::BusinessLogic,
        signature_hash: "h_inv".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    let tt_idx = graph.add_node(tax_types.clone());
    let ts_idx = graph.add_node(tax_service.clone());
    let cc_idx = graph.add_node(checkout_ctrl.clone());
    let ig_idx = graph.add_node(invoice_gen);

    // Flow inside corridor: checkout -> tax_service -> tax_types
    graph.add_edge(
        cc_idx,
        ts_idx,
        SemanticEdge {
            kind: EdgeKind::Calls,
            call_site_line: 15,
            is_dynamic_trace: false,
        },
    );
    graph.add_edge(
        ts_idx,
        tt_idx,
        SemanticEdge {
            kind: EdgeKind::ReferencesType,
            call_site_line: 8,
            is_dynamic_trace: false,
        },
    );

    // External dependency: invoice_gen -> tax_types
    graph.add_edge(
        ig_idx,
        tt_idx,
        SemanticEdge {
            kind: EdgeKind::ReferencesType,
            call_site_line: 12,
            is_dynamic_trace: false,
        },
    );

    let config = PruningConfig::default();
    let plan = CorridorSlicer::slice(
        &graph,
        &checkout_ctrl.symbol_id,
        SliceIntent::MutateContract,
        Some(&tax_types.symbol_id),
        &config,
    )
    .expect("Slice should succeed");

    // Mutation order must be bottom-up: tax.types first, tax.service second, checkout.controller third
    assert_eq!(plan.gates[0].node.symbol_id, tax_types.symbol_id);
    assert_eq!(plan.gates[1].node.symbol_id, tax_service.symbol_id);
    assert_eq!(plan.gates[2].node.symbol_id, checkout_ctrl.symbol_id);

    // Contagion on Gate 0 (tax.types) must detect invoice.generator
    let blast = &plan.gates[0].blast_radius;
    assert_eq!(blast.contagion_index, 1);
    assert_eq!(blast.external_dependents.len(), 1);
    assert_eq!(
        blast.external_dependents[0].file_path,
        "src/invoicing/invoice.generator.ts"
    );
}

#[test]
fn test_mutation_dag_diamond_topological_ordering() {
    let mut graph = CodebaseGraph::new();

    // Diamond: Controller calls both Service and TargetRepo. Service calls TargetRepo.
    let ctrl = SemanticNode {
        symbol_id: "src/ctrl.ts#Ctrl".to_string(),
        display_name: "Ctrl".to_string(),
        file_path: "src/ctrl.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::EntryGate,
        signature_hash: "h_c".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };
    let svc = SemanticNode {
        symbol_id: "src/svc.ts#Svc".to_string(),
        display_name: "Svc".to_string(),
        file_path: "src/svc.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::BusinessLogic,
        signature_hash: "h_s".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };
    let repo = SemanticNode {
        symbol_id: "src/repo.ts#Repo".to_string(),
        display_name: "Repo".to_string(),
        file_path: "src/repo.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::DataRepository,
        signature_hash: "h_r".to_string(),
        mutates_domain_state: true,
        is_exported: true,
    };

    let c_idx = graph.add_node(ctrl.clone());
    let s_idx = graph.add_node(svc.clone());
    let r_idx = graph.add_node(repo.clone());

    // Edges: Ctrl -> Svc, Svc -> Repo, Ctrl -> Repo
    graph.add_edge(
        c_idx,
        s_idx,
        SemanticEdge {
            kind: EdgeKind::Calls,
            call_site_line: 3,
            is_dynamic_trace: false,
        },
    );
    graph.add_edge(
        s_idx,
        r_idx,
        SemanticEdge {
            kind: EdgeKind::Calls,
            call_site_line: 5,
            is_dynamic_trace: false,
        },
    );
    graph.add_edge(
        c_idx,
        r_idx,
        SemanticEdge {
            kind: EdgeKind::Calls,
            call_site_line: 7,
            is_dynamic_trace: false,
        },
    );

    let plan = CorridorSlicer::slice(
        &graph,
        &ctrl.symbol_id,
        SliceIntent::MutateContract,
        Some(&repo.symbol_id),
        &PruningConfig::default(),
    )
    .expect("Slice must succeed");

    assert_eq!(plan.gates.len(), 3);
    assert_eq!(plan.gates[0].node.symbol_id, repo.symbol_id);
    assert_eq!(
        plan.gates[1].node.symbol_id, svc.symbol_id,
        "Callee Svc must be mutated before caller Ctrl in diamond dependency"
    );
    assert_eq!(plan.gates[2].node.symbol_id, ctrl.symbol_id);
}
