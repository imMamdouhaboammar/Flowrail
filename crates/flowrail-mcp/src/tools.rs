use anyhow::{Result, anyhow};
use flowrail_core::artifact::{AiEnrichment, FlowrailArtifact};
use flowrail_core::frecency::FrecencyStore;
use flowrail_core::git::GitStatusResolver;
use flowrail_core::graph::{CodebaseGraph, NodeRole};
use flowrail_core::grep::{CorridorGrep, GrepScope};
use flowrail_core::pruning::PruningConfig;
use flowrail_core::search::FuzzySearchEngine;
use flowrail_core::slicer::{CorridorSlicer, GateStatus, SliceIntent};
use flowrail_core::state::CorridorSession;
use flowrail_ts_bridge::ast_slicer::TsAstSlicer;
use flowrail_ts_bridge::scip_loader::ScipGraphBuilder;
use flowrail_ts_bridge::verifier::TsGateVerifier;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub struct CorridorMcpEngine {
    pub workspace_root: PathBuf,
    pub graph: CodebaseGraph,
    pub session: Option<CorridorSession>,
    pub run_tsc_on_advance: bool,
    pub git_resolver: GitStatusResolver,
    pub frecency: FrecencyStore,
}

impl CorridorMcpEngine {
    pub fn new(workspace_root: PathBuf, graph: CodebaseGraph, run_tsc_on_advance: bool) -> Self {
        let git_resolver = GitStatusResolver::new(workspace_root.clone());
        let frecency = FrecencyStore::new();
        Self {
            workspace_root,
            graph,
            session: None,
            run_tsc_on_advance,
            git_resolver,
            frecency,
        }
    }

