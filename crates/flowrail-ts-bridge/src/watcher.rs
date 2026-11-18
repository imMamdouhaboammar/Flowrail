use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherEvent {
    Modified(PathBuf),
    Created(PathBuf),
    Deleted(PathBuf),
}

pub struct BackgroundWatcher {
    stop_signal: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl BackgroundWatcher {
    pub fn spawn(
        workspace_root: PathBuf,
        poll_interval: Duration,
    ) -> (Self, Receiver<WatcherEvent>) {
        let (tx, rx) = channel();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop_signal);
        let mut snapshot = Self::scan_timestamps(&workspace_root);

        let handle = thread::spawn(move || {
            while !stop_clone.load(Ordering::Relaxed) {
                thread::sleep(poll_interval);

                let current = Self::scan_timestamps(&workspace_root);

                // Detect created and modified
                for (path, mtime) in &current {
                    match snapshot.get(path) {
                        Some(old_mtime) => {
                            if mtime > old_mtime {
                                let _ = tx.send(WatcherEvent::Modified(path.clone()));
                            }
                        }
                        None => {
                            let _ = tx.send(WatcherEvent::Created(path.clone()));
                        }
                    }
                }

                // Detect deleted
                for path in snapshot.keys() {
                    if !current.contains_key(path) {
                        let _ = tx.send(WatcherEvent::Deleted(path.clone()));
                    }
                }

                snapshot = current;
            }
        });

        (
            Self {
                stop_signal,
                handle: Some(handle),
            },
            rx,
        )
    }

    pub fn scan_timestamps(dir: &Path) -> HashMap<PathBuf, SystemTime> {
        let mut map = HashMap::new();
        Self::collect_ts_timestamps(dir, &mut map);
        map
    }

    fn collect_ts_timestamps(dir: &Path, map: &mut HashMap<PathBuf, SystemTime>) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if !name.starts_with('.')
                    && name != "node_modules"
                    && name != "dist"
                    && name != "target"
                    && name != "do-not-care"
                {
                    Self::collect_ts_timestamps(&path, map);
                }
            } else if let Some(ext) = path.extension()
                && (ext == "ts" || ext == "tsx")
                && let Ok(meta) = fs::metadata(&path)
                && let Ok(modified) = meta.modified()
            {
                map.insert(path, modified);
            }
        }
    }

    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for BackgroundWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}
