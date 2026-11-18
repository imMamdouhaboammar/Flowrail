use flowrail_core::grep::CorridorGrep;
use std::fs;

#[test]
fn test_grep_definitions_only() {
    let temp_dir = std::env::temp_dir().join("flowrail_grep_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let sample_file = temp_dir.join("service.ts");
    let content = r#"
import { Logger } from "./logger";

export interface ServiceContract {
  run(): void;
}

// Helper comment
export function executeService() {
  const result = runCalculation();
  return result;
}

function helperInternal() {
  console.log("running");
}
"#;
    fs::write(&sample_file, content).unwrap();

    let report = CorridorGrep::search(
        &temp_dir,
        std::slice::from_ref(&sample_file),
        "Service",
        true, // definitions_only
        None,
        1,
        20,
        None,
    )
    .unwrap();

    assert_eq!(report.files.len(), 1);
    let matched_lines: Vec<_> = report.files[0]
        .lines
        .iter()
        .filter(|l| l.is_match)
        .collect();

    // Should match `export interface ServiceContract` and `export function executeService`
    assert_eq!(matched_lines.len(), 2);
    assert!(
        matched_lines[0]
            .content
            .contains("interface ServiceContract")
    );
    assert!(matched_lines[1].content.contains("function executeService"));

    let formatted = CorridorGrep::format_report_compact(&report);
    assert!(formatted.contains("[DEF]"));
    assert!(formatted.contains("service.ts"));

    let _ = fs::remove_dir_all(&temp_dir);
}
