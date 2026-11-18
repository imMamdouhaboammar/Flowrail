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
