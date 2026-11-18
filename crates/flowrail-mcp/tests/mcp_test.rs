use flowrail_core::graph::{CodebaseGraph, NodeRole, SemanticNode};
use flowrail_mcp::tools::CorridorMcpEngine;
use serde_json::json;
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
