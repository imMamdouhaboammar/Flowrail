use flowrail_core::graph::{
    CodebaseGraph, EdgeConfidence, EdgeKind, NodeRole, SemanticEdge, SemanticNode,
};
use flowrail_core::pruning::{PruneEvaluator, PruningConfig};

#[test]
fn test_hard_pruned_paths() {
    let mut graph = CodebaseGraph::new();
    let logger_node = SemanticNode {
        symbol_id: "src/utils/logger.ts#Logger.log".to_string(),
        display_name: "Logger.log".to_string(),
        file_path: "src/utils/logger.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::UtilityNoise,
        signature_hash: "hash_log".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };
    let idx = graph.add_node(logger_node);
    let config = PruningConfig::default();

    let record = PruneEvaluator::evaluate(idx, &graph, &config, None);
    assert!(
        record.should_prune,
        "Logger file path should be hard pruned"
    );
}

#[test]
fn test_contract_definition_never_pruned() {
    let mut graph = CodebaseGraph::new();
    let contract_node = SemanticNode {
        symbol_id: "src/tax/types.ts#TaxCalculationResult".to_string(),
        display_name: "TaxCalculationResult".to_string(),
        file_path: "src/tax/types.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::ContractDefinition,
        signature_hash: "hash_tax".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };
    let idx = graph.add_node(contract_node);

    // Simulate 50 incoming edges to create extreme fan-in
    for i in 0..50 {
        let caller = SemanticNode {
            symbol_id: format!("src/caller_{}.ts#fn", i),
            display_name: format!("fn_{}", i),
            file_path: format!("src/caller_{}.ts", i),
            byte_range: (0, 50),
            line_range: (1, 5),
            role: NodeRole::BusinessLogic,
            signature_hash: "h".to_string(),
            mutates_domain_state: false,
            is_exported: true,
        };
        let c_idx = graph.add_node(caller);
        graph.add_edge(
            c_idx,
            idx,
            SemanticEdge {
                kind: EdgeKind::ReferencesType,
                call_site_line: 2,
                is_dynamic_trace: false,
                confidence: EdgeConfidence::Extracted,
            },
        );
    }

    let config = PruningConfig::default();
    let record = PruneEvaluator::evaluate(idx, &graph, &config, None);
    assert!(
        !record.should_prune,
        "ContractDefinition must never be pruned regardless of fan-in"
    );
}

#[test]
fn test_fan_in_utility_pruned() {
    let mut graph = CodebaseGraph::new();
    let util_node = SemanticNode {
        symbol_id: "src/common/format.ts#formatDate".to_string(),
        display_name: "formatDate".to_string(),
        file_path: "src/common/format.ts".to_string(),
        byte_range: (0, 80),
        line_range: (1, 8),
        role: NodeRole::BusinessLogic, // even if marked business logic initially
        signature_hash: "hash_fmt".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };
    let idx = graph.add_node(util_node);

    // 20 callers, 0 domain out-degree, 0 domain mutation
    for i in 0..20 {
        let caller = SemanticNode {
            symbol_id: format!("src/app_{}.ts#fn", i),
            display_name: format!("app_{}", i),
            file_path: format!("src/app_{}.ts", i),
            byte_range: (0, 50),
            line_range: (1, 5),
            role: NodeRole::BusinessLogic,
            signature_hash: "h".to_string(),
            mutates_domain_state: false,
            is_exported: true,
        };
        let c_idx = graph.add_node(caller);
        graph.add_edge(
            c_idx,
            idx,
            SemanticEdge {
                kind: EdgeKind::Calls,
                call_site_line: 2,
                is_dynamic_trace: false,
                confidence: EdgeConfidence::Extracted,
            },
        );
    }

    let config = PruningConfig::default();
    let record = PruneEvaluator::evaluate(idx, &graph, &config, None);
    assert!(
        record.should_prune,
        "High fan-in node with 0 domain out-degree and no state mutation must be pruned"
    );
}

#[test]
fn test_target_symbol_short_name_never_pruned() {
    let mut graph = CodebaseGraph::new();
    let target_node = SemanticNode {
        symbol_id: "src/utils/logger.ts#Logger.log".to_string(),
        display_name: "Logger.log".to_string(),
        file_path: "src/utils/logger.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 10),
        role: NodeRole::UtilityNoise,
        signature_hash: "hash_log".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    };
    let idx = graph.add_node(target_node);
    let config = PruningConfig::default();

    // Even though it matches ignored prefix src/utils/logger, passing short name "Logger.log" as target protects it
    let record = PruneEvaluator::evaluate(idx, &graph, &config, Some("Logger.log"));
    assert!(
        !record.should_prune,
        "Target symbol with short display_name must NEVER be pruned"
    );
    assert_eq!(record.reason, "Protected: Target Symbol");
}
