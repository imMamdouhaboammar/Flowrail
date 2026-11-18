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
    pub is_breaking: bool,
    pub breaking_reasons: Vec<String>,
    pub added_symbols: Vec<String>,
    pub removed_symbols: Vec<String>,
    pub modified_symbols: Vec<String>,
}

pub struct ContractHasher;

impl ContractHasher {
    /// Normalizes whitespace and structural punctuation from extracted TypeScript signature strings
    /// ensuring formatting-invariant token comparison and a deterministic BLAKE3 hash.
    pub fn normalize_signature_tokens(sig: &str) -> String {
        let mut spaced = String::with_capacity(sig.len() * 2);
        for c in sig.chars() {
            if matches!(c, '{' | '}' | '(' | ')' | ':' | ';' | ',') {
                spaced.push(' ');
                spaced.push(c);
                spaced.push(' ');
            } else {
                spaced.push(c);
            }
        }
        spaced.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Computes a deterministic BLAKE3 hash over sorted normalized symbol contracts.
    pub fn hash_normalized_surface(contracts: &[ExportedSymbolContract]) -> String {
        let mut hasher = blake3::Hasher::new();
        let mut sorted: Vec<&ExportedSymbolContract> = contracts.iter().collect();
        sorted.sort_by(|a, b| a.symbol_name.cmp(&b.symbol_name));

        for item in sorted {
            hasher.update(item.symbol_name.as_bytes());
            hasher.update(b":");
            let compact = Self::normalize_signature_tokens(&item.normalized_signature);
            hasher.update(compact.as_bytes());
            hasher.update(b";");
        }
        hasher.finalize().to_hex().to_string()
    }

    fn index_surface_contracts(
        contracts: &[ExportedSymbolContract],
    ) -> HashMap<&String, (String, ContractKind)> {
        contracts
            .iter()
            .map(|c| {
                let compact = Self::normalize_signature_tokens(&c.normalized_signature);
                (&c.symbol_name, (compact, c.kind))
            })
            .collect()
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
        let mut breaking_reasons = Vec::new();

        let before_map = Self::index_surface_contracts(before);
        let after_map = Self::index_surface_contracts(after);

        for (name, (sig_b, kind_b)) in &before_map {
            if let Some((sig_a, kind_a)) = after_map.get(name) {
                if sig_b != sig_a || kind_b != kind_a {
                    modified.push((*name).clone());
                    if let Some(reason) =
                        Self::classify_symbol_change(name, *kind_b, sig_b, *kind_a, sig_a)
                    {
                        breaking_reasons.push(reason);
                    }
                }
            } else {
                removed.push((*name).clone());
                breaking_reasons.push(format!("Removed exported symbol '{}'", name));
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
        breaking_reasons.sort();

        let is_mutated = previous_hash != current_hash;
        let is_breaking = !breaking_reasons.is_empty();

        ContractDiff {
            previous_hash,
            current_hash,
            is_mutated,
            is_breaking,
            breaking_reasons,
            added_symbols: added,
            removed_symbols: removed,
            modified_symbols: modified,
        }
    }

    /// Classifies whether a mutation to an exported symbol is a breaking change or backward-compatible.
    /// - Removing properties or parameters: BREAKING
    /// - Adding non-optional properties or required parameters: BREAKING
    /// - Adding optional properties (e.g. `foo?: string`) or optional parameters: COMPATIBLE
    fn classify_symbol_change(
        name: &str,
        kind_before: ContractKind,
        sig_before: &str,
        kind_after: ContractKind,
        sig_after: &str,
    ) -> Option<String> {
        if kind_before != kind_after {
            return Some(format!(
                "Symbol '{}' changed kind from {:?} to {:?}",
                name, kind_before, kind_after
            ));
        }

        match kind_before {
            ContractKind::Interface | ContractKind::TypeAlias => {
                let props_before = Self::parse_interface_properties(sig_before);
                let props_after = Self::parse_interface_properties(sig_after);

                // Any removed property is breaking
                for (prop, (opt_before, type_before)) in &props_before {
                    if let Some((opt_after, type_after)) = props_after.get(prop) {
                        if *opt_before && !opt_after {
                            return Some(format!(
                                "Optional property '{}' in interface '{}' became required",
                                prop, name
                            ));
                        }
                        if type_before != type_after {
                            return Some(format!(
                                "Property '{}' in interface '{}' changed type from '{}' to '{}'",
                                prop, name, type_before, type_after
                            ));
                        }
                    } else {
                        return Some(format!(
                            "Property '{}' was removed from interface '{}'",
                            prop, name
                        ));
                    }
                }

                // Any newly added required property without '?' is breaking
                for (prop, (opt_after, _)) in &props_after {
                    if !props_before.contains_key(prop) && !opt_after {
                        return Some(format!(
                            "Required property '{}' was added to interface '{}' without optional marker",
                            prop, name
                        ));
                    }
                }

                None
            }
            ContractKind::FunctionSignature => {
                let (params_b, ret_b) = Self::parse_function_signature(sig_before);
                let (params_a, ret_a) = Self::parse_function_signature(sig_after);

                if ret_b != ret_a && !ret_b.is_empty() && !ret_a.is_empty() {
                    return Some(format!(
                        "Function '{}' return type changed from '{}' to '{}'",
                        name, ret_b, ret_a
                    ));
                }

                // Existing parameters must stay compatible
                for (i, param_b) in params_b.iter().enumerate() {
                    if let Some(param_a) = params_a.get(i) {
                        if param_b.2 != param_a.2 {
                            return Some(format!(
                                "Function '{}' parameter {} changed type from '{}' to '{}'",
                                name, param_b.0, param_b.2, param_a.2
                            ));
                        }
                    } else {
                        return Some(format!(
                            "Function '{}' parameter {} was removed",
                            name, param_b.0
                        ));
                    }
                }

                // Any new parameters in after must be optional
                if params_a.len() > params_b.len() {
                    for param_a in &params_a[params_b.len()..] {
                        if !param_a.1 {
                            return Some(format!(
                                "Required parameter '{}' was added to function '{}' without default",
                                param_a.0, name
                            ));
                        }
                    }
                }

                None
            }
            ContractKind::ClassPublicSurface => {
                // If the class signature changed, treat as breaking if methods/properties were removed
                Some(format!("Class '{}' public surface was modified", name))
            }
        }
    }

    fn parse_interface_properties(sig: &str) -> HashMap<String, (bool, String)> {
        let mut props = HashMap::new();
        let body = if let Some(start) = sig.find('{') {
            if let Some(end) = sig.rfind('}') {
                &sig[start + 1..end]
            } else {
                &sig[start + 1..]
            }
        } else {
            sig
        };

        for raw_line in body.split([';', '\n']) {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((left, right)) = line.split_once(':') {
                let left_clean = left
                    .trim()
                    .trim_start_matches("readonly")
                    .trim_start_matches("public")
                    .trim();
                let is_optional = left_clean.ends_with('?');
                let prop_name = left_clean.trim_end_matches('?').trim().to_string();
                let type_clean = right.trim().trim_end_matches(';').trim().to_string();
                if !prop_name.is_empty() {
                    props.insert(prop_name, (is_optional, type_clean));
                }
            }
        }
        props
    }

    fn parse_function_signature(sig: &str) -> (Vec<(String, bool, String)>, String) {
        let mut params = Vec::new();

        if let Some(open_paren) = sig.find('(')
            && let Some(close_paren) = sig.rfind(')')
        {
            let param_str = &sig[open_paren + 1..close_paren];
            let after_paren = &sig[close_paren + 1..];
            let ret_type = if let Some((_, r)) = after_paren.split_once(':') {
                r.trim().trim_end_matches('{').trim().to_string()
            } else {
                String::new()
            };

            for raw_param in param_str.split(',') {
                let param = raw_param.trim();
                if param.is_empty() {
                    continue;
                }
                let is_optional = param.contains('?') || param.contains('=');
                if let Some((name_part, type_part)) = param.split_once(':') {
                    let name = name_part.trim().trim_end_matches('?').trim().to_string();
                    let typ = type_part.split('=').next().unwrap_or("").trim().to_string();
                    params.push((name, is_optional, typ));
                } else {
                    params.push((param.to_string(), is_optional, String::new()));
                }
            }
            return (params, ret_type);
        }
        (params, String::new())
    }
}
