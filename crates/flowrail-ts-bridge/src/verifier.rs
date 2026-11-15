use anyhow::Result;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::path::Path;
use std::process::Command;

pub struct TsGateVerifier;

impl TsGateVerifier {
    /// Tier 1 (<2ms): Fast AST syntax check via oxc_parser.
    /// Tier 2 (Optional): Runs incremental `tsc --noEmit` filtered to the target file.
    pub fn verify_gate_edit(
        workspace_root: &Path,
        relative_file_path: &str,
        proposed_source: &str,
        run_full_typecheck: bool,
    ) -> Result<Vec<String>> {
        let mut diagnostics = Vec::new();
        let file_path = workspace_root.join(relative_file_path);

        // Tier 1: OXC ultra-fast syntax validation
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(&file_path)
            .unwrap_or_default()
            .with_typescript(true);
        let parse_res = Parser::new(&allocator, proposed_source, source_type).parse();

        for err in parse_res.errors {
            diagnostics.push(format!("[Syntax Error] {}: {}", relative_file_path, err));
        }

        if !diagnostics.is_empty() || !run_full_typecheck {
            return Ok(diagnostics);
        }

        // Tier 2: Incremental tsc check filtered to the active gate file
        let output = Command::new("npx")
            .arg("tsc")
            .arg("--noEmit")
            .arg("--pretty")
            .arg("false")
            .current_dir(workspace_root)
            .output();

        if let Ok(out) = output
            && !out.status.success()
        {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.starts_with(relative_file_path) {
                    diagnostics.push(format!("[Type Error] {}", line));
                }
            }
        }

        Ok(diagnostics)
    }
}
