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

    /// Resolves a symbol query to a NodeIndex using prioritized matching:
    /// 1. Exact match on symbol_id
    /// 2. Exact match on display_name (preferring exported & non-utility nodes)
    /// 3. File + symbol qualifier match ("file.ts:Symbol" or "file.ts#Symbol")
    /// 4. Suffix match on symbol_id (e.g. SCIP URI ending in "#Symbol" or "/Symbol#" or ".Symbol")
    /// 5. Case-insensitive substring/prefix match on display_name
    pub fn resolve_symbol(&self, query: &str) -> Option<NodeIndex> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return None;
        }

        // 1. Exact symbol_id match
        if let Some(&idx) = self.symbol_to_index.get(trimmed_query) {
            return Some(idx);
        }

        // 2. Exact display_name match
        let mut candidates: Vec<(NodeIndex, &SemanticNode)> = Vec::new();
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            if node.display_name == trimmed_query {
                candidates.push((idx, node));
            }
        }
        if !candidates.is_empty() {
            candidates.sort_by_key(|(_, n)| {
                let role_score = match n.role {
                    NodeRole::EntryGate => 0,
                    NodeRole::BusinessLogic => 1,
                    NodeRole::ContractDefinition => 2,
                    NodeRole::DataRepository => 3,
                    NodeRole::ExternalBoundary => 4,
                    NodeRole::UtilityNoise => 5,
                };
                let export_score = if n.is_exported { 0 } else { 1 };
                (export_score, role_score, n.file_path.len())
            });
            return Some(candidates[0].0);
        }

        // 3. Qualified path match (e.g. "checkout.ts#process" or "src/checkout.ts:CheckoutController")
        if let Some((file_hint, name_hint)) = trimmed_query
            .split_once(':')
            .or_else(|| trimmed_query.split_once('#'))
        {
            let file_hint = file_hint.trim();
            let name_hint = name_hint.trim();
            for idx in self.graph.node_indices() {
                let node = &self.graph[idx];
                let path_matches = node.file_path.ends_with(file_hint);
                let name_matches = node.display_name == name_hint
                    || node.display_name.ends_with(name_hint)
                    || node.symbol_id.ends_with(name_hint);
                if path_matches && name_matches {
                    return Some(idx);
                }
            }
        }

        // 4. SCIP-style symbol suffix match (e.g. "CheckoutController" matching ".../checkout.controller.ts/CheckoutController#")
        let scip_suffix_hash = format!("{trimmed_query}#");
        let scip_suffix_paren = format!("{trimmed_query}()");
        let scip_suffix_dot = format!(".{trimmed_query}");
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            if node.symbol_id.ends_with(&scip_suffix_hash)
                || node.symbol_id.ends_with(&scip_suffix_paren)
                || node.symbol_id.ends_with(&scip_suffix_dot)
                || node.symbol_id.ends_with(trimmed_query)
            {
                candidates.push((idx, node));
            }
        }
        if !candidates.is_empty() {
            candidates.sort_by_key(|(_, n)| {
                let export_score = if n.is_exported { 0 } else { 1 };
                (export_score, n.symbol_id.len())
            });
            return Some(candidates[0].0);
        }

        // 5. Case-insensitive display_name match
        let query_lower = trimmed_query.to_lowercase();
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            if node.display_name.to_lowercase() == query_lower {
                return Some(idx);
            }
        }

        None
    }

    pub fn resolve_node(&self, query: &str) -> Option<&SemanticNode> {
        self.resolve_symbol(query)
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
