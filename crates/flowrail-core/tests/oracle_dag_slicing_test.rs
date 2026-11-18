use flowrail_core::graph::{CodebaseGraph, EdgeKind, NodeRole, SemanticEdge, SemanticNode};
use flowrail_core::pruning::PruningConfig;
use flowrail_core::slicer::{CorridorSlicer, SliceIntent};

#[test]
fn test_oracle_dag_mutation_topological_order_invariant() {
    // Construct a multi-tiered pipeline: Controller -> Service -> Repository -> DatabaseDriver
    // and a secondary dependent: AdminWorker -> Repository
    let mut graph = CodebaseGraph::new();

    let node_controller = SemanticNode {
        symbol_id: "src/order/order.controller.ts#OrderController".to_string(),
        display_name: "OrderController".to_string(),
        file_path: "src/order/order.controller.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::EntryGate,
        signature_hash: "hash_ctrl".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    let node_service = SemanticNode {
        symbol_id: "src/order/order.service.ts#OrderService".to_string(),
        display_name: "OrderService".to_string(),
        file_path: "src/order/order.service.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::BusinessLogic,
        signature_hash: "hash_svc".to_string(),
        mutates_domain_state: true,
        is_exported: true,
    };

    let node_repo = SemanticNode {
        symbol_id: "src/order/order.repository.ts#OrderRepository".to_string(),
        display_name: "OrderRepository".to_string(),
        file_path: "src/order/order.repository.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::DataRepository,
        signature_hash: "hash_repo".to_string(),
        mutates_domain_state: true,
        is_exported: true,
    };

    let node_admin = SemanticNode {
        symbol_id: "src/admin/admin.worker.ts#AdminWorker".to_string(),
        display_name: "AdminWorker".to_string(),
        file_path: "src/admin/admin.worker.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::BusinessLogic,
        signature_hash: "hash_admin".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    let idx_ctrl = graph.add_node(node_controller);
    let idx_svc = graph.add_node(node_service);
    let idx_repo = graph.add_node(node_repo);
    let idx_admin = graph.add_node(node_admin);

    // Edges: Controller -> Service -> Repository
    graph.add_edge(
        idx_ctrl,
        idx_svc,
        SemanticEdge {
            kind: EdgeKind::Calls,
            call_site_line: 5,
            is_dynamic_trace: false,
        },
    );
    graph.add_edge(
        idx_svc,
        idx_repo,
        SemanticEdge {
            kind: EdgeKind::MutatesState,
            call_site_line: 7,
            is_dynamic_trace: false,
        },
    );
    // External dependency: AdminWorker -> Repository
    graph.add_edge(
        idx_admin,
        idx_repo,
        SemanticEdge {
            kind: EdgeKind::Calls,
            call_site_line: 12,
            is_dynamic_trace: false,
        },
    );

    let plan = CorridorSlicer::build_corridor(
        &graph,
        "OrderController",
        SliceIntent::MutateContract,
        Some("OrderRepository"),
        &PruningConfig::default(),
    )
    .expect("Should slice mutation DAG");

    // Invariant 1: Gate count must equal nodes in corridor (Repo, Service, Controller = 3)
    assert_eq!(plan.gates.len(), 3);

    // Invariant 2: Gate 0 must be the target mutation node (OrderRepository)
    assert_eq!(
        plan.gates[0].node.symbol_id,
        "src/order/order.repository.ts#OrderRepository"
    );
    assert_eq!(
        plan.gates[0].status,
        flowrail_core::slicer::GateStatus::Unlocked
    );

    // Invariant 3: Gate 1 must be OrderService, Gate 2 must be OrderController
    assert_eq!(
        plan.gates[1].node.symbol_id,
        "src/order/order.service.ts#OrderService"
    );
    assert_eq!(
        plan.gates[1].status,
        flowrail_core::slicer::GateStatus::Locked
    );

    assert_eq!(
        plan.gates[2].node.symbol_id,
        "src/order/order.controller.ts#OrderController"
    );
    assert_eq!(
        plan.gates[2].status,
        flowrail_core::slicer::GateStatus::Locked
    );

    // Invariant 4: Blast Radius Contagion Conservation
    // OrderRepository has external dependent AdminWorker (out of corridor)
    let repo_gate = &plan.gates[0];
    assert_eq!(repo_gate.blast_radius.contagion_index, 1);
    assert_eq!(repo_gate.blast_radius.external_dependents.len(), 1);
    assert_eq!(
        repo_gate.blast_radius.external_dependents[0].symbol_id,
        "src/admin/admin.worker.ts#AdminWorker"
    );
}
