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
    assert!(search_stdout.contains("CheckoutController"));

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
    assert!(grep_stdout.contains("InvoiceGenerator") && grep_stdout.contains("matches in"));
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

struct TempDirGuard(std::path::PathBuf);

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_cli_init() {
    let guard = TempDirGuard::new("flowrail_init_test");
    let temp_dir = guard.path();

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
    assert!(temp_dir.join(".mcp.json").exists());
}

#[test]
fn test_cli_artifact_lifecycle() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/sample-ts-ecommerce");

    let guard = TempDirGuard::new("flowrail_art_cli");
    let temp_workspace = guard.path().to_path_buf();

    // Copy fixture files into temp workspace
    let copy_status = Command::new("cp")
        .arg("-R")
        .arg(format!("{}/", fixture_dir.to_str().unwrap()))
        .arg(temp_workspace.to_str().unwrap())
        .status()
        .expect("Copy fixture files");
    assert!(copy_status.success());

    let entry = "src/modules/checkout/checkout.controller.ts#CheckoutController";

    // 1. Slice with --artifact
    let slice_out = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("--workspace")
        .arg(temp_workspace.to_str().unwrap())
        .arg("slice")
        .arg("--entry")
        .arg(entry)
        .arg("--artifact")
        .output()
        .expect("flowrail slice --artifact should run");

    assert!(slice_out.status.success());
    let slice_stdout = String::from_utf8_lossy(&slice_out.stdout);
    assert!(slice_stdout.contains("\"schema_version\": \"flowrail.artifact.v1\""));
    assert!(slice_stdout.contains("\"headline_action\""));

    let latest_art_file = temp_workspace.join(".flowrail/artifacts/latest.json");
    assert!(latest_art_file.exists(), "latest.json must be generated");

    // 2. Artifact show
    let show_out = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("--workspace")
        .arg(temp_workspace.to_str().unwrap())
        .arg("artifact")
        .arg("show")
        .output()
        .expect("flowrail artifact show should run");

    assert!(show_out.status.success());
    let show_stdout = String::from_utf8_lossy(&show_out.stdout);
    assert!(show_stdout.contains("flowrail.artifact.v1"));

    // 3. Artifact list
    let list_out = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("--workspace")
        .arg(temp_workspace.to_str().unwrap())
        .arg("artifact")
        .arg("list")
        .output()
        .expect("flowrail artifact list should run");

    assert!(list_out.status.success());
    let list_stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(list_stdout.contains("Flowrail Generated Artifacts"));

    // 4. Artifact validate
    let val_out = Command::new(env!("CARGO_BIN_EXE_flowrail"))
        .arg("--workspace")
        .arg(temp_workspace.to_str().unwrap())
        .arg("artifact")
        .arg("validate")
        .arg("--file")
        .arg(latest_art_file.to_str().unwrap())
        .output()
        .expect("flowrail artifact validate should run");

    assert!(val_out.status.success());
    let val_stdout = String::from_utf8_lossy(&val_out.stdout);
    assert!(val_stdout.contains("Validation PASSED"));
    assert!(val_stdout.contains("Zero Hallucination"));
    assert!(val_stdout.contains("ADHD Constraints"));
    assert!(val_stdout.contains("Anti-Slop Discipline"));
}
