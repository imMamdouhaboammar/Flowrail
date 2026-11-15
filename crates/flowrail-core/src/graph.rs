use indexmap::IndexMap;
use petgraph::Direction;
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeRole {
    EntryGate,
    BusinessLogic,
    ContractDefinition,
    DataRepository,
    ExternalBoundary,
    UtilityNoise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    Calls,
    ReferencesType,
    ImplementsInterface,
    MutatesState,
    ReadsProperty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticNode {
    pub symbol_id: String,
    pub display_name: String,
    pub file_path: String,
    pub byte_range: (usize, usize),
    pub line_range: (usize, usize),
    pub role: NodeRole,
    pub signature_hash: String,
    pub mutates_domain_state: bool,
    pub is_exported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticEdge {
    pub kind: EdgeKind,
    pub call_site_line: usize,
    #[serde(alias = "is_runtime_dynamic")]
    pub is_dynamic_trace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseGraph {
    pub graph: DiGraph<SemanticNode, SemanticEdge>,
    pub symbol_to_index: IndexMap<String, NodeIndex>,
}

impl Default for CodebaseGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CodebaseGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            symbol_to_index: IndexMap::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn add_node(&mut self, node: SemanticNode) -> NodeIndex {
        self.upsert_node(node)
    }

    pub fn upsert_node(&mut self, node: SemanticNode) -> NodeIndex {
        if let Some(&idx) = self.symbol_to_index.get(&node.symbol_id) {
            self.graph[idx] = node;
            idx
        } else {
            let symbol_id = node.symbol_id.clone();
            let idx = self.graph.add_node(node);
            self.symbol_to_index.insert(symbol_id, idx);
            idx
        }
    }

    pub fn get_node_index(&self, symbol_id: &str) -> Option<NodeIndex> {
        self.symbol_to_index.get(symbol_id).copied()
    }

    pub fn get_node(&self, idx: NodeIndex) -> Option<&SemanticNode> {
        self.graph.node_weight(idx)
    }

    pub fn get_node_mut(&mut self, idx: NodeIndex) -> Option<&mut SemanticNode> {
        self.graph.node_weight_mut(idx)
    }

    pub fn get_node_by_symbol(&self, symbol_id: &str) -> Option<&SemanticNode> {
        self.get_node_index(symbol_id)
            .and_then(|idx| self.get_node(idx))
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, edge: SemanticEdge) -> EdgeIndex {
        self.graph.add_edge(from, to, edge)
    }

    pub fn add_edge_by_symbols(
        &mut self,
        from_symbol: &str,
        to_symbol: &str,
        edge: SemanticEdge,
    ) -> Option<EdgeIndex> {
        let from_idx = *self.symbol_to_index.get(from_symbol)?;
        let to_idx = *self.symbol_to_index.get(to_symbol)?;
        Some(self.graph.add_edge(from_idx, to_idx, edge))
    }

    pub fn in_degree(&self, idx: NodeIndex) -> usize {
        self.graph.edges_directed(idx, Direction::Incoming).count()
    }

    pub fn out_degree_domain(&self, idx: NodeIndex) -> usize {
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .filter(|edge| {
                let target = &self.graph[edge.target()];
                target.role != NodeRole::UtilityNoise
            })
            .count()
    }
}
