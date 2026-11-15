use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedSymbolContract {
    pub symbol_name: String,
    pub kind: ContractKind,
    pub normalized_signature: String,
    pub signature_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractKind {
    Interface,
    TypeAlias,
    FunctionSignature,
    ClassPublicSurface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractDiff {
    pub previous_hash: String,
    pub current_hash: String,
    pub is_mutated: bool,
    pub added_symbols: Vec<String>,
    pub removed_symbols: Vec<String>,
    pub modified_symbols: Vec<String>,
}

pub struct ContractHasher;

impl ContractHasher {
    /// Normalizes whitespace from extracted TypeScript signature strings
    /// and computes a deterministic BLAKE3 hash.
    pub fn hash_normalized_surface(contracts: &[ExportedSymbolContract]) -> String {
        let mut hasher = blake3::Hasher::new();
        let mut sorted: Vec<&ExportedSymbolContract> = contracts.iter().collect();
        sorted.sort_by(|a, b| a.symbol_name.cmp(&b.symbol_name));

        for item in sorted {
            hasher.update(item.symbol_name.as_bytes());
            hasher.update(b":");
            let compact: String = item
                .normalized_signature
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            hasher.update(compact.as_bytes());
            hasher.update(b";");
        }
        hasher.finalize().to_hex().to_string()
    }

    pub fn diff_surfaces(
        before: &[ExportedSymbolContract],
        after: &[ExportedSymbolContract],
    ) -> ContractDiff {
        let previous_hash = Self::hash_normalized_surface(before);
        let current_hash = Self::hash_normalized_surface(after);

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut modified = Vec::new();

        let before_map: HashMap<_, _> = before
            .iter()
            .map(|c| {
                let compact = c
                    .normalized_signature
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                (&c.symbol_name, (compact, c.kind))
            })
            .collect();

        let after_map: HashMap<_, _> = after
            .iter()
            .map(|c| {
                let compact = c
                    .normalized_signature
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                (&c.symbol_name, (compact, c.kind))
            })
            .collect();

        for (name, (sig_b, kind_b)) in &before_map {
            if let Some((sig_a, kind_a)) = after_map.get(name) {
                if sig_b != sig_a || kind_b != kind_a {
                    modified.push((*name).clone());
                }
            } else {
                removed.push((*name).clone());
            }
        }

        for name in after_map.keys() {
            if !before_map.contains_key(name) {
                added.push((*name).clone());
            }
        }

        added.sort();
        removed.sort();
        modified.sort();

        let is_mutated = previous_hash != current_hash;

        ContractDiff {
            previous_hash,
            current_hash,
            is_mutated,
            added_symbols: added,
            removed_symbols: removed,
            modified_symbols: modified,
        }
    }
}
