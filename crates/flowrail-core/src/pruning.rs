use crate::graph::{CodebaseGraph, NodeRole};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningConfig {
    pub max_fan_in_utility_threshold: usize,
    pub max_corridor_depth: usize,
    pub include_type_only_edges: bool,
    pub ignore_path_prefixes: Vec<String>,
    pub ignore_symbol_substrings: Vec<String>,
}

impl Default for PruningConfig {
    fn default() -> Self {
        Self {
            max_fan_in_utility_threshold: 15,
            max_corridor_depth: 12,
            include_type_only_edges: true,
            ignore_path_prefixes: vec![
                "node_modules/".to_string(),
                "src/core/logger".to_string(),
                "src/core/telemetry".to_string(),
                "src/utils/logger".to_string(),
                "src/utils/telemetry".to_string(),
                "src/utils/metrics".to_string(),
            ],
            ignore_symbol_substrings: vec![
                "Logger.".to_string(),
                "Tracer.".to_string(),
                "Metrics.".to_string(),
                "ConfigService.get".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunedNodeRecord {
    pub symbol_id: String,
    pub reason: String,
    pub noise_score: f64,
    pub should_prune: bool,
}

pub struct PruneEvaluator;

impl PruneEvaluator {
    pub fn evaluate(
        node_idx: NodeIndex,
        graph: &CodebaseGraph,
        config: &PruningConfig,
        target_symbol: Option<&str>,
    ) -> PrunedNodeRecord {
        let node = &graph.graph[node_idx];

        // RULE 1: Never prune target symbol or contract definitions
        if let Some(target) = target_symbol
            && (node.symbol_id == target
                || node.display_name == target
                || node.symbol_id.ends_with(&format!("#{}", target))
                || node.symbol_id.ends_with(&format!(".{}", target)))
        {
            return PrunedNodeRecord {
                symbol_id: node.symbol_id.clone(),
                reason: "Protected: Target Symbol".to_string(),
                noise_score: 0.0,
                should_prune: false,
            };
        }

        if node.role == NodeRole::ContractDefinition {
            return PrunedNodeRecord {
                symbol_id: node.symbol_id.clone(),
                reason: "Protected: Contract Definition".to_string(),
                noise_score: 0.0,
                should_prune: false,
            };
        }

        // RULE 2: Hard prune based on path prefix
        for prefix in &config.ignore_path_prefixes {
            if node.file_path.starts_with(prefix) {
                return PrunedNodeRecord {
                    symbol_id: node.symbol_id.clone(),
                    reason: format!("Hard Pruned: Ignored Path Prefix '{}'", prefix),
                    noise_score: 1.0,
                    should_prune: true,
                };
            }
        }

        // RULE 3: Hard prune based on symbol substrings (e.g. Logger.*)
        for substr in &config.ignore_symbol_substrings {
            if node.display_name.contains(substr) || node.symbol_id.contains(substr) {
                return PrunedNodeRecord {
                    symbol_id: node.symbol_id.clone(),
                    reason: format!("Hard Pruned: Ignored Symbol Substring '{}'", substr),
                    noise_score: 1.0,
                    should_prune: true,
                };
            }
        }

        // RULE 4: Calculate Mathematical Noise Score
        let in_degree = graph.in_degree(node_idx);
        let out_degree_domain = graph.out_degree_domain(node_idx);
        let state_mutation_weight = if node.mutates_domain_state { 0.9 } else { 0.0 };

        let ratio = (in_degree as f64) / ((out_degree_domain.max(1)) as f64);
        let noise_score = ratio * (1.0 - state_mutation_weight);

        if in_degree >= config.max_fan_in_utility_threshold
            && out_degree_domain == 0
            && !node.mutates_domain_state
        {
            return PrunedNodeRecord {
                symbol_id: node.symbol_id.clone(),
                reason: format!(
                    "High Fan-In Utility Noise: in_degree={}, out_domain={}, score={:.2}",
                    in_degree, out_degree_domain, noise_score
                ),
                noise_score,
                should_prune: true,
            };
        }

        PrunedNodeRecord {
            symbol_id: node.symbol_id.clone(),
            reason: "Retained: Domain Relevant".to_string(),
            noise_score,
            should_prune: false,
        }
    }
}
