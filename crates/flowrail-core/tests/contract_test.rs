use flowrail_core::contract::{ContractHasher, ContractKind, ExportedSymbolContract};

#[test]
fn test_contract_hasher_whitespace_invariance() {
    let contract_a = vec![ExportedSymbolContract {
        symbol_name: "TaxCalculationResult".to_string(),
        kind: ContractKind::Interface,
        normalized_signature: "interface TaxCalculationResult { vat: number; total: number; }"
            .to_string(),
        signature_hash: "".to_string(),
    }];

    let contract_b = vec![ExportedSymbolContract {
        symbol_name: "TaxCalculationResult".to_string(),
        kind: ContractKind::Interface,
        normalized_signature:
            "interface   TaxCalculationResult  {\n  vat: number;\n  total: number;\n}".to_string(),
        signature_hash: "".to_string(),
    }];

    let hash_a = ContractHasher::hash_normalized_surface(&contract_a);
    let hash_b = ContractHasher::hash_normalized_surface(&contract_b);

    assert_eq!(
        hash_a, hash_b,
        "Signatures differing only in whitespace must produce identical hash"
    );
}

#[test]
fn test_contract_hasher_diff_detection() {
    let before = vec![ExportedSymbolContract {
        symbol_name: "TaxCalculationResult".to_string(),
        kind: ContractKind::Interface,
        normalized_signature: "interface TaxCalculationResult { vat: number; total: number; }"
            .to_string(),
        signature_hash: "hash_before".to_string(),
    }];

    let after = vec![ExportedSymbolContract {
        symbol_name: "TaxCalculationResult".to_string(),
        kind: ContractKind::Interface,
        normalized_signature:
            "interface TaxCalculationResult { vat: number; total: number; compound: boolean; }"
                .to_string(),
        signature_hash: "hash_after".to_string(),
    }];

    let diff = ContractHasher::diff_surfaces(&before, &after);
    assert!(diff.is_mutated);
    assert!(
        diff.is_breaking,
        "Adding required property is a breaking change"
    );
    assert_eq!(
        diff.modified_symbols,
        vec!["TaxCalculationResult".to_string()]
    );
    assert!(diff.added_symbols.is_empty());
    assert!(diff.removed_symbols.is_empty());
}

#[test]
fn test_contract_compatible_optional_property() {
    let before = vec![ExportedSymbolContract {
        symbol_name: "UserProfile".to_string(),
        kind: ContractKind::Interface,
        normalized_signature: "interface UserProfile { id: string; name: string; }".to_string(),
        signature_hash: "hash_user_1".to_string(),
    }];

    let after = vec![ExportedSymbolContract {
        symbol_name: "UserProfile".to_string(),
        kind: ContractKind::Interface,
        normalized_signature: "interface UserProfile { id: string; name: string; bio?: string; }"
            .to_string(),
        signature_hash: "hash_user_2".to_string(),
    }];

    let diff = ContractHasher::diff_surfaces(&before, &after);
    assert!(diff.is_mutated);
    assert!(
        !diff.is_breaking,
        "Adding optional property bio?: string is backward-compatible"
    );
    assert!(diff.breaking_reasons.is_empty());
}

#[test]
fn test_contract_compatible_function_optional_parameter() {
    let before = vec![ExportedSymbolContract {
        symbol_name: "fetchUser".to_string(),
        kind: ContractKind::FunctionSignature,
        normalized_signature: "function fetchUser(userId: string): Promise<User>".to_string(),
        signature_hash: "hash_fn_1".to_string(),
    }];

    let after = vec![ExportedSymbolContract {
        symbol_name: "fetchUser".to_string(),
        kind: ContractKind::FunctionSignature,
        normalized_signature: "function fetchUser(userId: string, cache?: boolean): Promise<User>"
            .to_string(),
        signature_hash: "hash_fn_2".to_string(),
    }];

    let diff = ContractHasher::diff_surfaces(&before, &after);
    assert!(diff.is_mutated);
    assert!(
        !diff.is_breaking,
        "Adding optional parameter cache?: boolean is backward-compatible"
    );
}

#[test]
fn test_diff_surfaces_punctuation_spacing_invariance() {
    let before = vec![ExportedSymbolContract {
        symbol_name: "CalculateTaxPayload".to_string(),
        kind: ContractKind::Interface,
        normalized_signature: "interface CalculateTaxPayload { id: string; amount: number; }"
            .to_string(),
        signature_hash: "".to_string(),
    }];

    let after = vec![ExportedSymbolContract {
        symbol_name: "CalculateTaxPayload".to_string(),
        kind: ContractKind::Interface,
        normalized_signature: "interface CalculateTaxPayload { id : string ; amount : number ; }"
            .to_string(),
        signature_hash: "".to_string(),
    }];

    let diff = ContractHasher::diff_surfaces(&before, &after);
    assert!(
        !diff.is_mutated,
        "Formatting spacing around punctuation must not mutate contract"
    );
    assert!(
        diff.modified_symbols.is_empty(),
        "Formatting spacing around punctuation must not mark symbol as modified"
    );
    assert!(!diff.is_breaking);
}
