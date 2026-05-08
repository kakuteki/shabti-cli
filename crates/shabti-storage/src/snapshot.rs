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

/// Returns true if the given path is a symbolic link.
///
/// Uses `symlink_metadata` so the check itself does not follow any link.
fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Verify that `path` is strictly contained within `base` after canonicalising.
///
/// This prevents path-traversal attacks where a crafted file name (e.g. `../secret`)
/// could escape the intended directory boundary.
fn assert_within_base(path: &Path, base: &Path) -> ShabtiResult<()> {
    let canon_path = path
        .canonicalize()
        .map_err(ShabtiError::Io)?;
    let canon_base = base
        .canonicalize()
        .map_err(ShabtiError::Io)?;
    if !canon_path.starts_with(&canon_base) {
        return Err(ShabtiError::Storage(format!(
            "パスがベースディレクトリ外にあります: {:?} (base: {:?})",
            path, base
        )));
    }
    Ok(())
}

/// Create a snapshot by copying storage files to a new subdirectory.
///
/// Layout: `<snapshots_dir>/<uuid>/shabti.log`, `events.jsonl`, `graph.json`
///
/// Missing source files are silently skipped.
/// Symbolic links are rejected to prevent TOCTOU attacks.
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
            // Reject symbolic links before copying to prevent TOCTOU attacks
            if is_symlink(&src) {
                return Err(ShabtiError::Storage(format!(
                    "シンボリックリンクはスナップショット操作に使用できません: {:?}",
                    src
                )));
            }

            let dst = snap_path.join(name);

            // Verify source is within the expected data directory
            assert_within_base(&src, data_dir)?;

            fs::copy(&src, &dst).map_err(ShabtiError::Io)?;
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
/// Symbolic links are rejected to prevent TOCTOU attacks.
pub fn restore_snapshot(snapshot: &Snapshot, data_dir: &Path) -> ShabtiResult<()> {
    fs::create_dir_all(data_dir).map_err(ShabtiError::Io)?;

    for &name in STORAGE_FILES {
        let src = snapshot.snapshot_dir.join(name);
        if src.exists() {
            // Reject symbolic links before copying to prevent TOCTOU attacks
            if is_symlink(&src) {
                return Err(ShabtiError::Storage(format!(
                    "シンボリックリンクはスナップショット操作に使用できません: {:?}",
                    src
                )));
            }

            let dst = data_dir.join(name);

            // Verify source is within the expected snapshot directory
            assert_within_base(&src, &snapshot.snapshot_dir)?;

            fs::copy(&src, &dst).map_err(ShabtiError::Io)?;
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
