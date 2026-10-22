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
    /// Successful payload bytes plus conservative upper bounds for failed open reads.
    pub accounted_read_bytes: u64,
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
        if inspected >= MAX_RECOVERY_FILES
            || total.saturating_add(metadata.len()).saturating_add(1) > MAX_TOTAL_BYTES
        {
            result.truncated = true;
            break;
        }
        inspected += 1;
        let admitted = (|| {
            if !metadata.file_type().is_file() {
                return Err("RECOVERY_UNSAFE_FILE".into());
            }
            // Acquisition rejects symlinks and special files and bounds the open read.
            let bytes = read_recovery_bytes(&path, &mut total, MAX_TOTAL_BYTES)?;
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
    result.accounted_read_bytes = total;
    result.entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(result)
}

/// The open-handle acquisition may read one extra byte to detect growth. Reserve
/// that byte from the remaining aggregate budget rather than trusting a prior
/// directory-stat length. A failed acquisition is charged its maximum possible
/// read, so even changing files cannot silently exceed this conservative bound.
fn read_recovery_bytes(path: &Path, used: &mut u64, budget: u64) -> Result<Vec<u8>, String> {
    let remaining = budget.saturating_sub(*used);
    if remaining <= 1 {
        return Err("RECOVERY_TOTAL_BYTES_LIMIT".into());
    }
    let limit = (remaining - 1).min(stage1_io::MAX_BYTES as u64) as usize;
    match crate::local_input::read(path, limit, "RECOVERY") {
        Ok(bytes) => {
            *used += bytes.len() as u64;
            Ok(bytes)
        }
        Err(error) => {
            *used += limit as u64 + 1;
            Err(error)
        }
    }
}

#[cfg(test)]
mod read_budget_tests {
    use super::*;
    #[test]
    fn grown_file_is_charged_by_actual_open_bytes_not_previous_metadata() {
        let area = tempfile::tempdir().unwrap();
        let path = area.path().join("recovery.kernspan");
        fs::write(&path, b"x").unwrap();
        let previous = fs::metadata(&path).unwrap().len();
        fs::write(&path, vec![b' '; 2048]).unwrap();
        let mut used = 0;
        let bytes = read_recovery_bytes(&path, &mut used, 4096).unwrap();
        assert_eq!(previous, 1);
        assert_eq!(bytes.len(), 2048);
        assert_eq!(used, 2048);
    }
    #[test]
    fn failed_grown_file_reserves_detection_byte_within_total_budget() {
        let area = tempfile::tempdir().unwrap();
        let path = area.path().join("recovery.kernspan");
        fs::write(&path, vec![b'x'; 2048]).unwrap();
        let mut used = 100;
        assert!(read_recovery_bytes(&path, &mut used, 1024).is_err());
        assert_eq!(used, 1024);
        assert!(read_recovery_bytes(&path, &mut used, 1024).is_err());
        assert_eq!(used, 1024);
    }
}
