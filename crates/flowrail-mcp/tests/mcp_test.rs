use flowrail_core::graph::{CodebaseGraph, NodeRole, SemanticNode};
use flowrail_mcp::tools::CorridorMcpEngine;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

struct TempDirGuard(PathBuf);

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_mcp_tools_lifecycle() {
    let guard = TempDirGuard::new("flowrail_mcp_test");
    let temp_dir = guard.path().to_path_buf();
    fs::create_dir_all(temp_dir.join("src/checkout")).unwrap();

    let file_path = "src/checkout/checkout.controller.ts";
    let abs_path = temp_dir.join(file_path);
    let original_code = r#"export class CheckoutController {
    processCheckout(): void {
        console.log("processing");
    }
}
"#;
    fs::write(&abs_path, original_code).unwrap();

    let mut graph = CodebaseGraph::new();
    let symbol_id = format!("{}#CheckoutController.processCheckout", file_path);
    graph.upsert_node(SemanticNode {
        symbol_id: symbol_id.clone(),
        display_name: "CheckoutController.processCheckout".to_string(),
        file_path: file_path.to_string(),
        byte_range: (0, 100),
        line_range: (1, 5),
        role: NodeRole::EntryGate,
        signature_hash: "hash_init".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });

    let mut engine = CorridorMcpEngine::new(temp_dir.clone(), graph, false);

    // 1. Discover entrypoints with fuzzy search and badges
    let disc_res = engine
        .dispatch_tool(
            "corridor_discover_entrypoints",
            &json!({
                "query": "checkout"
            }),
        )
        .unwrap();
    assert!(disc_res.contains("CheckoutController.processCheckout"));
    assert!(disc_res.contains("git_badge"));

    // 1.5 Find symbols with typo tolerance
    let find_res = engine
        .dispatch_tool(
            "corridor_find_symbols",
            &json!({
                "query": "Chekout"
            }),
        )
        .unwrap();
    assert!(find_res.contains("CheckoutController"));

    // 2. Open corridor
    let open_res = engine
        .dispatch_tool(
            "corridor_open",
            &json!({
                "entry_symbol": symbol_id,
                "intent": "READ_FLOW"
            }),
        )
        .unwrap();
    assert!(open_res.contains("cor_"));
    assert!(open_res.contains("active_gate_index\": 0"));

    // 2.5 Grep inside active corridor
    let grep_res = engine
        .dispatch_tool(
            "corridor_grep",
            &json!({
                "query": "processCheckout",
                "scope": "corridor"
            }),
        )
        .unwrap();
    assert!(grep_res.contains("total_matches\": 1"));
    assert!(grep_res.contains("formatted_text"));

    // 3. Inspect Gate 0 (normal and compact)
    let inspect_res = engine
        .dispatch_tool(
            "corridor_inspect_gate",
            &json!({
                "gate_index": 0
            }),
        )
        .unwrap();
    assert!(inspect_res.contains("CheckoutController"));

    let inspect_compact = engine
        .dispatch_tool(
            "corridor_inspect_gate",
            &json!({
                "gate_index": 0,
                "compact": true
            }),
        )
        .unwrap();
    assert!(inspect_compact.contains("compact_token_optimized"));

    // 4. Propose edit
    let proposed_code = r#"export class CheckoutController {
    processCheckout(): void {
        console.log("processing with flowrail");
    }
}
"#;
    let edit_res = engine
        .dispatch_tool(
            "corridor_propose_edit",
            &json!({
                "gate_index": 0,
                "new_file_content": proposed_code
            }),
        )
        .unwrap();
    assert!(edit_res.contains("diagnostics_clean\": true"));

    // 5. Advance gate
    let adv_res = engine
        .dispatch_tool("corridor_advance_gate", &json!({}))
        .unwrap();
    assert!(adv_res.contains("Completed"));

    // 6. Rollback to Gate 0
    let rb_res = engine
        .dispatch_tool(
            "corridor_abort_or_rollback",
            &json!({
                "to_gate_index": 0
            }),
        )
        .unwrap();
    assert!(rb_res.contains("rolled_back_to_gate\": 0"));

    let restored = fs::read_to_string(&abs_path).unwrap();
    assert_eq!(restored, original_code);
}

