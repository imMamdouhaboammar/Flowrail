use flowrail_core::graph::{CodebaseGraph, EdgeKind, NodeRole, SemanticEdge, SemanticNode};

#[test]
fn test_graph_add_nodes_and_edges() {
    let mut graph = CodebaseGraph::new();

    let node_entry = SemanticNode {
        symbol_id: "src/checkout/controller.ts#CheckoutController.processCheckout".to_string(),
        display_name: "CheckoutController.processCheckout".to_string(),
        file_path: "src/checkout/controller.ts".to_string(),
        byte_range: (100, 350),
        line_range: (10, 25),
        role: NodeRole::EntryGate,
        signature_hash: "hash_entry_1".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };

    let node_service = SemanticNode {
        symbol_id: "src/checkout/service.ts#CheckoutService.execute".to_string(),
        display_name: "CheckoutService.execute".to_string(),
        file_path: "src/checkout/service.ts".to_string(),
        byte_range: (200, 600),
        line_range: (15, 45),
        role: NodeRole::BusinessLogic,
        signature_hash: "hash_service_1".to_string(),
        mutates_domain_state: true,
        is_exported: true,
    };

    let idx1 = graph.add_node(node_entry.clone());
    let idx2 = graph.add_node(node_service.clone());

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.get_node_index(&node_entry.symbol_id), Some(idx1));
    assert_eq!(graph.get_node_index(&node_service.symbol_id), Some(idx2));

    graph.add_edge(
        idx1,
        idx2,
        SemanticEdge {
            kind: EdgeKind::Calls,
            call_site_line: 18,
            is_dynamic_trace: false,
        },
    );

    assert_eq!(graph.edge_count(), 1);

    let found_node = graph.get_node(idx1).expect("node should exist");
    assert_eq!(found_node.role, NodeRole::EntryGate);
    assert_eq!(
        found_node.display_name,
        "CheckoutController.processCheckout"
    );

    // Test resolve_symbol
    // 1. Exact symbol_id
    assert_eq!(
        graph.resolve_symbol("src/checkout/controller.ts#CheckoutController.processCheckout"),
        Some(idx1)
    );
    // 2. Exact display_name
    assert_eq!(
        graph.resolve_symbol("CheckoutController.processCheckout"),
        Some(idx1)
    );
    assert_eq!(graph.resolve_symbol("CheckoutService.execute"), Some(idx2));
    // 3. Qualified path match
    assert_eq!(
        graph.resolve_symbol("controller.ts:CheckoutController.processCheckout"),
        Some(idx1)
    );
    assert_eq!(
        graph.resolve_symbol("service.ts#CheckoutService.execute"),
        Some(idx2)
    );
    // 4. Case-insensitive
    assert_eq!(graph.resolve_symbol("checkoutservice.execute"), Some(idx2));

    // 5. SCIP-style symbol suffix resolution
    let scip_node = SemanticNode {
        symbol_id: "scip-typescript npm @ecommerce/core 1.0.0 src/modules/checkout/checkout.controller.ts/CheckoutController#".to_string(),
        display_name: "CheckoutController".to_string(),
        file_path: "src/modules/checkout/checkout.controller.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::EntryGate,
        signature_hash: "hash_scip_1".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };
    let idx_scip = graph.add_node(scip_node);
    assert_eq!(graph.resolve_symbol("CheckoutController"), Some(idx_scip));
    assert_eq!(
        graph.resolve_symbol("checkout.controller.ts:CheckoutController"),
        Some(idx_scip)
    );
}
