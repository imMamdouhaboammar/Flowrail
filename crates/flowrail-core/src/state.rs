use crate::graph::{NodeRole, SemanticNode};
use crate::slicer::{BlastRadiusReport, CorridorGate, CorridorPlan, GateStatus};
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionPhase {
    Idle,
    ActiveGate(usize),
    BlockedAtGate { gate_index: usize, reason: String },
    Completed,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCheckpoint {
    pub gate_index: usize,
    pub file_path: String,
    pub original_content: String,
    #[serde(default = "default_true")]
    pub file_existed_originally: bool,
    pub proposed_content: Option<String>,
    pub pre_edit_contract_hash: String,
    pub post_edit_contract_hash: Option<String>,
    pub blast_radius_acknowledged: bool,
    pub diagnostics_clean: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitFreshness {
    pub is_stale: bool,
    pub built_at_commit: Option<String>,
    pub current_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorSession {
    pub workspace_root: PathBuf,
    pub plan: CorridorPlan,
    pub phase: SessionPhase,
    pub checkpoints: HashMap<usize, GateCheckpoint>,
    #[serde(default)]
    pub built_at_commit: Option<String>,
}

impl CorridorSession {
    pub fn new(plan: CorridorPlan, workspace_root: PathBuf) -> Self {
        let phase = if plan.gates.is_empty() {
            SessionPhase::Completed
        } else {
            SessionPhase::ActiveGate(0)
        };
        let built_at_commit = crate::git::GitStatusResolver::get_head_commit(&workspace_root);

        Self {
            workspace_root,
            plan,
            phase,
            checkpoints: HashMap::new(),
            built_at_commit,
        }
    }

    pub fn check_git_freshness(&self) -> GitFreshness {
        let current_commit = crate::git::GitStatusResolver::get_head_commit(&self.workspace_root);
        let is_stale = match (&self.built_at_commit, &current_commit) {
            (Some(built), Some(current)) => built != current,
            _ => false,
        };
        GitFreshness {
            is_stale,
            built_at_commit: self.built_at_commit.clone(),
            current_commit,
        }
    }

    pub fn current_gate_index(&self) -> Result<usize> {
        match &self.phase {
            SessionPhase::ActiveGate(idx) => Ok(*idx),
            SessionPhase::BlockedAtGate { gate_index, .. } => Ok(*gate_index),
            SessionPhase::Idle => bail!("No active corridor session. Call corridor_open first."),
            SessionPhase::Completed => bail!("Corridor session is already completed."),
        }
    }

    /// Enforces Visibility Lock: Rejects full source inspection for future Locked gates.
    pub fn inspect_gate(&self, requested_index: usize) -> Result<&CorridorGate> {
        let gate = self
            .plan
            .gates
            .get(requested_index)
            .ok_or_else(|| anyhow!("Gate index {} out of bounds", requested_index))?;

        if gate.status == GateStatus::Locked {
            bail!(
                "VISIBILITY_LOCK_VIOLATION: Gate {} ({}) is currently LOCKED. \
                Complete and verify Gate {} first to prevent context pollution.",
                requested_index,
                gate.node.symbol_id,
                self.current_gate_index().unwrap_or(0)
            );
        }

        Ok(gate)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_proposed_edit(
        &mut self,
        gate_index: usize,
        original_content: String,
        file_existed_originally: bool,
        proposed_content: String,
        pre_hash: String,
        post_hash: String,
        diagnostics_errors: Vec<String>,
        acknowledge_blast_radius: bool,
    ) -> Result<()> {
        let active_idx = self.current_gate_index()?;
        if gate_index != active_idx {
            bail!(
                "OUT_OF_ORDER_EDIT: Cannot edit Gate {} while active gate is {}.",
                gate_index,
                active_idx
            );
        }

        let diagnostics_clean = diagnostics_errors.is_empty();
        let chk = GateCheckpoint {
            gate_index,
            file_path: self.plan.gates[gate_index].node.file_path.clone(),
            original_content,
            file_existed_originally,
            proposed_content: Some(proposed_content),
            pre_edit_contract_hash: pre_hash,
            post_edit_contract_hash: Some(post_hash),
            blast_radius_acknowledged: acknowledge_blast_radius,
            diagnostics_clean,
        };

        self.checkpoints.insert(gate_index, chk);

        if !diagnostics_clean {
            let err_summary = diagnostics_errors.join("\n");
            self.plan.gates[gate_index].status = GateStatus::Blocked;
            self.phase = SessionPhase::BlockedAtGate {
                gate_index,
                reason: format!("TypeScript syntax/semantic errors:\n{}", err_summary),
            };
        } else {
            self.phase = SessionPhase::ActiveGate(gate_index);
        }

        Ok(())
    }

    pub fn get_or_snapshot_file(&self, gate_index: usize, file_path: &str) -> (String, bool) {
        let abs_path = self.workspace_root.join(file_path);
        let file_existed = abs_path.exists();
        let original_content = if let Some(existing) = self.checkpoints.get(&gate_index) {
            existing.original_content.clone()
        } else if file_existed {
            std::fs::read_to_string(&abs_path).unwrap_or_default()
        } else {
            String::new()
        };
        let file_existed_originally = self
            .checkpoints
            .get(&gate_index)
            .map(|c| c.file_existed_originally)
            .unwrap_or(file_existed);
        (original_content, file_existed_originally)
    }

    pub fn propose_gate_edit(
        &mut self,
        gate_index: usize,
        file_path: &str,
        proposed_content: &str,
        post_contract_hash: &str,
        acknowledge_blast_radius: bool,
    ) -> Result<()> {
        let abs_path = self.workspace_root.join(file_path);
        let (original_content, file_existed_originally) =
            self.get_or_snapshot_file(gate_index, file_path);

        let pre_hash = self
            .checkpoints
            .get(&gate_index)
            .map(|c| c.pre_edit_contract_hash.clone())
            .unwrap_or_else(|| self.plan.gates[gate_index].node.signature_hash.clone());

        if let Some(parent) = abs_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&abs_path, proposed_content)?;

        self.record_proposed_edit(
            gate_index,
            original_content,
            file_existed_originally,
            proposed_content.to_string(),
            pre_hash,
            post_contract_hash.to_string(),
            Vec::new(),
            acknowledge_blast_radius,
        )
    }

    /// The Enforcement Barrier: Verifies diagnostics and checks if a contract mutation
    /// requires dynamic injection of external blast-radius dependents.
    pub fn advance_gate(&mut self) -> Result<SessionPhase> {
        let idx = self.current_gate_index()?;
        if let SessionPhase::BlockedAtGate { reason, .. } = &self.phase {
            bail!("GATE_BLOCKED: Cannot advance Gate {}: {}", idx, reason);
        }

        if let Some(chk) = self.checkpoints.get(&idx) {
            if !chk.diagnostics_clean {
                bail!("Gate {} has unresolved compiler diagnostics.", idx);
            }

            let contract_mutated = chk
                .post_edit_contract_hash
                .as_ref()
                .map(|post| post != &chk.pre_edit_contract_hash)
                .unwrap_or(false);

            let external_deps = self.plan.gates[idx]
                .blast_radius
                .external_dependents
                .clone();

            if contract_mutated && !external_deps.is_empty() && !chk.blast_radius_acknowledged {
                let dep_list: Vec<String> = external_deps
                    .iter()
                    .map(|d| format!("- {}:{} ({})", d.file_path, d.call_site_line, d.symbol_id))
                    .collect();
                bail!(
                    "CONTAGION_BARRIER: Gate {} mutated its exported contract hash, \
                    affecting {} external dependent(s) outside the current corridor:\n{}\n\
                    Re-submit edit with `acknowledge_blast_radius: true` to dynamically append \
                    these files as mandatory gates, or make the contract change backward-compatible.",
                    idx,
                    external_deps.len(),
                    dep_list.join("\n")
                );
            }

            // Dynamically expand corridor if blast radius was acknowledged
            if contract_mutated && !external_deps.is_empty() && chk.blast_radius_acknowledged {
                for ext in external_deps {
                    let already_in_plan = self
                        .plan
                        .gates
                        .iter()
                        .any(|g| g.node.symbol_id == ext.symbol_id);
                    if !already_in_plan {
                        let new_gate_idx = self.plan.gates.len();
                        self.plan.gates.push(CorridorGate {
                            gate_index: new_gate_idx,
                            status: GateStatus::Locked,
                            node: SemanticNode {
                                symbol_id: ext.symbol_id.clone(),
                                display_name: ext.symbol_id.clone(),
                                file_path: ext.file_path.clone(),
                                byte_range: (0, 0),
                                line_range: (
                                    ext.call_site_line.saturating_sub(15),
                                    ext.call_site_line + 25,
                                ),
                                role: NodeRole::BusinessLogic,
                                signature_hash: String::new(),
                                mutates_domain_state: false,
                                is_exported: true,
                            },
                            upstream_contracts: vec![self.plan.gates[idx].node.symbol_id.clone()],
                            downstream_in_corridor: Vec::new(),
                            blast_radius: BlastRadiusReport::safe_internal(),
                        });
                    }
                }
            }
        }

        self.plan.gates[idx].status = GateStatus::Verified;
        let next_idx = idx + 1;

        if next_idx < self.plan.gates.len() {
            self.plan.gates[next_idx].status = GateStatus::Unlocked;
            self.phase = SessionPhase::ActiveGate(next_idx);
        } else {
            self.phase = SessionPhase::Completed;
        }

        Ok(self.phase.clone())
    }

    pub fn rollback_to_gate(&mut self, target_gate: usize) -> Result<Vec<(PathBuf, String)>> {
        if target_gate >= self.plan.gates.len() {
            bail!("Invalid target gate index {} for rollback", target_gate);
        }

        let mut restored_files = Vec::new();
        let mut indices_to_revert: Vec<usize> = self
            .checkpoints
            .keys()
            .copied()
            .filter(|&k| k >= target_gate)
            .collect();
        indices_to_revert.sort_unstable_by(|a, b| b.cmp(a));

        for k in indices_to_revert {
            let Some(chk) = self.checkpoints.get(&k) else {
                continue;
            };
            let abs_path = self.workspace_root.join(&chk.file_path);
            if chk.file_existed_originally {
                std::fs::write(&abs_path, &chk.original_content)?;
            } else if abs_path.exists() {
                let _ = std::fs::remove_file(&abs_path);
            }
            restored_files.push((abs_path, chk.file_path.clone()));
            self.checkpoints.remove(&k);
        }

        for (i, gate) in self.plan.gates.iter_mut().enumerate() {
            if i < target_gate {
                gate.status = GateStatus::Verified;
            } else if i == target_gate {
                gate.status = GateStatus::Unlocked;
            } else {
                gate.status = GateStatus::Locked;
            }
        }

        self.phase = SessionPhase::ActiveGate(target_gate);
        Ok(restored_files)
    }

    pub fn save_to_disk(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(self)?;
        std::fs::write(path, serialized)?;
        Ok(())
    }
}
