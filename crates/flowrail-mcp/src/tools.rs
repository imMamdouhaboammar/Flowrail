use anyhow::{Result, anyhow};
use flowrail_core::graph::{CodebaseGraph, NodeRole};
use flowrail_core::pruning::PruningConfig;
use flowrail_core::slicer::{CorridorSlicer, GateStatus, SliceIntent};
use flowrail_core::state::CorridorSession;
use flowrail_ts_bridge::ast_slicer::TsAstSlicer;
use flowrail_ts_bridge::verifier::TsGateVerifier;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

pub struct CorridorMcpEngine {
    pub workspace_root: PathBuf,
    pub graph: CodebaseGraph,
    pub session: Option<CorridorSession>,
    pub run_tsc_on_advance: bool,
}

impl CorridorMcpEngine {
    pub fn new(workspace_root: PathBuf, graph: CodebaseGraph, run_tsc_on_advance: bool) -> Self {
        Self {
            workspace_root,
            graph,
            session: None,
            run_tsc_on_advance,
        }
    }

    pub fn list_tool_definitions() -> Value {
        json!({
            "tools": [
                {
                    "name": "corridor_discover_entrypoints",
                    "description": "Lists Entry Gates (HTTP routes, controllers, CLI handlers) in the repository without dumping file contents.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "filter": { "type": "string", "description": "Optional substring filter for symbol or file path" }
                        }
                    }
                },
                {
                    "name": "flowrail_discover_entrypoints",
                    "description": "Alias for corridor_discover_entrypoints",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "filter": { "type": "string", "description": "Optional substring filter for symbol or file path" }
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
                            "target_mutation_symbol": { "type": "string", "description": "Required when intent is MUTATE_CONTRACT" }
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
                            "target_mutation_symbol": { "type": "string", "description": "Required when intent is MUTATE_CONTRACT" }
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
                            "gate_index": { "type": "integer" }
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
                            "gate_index": { "type": "integer" }
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
                }
            ]
        })
    }

    pub fn dispatch_tool(&mut self, name: &str, args: &Value) -> Result<String> {
        match name {
            "corridor_discover_entrypoints" | "flowrail_discover_entrypoints" => {
                self.tool_discover_entrypoints(args)
            }
            "corridor_open" | "flowrail_open" => self.tool_open(args),
            "corridor_inspect_gate" | "flowrail_inspect_gate" => self.tool_inspect_gate(args),
            "corridor_propose_edit" | "flowrail_propose_edit" => self.tool_propose_edit(args),
            "corridor_advance_gate" | "flowrail_advance_gate" => self.tool_advance_gate(),
            "corridor_abort_or_rollback" | "flowrail_abort_or_rollback" => self.tool_rollback(args),
            _ => Err(anyhow!("Unknown Flowrail / Corridor MCP tool: {}", name)),
        }
    }

    fn tool_discover_entrypoints(&self, args: &Value) -> Result<String> {
        let filter = args
            .get("filter")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let mut entrypoints = Vec::new();
        for idx in self.graph.graph.node_indices() {
            let node = &self.graph.graph[idx];
            let is_candidate = node.role == NodeRole::EntryGate
                || (filter.len() > 2 && node.symbol_id.to_lowercase().contains(&filter));

            let matches_filter = filter.is_empty()
                || node.symbol_id.to_lowercase().contains(&filter)
                || node.file_path.to_lowercase().contains(&filter);

            if is_candidate && matches_filter {
                entrypoints.push(json!({
                    "symbol_id": node.symbol_id,
                    "display_name": node.display_name,
                    "file_path": node.file_path,
                    "line_range": node.line_range,
                    "role": node.role
                }));
            }
        }

        Ok(serde_json::to_string_pretty(&json!({
            "entrypoints_count": entrypoints.len(),
            "entrypoints": entrypoints
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
        let config = PruningConfig::default();

        let plan = CorridorSlicer::build_corridor(
            &self.graph,
            entry_symbol,
            intent,
            target_mutation,
            &config,
        )?;

        let session = CorridorSession::new(plan, self.workspace_root.clone());
        let gate_summaries: Vec<Value> = session
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
            .collect();

        let corridor_id = session.plan.corridor_id.clone();
        let pruned_count = session.plan.pruned_nodes.len();
        self.session = Some(session);

        let state_file = self.workspace_root.join(".corridor/active_session.json");
        let _ = self.session.as_ref().unwrap().save_to_disk(&state_file);

        Ok(serde_json::to_string_pretty(&json!({
            "corridor_id": corridor_id,
            "active_gate_index": 0,
            "gates_count": gate_summaries.len(),
            "pruned_utility_nodes_count": pruned_count,
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

        let gate = session.inspect_gate(gate_index)?;
        let abs_path = self.workspace_root.join(&gate.node.file_path);
        let full_text = fs::read_to_string(&abs_path).unwrap_or_default();

        let code_slice =
            TsAstSlicer::slice_lines(&full_text, gate.node.line_range.0, gate.node.line_range.1);

        Ok(serde_json::to_string_pretty(&json!({
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
        }))?)
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

        let rel_path = session
            .plan
            .gates
            .get(gate_index)
            .ok_or_else(|| anyhow!("Invalid gate_index"))?
            .node
            .file_path
            .clone();

        let abs_path = self.workspace_root.join(&rel_path);
        let original_content = session
            .checkpoints
            .get(&gate_index)
            .map(|c| c.original_content.clone())
            .unwrap_or_else(|| fs::read_to_string(&abs_path).unwrap_or_default());

        let pre_hash = TsAstSlicer::compute_file_contract_hash(&abs_path, &original_content)
            .unwrap_or_else(|_| "invalid_pre_ast".to_string());

        // Write proposed edit to working tree
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs_path, new_content)?;

        let post_hash = TsAstSlicer::compute_file_contract_hash(&abs_path, new_content)
            .unwrap_or_else(|_| "syntax_error_post_ast".to_string());

        let diagnostics = TsGateVerifier::verify_gate_edit(
            &self.workspace_root,
            &rel_path,
            new_content,
            self.run_tsc_on_advance,
        )?;

        session.record_proposed_edit(
            gate_index,
            original_content,
            new_content.to_string(),
            pre_hash.clone(),
            post_hash.clone(),
            diagnostics.clone(),
            ack_blast,
        )?;

        let state_file = self.workspace_root.join(".corridor/active_session.json");
        let _ = session.save_to_disk(&state_file);

        Ok(serde_json::to_string_pretty(&json!({
            "gate_index": gate_index,
            "file_path": rel_path,
            "contract_mutated": pre_hash != post_hash,
            "pre_contract_hash": pre_hash,
            "post_contract_hash": post_hash,
            "diagnostics_clean": diagnostics.is_empty(),
            "diagnostics": diagnostics,
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
            "unlocked_gate": next_gate_detail
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

        let reverted_paths: Vec<String> = reverted.into_iter().map(|(_, rel)| rel).collect();
        Ok(serde_json::to_string_pretty(&json!({
            "rolled_back_to_gate": to_gate,
            "reverted_files": reverted_paths,
            "active_phase": session.phase
        }))?)
    }
}
