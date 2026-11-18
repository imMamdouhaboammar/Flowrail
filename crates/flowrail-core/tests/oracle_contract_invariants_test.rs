use flowrail_core::contract::{ContractHasher, ContractKind, ExportedSymbolContract};

/// Deterministic 32-bit LCG pseudorandom generator for repeatable property fuzzing.
struct SeededPrng {
    state: u32,
}

impl SeededPrng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    fn next_range(&mut self, min: usize, max: usize) -> usize {
        let span = max - min + 1;
        min + (self.next_u32() as usize % span)
    }

    fn random_whitespace(&mut self) -> String {
        let ws_chars = [' ', '\t', '\n', '\r'];
        let len = self.next_range(1, 5);
        (0..len)
            .map(|_| ws_chars[self.next_range(0, ws_chars.len() - 1)])
            .collect()
    }
}

#[test]
fn test_oracle_metamorphic_whitespace_and_permutation_invariance() {
    // Seed: 0xC0478AC1
    let mut prng = SeededPrng::new(0xC0478AC1);

    let baseline_contract = vec![
        ExportedSymbolContract {
            symbol_name: "OrderPayload".to_string(),
            kind: ContractKind::Interface,
            normalized_signature:
                "interface OrderPayload { id: string; amount: number; currency: string; }"
                    .to_string(),
            signature_hash: "".to_string(),
        },
        ExportedSymbolContract {
            symbol_name: "calculateTax".to_string(),
            kind: ContractKind::FunctionSignature,
            normalized_signature: "function calculateTax(amount: number, rate: number): number"
                .to_string(),
            signature_hash: "".to_string(),
        },
    ];

    let baseline_hash = ContractHasher::hash_normalized_surface(&baseline_contract);

    // Test 100 randomized whitespace and formatting permutations
    for iteration in 0..100 {
        let ws1 = prng.random_whitespace();
        let ws2 = prng.random_whitespace();
        let ws3 = prng.random_whitespace();
        let ws4 = prng.random_whitespace();

        let mutated_contract = vec![
            ExportedSymbolContract {
                symbol_name: "calculateTax".to_string(), // Reversed array order to verify sort-invariance
                kind: ContractKind::FunctionSignature,
                normalized_signature: format!(
                    "function{ws1}calculateTax({ws2}amount:{ws3}number,{ws4}rate:{ws1}number){ws2}:{ws3}number"
                ),
                signature_hash: "".to_string(),
            },
            ExportedSymbolContract {
                symbol_name: "OrderPayload".to_string(),
                kind: ContractKind::Interface,
                normalized_signature: format!(
                    "interface{ws1}OrderPayload{ws2}{{{ws3}id:{ws4}string;{ws1}amount:{ws2}number;{ws3}currency:{ws4}string;{ws1}}}"
                ),
                signature_hash: "".to_string(),
            },
        ];

        let mutated_hash = ContractHasher::hash_normalized_surface(&mutated_contract);
        assert_eq!(
            baseline_hash, mutated_hash,
            "Failed at iteration {}: Whitespace and symbol order permutation must not alter hash",
            iteration
        );
    }
}

#[test]
fn test_oracle_backward_compatibility_monotonicity_property() {
    let mut prng = SeededPrng::new(0x7A55C0DE);

    let base_contract = vec![ExportedSymbolContract {
        symbol_name: "Invoice".to_string(),
        kind: ContractKind::Interface,
        normalized_signature: "interface Invoice { id: string; total: number; }".to_string(),
        signature_hash: "".to_string(),
    }];

    // Test 50 successive additions of optional properties
    for i in 0..50 {
        let optional_prop_name = format!("optionalField_{}_{}", i, prng.next_u32() % 1000);
        let extended_sig = format!(
            "interface Invoice {{ id: string; total: number; {}?: string; }}",
            optional_prop_name
        );

        let after_contract = vec![ExportedSymbolContract {
            symbol_name: "Invoice".to_string(),
            kind: ContractKind::Interface,
            normalized_signature: extended_sig,
            signature_hash: "".to_string(),
        }];

        let diff = ContractHasher::diff_surfaces(&base_contract, &after_contract);
        assert!(diff.is_mutated, "Adding field must trigger mutation flag");
        assert!(
            !diff.is_breaking,
            "Adding optional property '{}' must strictly be backward-compatible (is_breaking: false)",
            optional_prop_name
        );
        assert!(
            diff.breaking_reasons.is_empty(),
            "Breaking reasons must be empty for backward-compatible addition"
        );
    }
}

#[test]
fn test_oracle_breaking_change_detection_property() {
    let mut prng = SeededPrng::new(0xB4ADF00D);

    let base_contract = vec![ExportedSymbolContract {
        symbol_name: "UserProfile".to_string(),
        kind: ContractKind::Interface,
        normalized_signature: "interface UserProfile { id: string; email: string; }".to_string(),
        signature_hash: "".to_string(),
    }];

    // Test 50 required property additions without '?'
    for i in 0..50 {
        let required_prop_name = format!("mandatoryField_{}_{}", i, prng.next_u32() % 1000);
        let breaking_sig = format!(
            "interface UserProfile {{ id: string; email: string; {}: string; }}",
            required_prop_name
        );

        let after_contract = vec![ExportedSymbolContract {
            symbol_name: "UserProfile".to_string(),
            kind: ContractKind::Interface,
            normalized_signature: breaking_sig,
            signature_hash: "".to_string(),
        }];

        let diff = ContractHasher::diff_surfaces(&base_contract, &after_contract);
        assert!(diff.is_mutated);
        assert!(
            diff.is_breaking,
            "Adding required property '{}' must be detected as a breaking change",
            required_prop_name
        );
        assert!(
            diff.breaking_reasons
                .iter()
                .any(|r| r.contains(&required_prop_name)),
            "Breaking reasons must mention the violating property"
        );
    }
}
