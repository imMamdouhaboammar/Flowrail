use crate::git::{GitFileStatus, GitStatusResolver};
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrepScope {
    Workspace,
    Corridor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepHitLine {
    pub line_number: usize,
    pub content: String,
    pub is_definition: bool,
    pub is_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepFileResult {
    pub file_path: String,
    pub git_status: GitFileStatus,
    pub lines: Vec<GrepHitLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepReport {
    pub total_matches: usize,
    pub files: Vec<GrepFileResult>,
}

pub struct CorridorGrep;

impl CorridorGrep {
    #[allow(clippy::too_many_arguments)]
    pub fn search(
        workspace_root: &Path,
        files: &[PathBuf],
        query: &str,
        definitions_only: bool,
        case_sensitive: Option<bool>,
        context_lines: usize,
        max_matches: usize,
        git_resolver: Option<&GitStatusResolver>,
    ) -> Result<GrepReport> {
        let is_case_sens =
            case_sensitive.unwrap_or_else(|| query.chars().any(|c| c.is_uppercase()));
        let pattern_str = if is_case_sens {
            regex::escape(query)
        } else {
            format!("(?i){}", regex::escape(query))
        };
        let re = Regex::new(&pattern_str)?;

        let mut file_results = Vec::new();
        let mut total_matches = 0;

        for file_path in files {
            if total_matches >= max_matches {
                break;
            }

            let full_path = if file_path.is_absolute() {
                file_path.clone()
            } else {
                workspace_root.join(file_path)
            };

            let content = match fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let rel_path = match full_path.strip_prefix(workspace_root) {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => file_path.to_string_lossy().to_string(),
            };

            let git_status = git_resolver
                .map(|r| r.get_file_status(&rel_path))
                .unwrap_or(GitFileStatus::Clean);

            let lines: Vec<&str> = content.lines().collect();
            let mut matching_indices = Vec::new();

            for (idx, line) in lines.iter().enumerate() {
                if re.is_match(line) {
                    let is_def = Self::is_definition_line(line);
                    if definitions_only && !is_def {
                        continue;
                    }
                    matching_indices.push((idx, is_def));
                }
            }

            if matching_indices.is_empty() {
                continue;
            }

            let mut included_indices: HashSet<usize> = HashSet::new();
            for &(match_idx, _) in &matching_indices {
                let start = match_idx.saturating_sub(context_lines);
                let end = (match_idx + context_lines + 1).min(lines.len());
                for i in start..end {
                    included_indices.insert(i);
                }
            }

            let mut sorted_indices: Vec<usize> = included_indices.into_iter().collect();
            sorted_indices.sort_unstable();

            let match_set: HashSet<usize> = matching_indices.iter().map(|(idx, _)| *idx).collect();

            let mut hit_lines = Vec::new();
            for idx in sorted_indices {
                let is_m = match_set.contains(&idx);
                let is_d = Self::is_definition_line(lines[idx]);
                hit_lines.push(GrepHitLine {
                    line_number: idx + 1,
                    content: lines[idx].to_string(),
                    is_definition: is_d,
                    is_match: is_m,
                });
            }

            total_matches += matching_indices.len();
            file_results.push(GrepFileResult {
                file_path: rel_path,
                git_status,
                lines: hit_lines,
            });
        }

        Ok(GrepReport {
            total_matches,
            files: file_results,
        })
    }

    pub fn is_definition_line(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            return false;
        }

        trimmed.starts_with("export function ")
            || trimmed.starts_with("export async function ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("async function ")
            || trimmed.starts_with("export class ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("export interface ")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("export type ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("export const ")
            || trimmed.starts_with("export default ")
            || trimmed.starts_with("public ")
            || trimmed.starts_with("private ")
            || trimmed.starts_with("protected ")
            || (trimmed.contains('(')
                && (trimmed.ends_with('{') || trimmed.contains("): "))
                && !trimmed.starts_with("if")
                && !trimmed.starts_with("for")
                && !trimmed.starts_with("while")
                && !trimmed.starts_with("catch")
                && !trimmed.starts_with("return ")
                && !trimmed.starts_with("const ")
                && !trimmed.starts_with("let "))
            || (trimmed.contains("fn ") && trimmed.starts_with("pub "))
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
    }

    pub fn format_report_compact(report: &GrepReport) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Found {} matches in {} files:\n",
            report.total_matches,
            report.files.len()
        ));

        for f in &report.files {
            out.push_str(&format!(
                "\nFile: {} {}\n",
                f.file_path,
                f.git_status.badge()
            ));
            for l in &f.lines {
                let mark = if l.is_match { ">" } else { " " };
                let def_tag = if l.is_definition { "[DEF]" } else { "     " };
                out.push_str(&format!(
                    "{} {:4} {} | {}\n",
                    mark, l.line_number, def_tag, l.content
                ));
            }
        }
        out
    }
}
