//! App-level create-new semantics: an existing destination leaf, including a
//! dangling symlink, is refused. Craft stages private bytes inside an exclusively
//! created private directory; hard-link placement never replaces/follows a leaf.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        // Remove only these owned entries, never recursively remove unexpected data.
        let _ = fs::remove_file(self.0.join("payload"));
        let _ = fs::remove_dir(&self.0);
    }
}

pub(crate) fn write_private_new(destination: &Path, bytes: &[u8]) -> Result<(), String> {
    write_with_before_place(destination, bytes, || {})
}

fn write_with_before_place(
    destination: &Path,
    bytes: &[u8],
    before_place: impl FnOnce(),
) -> Result<(), String> {
    if destination.file_name().is_none() {
        return Err("EXTERNAL_COPY_DESTINATION_NAME_REQUIRED".into());
    }
    match fs::symlink_metadata(destination) {
        Ok(_) => return Err("EXTERNAL_COPY_DESTINATION_EXISTS: existing files, directories and dangling symlinks are refused".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (),
        Err(error) => return Err(format!("EXTERNAL_COPY_DESTINATION_INSPECTION: {error}")),
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let directory = parent.join(craft_atomic::temp_name("mbse-source-copy"));
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&directory)
        .map_err(|e| format!("EXTERNAL_COPY_PRIVATE_STAGE: {e}"))?;
    let stage = Stage(directory);
    let payload = stage.0.join("payload");
    craft_atomic::WriteOptions::default()
        .with_new_file_permissions(craft_atomic::NewFilePermissions::Private)
        .write_new(&payload, bytes)
        .map_err(|e| format!("EXTERNAL_COPY_STAGE_WRITE: {e}"))?;
    before_place();
    // No copy fallback: filesystems without hard links are explicitly unsupported.
    // The atomic link operation refuses a leaf that appears after inspection,
    // including a dangling symlink; no destination entry is followed/replaced.
    fs::hard_link(&payload, destination).map_err(|e| {
        format!(
            "EXTERNAL_COPY_CREATE_NEW_PLACEMENT: {e}; no destination was created by this operation"
        )
    })?;
    // As in Craft's create-new path, directory sync is best effort after placement.
    let _ = fs::File::open(parent).and_then(|dir| dir.sync_all());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_destination_and_late_arrival_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("copy.xml");
        fs::write(&output, b"existing").unwrap();
        assert!(write_private_new(&output, b"new").is_err());
        assert_eq!(fs::read(&output).unwrap(), b"existing");
        fs::remove_file(&output).unwrap();
        assert!(
            write_with_before_place(&output, b"new", || fs::write(&output, b"arrived").unwrap())
                .is_err()
        );
        assert_eq!(fs::read(&output).unwrap(), b"arrived");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[cfg(unix)]
    #[test]
    fn dangling_symlink_and_late_symlink_never_create_target() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("copy.xml");
        let target = dir.path().join("must-not-exist.xml");
        symlink(&target, &output).unwrap();
        assert!(write_private_new(&output, b"new").is_err());
        assert!(!target.exists());
        assert!(fs::symlink_metadata(&output)
            .unwrap()
            .file_type()
            .is_symlink());
        fs::remove_file(&output).unwrap();
        assert!(
            write_with_before_place(&output, b"new", || symlink(&target, &output).unwrap())
                .is_err()
        );
        assert!(!target.exists());
        assert!(fs::symlink_metadata(&output)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn private_success_cleans_stage() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("copy.xml");
        write_private_new(&output, b"exact bytes").unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"exact bytes");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&output).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
