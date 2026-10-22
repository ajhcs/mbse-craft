//! Bounded read-only recovery inventory. Discovery never restores or deletes work.
use crate::{
    stage1::{DocumentId, RecordKind},
    stage1_io,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
pub const MAX_SCAN_ENTRIES: usize = 256;
pub const MAX_RECOVERY_FILES: usize = 32;
pub const MAX_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
#[derive(Clone, Debug)]
pub struct RecoveryEntry {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub document_id: DocumentId,
    pub model_revision: u64,
    pub raw_version: u64,
    pub snapshot_hash: String,
    pub record_count: usize,
    pub system_names: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct RecoveryRejection {
    pub path: PathBuf,
    pub reason: String,
}
#[derive(Clone, Debug, Default)]
pub struct RecoveryInventory {
    pub entries: Vec<RecoveryEntry>,
    pub rejected: Vec<RecoveryRejection>,
    pub truncated: bool,
}
/// Only immediate persistent session reservations are inspected. Unknown or
/// malformed files produce explicit diagnostics; no fallback paths are searched.
pub fn discover_recoveries(state_root: &Path) -> Result<RecoveryInventory, String> {
    let mut result = RecoveryInventory::default();
    let metadata = match fs::symlink_metadata(state_root) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(result),
        Err(error) => return Err(format!("RECOVERY_STATE_ROOT: {error}")),
    };
    if !metadata.file_type().is_dir() {
        return Err("RECOVERY_UNSAFE_STATE_ROOT".into());
    }
    let mut total = 0u64;
    let mut inspected = 0usize;
    for (index, entry) in fs::read_dir(state_root)
        .map_err(|e| e.to_string())?
        .enumerate()
    {
        if index >= MAX_SCAN_ENTRIES {
            result.truncated = true;
            break;
        }
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry
            .file_name()
            .to_string_lossy()
            .starts_with("stage1-session-")
        {
            continue;
        }
        let dir = entry.path();
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            result.rejected.push(RecoveryRejection {
                path: dir,
                reason: "RECOVERY_UNSAFE_SESSION_DIRECTORY".into(),
            });
            continue;
        }
        let path = dir.join("recovery.kernspan");
        let metadata = match fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                result.rejected.push(RecoveryRejection {
                    path,
                    reason: error.to_string(),
                });
                continue;
            }
        };
        if metadata.len() > stage1_io::MAX_BYTES as u64 {
            result.rejected.push(RecoveryRejection {
                path,
                reason: "RECOVERY_DOCUMENT_LIMIT".into(),
            });
            continue;
        }
        if inspected >= MAX_RECOVERY_FILES || total.saturating_add(metadata.len()) > MAX_TOTAL_BYTES
        {
            result.truncated = true;
            break;
        }
        inspected += 1;
        total = total.saturating_add(metadata.len());
        let admitted = (|| {
            if !metadata.file_type().is_file() {
                return Err("RECOVERY_UNSAFE_FILE".into());
            }
            // Acquisition rejects symlinks and special files and bounds the open read.
            let bytes = crate::local_input::read(&path, stage1_io::MAX_BYTES, "RECOVERY")?;
            let snapshot = stage1_io::decode_recovery(&bytes)?;
            Ok(RecoveryEntry {
                path: path.clone(),
                modified: metadata.modified().ok(),
                document_id: snapshot.document_id.clone(),
                model_revision: snapshot.model_revision,
                raw_version: snapshot.raw_version,
                snapshot_hash: snapshot.digest(),
                record_count: snapshot.records.len(),
                system_names: snapshot
                    .records
                    .values()
                    .filter(|r| r.kind == RecordKind::System)
                    .map(|r| r.name.clone())
                    .collect(),
            })
        })();
        match admitted {
            Ok(entry) => result.entries.push(entry),
            Err(reason) => result.rejected.push(RecoveryRejection { path, reason }),
        }
    }
    result.entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(result)
}
