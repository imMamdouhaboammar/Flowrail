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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoChangeCoupling {
    pub file_path: String,
    pub co_change_count: usize,
    pub confidence_percentage: usize,
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

    pub fn get_head_commit(workspace_root: &Path) -> Option<String> {
        let repo = Repository::discover(workspace_root).ok()?;
        let head = repo.head().ok()?;
        let target = head.target()?;
        let mut composite_head = target.to_string();

        // Inspect git submodules deterministically to detect submodule HEAD updates
        if let Ok(mut submodules) = repo.submodules() {
            submodules.sort_by(|a, b| a.path().cmp(b.path()));
            for sm in submodules {
                let sm_path = sm.path().to_string_lossy();
                let sm_oid = sm
                    .workdir_id()
                    .or_else(|| sm.head_id())
                    .or_else(|| sm.index_id());
                if let Some(oid) = sm_oid {
                    composite_head.push('|');
                    composite_head.push_str(&sm_path);
                    composite_head.push(':');
                    composite_head.push_str(&oid.to_string());
                }
            }
        }

        Some(composite_head)
    }

    /// Discovers files that historically change together with `target_files` (CodeScene pattern).
    /// Inspects the last `max_commits` commits via git revwalk.
    pub fn find_co_changed_files(
        workspace_root: &Path,
        target_files: &[String],
        max_commits: usize,
        min_co_changes: usize,
    ) -> Vec<CoChangeCoupling> {
        let Ok(repo) = Repository::discover(workspace_root) else {
            return Vec::new();
        };

        let mut revwalk = match repo.revwalk() {
            Ok(rw) => rw,
            Err(_) => return Vec::new(),
        };

        let _ = revwalk.push_head();
        let _ = revwalk.set_sorting(git2::Sort::TIME);

        let target_set: std::collections::HashSet<&str> =
            target_files.iter().map(|s| s.as_str()).collect();
        if target_set.is_empty() {
            return Vec::new();
        }

        let mut target_commit_count = 0usize;
        let mut co_change_counts: HashMap<String, usize> = HashMap::new();

        for oid_res in revwalk.take(max_commits) {
            let Ok(oid) = oid_res else { continue };
            let Ok(commit) = repo.find_commit(oid) else {
                continue;
            };
            if commit.parent_count() > 1 {
                // Skip merge commits for clean delta attribution
                continue;
            }

            let tree = match commit.tree() {
                Ok(t) => t,
                Err(_) => continue,
            };

            let parent_tree = if commit.parent_count() == 1 {
                commit.parent(0).ok().and_then(|p| p.tree().ok())
            } else {
                None
            };

            let mut diff_opts = git2::DiffOptions::new();
            let diff = match repo.diff_tree_to_tree(
                parent_tree.as_ref(),
                Some(&tree),
                Some(&mut diff_opts),
            ) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let mut commit_files = std::collections::HashSet::new();
            for delta in diff.deltas() {
                if let Some(path) = delta.new_file().path() {
                    commit_files.insert(path.to_string_lossy().to_string());
                }
            }

            let has_target = commit_files.iter().any(|f| target_set.contains(f.as_str()));
            if has_target {
                target_commit_count += 1;
                for f in commit_files {
                    if !target_set.contains(f.as_str()) {
                        *co_change_counts.entry(f).or_insert(0) += 1;
                    }
                }
            }
        }

        if target_commit_count == 0 {
            return Vec::new();
        }

        let mut results: Vec<CoChangeCoupling> = co_change_counts
            .into_iter()
            .filter(|(_, count)| *count >= min_co_changes)
            .map(|(file_path, count)| {
                let conf = (count * 100) / target_commit_count;
                CoChangeCoupling {
                    file_path,
                    co_change_count: count,
                    confidence_percentage: conf,
                }
            })
            .collect();

        results.sort_by_key(|a| std::cmp::Reverse(a.co_change_count));
        results
    }
}
