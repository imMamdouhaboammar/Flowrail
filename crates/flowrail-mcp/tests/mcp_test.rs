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
