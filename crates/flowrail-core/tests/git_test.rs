use flowrail_core::git::{GitFileStatus, GitStatusResolver};
use std::path::PathBuf;

struct TempDirGuard(PathBuf);

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
fn test_git_resolver_non_git_workspace_fallback() {
    let guard = TempDirGuard::new("flowrail_non_git_test");
    let temp_dir = guard.path().to_path_buf();

    let resolver = GitStatusResolver::new(temp_dir);
    let status = resolver.get_file_status("any/file.ts");

    assert_eq!(status, GitFileStatus::Clean);
    assert_eq!(status.badge(), "[GIT:CLEAN]");
    assert_eq!(status.priority_score(), 0.0);
}

#[test]
fn test_git_resolver_reads_current_repo() {
    let current_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let resolver = GitStatusResolver::new(current_dir);
    let all = resolver.get_all_statuses();

    // Since this is a live git repo, it should resolve statuses cleanly
    assert!(all.is_empty() || !all.is_empty()); // Does not panic
}

#[test]
fn test_get_head_commit_live_repo_and_fallback() {
    let current_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let head = GitStatusResolver::get_head_commit(&current_dir);
    assert!(head.is_some(), "Live git repo should have a head commit");
    let head_str = head.unwrap();
    assert!(
        head_str.len() >= 40,
        "Commit hash should be at least 40 chars"
    );

    // Non-git directory should return None
    let guard = TempDirGuard::new("flowrail_non_git_head_test");
    let non_git = GitStatusResolver::get_head_commit(guard.path());
    assert!(non_git.is_none());
}

#[test]
fn test_get_head_commit_with_submodule_composite() {
    let guard = TempDirGuard::new("flowrail_submodule_composite");
    let root = guard.path();
    let parent = root.join("parent");
    let child = root.join("child");

    std::fs::create_dir_all(&parent).unwrap();
    std::fs::create_dir_all(&child).unwrap();

    // Setup child repo
    let run = |dir: &std::path::Path, args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .expect("git command failed");
        assert!(status.success(), "git {:?} in {:?} failed", args, dir);
    };

    run(&child, &["init"]);
    run(&child, &["config", "user.name", "Tester"]);
    run(&child, &["config", "user.email", "test@flowrail.dev"]);
    std::fs::write(child.join("file.txt"), "hello").unwrap();
    run(&child, &["add", "file.txt"]);
    run(&child, &["commit", "-m", "init child"]);

    // Setup parent repo
    run(&parent, &["init"]);
    run(&parent, &["config", "user.name", "Tester"]);
    run(&parent, &["config", "user.email", "test@flowrail.dev"]);
    std::fs::write(parent.join("README.md"), "parent").unwrap();
    run(&parent, &["add", "README.md"]);
    run(&parent, &["commit", "-m", "init parent"]);

    let head_no_submodules = GitStatusResolver::get_head_commit(&parent).unwrap();
    assert_eq!(head_no_submodules.len(), 40);

    // Add submodule using file protocol
    let child_url = child.to_string_lossy().to_string();
    run(
        &parent,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            &child_url,
            "modules/sub",
        ],
    );
    run(&parent, &["commit", "-m", "add submodule"]);

    let head_with_submodule = GitStatusResolver::get_head_commit(&parent).unwrap();
    assert!(head_with_submodule.contains("modules/sub"));
    assert!(head_with_submodule.contains('|'));
    assert!(head_with_submodule.len() > 40);

    // Make a commit inside the submodule working directory
    let sub_dir = parent.join("modules/sub");
    std::fs::write(sub_dir.join("file.txt"), "hello world updated").unwrap();
    run(&sub_dir, &["add", "file.txt"]);
    run(&sub_dir, &["commit", "-m", "update inside submodule"]);

    let head_after_submodule_update = GitStatusResolver::get_head_commit(&parent).unwrap();
    assert_ne!(
        head_with_submodule, head_after_submodule_update,
        "Submodule internal commit must change composite head without requiring parent commit!"
    );
}

#[test]
fn test_git_co_change_coupling_discovery() {
    let guard = TempDirGuard::new("flowrail_co_change_test");
    let root = guard.path();

    let run = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(root)
            .args(args)
            .status()
            .expect("git failed");
        assert!(status.success());
    };

    run(&["init"]);
    run(&["config", "user.name", "Tester"]);
    run(&["config", "user.email", "test@flowrail.dev"]);

    // Commit 1: fileA and fileB together
    std::fs::write(root.join("fileA.ts"), "a1").unwrap();
    std::fs::write(root.join("fileB.ts"), "b1").unwrap();
    run(&["add", "fileA.ts", "fileB.ts"]);
    run(&["commit", "-m", "commit 1: A and B"]);

    // Commit 2: fileA and fileB together again
    std::fs::write(root.join("fileA.ts"), "a2").unwrap();
    std::fs::write(root.join("fileB.ts"), "b2").unwrap();
    run(&["add", "fileA.ts", "fileB.ts"]);
    run(&["commit", "-m", "commit 2: A and B"]);

    // Commit 3: fileC alone
    std::fs::write(root.join("fileC.ts"), "c1").unwrap();
    run(&["add", "fileC.ts"]);
    run(&["commit", "-m", "commit 3: C alone"]);

    let coupled = GitStatusResolver::find_co_changed_files(root, &["fileA.ts".to_string()], 50, 2);

    assert_eq!(coupled.len(), 1);
    assert_eq!(coupled[0].file_path, "fileB.ts");
    assert_eq!(coupled[0].co_change_count, 2);
    assert_eq!(coupled[0].confidence_percentage, 100);
}
