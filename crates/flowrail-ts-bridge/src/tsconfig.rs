use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspacePackage {
    pub name: String,
    pub rel_dir: PathBuf,
    pub main: Option<String>,
    pub types: Option<String>,
    pub exports: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct TsConfigResolver {
    pub base_url: Option<String>,
    pub paths: HashMap<String, Vec<String>>,
    pub packages: HashMap<String, WorkspacePackage>,
}

impl TsConfigResolver {
    /// Loads tsconfig.json and discovers all monorepo workspace packages from workspace_root,
    /// safely stripping comments (jsonc support) and indexing package.json manifests.
    pub fn load_from_workspace(workspace_root: &Path) -> Option<Self> {
        let mut resolver = Self::default();
        let mut has_config = false;

        let tsconfig_path = workspace_root.join("tsconfig.json");
        if tsconfig_path.exists()
            && let Ok(content) = fs::read_to_string(&tsconfig_path)
        {
            let clean_json = Self::strip_jsonc_comments(&content);
            if let Ok(parsed) = serde_json::from_str::<Value>(&clean_json) {
                has_config = true;
                if let Some(compiler_options) = parsed.get("compilerOptions") {
                    resolver.base_url = compiler_options
                        .get("baseUrl")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    if let Some(paths_obj) =
                        compiler_options.get("paths").and_then(|v| v.as_object())
                    {
                        for (pattern, targets) in paths_obj {
                            if let Some(targets_arr) = targets.as_array() {
                                let list: Vec<String> = targets_arr
                                    .iter()
                                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                                    .collect();
                                resolver.paths.insert(pattern.clone(), list);
                            }
                        }
                    }
                }
            }
        }

        // Discover monorepo packages (pnpm, npm/yarn workspaces, Lerna, Turborepo)
        resolver.discover_workspace_packages(workspace_root);
        if !resolver.packages.is_empty() {
            has_config = true;
        }

        if has_config { Some(resolver) } else { None }
    }

    /// Discovers all workspace packages defined across a monorepo by scanning for package.json
    /// files (e.g. in packages/*, apps/*, libs/*) without traversing node_modules.
    pub fn discover_workspace_packages(&mut self, workspace_root: &Path) {
        let mut queue = vec![(workspace_root.to_path_buf(), 0usize)];
        let ignored_dirs = [
            "node_modules",
            ".git",
            "dist",
            "build",
            "target",
            ".turbo",
            ".next",
            ".corridor",
        ];

        while let Some((curr_dir, depth)) = queue.pop() {
            if depth > 4 {
                continue;
            }

            let entries = match fs::read_dir(&curr_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let file_type = match entry.file_type() {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };
                let path = entry.path();
                let file_name = entry.file_name();
                let name_str = file_name.to_string_lossy();

                if file_type.is_dir() {
                    if !ignored_dirs.iter().any(|d| *d == name_str) {
                        queue.push((path, depth + 1));
                    }
                } else if file_type.is_file()
                    && name_str == "package.json"
                    && let Ok(content) = fs::read_to_string(&path)
                    && let Ok(parsed) = serde_json::from_str::<Value>(&content)
                    && let Some(pkg_name) = parsed.get("name").and_then(|v| v.as_str())
                    && !pkg_name.trim().is_empty()
                {
                    let rel_dir = match curr_dir.strip_prefix(workspace_root) {
                        Ok(r) => Self::normalize_path(r),
                        Err(_) => PathBuf::new(),
                    };

                    let main = parsed
                        .get("main")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let types = parsed
                        .get("types")
                        .or_else(|| parsed.get("typings"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let mut exports_map = HashMap::new();
                    if let Some(exports_val) = parsed.get("exports") {
                        Self::extract_exports(exports_val, &mut exports_map);
                    }

                    self.packages.insert(
                        pkg_name.to_string(),
                        WorkspacePackage {
                            name: pkg_name.to_string(),
                            rel_dir,
                            main,
                            types,
                            exports: exports_map,
                        },
                    );
                }
            }
        }
    }

    fn extract_exports(exports_val: &Value, map: &mut HashMap<String, String>) {
        if let Some(s) = exports_val.as_str() {
            map.insert(".".to_string(), s.to_string());
        } else if let Some(obj) = exports_val.as_object() {
            for (key, val) in obj {
                if let Some(target_str) = val.as_str() {
                    map.insert(key.clone(), target_str.to_string());
                } else if let Some(cond_obj) = val.as_object() {
                    let target = cond_obj
                        .get("types")
                        .or_else(|| cond_obj.get("import"))
                        .or_else(|| cond_obj.get("default"))
                        .or_else(|| cond_obj.get("require"))
                        .and_then(|v| v.as_str());
                    if let Some(t) = target {
                        map.insert(key.clone(), t.to_string());
                    }
                }
            }
        }
    }

    /// Normalizes import specifiers by stripping `npm:` or `workspace:` protocols
    /// and package version tags (e.g. `npm:@ecommerce/core@^1.0.0/tax` -> `@ecommerce/core/tax`).
    pub fn clean_specifier(specifier: &str) -> String {
        let trimmed = specifier.trim();
        let without_protocol = if let Some(s) = trimmed.strip_prefix("npm:") {
            s
        } else if let Some(s) = trimmed.strip_prefix("workspace:") {
            s
        } else {
            trimmed
        };

        if without_protocol.starts_with('@') {
            if let Some(slash_idx) = without_protocol.find('/') {
                let scope = &without_protocol[..slash_idx];
                let rest = &without_protocol[slash_idx + 1..];
                if let Some(at_idx) = rest.find('@') {
                    let name = &rest[..at_idx];
                    let after_ver = &rest[at_idx + 1..];
                    let subpath = if let Some(next_slash) = after_ver.find('/') {
                        &after_ver[next_slash..]
                    } else {
                        ""
                    };
                    return format!("{}/{}{}", scope, name, subpath);
                }
            }
        } else if let Some(at_idx) = without_protocol.find('@') {
            let name = &without_protocol[..at_idx];
            let after_ver = &without_protocol[at_idx + 1..];
            let subpath = if let Some(next_slash) = after_ver.find('/') {
                &after_ver[next_slash..]
            } else {
                ""
            };
            return format!("{}{}", name, subpath);
        }

        without_protocol.to_string()
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

    /// Resolves an import specifier against discovered monorepo workspace packages.
    pub fn resolve_workspace_package(
        &self,
        workspace_root: &Path,
        specifier: &str,
    ) -> Option<PathBuf> {
        let clean_spec = Self::clean_specifier(specifier);

        // 1. Exact match on package name (root package entry)
        if let Some(pkg) = self.packages.get(&clean_spec) {
            let pkg_abs = workspace_root.join(&pkg.rel_dir);

            // Priority a: exports["."]
            if let Some(exp_target) = pkg.exports.get(".") {
                let cand = pkg_abs.join(exp_target);
                if let Some(resolved) = Self::probe_extensions(&cand)
                    && let Ok(rel) = resolved.strip_prefix(workspace_root)
                {
                    return Some(Self::normalize_path(rel));
                }
            }

            // Priority b: types field
            if let Some(ref types) = pkg.types {
                let cand = pkg_abs.join(types);
                if let Some(resolved) = Self::probe_extensions(&cand)
                    && let Ok(rel) = resolved.strip_prefix(workspace_root)
                {
                    return Some(Self::normalize_path(rel));
                }
            }

            // Priority c: main field
            if let Some(ref main) = pkg.main {
                let cand = pkg_abs.join(main);
                if let Some(resolved) = Self::probe_extensions(&cand)
                    && let Ok(rel) = resolved.strip_prefix(workspace_root)
                {
                    return Some(Self::normalize_path(rel));
                }
            }

            // Priority d: Standard src/index.ts or index.ts conventions
            let standard_entrypoints = [
                pkg_abs.join("src/index.ts"),
                pkg_abs.join("src/index.tsx"),
                pkg_abs.join("index.ts"),
                pkg_abs.join("index.tsx"),
            ];
            for cand in standard_entrypoints {
                if let Some(resolved) = Self::probe_extensions(&cand)
                    && let Ok(rel) = resolved.strip_prefix(workspace_root)
                {
                    return Some(Self::normalize_path(rel));
                }
            }
        }

        // 2. Subpath package export match (e.g. @ecommerce/core/tax)
        for (pkg_name, pkg) in &self.packages {
            if clean_spec.starts_with(pkg_name) {
                let remainder = &clean_spec[pkg_name.len()..];
                if let Some(subpath) = remainder.strip_prefix('/') {
                    let pkg_abs = workspace_root.join(&pkg.rel_dir);

                    // a) Exact subpath in exports (e.g. "./tax")
                    let subpath_key = format!("./{}", subpath);
                    if let Some(exp_target) = pkg.exports.get(&subpath_key) {
                        let cand = pkg_abs.join(exp_target);
                        if let Some(resolved) = Self::probe_extensions(&cand)
                            && let Ok(rel) = resolved.strip_prefix(workspace_root)
                        {
                            return Some(Self::normalize_path(rel));
                        }
                    }

                    // b) Wildcard in exports (e.g. "./*" -> "./src/*.ts")
                    if let Some(wildcard_target) = pkg.exports.get("./*") {
                        let substituted = wildcard_target.replace('*', subpath);
                        let cand = pkg_abs.join(&substituted);
                        if let Some(resolved) = Self::probe_extensions(&cand)
                            && let Ok(rel) = resolved.strip_prefix(workspace_root)
                        {
                            return Some(Self::normalize_path(rel));
                        }
                    }

                    // c) Direct relative probing in package src or root
                    let cand_src = pkg_abs.join("src").join(subpath);
                    if let Some(resolved) = Self::probe_extensions(&cand_src)
                        && let Ok(rel) = resolved.strip_prefix(workspace_root)
                    {
                        return Some(Self::normalize_path(rel));
                    }

                    let cand_root = pkg_abs.join(subpath);
                    if let Some(resolved) = Self::probe_extensions(&cand_root)
                        && let Ok(rel) = resolved.strip_prefix(workspace_root)
                    {
                        return Some(Self::normalize_path(rel));
                    }
                }
            }
        }

        None
    }

    /// Resolves an import specifier (e.g. `./checkout.service`, `@/modules/order`, `@ecommerce/core`, or `npm:@ecommerce/tax@^1.0.0`)
    /// relative to the importing file, tsconfig path aliases, and monorepo workspace packages.
    /// Returns the resolved normalized relative path from `workspace_root` if found on disk.
    pub fn resolve_module(
        &self,
        workspace_root: &Path,
        importing_file: &Path,
        specifier: &str,
    ) -> Option<PathBuf> {
        let clean = Self::clean_specifier(specifier);
        if clean.is_empty() {
            return None;
        }

        // Case 1: Relative imports (starts with '.' or '/')
        if clean.starts_with('.') || clean.starts_with('/') {
            let parent_dir = importing_file.parent().unwrap_or(Path::new(""));
            let candidate_abs = workspace_root.join(parent_dir).join(&clean);
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
                if clean.starts_with(prefix) && clean.ends_with(suffix) {
                    let wildcard = &clean[prefix.len()..clean.len() - suffix.len()];
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
            } else if pattern == &clean {
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

        // Case 3: Monorepo workspace packages (pnpm, npm/yarn workspaces, Lerna, Turborepo, npm: aliases)
        if let Some(resolved) = self.resolve_workspace_package(workspace_root, &clean) {
            return Some(resolved);
        }

        // Case 4: baseUrl fallback
        if self.base_url.is_some() {
            let candidate_abs = base_dir.join(&clean);
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
