use crate::frecency::FrecencyStore;
use crate::git::{GitFileStatus, GitStatusResolver};
use crate::graph::{CodebaseGraph, NodeRole, SemanticNode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryConstraints {
    pub raw_query: String,
    pub terms: Vec<String>,
    pub positive_globs: Vec<String>,
    pub negative_globs: Vec<String>,
    pub path_scope: Option<String>,
}

impl QueryConstraints {
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        let mut terms = Vec::new();
        let mut positive_globs = Vec::new();
        let mut negative_globs = Vec::new();
        let mut path_scope = None;

        for part in trimmed.split_whitespace() {
            if let Some(stripped) = part.strip_prefix('!') {
                negative_globs.push(stripped.trim_matches('/').to_lowercase());
            } else if part.contains('*') {
                positive_globs.push(part.to_lowercase());
            } else if part.ends_with('/') || part.contains('/') && !part.contains('.') {
                path_scope = Some(part.trim_matches('/').to_lowercase());
            } else {
                terms.push(part.to_string());
            }
        }

        Self {
            raw_query: trimmed.to_string(),
            terms,
            positive_globs,
            negative_globs,
            path_scope,
        }
    }

    pub fn matches_path(&self, file_path: &str) -> bool {
        let lower = file_path.to_lowercase();

        // Check negative globs
        for neg in &self.negative_globs {
            let clean_neg = neg.replace("**", "");
            if lower.contains(&clean_neg) {
                return false;
            }
        }

        // Check path scope
        if let Some(scope) = &self.path_scope
            && !lower.contains(scope)
        {
            return false;
        }

        // Check positive globs (e.g. *.ts)
        if !self.positive_globs.is_empty() {
            let matched_any = self.positive_globs.iter().any(|glob| {
                if let Some(ext) = glob.strip_prefix("*.") {
                    lower.ends_with(&format!(".{}", ext))
                } else {
                    lower.contains(&glob.replace('*', ""))
                }
            });
            if !matched_any {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub symbol_id: String,
    pub display_name: String,
    pub file_path: String,
    pub line_range: (usize, usize),
    pub role: NodeRole,
    pub score: f64,
    pub git_status: GitFileStatus,
    pub frecency_score: f64,
    pub stage: u8,
}

pub struct FuzzySearchEngine<'a> {
    graph: &'a CodebaseGraph,
    git_resolver: Option<&'a GitStatusResolver>,
    frecency_store: Option<&'a FrecencyStore>,
}

impl<'a> FuzzySearchEngine<'a> {
    pub fn new(
        graph: &'a CodebaseGraph,
        git_resolver: Option<&'a GitStatusResolver>,
        frecency_store: Option<&'a FrecencyStore>,
    ) -> Self {
        Self {
            graph,
            git_resolver,
            frecency_store,
        }
    }

    pub fn search(
        &self,
        query: &str,
        definitions_only: bool,
        git_only: bool,
        max_results: usize,
    ) -> Vec<SearchHit> {
        let constraints = QueryConstraints::parse(query);
        let mut candidates = Vec::new();

        for idx in self.graph.graph.node_indices() {
            let node = &self.graph.graph[idx];

            if definitions_only
                && !matches!(
                    node.role,
                    NodeRole::ContractDefinition | NodeRole::EntryGate
                )
            {
                continue;
            }

            if !constraints.matches_path(&node.file_path) {
                continue;
            }

            let git_status = self
                .git_resolver
                .map(|r| r.get_file_status(&node.file_path))
                .unwrap_or(GitFileStatus::Clean);

            if git_only && git_status == GitFileStatus::Clean {
                continue;
            }

            let frecency_score = self
                .frecency_store
                .map(|f| f.get_score(&node.symbol_id))
                .unwrap_or(0.0);

            // Compute score across terms
            let (match_score, stage) = self.score_node(node, &constraints);

            if match_score > 0.0 || constraints.terms.is_empty() {
                let role_bonus = match node.role {
                    NodeRole::EntryGate => 100.0,
                    NodeRole::ContractDefinition => 80.0,
                    NodeRole::BusinessLogic => 40.0,
                    _ => 0.0,
                };

                let total_score =
                    match_score + git_status.priority_score() + frecency_score + role_bonus;

                candidates.push(SearchHit {
                    symbol_id: node.symbol_id.clone(),
                    display_name: node.display_name.clone(),
                    file_path: node.file_path.clone(),
                    line_range: node.line_range,
                    role: node.role,
                    score: total_score,
                    git_status,
                    frecency_score,
                    stage,
                });
            }
        }

        // Sort descending by score
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(max_results);
        candidates
    }

    fn score_node(&self, node: &SemanticNode, constraints: &QueryConstraints) -> (f64, u8) {
        if constraints.terms.is_empty() {
            return (100.0, 1);
        }

        let symbol_lower = node.symbol_id.to_lowercase();
        let display_lower = node.display_name.to_lowercase();
        let path_lower = node.file_path.to_lowercase();

        let mut total_term_score = 0.0;
        let mut min_stage = 4;

        for term in &constraints.terms {
            let t_lower = term.to_lowercase();

            // Stage 1: Exact match
            if display_lower == t_lower || symbol_lower == t_lower {
                total_term_score += 1000.0 + (term.len() as f64 * 10.0);
                min_stage = min_stage.min(1);
                continue;
            }

            // Stage 1.5: Prefix / camelCase acronym match
            if display_lower.starts_with(&t_lower) {
                total_term_score += 800.0 + (term.len() as f64 * 5.0);
                min_stage = min_stage.min(1);
                continue;
            }

            if Self::is_camel_case_acronym(&node.display_name, term) {
                total_term_score += 650.0 + (term.len() as f64 * 10.0);
                min_stage = min_stage.min(2);
                continue;
            }

            // Stage 2: Substring match
            if display_lower.contains(&t_lower) {
                total_term_score += 450.0 + (term.len() as f64 * 5.0);
                min_stage = min_stage.min(2);
                continue;
            }

            if symbol_lower.contains(&t_lower) || path_lower.contains(&t_lower) {
                total_term_score += 300.0 + (term.len() as f64 * 3.0);
                min_stage = min_stage.min(2);
                continue;
            }

            // Stage 3: Typo-tolerant distance match (Levenshtein against words or full string)
            let words = Self::split_words(&node.display_name);
            let mut best_word_dist = usize::MAX;
            for w in &words {
                let d = Self::levenshtein_distance(&t_lower, &w.to_lowercase());
                if d < best_word_dist {
                    best_word_dist = d;
                }
            }

            let full_dist = Self::levenshtein_distance(&t_lower, &display_lower);
            let dist = best_word_dist.min(full_dist);
            if t_lower.len() >= 4 && dist <= 2 {
                total_term_score += 250.0 - (dist as f64 * 60.0);
                min_stage = min_stage.min(3);
                continue;
            } else if t_lower.len() >= 3 && dist <= 1 {
                total_term_score += 200.0 - (dist as f64 * 60.0);
                min_stage = min_stage.min(3);
                continue;
            }

            // If term not matched at all
            return (0.0, 4);
        }

        (total_term_score, min_stage)
    }

    fn is_camel_case_acronym(target: &str, acronym: &str) -> bool {
        let uppers: String = target.chars().filter(|c| c.is_uppercase()).collect();
        if !uppers.is_empty() && uppers.to_lowercase() == acronym.to_lowercase() {
            return true;
        }

        // Check word starts after underscores or hyphens
        let mut word_starts = String::new();
        let mut prev_delim = true;
        for c in target.chars() {
            if c == '_' || c == '-' {
                prev_delim = true;
            } else if prev_delim || c.is_uppercase() {
                word_starts.push(c);
                prev_delim = false;
            }
        }

        word_starts.to_lowercase().contains(&acronym.to_lowercase())
    }

    pub fn split_words(s: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut current = String::new();
        for c in s.chars() {
            if c == '_' || c == '-' || c == '/' || c == '.' || c == '#' {
                if !current.is_empty() {
                    words.push(current);
                    current = String::new();
                }
            } else if c.is_uppercase() {
                if !current.is_empty() && !current.chars().all(|ch| ch.is_uppercase()) {
                    words.push(current);
                    current = String::new();
                }
                current.push(c);
            } else {
                current.push(c);
            }
        }
        if !current.is_empty() {
            words.push(current);
        }
        words
    }

    pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let v1: Vec<char> = s1.chars().collect();
        let v2: Vec<char> = s2.chars().collect();
        let l1 = v1.len();
        let l2 = v2.len();

        if l1 == 0 {
            return l2;
        }
        if l2 == 0 {
            return l1;
        }

        let mut prev_row: Vec<usize> = (0..=l2).collect();
        let mut curr_row: Vec<usize> = vec![0; l2 + 1];

        for (i, &ch1) in v1.iter().enumerate() {
            curr_row[0] = i + 1;
            for j in 0..l2 {
                let cost = if ch1 == v2[j] { 0 } else { 1 };
                curr_row[j + 1] = std::cmp::min(
                    curr_row[j] + 1,
                    std::cmp::min(prev_row[j + 1] + 1, prev_row[j] + cost),
                );
            }
            prev_row.copy_from_slice(&curr_row);
        }

        curr_row[l2]
    }
}
