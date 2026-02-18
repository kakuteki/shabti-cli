use std::fs;
use shabti_storage::snapshot::{create_snapshot, list_snapshots, restore_snapshot};
use tempfile::TempDir;

/// Helper: write dummy storage files into a data directory.
fn seed_data_dir(dir: &std::path::Path) {
    fs::write(dir.join("shabti.log"), b"log-data-v1").unwrap();
    fs::write(dir.join("events.jsonl"), b"events-v1\n").unwrap();
    fs::write(dir.join("graph.json"), b"{}").unwrap();
}

// ============================================================
// create_snapshot
// ============================================================

#[test]
fn create_snapshot_copies_files() {
    let data_dir = TempDir::new().unwrap();
    let snap_dir = TempDir::new().unwrap();
    seed_data_dir(data_dir.path());

    let snap = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();

    assert!(snap.snapshot_dir.exists());
    assert!(snap.snapshot_dir.join("shabti.log").exists());
    assert!(snap.snapshot_dir.join("events.jsonl").exists());
    assert!(snap.snapshot_dir.join("graph.json").exists());

    // Content should match
    let log = fs::read_to_string(snap.snapshot_dir.join("shabti.log")).unwrap();
    assert_eq!(log, "log-data-v1");
}

#[test]
fn create_snapshot_assigns_unique_ids() {
    let data_dir = TempDir::new().unwrap();
    let snap_dir = TempDir::new().unwrap();
    seed_data_dir(data_dir.path());

    let s1 = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();
    let s2 = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();

    assert_ne!(s1.id, s2.id);
    assert_ne!(s1.snapshot_dir, s2.snapshot_dir);
}

#[test]
fn create_snapshot_skips_missing_files() {
    let data_dir = TempDir::new().unwrap();
    let snap_dir = TempDir::new().unwrap();
    // Only write one file
    fs::write(data_dir.path().join("shabti.log"), b"data").unwrap();

    let snap = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();
    assert!(snap.snapshot_dir.join("shabti.log").exists());
    // Missing files should not cause error, just skip
    assert!(!snap.snapshot_dir.join("events.jsonl").exists());
}

// ============================================================
// restore_snapshot
// ============================================================

#[test]
fn restore_snapshot_overwrites_data() {
    let data_dir = TempDir::new().unwrap();
    let snap_dir = TempDir::new().unwrap();
    seed_data_dir(data_dir.path());

    let snap = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();

    // Modify data after snapshot
    fs::write(data_dir.path().join("shabti.log"), b"log-data-v2").unwrap();
    fs::write(data_dir.path().join("events.jsonl"), b"events-v2\n").unwrap();

    // Restore should bring back v1
    restore_snapshot(&snap, data_dir.path()).unwrap();

    let log = fs::read_to_string(data_dir.path().join("shabti.log")).unwrap();
    assert_eq!(log, "log-data-v1");
    let events = fs::read_to_string(data_dir.path().join("events.jsonl")).unwrap();
    assert_eq!(events, "events-v1\n");
}

#[test]
fn restore_snapshot_handles_missing_snapshot_file() {
    let data_dir = TempDir::new().unwrap();
    let snap_dir = TempDir::new().unwrap();
    // Only snapshot shabti.log
    fs::write(data_dir.path().join("shabti.log"), b"data").unwrap();

    let snap = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();

    // Restoring should succeed even if snapshot only has partial files
    restore_snapshot(&snap, data_dir.path()).unwrap();
    let log = fs::read_to_string(data_dir.path().join("shabti.log")).unwrap();
    assert_eq!(log, "data");
}

// ============================================================
// list_snapshots
// ============================================================

#[test]
fn list_snapshots_empty() {
    let snap_dir = TempDir::new().unwrap();
    let snaps = list_snapshots(snap_dir.path()).unwrap();
    assert!(snaps.is_empty());
}

#[test]
fn list_snapshots_returns_all() {
    let data_dir = TempDir::new().unwrap();
    let snap_dir = TempDir::new().unwrap();
    seed_data_dir(data_dir.path());

    let s1 = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();
    let s2 = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();

    let snaps = list_snapshots(snap_dir.path()).unwrap();
    assert_eq!(snaps.len(), 2);

    let ids: Vec<_> = snaps.iter().map(|s| s.id).collect();
    assert!(ids.contains(&s1.id));
    assert!(ids.contains(&s2.id));
}

#[test]
fn list_snapshots_sorted_by_creation_time() {
    let data_dir = TempDir::new().unwrap();
    let snap_dir = TempDir::new().unwrap();
    seed_data_dir(data_dir.path());

    let _s1 = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();
    // Small delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(10));
    let _s2 = create_snapshot(data_dir.path(), snap_dir.path()).unwrap();

    let snaps = list_snapshots(snap_dir.path()).unwrap();
    assert!(snaps[0].created_at <= snaps[1].created_at);
}
