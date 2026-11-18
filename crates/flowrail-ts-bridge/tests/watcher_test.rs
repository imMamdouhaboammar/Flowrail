use flowrail_ts_bridge::watcher::{BackgroundWatcher, WatcherEvent};
use std::fs;
use std::thread;
use std::time::Duration;

#[test]
fn test_watcher_detects_creation_and_modification() {
    let temp_dir = std::env::temp_dir().join("flowrail_watcher_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let (mut watcher, rx) = BackgroundWatcher::spawn(temp_dir.clone(), Duration::from_millis(50));

    // Create a new TS file
    let file_path = temp_dir.join("order.ts");
    fs::write(&file_path, "export interface Order { id: string; }").unwrap();

    // Expect Created event
    let event = rx.recv_timeout(Duration::from_millis(500)).unwrap();
    assert_eq!(event, WatcherEvent::Created(file_path.clone()));

    // Sleep briefly to ensure mtime advances
    thread::sleep(Duration::from_millis(100));

    // Modify the file
    fs::write(
        &file_path,
        "export interface Order { id: string; total: number; }",
    )
    .unwrap();

    let event2 = rx.recv_timeout(Duration::from_millis(500)).unwrap();
    assert_eq!(event2, WatcherEvent::Modified(file_path.clone()));

    watcher.stop();
    let _ = fs::remove_dir_all(&temp_dir);
}