#[test]
fn test_propose_edit_out_of_order_rejects_without_disk_mutation() {
    let guard = TempDirGuard::new("flowrail_mcp_out_of_order");
    let temp_dir = guard.path().to_path_buf();
    fs::create_dir_all(temp_dir.join("src")).unwrap();

    let file0 = "src/gate0.ts";
    let file1 = "src/gate1.ts";
    let original_gate1 = "export const gate1 = 'original';\n";
    fs::write(temp_dir.join(file0), "export const gate0 = 'g0';\n").unwrap();
    fs::write(temp_dir.join(file1), original_gate1).unwrap();

    let mut graph = CodebaseGraph::new();
    let idx0 = graph.upsert_node(SemanticNode {
        symbol_id: "src/gate0.ts#gate0".to_string(),
        display_name: "gate0".to_string(),
        file_path: file0.to_string(),
        byte_range: (0, 30),
        line_range: (1, 2),
        role: NodeRole::EntryGate,
        signature_hash: "h0".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });
    let idx1 = graph.upsert_node(SemanticNode {
        symbol_id: "src/gate1.ts#gate1".to_string(),
        display_name: "gate1".to_string(),
        file_path: file1.to_string(),
        byte_range: (0, 30),
        line_range: (1, 2),
        role: NodeRole::BusinessLogic,
        signature_hash: "h1".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });
    graph.add_edge(
        idx0,
        idx1,
        flowrail_core::graph::SemanticEdge {
            kind: flowrail_core::graph::EdgeKind::Calls,
            call_site_line: 1,
            is_dynamic_trace: false,
            confidence: flowrail_core::graph::EdgeConfidence::Extracted,
        },
    );

    let mut engine = CorridorMcpEngine::new(temp_dir.clone(), graph, false);

    // Open corridor (Gate 0 active, Gate 1 locked)
    engine
        .dispatch_tool(
            "corridor_open",
            &json!({
                "entry_symbol": "src/gate0.ts#gate0",
                "intent": "READ_FLOW"
            }),
        )
        .unwrap();

    // Propose edit to Gate 1 (out of order!)
    let res = engine.dispatch_tool(
        "corridor_propose_edit",
        &json!({
            "gate_index": 1,
            "new_file_content": "export const gate1 = 'MUTATED_CORRUPTED';\n"
        }),
    );

    assert!(res.is_err(), "Out of order propose edit must be rejected");
    let err_str = res.err().unwrap().to_string();
    assert!(err_str.contains("OUT_OF_ORDER_EDIT") || err_str.contains("LOCKED"));

    // Verify disk was NOT mutated
    let content_on_disk = fs::read_to_string(temp_dir.join(file1)).unwrap();
    assert_eq!(
        content_on_disk, original_gate1,
        "File on disk must NOT be overwritten when out-of-order edit is rejected"
    );
}

