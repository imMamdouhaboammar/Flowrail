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

        // Tier 2: Incremental tsc check with process timeout
        let child = match Command::new("npx")
            .arg("tsc")
            .arg("--noEmit")
            .arg("--pretty")
            .arg("false")
            .current_dir(workspace_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return Ok(diagnostics),
        };

        match run_command_with_timeout(child, std::time::Duration::from_secs(10)) {
            Ok(out) if !out.status.success() => {
                parse_tsc_diagnostics(&out, relative_file_path, &mut diagnostics);
            }
            Err(true) => {
                diagnostics.push(format!(
                    "[Type Check Timeout] tsc process exceeded 10s limit on {}",
                    relative_file_path
                ));
            }
            _ => {}
        }

        Ok(diagnostics)
    }
}

fn parse_tsc_diagnostics(
    out: &std::process::Output,
    relative_file_path: &str,
    diagnostics: &mut Vec<String>,
) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{}\n{}", stdout, stderr);
    for line in combined.lines() {
        if line.starts_with(relative_file_path) || line.contains(relative_file_path) {
            diagnostics.push(format!("[Type Error] {}", line));
        }
    }
}

fn run_command_with_timeout(
    mut child: std::process::Child,
    timeout: std::time::Duration,
) -> Result<std::process::Output, bool> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().map_err(|_| false),
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                return Err(true);
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(_) => return Err(false),
        }
    }
}
