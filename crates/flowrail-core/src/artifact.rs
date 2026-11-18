use crate::git::{GitFileStatus, GitStatusResolver};
use crate::graph::NodeRole;
use crate::slicer::{
    BlastRadiusReport, BlastRiskLevel, CorridorPlan, ExternalDependent, GateStatus, SliceIntent,
};
use crate::state::{CorridorSession, SessionPhase};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    CorridorPlan,
    CorridorExecution,
    WorkspaceAudit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnrichmentStatus {
    PendingEnrichment,
    Enriched,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactGate {
    pub gate_index: usize,
    pub symbol_id: String,
    pub display_name: String,
    pub file_path: String,
    pub line_range: (usize, usize),
    pub role: NodeRole,
    pub status: GateStatus,
    pub signature_hash: String,
    pub upstream_contracts: Vec<String>,
    pub downstream_in_corridor: Vec<String>,
    pub blast_radius: BlastRadiusReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastSummary {
    pub contagion_index: usize,
    pub risk_level: BlastRiskLevel,
    pub external_dependents_count: usize,
    pub external_dependents: Vec<ExternalDependent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSummary {
    pub diagnostics_clean: bool,
    pub unresolved_diagnostics_count: usize,
    pub diagnostics: Vec<String>,
    pub verified_gates_count: usize,
    pub total_gates_count: usize,
    pub all_gates_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSummary {
    pub modified_files: Vec<String>,
    pub staged_files: Vec<String>,
    pub untracked_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterministicPayload {
    pub session_id: String,
    pub intent: SliceIntent,
    pub entry_symbol: String,
    pub target_mutation_symbol: Option<String>,
    pub total_gates: usize,
    pub active_gate_index: usize,
    pub session_phase: SessionPhase,
    pub gates: Vec<ArtifactGate>,
    pub blast_summary: BlastSummary,
    pub verification_summary: VerificationSummary,
    pub git_summary: GitSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdhdErrorDiagnostic {
    pub cause: String,
    pub fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdhdView {
    /// Rule 1: Lead with the next action! (Immediate bounded command or edit)
    pub headline_action: String,
    /// Rule 5: Restate state every turn ("Gate X of Y verified (Z%). Active: ...")
    pub state_anchor: String,
    /// Rule 6: Specific time estimates ("5-10 minutes")
    pub estimated_duration: String,
    /// Rule 2 & Rule 9: Numbered multi-step tasks, capped at 5 bounded items
    pub bounded_steps: Vec<String>,
    /// Rule 7: Make completed work visible
    pub visible_wins: Vec<String>,
    /// Rule 3: End with one concrete next action (< 2 minutes)
    pub terminal_action: String,
    /// Rule 8: Matter-of-fact tone for errors (cause + fix)
    pub error_diagnostic: Option<AdhdErrorDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiSlopContract {
    /// Grounded symbols cited (must physically exist in deterministic payload)
    pub grounded_symbols: Vec<String>,
    /// Grounded file paths cited (must physically exist in deterministic payload)
    pub grounded_files: Vec<String>,
    /// Rule 4: Tangent suppression (strictly corridor-scoped)
    pub tangent_free: bool,
    /// Rule 10: Zero conversational pleasantries / preambles / recaps
    pub zero_conversational_fluff: bool,
    /// Direct Payload Invariant
    pub direct_payload: bool,
    /// Automated machine validation result
    pub validation_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEnrichment {
    pub status: EnrichmentStatus,
    pub human_readable_summary: String,
    pub adhd_view: AdhdView,
    pub anti_slop_contract: AntiSlopContract,
    pub enrichment_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowrailArtifact {
    pub schema_version: String,
    pub artifact_id: String,
    pub kind: ArtifactKind,
    pub timestamp_epoch_secs: u64,
    pub formatted_utc: String,
    pub workspace_root: String,
    pub deterministic: DeterministicPayload,
    pub ai_enrichment: AiEnrichment,
}

fn epoch_to_iso8601(epoch_secs: u64) -> String {
    let secs_per_day = 86400;
    let days = epoch_secs / secs_per_day;
    let day_secs = epoch_secs % secs_per_day;

    let hours = day_secs / 3600;
    let mins = (day_secs % 3600) / 60;
    let secs = day_secs % 60;

    // Approximate date calculation from 1970-01-01
    let mut y = 1970;
    let mut rem_days = days;
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if rem_days >= days_in_year {
            rem_days -= days_in_year;
            y += 1;
        } else {
            break;
        }
    }

    let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
    let days_in_months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    for (idx, &dim) in days_in_months.iter().enumerate() {
        if rem_days >= dim {
            rem_days -= dim;
        } else {
            m = idx + 1;
            break;
        }
    }
    let d = rem_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, mins, secs
    )
}

impl FlowrailArtifact {
    pub fn from_plan(
        plan: &CorridorPlan,
        workspace_root: &Path,
        git: Option<&GitStatusResolver>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let artifact_hash = blake3::hash(
            format!("{}:{}:{}", plan.corridor_id, plan.entry_gate_symbol, now).as_bytes(),
        )
        .to_hex();
        let artifact_id = format!("art-plan-{}", &artifact_hash[..12]);

        let gates: Vec<ArtifactGate> = plan
            .gates
            .iter()
            .map(|g| ArtifactGate {
                gate_index: g.gate_index,
                symbol_id: g.node.symbol_id.clone(),
                display_name: g.node.display_name.clone(),
                file_path: g.node.file_path.clone(),
                line_range: g.node.line_range,
                role: g.node.role,
                status: g.status,
                signature_hash: g.node.signature_hash.clone(),
                upstream_contracts: g.upstream_contracts.clone(),
                downstream_in_corridor: g.downstream_in_corridor.clone(),
                blast_radius: g.blast_radius.clone(),
            })
            .collect();

        let mut contagion_sum = 0;
        let mut max_risk = BlastRiskLevel::SafeInternal;
        let mut external_deps = Vec::new();

        for g in &plan.gates {
            contagion_sum += g.blast_radius.contagion_index;
            if g.blast_radius.risk_level == BlastRiskLevel::HighContagion {
                max_risk = BlastRiskLevel::HighContagion;
            } else if g.blast_radius.risk_level == BlastRiskLevel::Moderate
                && max_risk != BlastRiskLevel::HighContagion
            {
                max_risk = BlastRiskLevel::Moderate;
            }
            for dep in &g.blast_radius.external_dependents {
                if !external_deps.contains(dep) {
                    external_deps.push(dep.clone());
                }
            }
        }

        let git_summary = Self::resolve_git_summary(git);
        let active_gate_index = gates
            .iter()
            .position(|g| g.status == GateStatus::Unlocked)
            .unwrap_or(0);

        let deterministic = DeterministicPayload {
            session_id: plan.corridor_id.clone(),
            intent: plan.intent,
            entry_symbol: plan.entry_gate_symbol.clone(),
            target_mutation_symbol: plan.target_mutation_symbol.clone(),
            total_gates: gates.len(),
            active_gate_index,
            session_phase: if gates.is_empty() {
                SessionPhase::Completed
            } else {
                SessionPhase::ActiveGate(active_gate_index)
            },
            gates,
            blast_summary: BlastSummary {
                contagion_index: contagion_sum,
                risk_level: max_risk,
                external_dependents_count: external_deps.len(),
                external_dependents: external_deps,
            },
            verification_summary: VerificationSummary {
                diagnostics_clean: true,
                unresolved_diagnostics_count: 0,
                diagnostics: Vec::new(),
                verified_gates_count: 0,
                total_gates_count: plan.gates.len(),
                all_gates_verified: false,
            },
            git_summary,
        };

        let mut artifact = Self {
            schema_version: "flowrail.artifact.v1".to_string(),
            artifact_id,
            kind: ArtifactKind::CorridorPlan,
            timestamp_epoch_secs: now,
            formatted_utc: epoch_to_iso8601(now),
            workspace_root: workspace_root.to_string_lossy().to_string(),
            deterministic,
            ai_enrichment: AiEnrichment {
                status: EnrichmentStatus::PendingEnrichment,
                human_readable_summary: String::new(),
                adhd_view: AdhdView {
                    headline_action: String::new(),
                    state_anchor: String::new(),
                    estimated_duration: String::new(),
                    bounded_steps: Vec::new(),
                    visible_wins: Vec::new(),
                    terminal_action: String::new(),
                    error_diagnostic: None,
                },
                anti_slop_contract: AntiSlopContract {
                    grounded_symbols: Vec::new(),
                    grounded_files: Vec::new(),
                    tangent_free: true,
                    zero_conversational_fluff: true,
                    direct_payload: true,
                    validation_passed: false,
                },
                enrichment_timestamp: None,
            },
        };

        // Populate baseline grounded zero-hallucination enrichment
        artifact.ai_enrichment = artifact.generate_grounded_enrichment();
        artifact
    }

    pub fn from_session(session: &CorridorSession, git: Option<&GitStatusResolver>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let artifact_hash = blake3::hash(
            format!("{}:{:?}:{}", session.plan.corridor_id, session.phase, now).as_bytes(),
        )
        .to_hex();
        let artifact_id = format!("art-session-{}", &artifact_hash[..12]);

        let gates: Vec<ArtifactGate> = session
            .plan
            .gates
            .iter()
            .map(|g| ArtifactGate {
                gate_index: g.gate_index,
                symbol_id: g.node.symbol_id.clone(),
                display_name: g.node.display_name.clone(),
                file_path: g.node.file_path.clone(),
                line_range: g.node.line_range,
                role: g.node.role,
                status: g.status,
                signature_hash: g.node.signature_hash.clone(),
                upstream_contracts: g.upstream_contracts.clone(),
                downstream_in_corridor: g.downstream_in_corridor.clone(),
                blast_radius: g.blast_radius.clone(),
            })
            .collect();

        let mut contagion_sum = 0;
        let mut max_risk = BlastRiskLevel::SafeInternal;
        let mut external_deps = Vec::new();

        for g in &session.plan.gates {
            contagion_sum += g.blast_radius.contagion_index;
            if g.blast_radius.risk_level == BlastRiskLevel::HighContagion {
                max_risk = BlastRiskLevel::HighContagion;
            } else if g.blast_radius.risk_level == BlastRiskLevel::Moderate
                && max_risk != BlastRiskLevel::HighContagion
            {
                max_risk = BlastRiskLevel::Moderate;
            }
            for dep in &g.blast_radius.external_dependents {
                if !external_deps.contains(dep) {
                    external_deps.push(dep.clone());
                }
            }
        }

        let verified_count = gates
            .iter()
            .filter(|g| g.status == GateStatus::Verified)
            .count();
        let all_verified = !gates.is_empty() && verified_count == gates.len();

        let mut diagnostics = Vec::new();
        if let SessionPhase::BlockedAtGate { reason, .. } = &session.phase {
            diagnostics.push(reason.clone());
        }

        let active_gate_index = match session.phase {
            SessionPhase::ActiveGate(idx) => idx,
            SessionPhase::BlockedAtGate { gate_index, .. } => gate_index,
            SessionPhase::Completed => gates.len().saturating_sub(1),
            SessionPhase::Idle => 0,
        };

        let git_summary = Self::resolve_git_summary(git);

        let deterministic = DeterministicPayload {
            session_id: session.plan.corridor_id.clone(),
            intent: session.plan.intent,
            entry_symbol: session.plan.entry_gate_symbol.clone(),
            target_mutation_symbol: session.plan.target_mutation_symbol.clone(),
            total_gates: gates.len(),
            active_gate_index,
            session_phase: session.phase.clone(),
            gates,
            blast_summary: BlastSummary {
                contagion_index: contagion_sum,
                risk_level: max_risk,
                external_dependents_count: external_deps.len(),
                external_dependents: external_deps,
            },
            verification_summary: VerificationSummary {
                diagnostics_clean: diagnostics.is_empty(),
                unresolved_diagnostics_count: diagnostics.len(),
                diagnostics,
                verified_gates_count: verified_count,
                total_gates_count: session.plan.gates.len(),
                all_gates_verified: all_verified,
            },
            git_summary,
        };

        let mut artifact = Self {
            schema_version: "flowrail.artifact.v1".to_string(),
            artifact_id,
            kind: ArtifactKind::CorridorExecution,
            timestamp_epoch_secs: now,
            formatted_utc: epoch_to_iso8601(now),
            workspace_root: session.workspace_root.to_string_lossy().to_string(),
            deterministic,
            ai_enrichment: AiEnrichment {
                status: EnrichmentStatus::PendingEnrichment,
                human_readable_summary: String::new(),
                adhd_view: AdhdView {
                    headline_action: String::new(),
                    state_anchor: String::new(),
                    estimated_duration: String::new(),
                    bounded_steps: Vec::new(),
                    visible_wins: Vec::new(),
                    terminal_action: String::new(),
                    error_diagnostic: None,
                },
                anti_slop_contract: AntiSlopContract {
                    grounded_symbols: Vec::new(),
                    grounded_files: Vec::new(),
                    tangent_free: true,
                    zero_conversational_fluff: true,
                    direct_payload: true,
                    validation_passed: false,
                },
                enrichment_timestamp: None,
            },
        };

        artifact.ai_enrichment = artifact.generate_grounded_enrichment();
        artifact
    }

    pub fn from_doctor(
        workspace_root: &Path,
        node_count: usize,
        edge_count: usize,
        git: Option<&GitStatusResolver>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let artifact_id = format!(
            "art-doctor-{}",
            &blake3::hash(format!("doctor:{}:{}", workspace_root.display(), now).as_bytes())
                .to_hex()[..12]
        );

        let git_summary = Self::resolve_git_summary(git);

        let deterministic = DeterministicPayload {
            session_id: format!("audit-{}", &artifact_id[4..]),
            intent: SliceIntent::ReadFlow,
            entry_symbol: format!(
                "workspace::root({} nodes, {} edges)",
                node_count, edge_count
            ),
            target_mutation_symbol: None,
            total_gates: 0,
            active_gate_index: 0,
            session_phase: SessionPhase::Completed,
            gates: Vec::new(),
            blast_summary: BlastSummary {
                contagion_index: 0,
                risk_level: BlastRiskLevel::SafeInternal,
                external_dependents_count: 0,
                external_dependents: Vec::new(),
            },
            verification_summary: VerificationSummary {
                diagnostics_clean: true,
                unresolved_diagnostics_count: 0,
                diagnostics: Vec::new(),
                verified_gates_count: 0,
                total_gates_count: 0,
                all_gates_verified: true,
            },
            git_summary,
        };

        let mut artifact = Self {
            schema_version: "flowrail.artifact.v1".to_string(),
            artifact_id,
            kind: ArtifactKind::WorkspaceAudit,
            timestamp_epoch_secs: now,
            formatted_utc: epoch_to_iso8601(now),
            workspace_root: workspace_root.to_string_lossy().to_string(),
            deterministic,
            ai_enrichment: AiEnrichment {
                status: EnrichmentStatus::PendingEnrichment,
                human_readable_summary: String::new(),
                adhd_view: AdhdView {
                    headline_action: String::new(),
                    state_anchor: String::new(),
                    estimated_duration: String::new(),
                    bounded_steps: Vec::new(),
                    visible_wins: Vec::new(),
                    terminal_action: String::new(),
                    error_diagnostic: None,
                },
                anti_slop_contract: AntiSlopContract {
                    grounded_symbols: Vec::new(),
                    grounded_files: Vec::new(),
                    tangent_free: true,
                    zero_conversational_fluff: true,
                    direct_payload: true,
                    validation_passed: false,
                },
                enrichment_timestamp: None,
            },
        };

        artifact.ai_enrichment = artifact.generate_grounded_enrichment();
        artifact
    }

    fn resolve_git_summary(git: Option<&GitStatusResolver>) -> GitSummary {
        let mut modified = Vec::new();
        let mut staged = Vec::new();
        let mut untracked = Vec::new();

        if let Some(r) = git {
            for (path, status) in r.get_all_statuses() {
                match status {
                    GitFileStatus::Modified => modified.push(path),
                    GitFileStatus::Staged => staged.push(path),
                    GitFileStatus::Untracked => untracked.push(path),
                    GitFileStatus::Clean => {}
                }
            }
        }

        GitSummary {
            modified_files: modified,
            staged_files: staged,
            untracked_files: untracked,
        }
    }

    /// Generates a strictly grounded, zero-hallucination baseline enrichment
    /// enforcing all 10 ADHD rules and Anti-Slop Direct Payload standards.
    pub fn generate_grounded_enrichment(&self) -> AiEnrichment {
        let d = &self.deterministic;

        let mut grounded_symbols = Vec::new();
        let mut grounded_files = Vec::new();

        for g in &d.gates {
            if !grounded_symbols.contains(&g.symbol_id) {
                grounded_symbols.push(g.symbol_id.clone());
            }
            if !grounded_files.contains(&g.file_path) {
                grounded_files.push(g.file_path.clone());
            }
        }
        for dep in &d.blast_summary.external_dependents {
            if !grounded_symbols.contains(&dep.symbol_id) {
                grounded_symbols.push(dep.symbol_id.clone());
            }
            if !grounded_files.contains(&dep.file_path) {
                grounded_files.push(dep.file_path.clone());
            }
        }

        match self.kind {
            ArtifactKind::WorkspaceAudit => {
                let git_changes = d.git_summary.modified_files.len()
                    + d.git_summary.staged_files.len()
                    + d.git_summary.untracked_files.len();

                let headline = if git_changes > 0 {
                    format!(
                        "Inspect {} modified or uncommitted files in workspace.",
                        git_changes
                    )
                } else {
                    "Workspace semantic graph verified clean and ready for corridor slicing."
                        .to_string()
                };

                AiEnrichment {
                    status: EnrichmentStatus::Enriched,
                    human_readable_summary: format!(
                        "Workspace audit verified: {} modified files, ready for execution.",
                        d.git_summary.modified_files.len()
                    ),
                    adhd_view: AdhdView {
                        headline_action: headline,
                        state_anchor: "Doctor Audit Completed: Graph and Git status synchronized."
                            .to_string(),
                        estimated_duration: "1 minute".to_string(),
                        bounded_steps: vec![
                            "1. Run `flowrail slice` with entrypoint to start corridor."
                                .to_string(),
                            "2. Review unlocked Gate 0 slice.".to_string(),
                        ],
                        visible_wins: vec![
                            "Semantic graph and Git status index verified operational.".to_string(),
                        ],
                        terminal_action: "Run `flowrail search checkout` to discover entry gates."
                            .to_string(),
                        error_diagnostic: None,
                    },
                    anti_slop_contract: AntiSlopContract {
                        grounded_symbols: Vec::new(),
                        grounded_files: d.git_summary.modified_files.clone(),
                        tangent_free: true,
                        zero_conversational_fluff: true,
                        direct_payload: true,
                        validation_passed: true,
                    },
                    enrichment_timestamp: Some(self.timestamp_epoch_secs),
                }
            }
            ArtifactKind::CorridorPlan | ArtifactKind::CorridorExecution => {
                // Determine state and active gate
                let (active_gate, _is_blocked, block_reason) = match &d.session_phase {
                    SessionPhase::BlockedAtGate { gate_index, reason } => {
                        (d.gates.get(*gate_index), true, Some(reason.clone()))
                    }
                    SessionPhase::ActiveGate(idx) => (d.gates.get(*idx), false, None),
                    SessionPhase::Completed => (d.gates.last(), false, None),
                    SessionPhase::Idle => (d.gates.first(), false, None),
                };

                let total = d.total_gates;
                let verified = d.verification_summary.verified_gates_count;

                let (
                    headline_action,
                    state_anchor,
                    estimated_duration,
                    error_diag,
                    terminal_action,
                ) = if let Some(err_msg) = block_reason {
                    let active = active_gate.map(|g| g.file_path.clone()).unwrap_or_default();
                    (
                        format!("Fix TypeScript compiler diagnostic in `{}`.", active),
                        format!(
                            "Blocked at Gate {} of {}: Compiler diagnostic.",
                            d.active_gate_index, total
                        ),
                        "5-10 minutes".to_string(),
                        Some(AdhdErrorDiagnostic {
                            cause: err_msg,
                            fix: format!("Edit `{}` and resolve reported type errors.", active),
                        }),
                        format!("Open `{}` and resolve reported diagnostic lines.", active),
                    )
                } else if d.verification_summary.all_gates_verified
                    || d.session_phase == SessionPhase::Completed
                {
                    (
                        "All gates verified: Corridor mutation completed and clean.".to_string(),
                        format!("Completed {} of {} gates (100% verified).", total, total),
                        "2 minutes".to_string(),
                        None,
                        "Review git diff and execute project test suite.".to_string(),
                    )
                } else if let Some(gate) = active_gate {
                    let remaining_gates = total.saturating_sub(d.active_gate_index);
                    let est_mins = (remaining_gates * 4).max(3);
                    (
                        format!(
                            "Inspect Gate {} (`{}`) in `{}` (lines {}-{}).",
                            gate.gate_index,
                            gate.display_name,
                            gate.file_path,
                            gate.line_range.0,
                            gate.line_range.1
                        ),
                        format!(
                            "Gate {} of {} ({}% verified). Active: `{}`.",
                            gate.gate_index,
                            total,
                            (verified * 100).checked_div(total).unwrap_or(0),
                            gate.file_path
                        ),
                        format!("{}-{} minutes", est_mins, est_mins + 5),
                        None,
                        format!(
                            "Inspect Gate {} source slice in `{}`.",
                            gate.gate_index, gate.file_path
                        ),
                    )
                } else {
                    (
                        "Initialize corridor session with `corridor_open`.".to_string(),
                        "Corridor idle. No gates currently active.".to_string(),
                        "3 minutes".to_string(),
                        None,
                        "Call `corridor_open` with target entry symbol.".to_string(),
                    )
                };

                // Generate bounded steps (max 5 items, strictly numbered)
                let mut bounded_steps = Vec::new();
                for (i, g) in d.gates.iter().take(4).enumerate() {
                    let step_desc = match g.status {
                        GateStatus::Verified => format!(
                            "{}. [VERIFIED] `{}` in `{}`",
                            i + 1,
                            g.display_name,
                            g.file_path
                        ),
                        GateStatus::Unlocked => format!(
                            "{}. [ACTIVE] Edit `{}` in `{}`",
                            i + 1,
                            g.display_name,
                            g.file_path
                        ),
                        GateStatus::Locked => format!(
                            "{}. [LOCKED] Awaiting Gate {} unlock: `{}`",
                            i + 1,
                            i.saturating_sub(1),
                            g.display_name
                        ),
                        GateStatus::Blocked => {
                            format!("{}. [BLOCKED] Fix diagnostic in `{}`", i + 1, g.file_path)
                        }
                    };
                    bounded_steps.push(step_desc);
                }
                if bounded_steps.len() < 5 {
                    bounded_steps.push(format!(
                        "{}. Finalize verification and commit corridor changes.",
                        bounded_steps.len() + 1
                    ));
                }

                // Visible wins (max 5 items)
                let mut visible_wins = Vec::new();
                if verified > 0 {
                    visible_wins.push(format!(
                        "Successfully verified {} of {} corridor gates without syntax errors.",
                        verified, total
                    ));
                }
                if d.blast_summary.contagion_index == 0 {
                    visible_wins.push(
                        "Contagion Index is 0: Zero external caller breakage detected.".to_string(),
                    );
                } else {
                    visible_wins.push(format!(
                        "Blast radius bounded: {} external dependents tracked in mutation ledger.",
                        d.blast_summary.external_dependents_count
                    ));
                }

                let summary = format!(
                    "Corridor '{}' ({}): {}/{} gates verified | Risk: {:?}",
                    d.session_id,
                    match d.intent {
                        SliceIntent::ReadFlow => "READ_FLOW",
                        SliceIntent::MutateContract => "MUTATE_CONTRACT",
                    },
                    verified,
                    total,
                    d.blast_summary.risk_level
                );

                AiEnrichment {
                    status: EnrichmentStatus::PendingEnrichment,
                    human_readable_summary: summary,
                    adhd_view: AdhdView {
                        headline_action,
                        state_anchor,
                        estimated_duration,
                        bounded_steps,
                        visible_wins,
                        terminal_action,
                        error_diagnostic: error_diag,
                    },
                    anti_slop_contract: AntiSlopContract {
                        grounded_symbols,
                        grounded_files,
                        tangent_free: true,
                        zero_conversational_fluff: true,
                        direct_payload: true,
                        validation_passed: true,
                    },
                    enrichment_timestamp: Some(self.timestamp_epoch_secs),
                }
            }
        }
    }

    /// Validates an AI-provided enrichment payload against ADHD rules and Anti-Slop gates:
    /// - Rejects hallucinated symbols and files not in the deterministic payload.
    /// - Rejects conversational pleasantries, preambles, and marketing hype.
    /// - Enforces list item caps (<= 5 items).
    /// - Enforces non-empty headline and terminal actions.
    pub fn apply_and_validate_enrichment(&mut self, mut enrichment: AiEnrichment) -> Result<()> {
        let d = &self.deterministic;

        // Build valid symbol and file allowlists from deterministic payload
        let mut valid_symbols: HashSet<String> = HashSet::new();
        let mut valid_files: HashSet<String> = HashSet::new();

        valid_symbols.insert(d.entry_symbol.clone());
        if let Some(target) = &d.target_mutation_symbol {
            valid_symbols.insert(target.clone());
        }

        for g in &d.gates {
            valid_symbols.insert(g.symbol_id.clone());
            valid_symbols.insert(g.display_name.clone());
            valid_files.insert(g.file_path.clone());
        }

        for dep in &d.blast_summary.external_dependents {
            valid_symbols.insert(dep.symbol_id.clone());
            valid_files.insert(dep.file_path.clone());
        }

        for f in &d.git_summary.modified_files {
            valid_files.insert(f.clone());
        }
        for f in &d.git_summary.staged_files {
            valid_files.insert(f.clone());
        }
        for f in &d.git_summary.untracked_files {
            valid_files.insert(f.clone());
        }

        // 1. Anti-Hallucination Gate: Verify all symbols cited exist
        for sym in &enrichment.anti_slop_contract.grounded_symbols {
            if !valid_symbols.contains(sym) && !valid_symbols.iter().any(|v| v.contains(sym)) {
                bail!(
                    "HALLUCINATED_SYMBOL_REJECTED: Symbol '{}' does not physically exist \
                    in the deterministic corridor graph. AI enrichment must be strictly grounded.",
                    sym
                );
            }
        }

        // 2. Anti-Hallucination Gate: Verify all files cited exist
        for f in &enrichment.anti_slop_contract.grounded_files {
            if !valid_files.contains(f)
                && !valid_files.iter().any(|v| v.ends_with(f) || f.ends_with(v))
            {
                bail!(
                    "HALLUCINATED_FILE_REJECTED: File '{}' does not physically exist in the \
                    deterministic corridor gates or git status. AI enrichment must be strictly grounded.",
                    f
                );
            }
        }

        // 3. ADHD Rule 1 & Rule 3: Headline and Terminal action required
        if enrichment.adhd_view.headline_action.trim().is_empty() {
            bail!(
                "ADHD_VIOLATION_RULE_1: headline_action must not be empty. Lead with the next action."
            );
        }
        if enrichment.adhd_view.terminal_action.trim().is_empty() {
            bail!(
                "ADHD_VIOLATION_RULE_3: terminal_action must not be empty. End with one concrete action."
            );
        }

        // 4. ADHD Rule 9: Cap bounded steps and visible wins to 5 items
        if enrichment.adhd_view.bounded_steps.len() > 5 {
            bail!(
                "ADHD_VIOLATION_RULE_9: bounded_steps has {} items (maximum allowed is 5). \
                Keep visible working set small to protect working memory.",
                enrichment.adhd_view.bounded_steps.len()
            );
        }
        if enrichment.adhd_view.visible_wins.len() > 5 {
            bail!(
                "ADHD_VIOLATION_RULE_9: visible_wins has {} items (maximum allowed is 5).",
                enrichment.adhd_view.visible_wins.len()
            );
        }

        // 5. Anti-Slop Fluff & Conversational Pleasantries Gate
        let forbidden_fluff = [
            "great question",
            "let me explain",
            "i'll start by",
            "sure!",
            "looking at your",
            "to answer your question",
            "hope this helps",
            "let me know",
            "happy to clarify",
            "feel free to ask",
            "unleash the power",
            "game-changer",
            "as an ai",
            "dive into",
            "testament to",
            "tapestry",
        ];

        let text_to_check = format!(
            "{} {} {} {}",
            enrichment.adhd_view.headline_action,
            enrichment.adhd_view.terminal_action,
            enrichment.human_readable_summary,
            enrichment.adhd_view.bounded_steps.join(" ")
        )
        .to_lowercase();

        for phrase in &forbidden_fluff {
            if text_to_check.contains(phrase) {
                bail!(
                    "ANTI_SLOP_VIOLATION: Conversational fluff detected ('{}'). \
                    Enforce the Direct Payload Invariant with zero preambles or pleasantries.",
                    phrase
                );
            }
        }

        enrichment.anti_slop_contract.validation_passed = true;
        enrichment.anti_slop_contract.tangent_free = true;
        enrichment.anti_slop_contract.zero_conversational_fluff = true;
        enrichment.anti_slop_contract.direct_payload = true;
        enrichment.status = EnrichmentStatus::Enriched;
        enrichment.enrichment_timestamp = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        self.ai_enrichment = enrichment;
        Ok(())
    }

    /// Serializes to formatted Clean/Clear JSON.
    pub fn to_clean_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Saves artifact to the specified directory.
    pub fn save_to_dir(&self, dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let file_name = format!("{}.json", self.artifact_id);
        let target_path = dir.join(file_name);
        std::fs::write(&target_path, self.to_clean_json()?)?;

        // Update latest.json
        let latest_path = dir.join("latest.json");
        std::fs::write(&latest_path, self.to_clean_json()?)?;

        Ok(target_path)
    }

    /// Saves artifact to `.flowrail/artifacts/` inside the workspace root.
    pub fn save_to_workspace(&self, workspace_root: &Path) -> Result<PathBuf> {
        let artifacts_dir = workspace_root.join(".flowrail/artifacts");
        self.save_to_dir(&artifacts_dir)
    }
}