#[test]
fn test_rollback_newly_created_file_deletes_file() {
    let guard = TempDirGuard::new("flowrail_mcp_new_file_rollback");
    let temp_dir = guard.path().to_path_buf();
    fs::create_dir_all(temp_dir.join("src")).unwrap();

    let new_file = "src/newly_created.ts";
    let abs_new_file = temp_dir.join(new_file);
    assert!(!abs_new_file.exists());

    let mut graph = CodebaseGraph::new();
    graph.upsert_node(SemanticNode {
        symbol_id: "src/newly_created.ts#NewSymbol".to_string(),
        display_name: "NewSymbol".to_string(),
        file_path: new_file.to_string(),
        byte_range: (0, 30),
        line_range: (1, 2),
        role: NodeRole::EntryGate,
        signature_hash: "h_new".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });

    let mut engine = CorridorMcpEngine::new(temp_dir.clone(), graph, false);
    engine
        .dispatch_tool(
            "corridor_open",
            &json!({
                "entry_symbol": "src/newly_created.ts#NewSymbol",
                "intent": "READ_FLOW"
            }),
        )
        .unwrap();

    // Propose edit that creates the new file
    engine
        .dispatch_tool(
            "corridor_propose_edit",
            &json!({
                "gate_index": 0,
                "new_file_content": "export const NewSymbol = 42;\n"
            }),
        )
        .unwrap();

    assert!(abs_new_file.exists());

    // Rollback to gate 0
    engine
        .dispatch_tool("corridor_abort_or_rollback", &json!({ "to_gate_index": 0 }))
        .unwrap();

    // Newly created file must be deleted from disk, not left as empty file!
    assert!(
        !abs_new_file.exists(),
        "Rollback of newly created file must delete file from disk"
    );
}

#[test]
fn test_mcp_artifact_export_and_enrichment() {
    let guard = TempDirGuard::new("flowrail_mcp_art");
    let temp_dir = guard.path().to_path_buf();
    fs::create_dir_all(temp_dir.join("src")).unwrap();

    let file_path = "src/service.ts";
    let abs_path = temp_dir.join(file_path);
    fs::write(&abs_path, "export class OrderService {}\n").unwrap();

    let mut graph = CodebaseGraph::new();
    let symbol_id = format!("{}#OrderService", file_path);
    graph.upsert_node(SemanticNode {
        symbol_id: symbol_id.clone(),
        display_name: "OrderService".to_string(),
        file_path: file_path.to_string(),
        byte_range: (0, 30),
        line_range: (1, 2),
        role: NodeRole::EntryGate,
        signature_hash: "hash_order".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });

    let mut engine = CorridorMcpEngine::new(temp_dir.clone(), graph, false);

    // 1. Open corridor
    let open_res = engine
        .dispatch_tool(
            "corridor_open",
            &json!({
                "entry_symbol": symbol_id,
                "intent": "READ_FLOW"
            }),
        )
        .unwrap();
    assert!(open_res.contains("artifact_id"));
    assert!(open_res.contains("artifact_file"));

    // Check that artifact was automatically saved to workspace
    let latest_artifact = temp_dir.join(".flowrail/artifacts/latest.json");
    assert!(
        latest_artifact.exists(),
        "latest.json must be auto-generated"
    );

    // 2. Export artifact tool
    let export_res = engine
        .dispatch_tool("corridor_export_artifact", &json!({ "save_to_disk": true }))
        .unwrap();
    assert!(export_res.contains("clean_json_artifact"));
    assert!(export_res.contains("flowrail.artifact.v1"));

    // 3. Enrich artifact tool with valid ADHD/anti-slop payload
    let enrich_res = engine
        .dispatch_tool(
            "corridor_enrich_artifact",
            &json!({
                "enrichment": {
                    "status": "Enriched",
                    "human_readable_summary": "OrderService read flow corridor",
                    "adhd_view": {
                        "headline_action": "Inspect `src/service.ts` line 1 for OrderService contract.",
                        "state_anchor": "Gate 0 of 1 unlocked. Corridor active.",
                        "estimated_duration": "3 minutes",
                        "bounded_steps": [
                            "1. Inspect OrderService in `src/service.ts`",
                            "2. Complete corridor read flow"
                        ],
                        "visible_wins": [
                            "Zero external contagion in OrderService"
                        ],
                        "terminal_action": "Open `src/service.ts` and inspect.",
                        "error_diagnostic": null
                    },
                    "anti_slop_contract": {
                        "grounded_symbols": [symbol_id],
                        "grounded_files": [file_path],
                        "tangent_free": true,
                        "zero_conversational_fluff": true,
                        "direct_payload": true,
                        "validation_passed": false
                    },
                    "enrichment_timestamp": null
                }
            }),
        )
        .unwrap();

    assert!(enrich_res.contains("ENRICHMENT_VERIFIED_AND_APPLIED"));
    assert!(enrich_res.contains("Inspect `src/service.ts` line 1"));
}

