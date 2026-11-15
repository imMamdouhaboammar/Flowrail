use crate::graph::{CodebaseGraph, EdgeKind, NodeRole, SemanticNode};
use crate::pruning::{PruneEvaluator, PrunedNodeRecord, PruningConfig};
use anyhow::{Result, anyhow};
use petgraph::Direction;
use petgraph::algo::kosaraju_scc;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SliceIntent {
    ReadFlow,
    MutateContract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateStatus {
    Unlocked,
    Locked,
    Verified,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlastRiskLevel {
    SafeInternal,
    Moderate,
    HighContagion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalDependent {
    pub symbol_id: String,
    pub file_path: String,
    pub call_site_line: usize,
    pub edge_kind: EdgeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlastRadiusReport {
    pub contagion_index: usize,
    pub risk_level: BlastRiskLevel,
    pub external_dependents: Vec<ExternalDependent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorridorGate {
    pub gate_index: usize,
    pub status: GateStatus,
    pub node: SemanticNode,
    pub upstream_contracts: Vec<String>,
    pub downstream_in_corridor: Vec<String>,
    pub blast_radius: BlastRadiusReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorPlan {
    pub corridor_id: String,
    pub intent: SliceIntent,
    pub entry_gate_symbol: String,
    pub target_mutation_symbol: Option<String>,
    pub gates: Vec<CorridorGate>,
    pub pruned_nodes: Vec<PrunedNodeRecord>,
}

pub struct CorridorSlicer;

impl CorridorSlicer {
    pub fn slice(
        graph: &CodebaseGraph,
        entry_symbol: &str,
        intent: SliceIntent,
        target_mutation_symbol: Option<&str>,
        config: &PruningConfig,
    ) -> Result<CorridorPlan> {
        Self::build_corridor(graph, entry_symbol, intent, target_mutation_symbol, config)
    }

    pub fn build_corridor(
        graph: &CodebaseGraph,
        entry_symbol: &str,
        intent: SliceIntent,
        target_mutation_symbol: Option<&str>,
        config: &PruningConfig,
    ) -> Result<CorridorPlan> {
        let entry_idx = *graph.symbol_to_index.get(entry_symbol).ok_or_else(|| {
            anyhow!(
                "Entry symbol '{}' not found in semantic graph",
                entry_symbol
            )
        })?;

        let mut pruned_records = Vec::new();
        let mut pruned_set = HashSet::new();

        // Step 1: Forward BFS from Entry Gate to isolate active business corridor
        let mut active_corridor_nodes = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((entry_idx, 0usize));
        active_corridor_nodes.insert(entry_idx);

        while let Some((curr_idx, depth)) = queue.pop_front() {
            if depth >= config.max_corridor_depth {
                continue;
            }

            for edge in graph.graph.edges_directed(curr_idx, Direction::Outgoing) {
                if !config.include_type_only_edges && edge.weight().kind == EdgeKind::ReferencesType
                {
                    continue;
                }

                let next_idx = edge.target();
                if active_corridor_nodes.contains(&next_idx) || pruned_set.contains(&next_idx) {
                    continue;
                }

                let record =
                    PruneEvaluator::evaluate(next_idx, graph, config, target_mutation_symbol);
                if record.should_prune {
                    pruned_set.insert(next_idx);
                    pruned_records.push(record);
                    continue;
                }

                active_corridor_nodes.insert(next_idx);
                queue.push_back((next_idx, depth + 1));
            }
        }

        // Step 2: Determine ordered sequence of nodes based on intent
        let ordered_indices = match intent {
            SliceIntent::ReadFlow => {
                Self::order_for_reading(graph, entry_idx, &active_corridor_nodes)
            }
            SliceIntent::MutateContract => {
                let target_sym = target_mutation_symbol.ok_or_else(|| {
                    anyhow!("target_mutation_symbol is required when intent is MutateContract")
                })?;
                let target_idx = *graph.symbol_to_index.get(target_sym).ok_or_else(|| {
                    anyhow!("Target mutation symbol '{}' not found in graph", target_sym)
                })?;
                active_corridor_nodes.insert(target_idx);
                Self::order_for_mutation(graph, target_idx, &active_corridor_nodes)
            }
        };

        // Step 3: Build Gates and calculate Blast Radius
        let mut gates = Vec::with_capacity(ordered_indices.len());
        for (i, &node_idx) in ordered_indices.iter().enumerate() {
            let node = graph.graph[node_idx].clone();
            let blast_radius = Self::compute_blast_radius(graph, node_idx, &active_corridor_nodes);

            let upstream_contracts = graph
                .graph
                .edges_directed(node_idx, Direction::Outgoing)
                .filter_map(|e| {
                    let target = &graph.graph[e.target()];
                    if target.role == NodeRole::ContractDefinition {
                        Some(target.symbol_id.clone())
                    } else {
                        None
                    }
                })
                .collect();

            let downstream_in_corridor = graph
                .graph
                .edges_directed(node_idx, Direction::Incoming)
                .filter_map(|e| {
                    let source_idx = e.source();
                    if active_corridor_nodes.contains(&source_idx) {
                        Some(graph.graph[source_idx].symbol_id.clone())
                    } else {
                        None
                    }
                })
                .collect();

            gates.push(CorridorGate {
                gate_index: i,
                status: if i == 0 {
                    GateStatus::Unlocked
                } else {
                    GateStatus::Locked
                },
                node,
                upstream_contracts,
                downstream_in_corridor,
                blast_radius,
            });
        }

        let hash_input = format!("{}:{:?}:{:?}", entry_symbol, intent, target_mutation_symbol);
        let corridor_hash = blake3::hash(hash_input.as_bytes()).to_hex();
        let corridor_id = format!("cor_{}", &corridor_hash[..8]);

        Ok(CorridorPlan {
            corridor_id,
            intent,
            entry_gate_symbol: entry_symbol.to_string(),
            target_mutation_symbol: target_mutation_symbol.map(|s| s.to_string()),
            gates,
            pruned_nodes: pruned_records,
        })
    }

    fn order_for_reading(
        graph: &CodebaseGraph,
        entry_idx: NodeIndex,
        corridor_nodes: &HashSet<NodeIndex>,
    ) -> Vec<NodeIndex> {
        let sccs = kosaraju_scc(&graph.graph);
        let mut ordered = Vec::new();
        let mut visited = HashSet::new();

        ordered.push(entry_idx);
        visited.insert(entry_idx);

        for scc in sccs.into_iter().rev() {
            for idx in scc {
                if corridor_nodes.contains(&idx) && !visited.contains(&idx) {
                    visited.insert(idx);
                    ordered.push(idx);
                }
            }
        }
        ordered
    }

    fn order_for_mutation(
        graph: &CodebaseGraph,
        target_idx: NodeIndex,
        corridor_nodes: &HashSet<NodeIndex>,
    ) -> Vec<NodeIndex> {
        let mut distance_map: HashMap<NodeIndex, usize> = HashMap::new();
        let mut queue = VecDeque::new();

        queue.push_back((target_idx, 0usize));
        distance_map.insert(target_idx, 0);

        while let Some((curr, dist)) = queue.pop_front() {
            for edge in graph.graph.edges_directed(curr, Direction::Incoming) {
                let caller = edge.source();
                if corridor_nodes.contains(&caller) && !distance_map.contains_key(&caller) {
                    distance_map.insert(caller, dist + 1);
                    queue.push_back((caller, dist + 1));
                }
            }
        }

        let mut impacted: Vec<(NodeIndex, usize)> = distance_map.into_iter().collect();
        impacted.sort_by_key(|&(_, dist)| dist);
        impacted.into_iter().map(|(idx, _)| idx).collect()
    }

    fn compute_blast_radius(
        graph: &CodebaseGraph,
        node_idx: NodeIndex,
        corridor_nodes: &HashSet<NodeIndex>,
    ) -> BlastRadiusReport {
        let mut external_dependents = Vec::new();

        for edge in graph.graph.edges_directed(node_idx, Direction::Incoming) {
            let caller_idx = edge.source();
            if !corridor_nodes.contains(&caller_idx) {
                let caller_node = &graph.graph[caller_idx];
                external_dependents.push(ExternalDependent {
                    symbol_id: caller_node.symbol_id.clone(),
                    file_path: caller_node.file_path.clone(),
                    call_site_line: edge.weight().call_site_line,
                    edge_kind: edge.weight().kind,
                });
            }
        }

        let contagion_index = external_dependents.len();
        let risk_level = match contagion_index {
            0 => BlastRiskLevel::SafeInternal,
            1..=3 => BlastRiskLevel::Moderate,
            _ => BlastRiskLevel::HighContagion,
        };

        BlastRadiusReport {
            contagion_index,
            risk_level,
            external_dependents,
        }
    }
}