    pub fn list_tool_definitions() -> Value {
        json!({
            "tools": [
                {
                    "name": "corridor_discover_entrypoints",
                    "description": "Discovers Entry Gates (HTTP routes, controllers, CLI handlers) with Git status awareness, typo-tolerant fuzzy matching, and frecency ranking.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Fuzzy query with optional constraint syntax (e.g. '*.ts !tests/ checkout')" },
                            "filter": { "type": "string", "description": "Alias for query" },
                            "git_only": { "type": "boolean", "description": "If true, only returns symbols in modified, staged, or untracked files" },
                            "definitions_only": { "type": "boolean", "default": true, "description": "Filter to EntryGate and ContractDefinition symbols" },
                            "max_results": { "type": "integer", "default": 20 }
                        }
                    }
                },
                {
                    "name": "flowrail_discover_entrypoints",
                    "description": "Alias for corridor_discover_entrypoints",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Fuzzy query with optional constraint syntax (e.g. '*.ts !tests/ checkout')" },
                            "filter": { "type": "string", "description": "Alias for query" },
                            "git_only": { "type": "boolean", "description": "If true, only returns symbols in modified, staged, or untracked files" },
                            "definitions_only": { "type": "boolean", "default": true, "description": "Filter to EntryGate and ContractDefinition symbols" },
                            "max_results": { "type": "integer", "default": 20 }
                        }
                    }
                },
                {
                    "name": "corridor_find_symbols",
                    "description": "Instant typo-tolerant symbol search across all graph nodes with Git status annotations and frecency weighting.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["query"],
                        "properties": {
                            "query": { "type": "string", "description": "Symbol name, acronym (e.g. 'CC'), or path token" },
                            "definitions_only": { "type": "boolean", "default": false },
                            "git_only": { "type": "boolean", "default": false },
                            "max_results": { "type": "integer", "default": 25 }
                        }
                    }
                },
                {
                    "name": "flowrail_find_symbols",
                    "description": "Alias for corridor_find_symbols",
                    "inputSchema": {
                        "type": "object",
                        "required": ["query"],
                        "properties": {
                            "query": { "type": "string", "description": "Symbol name, acronym (e.g. 'CC'), or path token" },
                            "definitions_only": { "type": "boolean", "default": false },
                            "git_only": { "type": "boolean", "default": false },
                            "max_results": { "type": "integer", "default": 25 }
                        }
                    }
                },
                {
                    "name": "corridor_grep",
                    "description": "High-performance in-memory content search. Scopes search to the active business corridor slice (default) or entire workspace with definition-first filtering.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["query"],
                        "properties": {
                            "query": { "type": "string", "description": "Search term or regex pattern" },
                            "scope": { "type": "string", "enum": ["corridor", "workspace"], "default": "corridor", "description": "corridor restricts grep strictly to files in the active corridor session to prevent context pollution" },
                            "definitions_only": { "type": "boolean", "default": false, "description": "Restricts matches to declaration lines (export function, interface, class, type)" },
                            "case_sensitive": { "type": "boolean", "description": "Smart-case applies when omitted" },
                            "context_lines": { "type": "integer", "default": 1 },
                            "max_matches": { "type": "integer", "default": 30 }
                        }
                    }
                },
                {
                    "name": "flowrail_grep",
                    "description": "Alias for corridor_grep",
                    "inputSchema": {
                        "type": "object",
                        "required": ["query"],
                        "properties": {
                            "query": { "type": "string", "description": "Search term or regex pattern" },
                            "scope": { "type": "string", "enum": ["corridor", "workspace"], "default": "corridor" },
                            "definitions_only": { "type": "boolean", "default": false },
                            "case_sensitive": { "type": "boolean" },
                            "context_lines": { "type": "integer", "default": 1 },
                            "max_matches": { "type": "integer", "default": 30 }
                        }
                    }
                },
                {
                    "name": "corridor_open",
                    "description": "Initializes a strict refactoring/reading corridor state machine and unlocks Gate 0.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["entry_symbol", "intent"],
                        "properties": {
                            "entry_symbol": { "type": "string", "description": "Exact symbol_id of the Entry Gate" },
                            "intent": { "type": "string", "enum": ["READ_FLOW", "MUTATE_CONTRACT"] },
                            "target_mutation_symbol": { "type": "string", "description": "Required when intent is MUTATE_CONTRACT" },
                            "detail_level": { "type": "string", "enum": ["minimal", "full"], "default": "full", "description": "Token efficiency mode. 'minimal' yields a concise summary." },
                            "max_depth": { "type": "integer", "description": "Maximum corridor traversal depth (Augment Code pattern). Default: 12." }
                        }
                    }
                },
                {
                    "name": "flowrail_open",
                    "description": "Alias for corridor_open",
                    "inputSchema": {
                        "type": "object",
                        "required": ["entry_symbol", "intent"],
                        "properties": {
                            "entry_symbol": { "type": "string", "description": "Exact symbol_id of the Entry Gate" },
                            "intent": { "type": "string", "enum": ["READ_FLOW", "MUTATE_CONTRACT"] },
                            "target_mutation_symbol": { "type": "string", "description": "Required when intent is MUTATE_CONTRACT" },
                            "detail_level": { "type": "string", "enum": ["minimal", "full"], "default": "full" },
                            "max_depth": { "type": "integer" }
                        }
                    }
                },
                {
                    "name": "corridor_inspect_gate",
                    "description": "Returns the exact source code slice, upstream contracts, and Blast Radius report for an UNLOCKED gate. Rejects LOCKED future gates.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["gate_index"],
                        "properties": {
                            "gate_index": { "type": "integer" },
                            "compact": { "type": "boolean", "default": false, "description": "When true, returns summary and contracts without dumping raw file lines" },
                            "detail_level": { "type": "string", "enum": ["minimal", "full"], "description": "Token efficiency mode. 'minimal' omits source code lines." }
                        }
                    }
                },
                {
                    "name": "flowrail_inspect_gate",
                    "description": "Alias for corridor_inspect_gate",
                    "inputSchema": {
                        "type": "object",
                        "required": ["gate_index"],
                        "properties": {
                            "gate_index": { "type": "integer" },
                            "compact": { "type": "boolean", "default": false },
                            "detail_level": { "type": "string", "enum": ["minimal", "full"] }
                        }
                    }
                },
                {
                    "name": "corridor_propose_edit",
                    "description": "Applies an edit to the active gate, hashes exported TypeScript contracts to detect mutations, and runs syntax/type diagnostics.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["gate_index", "new_file_content"],
                        "properties": {
                            "gate_index": { "type": "integer" },
                            "new_file_content": { "type": "string" },
                            "acknowledge_blast_radius": { "type": "boolean", "default": false }
                        }
                    }
                },
                {
                    "name": "flowrail_propose_edit",
                    "description": "Alias for corridor_propose_edit",
                    "inputSchema": {
                        "type": "object",
                        "required": ["gate_index", "new_file_content"],
                        "properties": {
                            "gate_index": { "type": "integer" },
                            "new_file_content": { "type": "string" },
                            "acknowledge_blast_radius": { "type": "boolean", "default": false }
                        }
                    }
                },
                {
                    "name": "corridor_advance_gate",
                    "description": "Verifies that the active gate compiles and has no unacknowledged cross-corridor contagion, then locks it and unlocks Gate N+1.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "flowrail_advance_gate",
                    "description": "Alias for corridor_advance_gate",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "corridor_abort_or_rollback",
                    "description": "Reverts file modifications back to a specific gate checkpoint and restores gate locks.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["to_gate_index"],
                        "properties": {
                            "to_gate_index": { "type": "integer" }
                        }
                    }
                },
                {
                    "name": "flowrail_abort_or_rollback",
                    "description": "Alias for corridor_abort_or_rollback",
                    "inputSchema": {
                        "type": "object",
                        "required": ["to_gate_index"],
                        "properties": {
                            "to_gate_index": { "type": "integer" }
                        }
                    }
                },
                {
                    "name": "corridor_export_artifact",
                    "description": "Exports the active corridor session or plan as a Clean/Clear JSON artifact enforcing ADHD and Anti-Slop standards. Saves to .flowrail/artifacts/latest.json.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "save_to_disk": { "type": "boolean", "default": true, "description": "Whether to write artifact to .flowrail/artifacts/" }
                        }
                    }
                },
                {
                    "name": "flowrail_export_artifact",
                    "description": "Alias for corridor_export_artifact",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "save_to_disk": { "type": "boolean", "default": true }
                        }
                    }
                },
                {
                    "name": "corridor_enrich_artifact",
                    "description": "Applies and validates AI enrichment on the active corridor artifact without hallucination, enforcing ADHD rules (lead with action, <=5 steps, terminal action) and Anti-Slop gates (grounded symbols/files, zero conversational fluff).",
                    "inputSchema": {
                        "type": "object",
                        "required": ["enrichment"],
                        "properties": {
                            "enrichment": {
                                "type": "object",
                                "description": "Enrichment payload containing adhd_view, anti_slop_contract, and human_readable_summary"
                            }
                        }
                    }
                },
                {
                    "name": "flowrail_enrich_artifact",
                    "description": "Alias for corridor_enrich_artifact",
                    "inputSchema": {
                        "type": "object",
                        "required": ["enrichment"],
                        "properties": {
                            "enrichment": {
                                "type": "object",
                                "description": "Enrichment payload containing adhd_view, anti_slop_contract, and human_readable_summary"
                            }
                        }
                    }
                }
            ]
        })
    }

    pub fn dispatch_tool(&mut self, name: &str, args: &Value) -> Result<String> {
        match name {
            "corridor_discover_entrypoints" | "flowrail_discover_entrypoints" => {
                self.tool_discover_entrypoints(args)
            }
            "corridor_find_symbols" | "flowrail_find_symbols" => self.tool_find_symbols(args),
            "corridor_grep" | "flowrail_grep" => self.tool_grep(args),
            "corridor_open" | "flowrail_open" => self.tool_open(args),
            "corridor_inspect_gate" | "flowrail_inspect_gate" => self.tool_inspect_gate(args),
            "corridor_propose_edit" | "flowrail_propose_edit" => self.tool_propose_edit(args),
            "corridor_advance_gate" | "flowrail_advance_gate" => self.tool_advance_gate(),
            "corridor_abort_or_rollback" | "flowrail_abort_or_rollback" => self.tool_rollback(args),
            "corridor_export_artifact" | "flowrail_export_artifact" => {
                self.tool_export_artifact(args)
            }
            "corridor_enrich_artifact" | "flowrail_enrich_artifact" => {
                self.tool_enrich_artifact(args)
            }
            _ => Err(anyhow!("Unknown Flowrail / Corridor MCP tool: {}", name)),
        }
    }

    fn tool_discover_entrypoints(&self, args: &Value) -> Result<String> {
        let query = args
            .get("query")
            .or_else(|| args.get("filter"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let git_only = args
            .get("git_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let definitions_only = args
            .get("definitions_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| (v as usize).clamp(1, 100))
            .unwrap_or(20);

        let search_engine =
            FuzzySearchEngine::new(&self.graph, Some(&self.git_resolver), Some(&self.frecency));

        let hits = search_engine.search(query, definitions_only, git_only, max_results);
        let pagerank = self.graph.compute_pagerank(0.85, 20);

        let mut entrypoints: Vec<Value> = hits
            .into_iter()
            .map(|h| {
                let pr_score = pagerank.get(&h.symbol_id).copied().unwrap_or(0.0);
                let final_score = h.score + (pr_score * 50.0);
                json!({
                    "symbol_id": h.symbol_id,
                    "display_name": h.display_name,
                    "file_path": h.file_path,
                    "line_range": h.line_range,
                    "role": h.role,
                    "pagerank_score": (pr_score * 1000.0).round() / 1000.0,
                    "git_status": h.git_status,
                    "git_badge": h.git_status.badge(),
                    "score": final_score,
                    "frecency_score": h.frecency_score,
                    "match_stage": h.stage
                })
            })
            .collect();

        entrypoints.sort_by(|a, b| {
            let sa = a["score"].as_f64().unwrap_or(0.0);
            let sb = b["score"].as_f64().unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(serde_json::to_string_pretty(&json!({
            "entrypoints_count": entrypoints.len(),
            "entrypoints": entrypoints
        }))?)
    }

    fn tool_find_symbols(&self, args: &Value) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing required parameter 'query'"))?;

        let definitions_only = args
            .get("definitions_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let git_only = args
            .get("git_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| (v as usize).clamp(1, 100))
            .unwrap_or(25);

        let search_engine =
            FuzzySearchEngine::new(&self.graph, Some(&self.git_resolver), Some(&self.frecency));

        let hits = search_engine.search(query, definitions_only, git_only, max_results);

        let symbols: Vec<Value> = hits
            .into_iter()
            .map(|h| {
                json!({
                    "symbol_id": h.symbol_id,
                    "display_name": h.display_name,
                    "file_path": h.file_path,
                    "line_range": h.line_range,
                    "def_location": format!("{}:{}", h.file_path, h.line_range.0),
                    "is_definition": matches!(h.role, NodeRole::ContractDefinition | NodeRole::EntryGate),
                    "role": h.role,
                    "git_status": h.git_status,
                    "git_badge": h.git_status.badge(),
                    "score": h.score,
                    "frecency_score": h.frecency_score
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&json!({
            "query": query,
            "symbols_count": symbols.len(),
            "symbols": symbols
        }))?)
    }

    fn tool_grep(&self, args: &Value) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing required parameter 'query'"))?;

        let scope_str = args
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                if self.session.is_some() {
                    "corridor"
                } else {
                    "workspace"
                }
            });

        let scope = match scope_str {
            "corridor" => GrepScope::Corridor,
            "workspace" => GrepScope::Workspace,
            _ => {
                return Err(anyhow!(
                    "Invalid scope '{}'. Use 'corridor' or 'workspace'",
                    scope_str
                ));
            }
        };

        let definitions_only = args
            .get("definitions_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let case_sensitive = args.get("case_sensitive").and_then(|v| v.as_bool());
        let context_lines = args
            .get("context_lines")
            .and_then(|v| v.as_u64())
            .map(|v| (v as usize).min(5))
            .unwrap_or(1);
        let max_matches = args
            .get("max_matches")
            .and_then(|v| v.as_u64())
            .map(|v| (v as usize).clamp(1, 100))
            .unwrap_or(30);

        let target_files = match scope {
            GrepScope::Corridor => {
                let session = self
                    .session
                    .as_ref()
                    .ok_or_else(|| anyhow!("No active corridor session for corridor-scoped grep. Pass scope: 'workspace' or call corridor_open first."))?;

                let mut set = HashSet::new();
                for gate in session
                    .plan
                    .gates
                    .iter()
                    .filter(|g| !matches!(g.status, GateStatus::Locked))
                {
                    set.insert(self.workspace_root.join(&gate.node.file_path));
                }
                set.into_iter().collect::<Vec<_>>()
            }
            GrepScope::Workspace => {
                let mut files = Vec::new();
                ScipGraphBuilder::collect_ts_files(&self.workspace_root, &mut files)?;
                files
            }
        };

        let report = CorridorGrep::search(
            &self.workspace_root,
            &target_files,
            query,
            definitions_only,
            case_sensitive,
            context_lines,
            max_matches,
            Some(&self.git_resolver),
        )?;

        let formatted = CorridorGrep::format_report_compact(&report);

        Ok(serde_json::to_string_pretty(&json!({
            "query": query,
            "scope": scope_str,
            "total_matches": report.total_matches,
            "files_matched": report.files.len(),
            "formatted_text": formatted,
            "report": report
        }))?)
    }

    fn tool_open(&mut self, args: &Value) -> Result<String> {
        let entry_symbol = args
            .get("entry_symbol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing required argument: entry_symbol"))?;

        let intent_str = args
            .get("intent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing required argument: intent"))?;

        let intent = match intent_str {
            "READ_FLOW" => SliceIntent::ReadFlow,
            "MUTATE_CONTRACT" => SliceIntent::MutateContract,
            _ => {
                return Err(anyhow!(
                    "Invalid intent '{}'. Use READ_FLOW or MUTATE_CONTRACT",
                    intent_str
                ));
            }
        };

        let target_mutation = args.get("target_mutation_symbol").and_then(|v| v.as_str());
        let mut config = PruningConfig::default();
        if let Some(md) = args.get("max_depth").and_then(|v| v.as_u64()) {
            config.max_corridor_depth = (md as usize).clamp(1, 100);
        }

        let plan = CorridorSlicer::build_corridor(
            &self.graph,
            entry_symbol,
            intent,
            target_mutation,
            &config,
        )?;

        // Record frecency access
        self.frecency.record_access(entry_symbol);

        let detail_level = args
            .get("detail_level")
            .and_then(|v| v.as_str())
            .unwrap_or("full");
        let is_minimal = detail_level == "minimal";

        let mut session = CorridorSession::new(plan, self.workspace_root.clone());

        let target_files: Vec<String> = session
            .plan
            .gates
            .iter()
            .map(|g| g.node.file_path.clone())
            .collect();

        let co_changes =
            GitStatusResolver::find_co_changed_files(&self.workspace_root, &target_files, 100, 2);
        let co_change_paths: Vec<String> = co_changes.into_iter().map(|c| c.file_path).collect();

        if !co_change_paths.is_empty() && !session.plan.gates.is_empty() {
            session.plan.gates[0].blast_radius.co_changed_files = co_change_paths.clone();
        }

        let gate_summaries: Vec<Value> = if is_minimal {
            session
                .plan
                .gates
                .iter()
                .map(|g| {
                    json!({
                        "gate_index": g.gate_index,
                        "status": g.status,
                        "symbol_id": g.node.symbol_id,
                        "blast_risk": g.blast_radius.risk_level
                    })
                })
                .collect()
        } else {
            session
                .plan
                .gates
                .iter()
                .map(|g| {
                    json!({
                        "gate_index": g.gate_index,
                        "status": g.status,
                        "symbol_id": g.node.symbol_id,
                        "file_path": g.node.file_path,
                        "role": g.node.role,
                        "blast_radius_risk": g.blast_radius.risk_level,
                        "external_dependents_count": g.blast_radius.contagion_index
                    })
                })
                .collect()
        };

        let corridor_id = session.plan.corridor_id.clone();
        let built_at_commit = session.built_at_commit.clone();
        let pruned_count = session.plan.pruned_nodes.len();
        self.session = Some(session);

        let state_file = self.workspace_root.join(".corridor/active_session.json");
        let _ = self.session.as_ref().unwrap().save_to_disk(&state_file);

        let artifact = FlowrailArtifact::from_session(
            self.session.as_ref().unwrap(),
            Some(&self.git_resolver),
        );
        let artifact_path = artifact.save_to_workspace(&self.workspace_root).ok();

        Ok(serde_json::to_string_pretty(&json!({
            "corridor_id": corridor_id,
            "artifact_id": artifact.artifact_id,
            "artifact_file": artifact_path.map(|p| p.to_string_lossy().to_string()),
            "built_at_commit": built_at_commit,
            "active_gate_index": 0,
            "gates_count": gate_summaries.len(),
            "pruned_utility_nodes_count": pruned_count,
            "co_changed_files_warning": if co_change_paths.is_empty() { Value::Null } else { json!(co_change_paths) },
            "roadmap": gate_summaries,
            "instruction": "Gate 0 is now UNLOCKED. Call `corridor_inspect_gate(0)` to read its code slice and begin."
        }))?)
    }

    fn tool_inspect_gate(&self, args: &Value) -> Result<String> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("No active corridor session. Call corridor_open first."))?;

        let gate_index = args
            .get("gate_index")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing gate_index"))? as usize;

        let detail_level = args.get("detail_level").and_then(|v| v.as_str());
        let compact = args
            .get("compact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || detail_level == Some("minimal");

        let freshness = session.check_git_freshness();

        let gate = session.inspect_gate(gate_index)?;
        let abs_path = self.workspace_root.join(&gate.node.file_path);

        // Record frecency
        self.frecency.record_access(&gate.node.symbol_id);

        let mut output = if compact {
            json!({
                "gate_index": gate.gate_index,
                "status": gate.status,
                "symbol_id": gate.node.symbol_id,
                "file_path": gate.node.file_path,
                "line_range": gate.node.line_range,
                "role": gate.node.role,
                "upstream_contracts": gate.upstream_contracts,
                "blast_radius": gate.blast_radius,
                "mode": "compact_token_optimized"
            })
        } else {
            let full_text = fs::read_to_string(&abs_path).unwrap_or_default();
            let code_slice = TsAstSlicer::slice_lines(
                &full_text,
                gate.node.line_range.0,
                gate.node.line_range.1,
            );

            json!({
                "gate_index": gate.gate_index,
                "status": gate.status,
                "symbol_id": gate.node.symbol_id,
                "file_path": gate.node.file_path,
                "line_range": gate.node.line_range,
                "role": gate.node.role,
                "upstream_contracts": gate.upstream_contracts,
                "downstream_in_corridor": gate.downstream_in_corridor,
                "blast_radius": gate.blast_radius,
                "code_slice": code_slice
            })
        };

        if freshness.is_stale {
            output["git_freshness_warning"] = json!({
                "is_stale": true,
                "built_at_commit": freshness.built_at_commit,
                "current_commit": freshness.current_commit,
                "warning": "HEAD has moved since corridor was opened. Consider verifying or re-opening corridor."
            });
        }

        Ok(serde_json::to_string_pretty(&output)?)
    }

    fn tool_propose_edit(&mut self, args: &Value) -> Result<String> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active corridor session."))?;

        let gate_index = args
            .get("gate_index")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing gate_index"))? as usize;

        let new_content = args
            .get("new_file_content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing new_file_content"))?;

        let ack_blast = args
            .get("acknowledge_blast_radius")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let active_idx = session.current_gate_index()?;
        if gate_index != active_idx {
            return Err(anyhow!(
                "OUT_OF_ORDER_EDIT: Cannot edit Gate {} while active gate is {}.",
                gate_index,
                active_idx
            ));
        }

        let gate = session
            .plan
            .gates
            .get(gate_index)
            .ok_or_else(|| anyhow!("Invalid gate_index"))?;

        if gate.status == GateStatus::Locked {
            return Err(anyhow!(
                "VISIBILITY_LOCK_VIOLATION: Gate {} ({}) is currently LOCKED.",
                gate_index,
                gate.node.symbol_id
            ));
        }

        let rel_path = gate.node.file_path.clone();
        let abs_path = self.workspace_root.join(&rel_path);
        let (original_content, file_existed_originally) =
            session.get_or_snapshot_file(gate_index, &rel_path);

        let pre_contracts = TsAstSlicer::extract_exported_contracts(&abs_path, &original_content)
            .unwrap_or_default();
        let pre_hash = TsAstSlicer::compute_file_contract_hash(&abs_path, &original_content)
            .unwrap_or_else(|_| "invalid_pre_ast".to_string());

        // Write proposed edit to working tree
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs_path, new_content)?;

        // Record frecency and invalidate git cache
        self.frecency.record_access(&rel_path);
        self.git_resolver.invalidate();

        let post_contracts =
            TsAstSlicer::extract_exported_contracts(&abs_path, new_content).unwrap_or_default();
        let post_hash = TsAstSlicer::compute_file_contract_hash(&abs_path, new_content)
            .unwrap_or_else(|_| "syntax_error_post_ast".to_string());
        let contract_diff =
            flowrail_core::contract::ContractHasher::diff_surfaces(&pre_contracts, &post_contracts);

        let diagnostics = TsGateVerifier::verify_gate_edit(
            &self.workspace_root,
            &rel_path,
            new_content,
            self.run_tsc_on_advance,
        )?;

        session.record_proposed_edit(
            gate_index,
            original_content,
            file_existed_originally,
            new_content.to_string(),
            pre_hash.clone(),
            post_hash.clone(),
            diagnostics.clone(),
            ack_blast,
        )?;

        let state_file = self.workspace_root.join(".corridor/active_session.json");
        let _ = session.save_to_disk(&state_file);

        let artifact = FlowrailArtifact::from_session(session, Some(&self.git_resolver));
        let artifact_path = artifact.save_to_workspace(&self.workspace_root).ok();

        Ok(serde_json::to_string_pretty(&json!({
            "gate_index": gate_index,
            "file_path": rel_path,
            "contract_mutated": pre_hash != post_hash,
            "contract_delta_kind": contract_diff.delta_kind,
            "is_breaking": contract_diff.is_breaking,
            "breaking_reasons": contract_diff.breaking_reasons,
            "pre_contract_hash": pre_hash,
            "post_contract_hash": post_hash,
            "diagnostics_clean": diagnostics.is_empty(),
            "diagnostics": diagnostics,
            "artifact_id": artifact.artifact_id,
            "artifact_file": artifact_path.map(|p| p.to_string_lossy().to_string()),
            "next_step": if diagnostics.is_empty() {
                "Call `corridor_advance_gate` to lock this gate and unlock the next one."
            } else {
                "Fix the reported diagnostics in this gate and call `corridor_propose_edit` again."
            }
        }))?)
    }

    fn tool_advance_gate(&mut self) -> Result<String> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active corridor session."))?;

        let new_phase = session.advance_gate()?;
        let state_file = self.workspace_root.join(".corridor/active_session.json");
        let _ = session.save_to_disk(&state_file);

        let artifact = FlowrailArtifact::from_session(session, Some(&self.git_resolver));
        let artifact_path = artifact.save_to_workspace(&self.workspace_root).ok();

        let unlocked_gate_idx = session
            .plan
            .gates
            .iter()
            .find(|g| g.status == GateStatus::Unlocked)
            .map(|g| g.gate_index);

        let next_gate_detail = if let Some(idx) = unlocked_gate_idx {
            let raw = self.tool_inspect_gate(&json!({ "gate_index": idx }))?;
            serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null)
        } else {
            Value::Null
        };

        Ok(serde_json::to_string_pretty(&json!({
            "session_phase": new_phase,
            "unlocked_gate": next_gate_detail,
            "artifact_id": artifact.artifact_id,
            "artifact_file": artifact_path.map(|p| p.to_string_lossy().to_string())
        }))?)
    }

    fn tool_rollback(&mut self, args: &Value) -> Result<String> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active corridor session."))?;

        let to_gate = args
            .get("to_gate_index")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing to_gate_index"))? as usize;

        let reverted = session.rollback_to_gate(to_gate)?;
        let state_file = self.workspace_root.join(".corridor/active_session.json");
        let _ = session.save_to_disk(&state_file);

        self.git_resolver.invalidate();

        let artifact = FlowrailArtifact::from_session(session, Some(&self.git_resolver));
        let artifact_path = artifact.save_to_workspace(&self.workspace_root).ok();

        let reverted_paths: Vec<String> = reverted.into_iter().map(|(_, rel)| rel).collect();
        Ok(serde_json::to_string_pretty(&json!({
            "rolled_back_to_gate": to_gate,
            "reverted_files": reverted_paths,
            "active_phase": session.phase,
            "artifact_id": artifact.artifact_id,
            "artifact_file": artifact_path.map(|p| p.to_string_lossy().to_string())
        }))?)
    }

    fn tool_export_artifact(&self, args: &Value) -> Result<String> {
        let session = self.session.as_ref().ok_or_else(|| {
            anyhow!("No active corridor session to export artifact. Call corridor_open first.")
        })?;

        let save_to_disk = args
            .get("save_to_disk")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let artifact = FlowrailArtifact::from_session(session, Some(&self.git_resolver));

        let saved_file = if save_to_disk {
            let p = artifact.save_to_workspace(&self.workspace_root)?;
            Some(p.to_string_lossy().to_string())
        } else {
            None
        };

        Ok(serde_json::to_string_pretty(&json!({
            "artifact_id": artifact.artifact_id,
            "saved_to_file": saved_file,
            "clean_json_artifact": artifact
        }))?)
    }

    fn tool_enrich_artifact(&mut self, args: &Value) -> Result<String> {
        let session = self.session.as_ref().ok_or_else(|| {
            anyhow!("No active corridor session to enrich. Call corridor_open first.")
        })?;

        let enrichment_val = args
            .get("enrichment")
            .ok_or_else(|| anyhow!("Missing required parameter 'enrichment'"))?;

        let enrichment: AiEnrichment = serde_json::from_value(enrichment_val.clone())
            .map_err(|e| anyhow!("Invalid AiEnrichment schema: {}", e))?;

        let mut artifact = FlowrailArtifact::from_session(session, Some(&self.git_resolver));
        artifact.apply_and_validate_enrichment(enrichment)?;

        let saved_path = artifact.save_to_workspace(&self.workspace_root)?;

        Ok(serde_json::to_string_pretty(&json!({
            "status": "ENRICHMENT_VERIFIED_AND_APPLIED",
            "artifact_id": artifact.artifact_id,
            "saved_to_file": saved_path.to_string_lossy().to_string(),
            "clean_json_artifact": artifact
        }))?)
    }
}
