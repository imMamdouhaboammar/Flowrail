use std::process::Command;

#[test]
fn test_cli_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("--help")
        .output()
        .expect("flowrail binary should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("flowrail") || stdout.contains("Deterministic"));
    assert!(stdout.contains("slice"));
    assert!(stdout.contains("mcp"));
}
