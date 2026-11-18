use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct TsConfigResolver {
    pub base_url: Option<String>,
    pub paths: HashMap<String, Vec<String>>,
}

impl TsConfigResolver {
    /// Loads tsconfig.json from the workspace root or parent directories,
    /// safely stripping comments (jsonc support).
    pub fn load_from_workspace(workspace_root: &Path) -> Option<Self> {
        let tsconfig_path = workspace_root.join("tsconfig.json");
        if !tsconfig_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&tsconfig_path).ok()?;
        let clean_json = Self::strip_jsonc_comments(&content);
        let parsed: Value = serde_json::from_str(&clean_json).ok()?;

        let compiler_options = parsed.get("compilerOptions")?;
        let base_url = compiler_options
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut paths_map = HashMap::new();
        if let Some(paths_obj) = compiler_options.get("paths").and_then(|v| v.as_object()) {
            for (pattern, targets) in paths_obj {
                if let Some(targets_arr) = targets.as_array() {
                    let list: Vec<String> = targets_arr
                        .iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect();
                    paths_map.insert(pattern.clone(), list);
                }
            }
        }

        Some(Self {
            base_url,
            paths: paths_map,
        })
    }

    /// Strips single-line (//) and multi-line (/* */) comments while respecting string literals.
    pub fn strip_jsonc_comments(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();
        let mut in_string = false;
        let mut escape = false;

        while let Some(c) = chars.next() {
            if in_string {
                result.push(c);
                if escape {
                    escape = false;
                } else if c == '\\' {
                    escape = true;
                } else if c == '"' {
                    in_string = false;
                }
            } else if c == '"' {
                in_string = true;
                result.push(c);
            } else if c == '/' {
                if let Some(&next_c) = chars.peek() {
                    if next_c == '/' {
                        // Skip until newline
                        chars.next();
                        for nc in chars.by_ref() {
                            if nc == '\n' {
                                result.push('\n');
                                break;
                            }
                        }
                    } else if next_c == '*' {
                        // Skip until */
                        chars.next();
                        let mut prev = ' ';
                        for nc in chars.by_ref() {
                            if prev == '*' && nc == '/' {
                                break;
                            }
                            if nc == '\n' {
                                result.push('\n');
                            }
                            prev = nc;
                        }
                    } else {
                        result.push(c);
                    }
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Normalizes a path by resolving '.' and '..' components without filesystem access.
    pub fn normalize_path(p: &Path) -> PathBuf {
        let mut components = Vec::new();
        for comp in p.components() {
            match comp {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::Normal(c) => {
                    components.push(c);
                }
                _ => {}
            }
        }
        let mut out = PathBuf::new();
        for c in components {
            out.push(c);
        }
        out
    }

    /// Resolves an import specifier (e.g. `./checkout.service` or `@/modules/order` or `@order/service`)
    /// relative to the importing file and tsconfig path aliases.
    /// Returns the resolved normalized relative path from `workspace_root` if found on disk.
    pub fn resolve_module(
        &self,
        workspace_root: &Path,
        importing_file: &Path,
        specifier: &str,
    ) -> Option<PathBuf> {
        let specifier = specifier.trim();
        if specifier.is_empty() {
            return None;
        }

        // Case 1: Relative imports (starts with '.' or '/')
        if specifier.starts_with('.') || specifier.starts_with('/') {
            let parent_dir = importing_file.parent().unwrap_or(Path::new(""));
            let candidate_abs = workspace_root.join(parent_dir).join(specifier);
            if let Some(resolved) = Self::probe_extensions(&candidate_abs)
                && let Ok(rel) = resolved.strip_prefix(workspace_root)
            {
                return Some(Self::normalize_path(rel));
            }
            return None;
        }

        // Case 2: tsconfig paths matching
        let base_dir = if let Some(ref base) = self.base_url {
            if base == "." || base == "./" {
                workspace_root.to_path_buf()
            } else {
                workspace_root.join(base)
            }
        } else {
            workspace_root.to_path_buf()
        };

        for (pattern, targets) in &self.paths {
            if let Some((prefix, suffix)) = pattern.split_once('*') {
                if specifier.starts_with(prefix) && specifier.ends_with(suffix) {
                    let wildcard = &specifier[prefix.len()..specifier.len() - suffix.len()];
                    for target in targets {
                        let substituted = target.replace('*', wildcard);
                        let candidate_abs = base_dir.join(&substituted);
                        if let Some(resolved) = Self::probe_extensions(&candidate_abs)
                            && let Ok(rel) = resolved.strip_prefix(workspace_root)
                        {
                            return Some(Self::normalize_path(rel));
                        }
                    }
                }
            } else if pattern == specifier {
                for target in targets {
                    let candidate_abs = base_dir.join(target);
                    if let Some(resolved) = Self::probe_extensions(&candidate_abs)
                        && let Ok(rel) = resolved.strip_prefix(workspace_root)
                    {
                        return Some(Self::normalize_path(rel));
                    }
                }
            }
        }

        // Case 3: baseUrl fallback
        if self.base_url.is_some() {
            let candidate_abs = base_dir.join(specifier);
            if let Some(resolved) = Self::probe_extensions(&candidate_abs)
                && let Ok(rel) = resolved.strip_prefix(workspace_root)
            {
                return Some(Self::normalize_path(rel));
            }
        }

        None
    }

    /// Checks for standard TypeScript/JavaScript file extensions and directory index files.
    /// Supports NodeNext/ESM imports where `.js`/`.mjs`/`.jsx` specifiers map to `.ts`/`.mts`/`.tsx` files on disk.
    fn probe_extensions(base_abs: &Path) -> Option<PathBuf> {
        // Direct file match
        if base_abs.is_file() {
            return Some(base_abs.to_path_buf());
        }

        let base_str = base_abs.to_string_lossy();
        let clean_base = if let Some(stripped) = base_str.strip_suffix(".js") {
            stripped
        } else if let Some(stripped) = base_str.strip_suffix(".jsx") {
            stripped
        } else if let Some(stripped) = base_str.strip_suffix(".mjs") {
            stripped
        } else if let Some(stripped) = base_str.strip_suffix(".cjs") {
            stripped
        } else {
            &base_str
        };

        let candidates = [
            PathBuf::from(format!("{}.ts", clean_base)),
            PathBuf::from(format!("{}.tsx", clean_base)),
            PathBuf::from(format!("{}.d.ts", clean_base)),
            PathBuf::from(format!("{}.mts", clean_base)),
            PathBuf::from(format!("{}.cts", clean_base)),
            PathBuf::from(format!("{}.js", clean_base)),
            PathBuf::from(format!("{}.jsx", clean_base)),
            base_abs.join("index.ts"),
            base_abs.join("index.tsx"),
            base_abs.join("index.d.ts"),
            base_abs.join("index.js"),
        ];

        candidates.into_iter().find(|cand| cand.is_file())
    }
}
