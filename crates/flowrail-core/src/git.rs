use git2::{Repository, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GitFileStatus {
    Clean,
    Modified,
    Staged,
    Untracked,
}

impl GitFileStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            GitFileStatus::Clean => "[GIT:CLEAN]",
            GitFileStatus::Modified => "[GIT:MODIFIED]",
            GitFileStatus::Staged => "[GIT:STAGED]",
            GitFileStatus::Untracked => "[GIT:UNTRACKED]",
        }
    }

    pub fn priority_score(&self) -> f64 {
        match self {
            GitFileStatus::Staged => 200.0,
            GitFileStatus::Modified => 150.0,
            GitFileStatus::Untracked => 100.0,
            GitFileStatus::Clean => 0.0,
        }
    }
}

pub struct GitStatusResolver {
    workspace_root: PathBuf,
    cache: Arc<RwLock<HashMap<String, GitFileStatus>>>,
    last_refresh: Arc<RwLock<Instant>>,
    ttl: Duration,
    version: AtomicU64,
}

impl GitStatusResolver {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self::with_ttl(workspace_root, Duration::from_millis(1500))
    }

    pub fn with_ttl(workspace_root: PathBuf, ttl: Duration) -> Self {
        Self {
            workspace_root,
            cache: Arc::new(RwLock::new(HashMap::new())),
            last_refresh: Arc::new(RwLock::new(Instant::now() - ttl * 2)),
            ttl,
            version: AtomicU64::new(0),
        }
    }

    pub fn get_file_status(&self, relative_path: &str) -> GitFileStatus {
        self.ensure_fresh();
        let guard = self.cache.read().unwrap();
        guard
            .get(relative_path)
            .copied()
            .unwrap_or(GitFileStatus::Clean)
    }

    pub fn get_all_statuses(&self) -> HashMap<String, GitFileStatus> {
        self.ensure_fresh();
        self.cache.read().unwrap().clone()
    }

    pub fn invalidate(&self) {
        self.version.fetch_add(1, Ordering::SeqCst);
        let mut last = self.last_refresh.write().unwrap();
        *last = Instant::now() - self.ttl * 2;
    }

    fn ensure_fresh(&self) {
        let is_stale = {
            let last = self.last_refresh.read().unwrap();
            last.elapsed() >= self.ttl
        };

        if is_stale {
            self.refresh();
        }
    }

    pub fn refresh(&self) {
        let statuses = Self::scan_repo(&self.workspace_root);
        let mut guard = self.cache.write().unwrap();
        *guard = statuses;
        let mut last = self.last_refresh.write().unwrap();
        *last = Instant::now();
        self.version.fetch_add(1, Ordering::SeqCst);
    }

    fn scan_repo(workspace_root: &Path) -> HashMap<String, GitFileStatus> {
        let mut map = HashMap::new();

        let repo = match Repository::discover(workspace_root) {
            Ok(r) => r,
            Err(_) => return map,
        };

        let workdir = match repo.workdir() {
            Some(w) => w,
            None => return map,
        };

        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_unmodified(false)
            .exclude_submodules(true);

        let statuses = match repo.statuses(Some(&mut opts)) {
            Ok(s) => s,
            Err(_) => return map,
        };

        for entry in statuses.iter() {
            if let Some(path_str) = entry.path() {
                let status_flags = entry.status();

                let status = if status_flags.contains(Status::INDEX_NEW)
                    || status_flags.contains(Status::INDEX_MODIFIED)
                    || status_flags.contains(Status::INDEX_DELETED)
                    || status_flags.contains(Status::INDEX_RENAMED)
                {
                    GitFileStatus::Staged
                } else if status_flags.contains(Status::WT_MODIFIED)
                    || status_flags.contains(Status::WT_DELETED)
                    || status_flags.contains(Status::WT_RENAMED)
                {
                    GitFileStatus::Modified
                } else if status_flags.contains(Status::WT_NEW) {
                    GitFileStatus::Untracked
                } else {
                    GitFileStatus::Clean
                };

                let full_path = workdir.join(path_str);
                let rel_to_workspace = match full_path.strip_prefix(workspace_root) {
                    Ok(rel) => rel.to_string_lossy().to_string(),
                    Err(_) => path_str.to_string(),
                };

                map.insert(rel_to_workspace, status);
            }
        }

        map
    }
}
