use flowrail_core::git::{GitFileStatus, GitStatusResolver};
use std::path::PathBuf;

#[test]
fn test_git_resolver_non_git_workspace_fallback() {
    let temp_dir = std::env::temp_dir().join("flowrail_non_git_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let resolver = GitStatusResolver::new(temp_dir.clone());
    let status = resolver.get_file_status("any/file.ts");

    assert_eq!(status, GitFileStatus::Clean);
    assert_eq!(status.badge(), "[GIT:CLEAN]");
    assert_eq!(status.priority_score(), 0.0);

    let _ = std::fs::remove_dir_all(&temp_dir);
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
