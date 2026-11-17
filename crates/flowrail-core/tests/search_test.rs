use flowrail_core::frecency::FrecencyStore;
use flowrail_core::graph::{CodebaseGraph, NodeRole, SemanticNode};
use flowrail_core::search::{FuzzySearchEngine, QueryConstraints};

fn sample_graph() -> CodebaseGraph {
    let mut g = CodebaseGraph::new();

    g.upsert_node(SemanticNode {
        symbol_id: "src/billing/checkout.ts#CheckoutController".to_string(),
        display_name: "CheckoutController".to_string(),
        file_path: "src/billing/checkout.ts".to_string(),
        byte_range: (0, 100),
        line_range: (1, 15),
        role: NodeRole::EntryGate,
        signature_hash: "hash_checkout".to_string(),
        mutates_domain_state: true,
        is_exported: true,
    });

    g.upsert_node(SemanticNode {
        symbol_id: "src/billing/invoice.ts#InvoiceContract".to_string(),
        display_name: "InvoiceContract".to_string(),
        file_path: "src/billing/invoice.ts".to_string(),
        byte_range: (0, 80),
        line_range: (1, 10),
        role: NodeRole::ContractDefinition,
        signature_hash: "hash_invoice".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });

    g.upsert_node(SemanticNode {
        symbol_id: "src/tax/calculator.ts#TaxCalculator".to_string(),
        display_name: "TaxCalculator".to_string(),
        file_path: "src/tax/calculator.ts".to_string(),
        byte_range: (0, 120),
        line_range: (1, 20),
        role: NodeRole::BusinessLogic,
        signature_hash: "hash_tax".to_string(),
        mutates_domain_state: false,
        is_exported: true,
    });

    g.upsert_node(SemanticNode {
        symbol_id: "tests/billing/checkout.test.ts#testCheckout".to_string(),
        display_name: "testCheckout".to_string(),
        file_path: "tests/billing/checkout.test.ts".to_string(),
        byte_range: (0, 50),
        line_range: (1, 8),
        role: NodeRole::UtilityNoise,
        signature_hash: "hash_test".to_string(),
        mutates_domain_state: false,
        is_exported: false,
    });

    g
}

#[test]
fn test_query_constraints_parsing() {
    let q = QueryConstraints::parse("*.ts !tests/ billing/ checkout");
    assert_eq!(q.terms, vec!["checkout"]);
    assert_eq!(q.positive_globs, vec!["*.ts"]);
    assert_eq!(q.negative_globs, vec!["tests"]);
    assert_eq!(q.path_scope, Some("billing".to_string()));

    assert!(q.matches_path("src/billing/checkout.ts"));
    assert!(!q.matches_path("tests/billing/checkout.test.ts"));
    assert!(!q.matches_path("src/tax/calculator.ts"));
}

#[test]
fn test_exact_and_substring_search() {
    let g = sample_graph();
    let engine = FuzzySearchEngine::new(&g, None, None);

    let hits = engine.search("checkout", false, false, 10);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].display_name, "CheckoutController");
    assert_eq!(hits[0].role, NodeRole::EntryGate);
}

#[test]
fn test_typo_tolerant_search() {
    let g = sample_graph();
    let engine = FuzzySearchEngine::new(&g, None, None);

    // Typo: "Chekout" missing 'c'
    let hits = engine.search("Chekout", false, false, 10);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].display_name, "CheckoutController");
    assert_eq!(hits[0].stage, 3);
}

#[test]
fn test_acronym_search() {
    let g = sample_graph();
    let engine = FuzzySearchEngine::new(&g, None, None);

    // Acronym: "CC" for CheckoutController
    let hits = engine.search("CC", false, false, 10);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].display_name, "CheckoutController");
}

#[test]
fn test_frecency_ranking_boost() {
    let g = sample_graph();
    let frecency = FrecencyStore::new();

    // Repeatedly access InvoiceContract
    for _ in 0..5 {
        frecency.record_access("src/billing/invoice.ts#InvoiceContract");
    }

    let engine_with_frecency = FuzzySearchEngine::new(&g, None, Some(&frecency));
    let hits = engine_with_frecency.search("", false, false, 10);

    // InvoiceContract should receive high score and lead
    assert!(!hits.is_empty());
    assert_eq!(hits[0].display_name, "InvoiceContract");
    assert!(hits[0].frecency_score > 100.0);
}
