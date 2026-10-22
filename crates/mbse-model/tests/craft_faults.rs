//! Actual pinned ArtCraft adapter fault oracles on isolated workspace files.
use craft_atomic::{AtomicFs, NewFilePermissions, RealFs, RenameRetry};
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};
struct RefuseRename;
impl AtomicFs for RefuseRename {
    fn create_new(&self, p: &Path, m: NewFilePermissions) -> io::Result<File> {
        RealFs.create_new(p, m)
    }
    fn sync(&self, f: &File) -> io::Result<()> {
        RealFs.sync(f)
    }
    fn rename(&self, _: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "injected rename refusal",
        ))
    }
    fn hard_link(&self, a: &Path, b: &Path) -> io::Result<()> {
        RealFs.hard_link(a, b)
    }
    fn remove(&self, p: &Path) -> io::Result<()> {
        RealFs.remove(p)
    }
    fn sync_dir(&self, p: &Path) -> io::Result<()> {
        RealFs.sync_dir(p)
    }
    fn set_permissions(&self, f: &File, p: std::fs::Permissions) -> io::Result<()> {
        RealFs.set_permissions(f, p)
    }
}
fn fixture() -> tempfile::TempDir {
    let root = std::env::var_os("TMPDIR").expect("explicit workspace TMPDIR");
    assert!(Path::new(&root).starts_with("/workspace"));
    tempfile::tempdir_in(root).unwrap()
}
fn no_stages(root: &Path) {
    assert!(std::fs::read_dir(root).unwrap().all(|p| !p
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".craft_tmp")));
}
#[test]
fn actual_craft_stream_failure_keeps_previous_bytes() {
    let area = fixture();
    let path = area.path().join("project.kernspan");
    std::fs::write(&path, b"previous exact project").unwrap();
    let result = craft_atomic::atomic_write_with(&path, |file| {
        file.write_all(b"incomplete replacement")?;
        Err(io::Error::new(
            io::ErrorKind::StorageFull,
            "injected payload failure",
        ))
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"previous exact project");
    no_stages(area.path());
}
#[test]
fn actual_craft_rename_refusal_keeps_previous_bytes() {
    let area = fixture();
    let path = area.path().join("project.kernspan");
    std::fs::write(&path, b"previous exact project").unwrap();
    let result = craft_atomic::atomic_write_custom(
        &RefuseRename,
        RenameRetry {
            attempts: 0,
            first_delay_ms: 0,
        },
        &path,
        b"replacement",
    );
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"previous exact project");
    no_stages(area.path());
}
