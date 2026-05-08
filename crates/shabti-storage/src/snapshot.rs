use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use shabti_core::error::{ShabtiError, ShabtiResult};
use uuid::Uuid;

/// Known storage file names inside a data directory.
const STORAGE_FILES: &[&str] = &["shabti.log", "events.jsonl", "graph.json"];

/// Metadata file name stored inside each snapshot directory.
const META_FILE: &str = "snapshot.meta";

/// A snapshot of the storage layer at a point in time.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: Uuid,
    pub created_at: i64,
    pub snapshot_dir: PathBuf,
}

/// Create a snapshot by copying storage files to a new subdirectory.
///
/// Layout: `<snapshots_dir>/<uuid>/shabti.log`, `events.jsonl`, `graph.json`
///
/// Missing source files are silently skipped.
pub fn create_snapshot(data_dir: &Path, snapshots_dir: &Path) -> ShabtiResult<Snapshot> {
    let id = Uuid::new_v4();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX epoch")
        .as_secs() as i64;

    let snap_path = snapshots_dir.join(id.to_string());
    fs::create_dir_all(&snap_path).map_err(ShabtiError::Io)?;

    // Copy each known storage file if it exists
    for &name in STORAGE_FILES {
        let src = data_dir.join(name);
        if src.exists() {
            fs::copy(&src, snap_path.join(name)).map_err(ShabtiError::Io)?;
        }
    }

    // Write metadata
    let meta = format!("{}\n{}", id, now);
    fs::write(snap_path.join(META_FILE), meta).map_err(ShabtiError::Io)?;

    Ok(Snapshot {
        id,
        created_at: now,
        snapshot_dir: snap_path,
    })
}

/// Restore a snapshot by copying its files back into the data directory.
///
/// Only files that exist in the snapshot are copied.
pub fn restore_snapshot(snapshot: &Snapshot, data_dir: &Path) -> ShabtiResult<()> {
    fs::create_dir_all(data_dir).map_err(ShabtiError::Io)?;

    for &name in STORAGE_FILES {
        let src = snapshot.snapshot_dir.join(name);
        if src.exists() {
            fs::copy(&src, data_dir.join(name)).map_err(ShabtiError::Io)?;
        }
    }

    Ok(())
}

/// List all available snapshots, sorted by creation time (oldest first).
pub fn list_snapshots(snapshots_dir: &Path) -> ShabtiResult<Vec<Snapshot>> {
    if !snapshots_dir.exists() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();

    let entries = fs::read_dir(snapshots_dir).map_err(ShabtiError::Io)?;
    for entry in entries {
        let entry = entry.map_err(ShabtiError::Io)?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // UUID形式でないディレクトリ名はスキップ
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if Uuid::parse_str(&dir_name).is_err() {
            continue;
        }

        let meta_path = path.join(META_FILE);
        if !meta_path.exists() {
            continue;
        }

        let meta = fs::read_to_string(&meta_path).map_err(ShabtiError::Io)?;
        let mut lines = meta.lines();

        let id = lines
            .next()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| ShabtiError::Storage("invalid snapshot metadata".into()))?;

        let created_at = lines
            .next()
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| ShabtiError::Storage("invalid snapshot timestamp".into()))?;

        snapshots.push(Snapshot {
            id,
            created_at,
            snapshot_dir: path,
        });
    }

    snapshots.sort_by_key(|s| s.created_at);
    Ok(snapshots)
}