#[test]
fn test_mcp_borrowed_patterns_confidence_detail_level_and_def_first() {
    let temp_dir = std::env::temp_dir().join(format!(
        "flowrail_borrowed_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(temp_dir.join("src")).unwrap();

    let mut graph = CodebaseGraph::new();
    let idx0 = graph.add_node(SemanticNode {
        symbol_id: "src/api.ts#handler".to_string(),
        display_name: "handler".to_string(),
        file_path: "src/api.ts".to_string(),
        byte_range: (0, 50),
        line_range: (1, 10),
        role: flowrail_core::graph::NodeRole::EntryGate,
        signature_hash: "hash_entry".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });
    let idx1 = graph.add_node(SemanticNode {
        symbol_id: "src/service.ts#Service".to_string(),
        display_name: "Service".to_string(),
        file_path: "src/service.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 20),
        role: flowrail_core::graph::NodeRole::ContractDefinition,
        signature_hash: "hash_service".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });
    // External dynamic dependent with Ambiguous confidence
    let idx_ext = graph.add_node(SemanticNode {
        symbol_id: "src/event_bus.ts#emit".to_string(),
        display_name: "emit".to_string(),
        file_path: "src/event_bus.ts".to_string(),
        byte_range: (0, 30),
        line_range: (1, 5),
        role: flowrail_core::graph::NodeRole::BusinessLogic,
        signature_hash: "hash_bus".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });

    graph.add_edge(
        idx0,
        idx1,
        flowrail_core::graph::SemanticEdge {
            kind: flowrail_core::graph::EdgeKind::Calls,
            call_site_line: 5,
            is_dynamic_trace: false,
            confidence: flowrail_core::graph::EdgeConfidence::Extracted,
        },
    );
    graph.add_edge(
        idx_ext,
        idx1,
        flowrail_core::graph::SemanticEdge {
            kind: flowrail_core::graph::EdgeKind::Calls,
            call_site_line: 2,
            is_dynamic_trace: true,
            confidence: flowrail_core::graph::EdgeConfidence::Ambiguous,
        },
    );

    let mut engine = CorridorMcpEngine::new(temp_dir.clone(), graph, false);

    // 1. Verify Definition-First Quick Location in find_symbols and PageRank in discover_entrypoints
    let symbols_res = engine
        .dispatch_tool("flowrail_find_symbols", &json!({ "query": "handler" }))
        .unwrap();
    let symbols_json: Value = serde_json::from_str(&symbols_res).unwrap();
    assert_eq!(symbols_json["symbols"][0]["def_location"], "src/api.ts:1");
    assert_eq!(symbols_json["symbols"][0]["is_definition"], true);

    let ep_res = engine
        .dispatch_tool(
            "flowrail_discover_entrypoints",
            &json!({ "query": "handler" }),
        )
        .unwrap();
    let ep_json: Value = serde_json::from_str(&ep_res).unwrap();
    assert!(ep_json["entrypoints"][0].get("pagerank_score").is_some());

    // 2. Verify detail_level="minimal" and max_depth in open
    let open_res = engine
        .dispatch_tool(
            "flowrail_open",
            &json!({
                "entry_symbol": "src/api.ts#handler",
                "intent": "READ_FLOW",
                "detail_level": "minimal",
                "max_depth": 2
            }),
        )
        .unwrap();
    let open_json: Value = serde_json::from_str(&open_res).unwrap();
    assert!(open_json["built_at_commit"].is_null() || open_json["built_at_commit"].is_string());
    assert_eq!(open_json["roadmap"][0]["status"], "Unlocked");
    assert!(open_json.get("co_changed_files_warning").is_some());

    // 3. Verify detail_level="minimal" in inspect_gate and Ambiguous Edge Confidence
    let inspect_res = engine
        .dispatch_tool(
            "flowrail_inspect_gate",
            &json!({
                "gate_index": 0,
                "detail_level": "minimal"
            }),
        )
        .unwrap();
    let inspect_json: Value = serde_json::from_str(&inspect_res).unwrap();
    assert_eq!(inspect_json["mode"], "compact_token_optimized");
    assert!(inspect_json.get("code_slice").is_none());

    // 4. Verify contract_delta_kind on propose_edit
    let propose_res = engine
        .dispatch_tool(
            "flowrail_propose_edit",
            &json!({
                "gate_index": 0,
                "new_file_content": "export function handler(): void {}"
            }),
        )
        .unwrap();
    let propose_json: Value = serde_json::from_str(&propose_res).unwrap();
    assert!(propose_json.get("contract_delta_kind").is_some());

    let _ = fs::remove_dir_all(temp_dir);
}


#[test]
fn test_propose_edit_dry_run_targeted_edits_diff_and_diagnostics_scope() {
    let temp_dir =
        std::env::temp_dir().join(format!("flowrail_mcp_dry_run_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("src")).unwrap();

    let original = "export function greet(name: string): string {\n    return name;\n}\n";
    let target_file = temp_dir.join("src/greet.ts");
    fs::write(&target_file, original).unwrap();

    let mut graph = CodebaseGraph::new();
    graph.upsert_node(SemanticNode {
        symbol_id: "src/greet.ts#greet".to_string(),
        display_name: "greet".to_string(),
        file_path: "src/greet.ts".to_string(),
        byte_range: (0, original.len()),
        line_range: (1, 3),
        role: NodeRole::EntryGate,
        signature_hash: "h1".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });

    let mut engine = CorridorMcpEngine::new(temp_dir.clone(), graph, false);
    engine
        .dispatch_tool(
            "flowrail_open",
            &json!({
                "entry_symbol": "src/greet.ts#greet",
                "intent": "READ_FLOW"
            }),
        )
        .unwrap();

    // 1. Dry-run with targeted edits[] must NOT mutate disk
    let dry_res = engine
        .dispatch_tool(
            "flowrail_propose_edit",
            &json!({
                "gate_index": 0,
                "dry_run": true,
                "edits": [
                    {
                        "old_text": "greet(name: string)",
                        "new_text": "greet(name: string, title: string)"
                    }
                ]
            }),
        )
        .unwrap();
    let dry_json: Value = serde_json::from_str(&dry_res).unwrap();
    assert_eq!(dry_json["dry_run"], true);
    assert_eq!(dry_json["is_breaking"], true);
    assert_eq!(dry_json["typecheck_ran"], false);
    assert_eq!(dry_json["diagnostics_scope"], "syntax_only");
    assert!(dry_json["warning"].as_str().unwrap().contains("typecheck_ran=false"));
    assert!(dry_json["diff"].as_str().unwrap().contains("+export function greet(name: string, title: string): string {"));
    assert_eq!(
        fs::read_to_string(&target_file).unwrap(),
        original,
        "dry_run must not write to disk"
    );

    // 2. Non-dry-run with targeted edits[] must apply to disk
    let apply_res = engine
        .dispatch_tool(
            "flowrail_propose_edit",
            &json!({
                "gate_index": 0,
                "edits": [
                    {
                        "old_text": "greet(name: string)",
                        "new_text": "greet(name: string, title: string)"
                    }
                ]
            }),
        )
        .unwrap();
    let apply_json: Value = serde_json::from_str(&apply_res).unwrap();
    assert_eq!(apply_json["dry_run"], false);
    assert!(fs::read_to_string(&target_file)
        .unwrap()
        .contains("title: string"));

    let _ = fs::remove_dir_all(&temp_dir);
}
