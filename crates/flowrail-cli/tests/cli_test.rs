use std::path::PathBuf;
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
    assert!(stdout.contains("search"));
    assert!(stdout.contains("grep"));
    assert!(stdout.contains("slice"));
    assert!(stdout.contains("mcp"));
    assert!(stdout.contains("doctor"));
}

#[test]
fn test_cli_doctor() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let output = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("--workspace")
        .arg(root.to_str().unwrap())
        .arg("doctor")
        .output()
        .expect("flowrail doctor should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Flowrail System Doctor"));
    assert!(stdout.contains("Semantic Nodes"));
    assert!(stdout.contains("Fuzzy Search & Frecency Engine: Ready"));
}

#[test]
fn test_cli_search_and_grep() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/sample-ts-ecommerce");

    // 1. Search
    let search_out = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("--workspace")
        .arg(fixture_dir.to_str().unwrap())
        .arg("search")
        .arg("checkout")
        .output()
        .expect("flowrail search should run");

    assert!(search_out.status.success());
    let search_stdout = String::from_utf8_lossy(&search_out.stdout);
    assert!(search_stdout.contains("CheckoutController") || search_stdout.contains("checkout"));

    // 2. Grep
    let grep_out = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("--workspace")
        .arg(fixture_dir.to_str().unwrap())
        .arg("grep")
        .arg("Invoice")
        .output()
        .expect("flowrail grep should run");

    assert!(grep_out.status.success());
    let grep_stdout = String::from_utf8_lossy(&grep_out.stdout);
    assert!(grep_stdout.contains("Invoice") || grep_stdout.contains("matches"));
}

#[test]
fn test_cli_demo() {
    let output = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("demo")
        .output()
        .expect("flowrail demo should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Flowrail Deterministic State-Machine Demo"));
    assert!(stdout.contains("Building Semantic Graph"));
    assert!(stdout.contains("Discovering Entrypoint Gates"));
    assert!(stdout.contains("Slicing Top-Down Read Corridor"));
    assert!(stdout.contains("Slicing Bottom-Up Mutation Plan"));
    assert!(stdout.contains("Flowrail Demo Completed Successfully"));
}

#[test]
fn test_cli_init() {
    let temp_dir = std::env::temp_dir().join(format!("flowrail_init_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("--workspace")
        .arg(temp_dir.to_str().unwrap())
        .arg("init")
        .arg("--target")
        .arg("all")
        .arg("--force")
        .output()
        .expect("flowrail init should run");

    assert!(output.status.success());
    assert!(temp_dir.join(".flowrail/mcp.json").exists());
    assert!(temp_dir.join(".cursor/mcp.json").exists());
    assert!(temp_dir.join(".claude/mcp.json").exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
