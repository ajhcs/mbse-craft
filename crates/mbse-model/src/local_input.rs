//! Bounded acquisition of explicit regular local files, before any parser sees bytes.
use std::{fs::OpenOptions, io::Read, path::Path};

pub fn validate_path(path: &Path, context: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        // Classify the original Windows namespace before any textual stripping.
        // In particular, VerbatimUNC must not become a relative "UNC\\..." path.
        if let Some(Component::Prefix(prefix)) = path.components().next() {
            if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) {
                return Err(format!(
                    "{context}_INPUT_PATH: explicit local disk path required"
                ));
            }
        }
        validate_windows_text(&path.to_string_lossy(), context)?;
    }
    #[cfg(not(windows))]
    let _ = (path, context);
    Ok(())
}

// Kept independent of the host path parser so the Windows lexical rules also
// have bounded regression coverage on non-Windows test hosts. No filesystem I/O.
#[cfg(any(windows, test))]
fn validate_windows_text(text: &str, context: &str) -> Result<(), String> {
    let text = text.replace('/', "\\");
    let verbatim = text.strip_prefix("\\\\?\\");
    let disk = verbatim.unwrap_or(&text);
    let absolute_drive = disk.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && disk.as_bytes().get(1) == Some(&b':')
        && disk.as_bytes().get(2) == Some(&b'\\');
    // A verbatim namespace is admitted only for an absolute ASCII disk drive,
    // even where a host parser would classify an unfamiliar prefix differently.
    if disk.starts_with('\\') || disk.contains('\0') || (verbatim.is_some() && !absolute_drive) {
        return Err(format!(
            "{context}_INPUT_PATH: explicit local disk path required"
        ));
    }
    let tail = if disk.as_bytes().get(1) == Some(&b':') {
        if !absolute_drive {
            return Err(format!(
                "{context}_INPUT_PATH: absolute drive path required"
            ));
        }
        &disk[3..]
    } else {
        disk
    };
    for part in tail.split('\\') {
        let stem = part
            .trim_end_matches([' ', '.'])
            .split('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        let device = matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.chars().count() == 4
            && matches!(stem.chars().nth(3), Some('1'..='9' | '¹' | '²' | '³')));
        if part.contains(':') || device {
            return Err(format!(
                "{context}_INPUT_PATH: device names and alternate streams rejected"
            ));
        }
    }
    Ok(())
}
fn regular(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.is_file() && meta.file_attributes() & 0x400 == 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        meta.is_file()
    }
}
pub fn read(path: &Path, limit: usize, context: &str) -> Result<Vec<u8>, String> {
    read_after_preflight(path, limit, context, || {})
}
fn read_after_preflight(
    path: &Path,
    limit: usize,
    context: &str,
    after_preflight: impl FnOnce(),
) -> Result<Vec<u8>, String> {
    read_with_hook(path, limit, context, after_preflight).map(|(bytes, _)| bytes)
}
pub fn read_with_metadata(
    path: &Path,
    limit: usize,
    context: &str,
) -> Result<(Vec<u8>, std::fs::Metadata), String> {
    read_with_hook(path, limit, context, || {})
}
fn read_with_hook(
    path: &Path,
    limit: usize,
    context: &str,
    after_preflight: impl FnOnce(),
) -> Result<(Vec<u8>, std::fs::Metadata), String> {
    validate_path(path, context)?;
    let not_regular =
        || format!("{context}_INPUT_NOT_REGULAR: choose an explicit regular local file");
    let io_error = |e: std::io::Error| format!("{context}_INPUT_IO: {e}");
    let check = |m: &std::fs::Metadata| {
        if !regular(m) {
            return Err(not_regular());
        }
        if m.len() > limit as u64 {
            return Err(format!("{context}_FILE_LIMIT: maximum {limit} bytes"));
        }
        Ok(())
    };
    check(&std::fs::symlink_metadata(path).map_err(io_error)?)?;
    after_preflight();
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A swapped FIFO/device cannot block open; a swapped symlink is never followed.
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_OPEN_REPARSE_POINT: inspect the leaf itself rather than follow a replacement.
        options.custom_flags(0x0020_0000);
    }
    let mut file = options.open(path).map_err(io_error)?;
    let before = file.metadata().map_err(io_error)?;
    check(&before)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err(format!("{context}_FILE_LIMIT: maximum {limit} bytes"));
    }
    let after = file.metadata().map_err(io_error)?;
    if !same_identity(&before, &after) {
        return Err(format!("{context}_INPUT_CHANGED"));
    }
    Ok((bytes, after))
}
fn same_identity(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        (
            a.dev(),
            a.ino(),
            a.len(),
            a.mtime(),
            a.mtime_nsec(),
            a.ctime(),
            a.ctime_nsec(),
        ) == (
            b.dev(),
            b.ino(),
            b.len(),
            b.mtime(),
            b.mtime_nsec(),
            b.ctime(),
            b.ctime_nsec(),
        )
    }
    #[cfg(not(unix))]
    {
        a.len() == b.len() && a.modified().ok() == b.modified().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_bytes_limits_directories_and_missing_paths() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("source.json");
        std::fs::write(&f, [0, 0xff, 13, 10]).unwrap();
        assert_eq!(read(&f, 4, "TEST").unwrap(), [0, 0xff, 13, 10]);
        assert!(read(&f, 3, "TEST")
            .unwrap_err()
            .starts_with("TEST_FILE_LIMIT"));
        assert!(read(d.path(), 4, "TEST")
            .unwrap_err()
            .starts_with("TEST_INPUT_NOT_REGULAR"));
        assert!(read(&d.path().join("missing"), 4, "TEST")
            .unwrap_err()
            .starts_with("TEST_INPUT_IO"));
    }
    #[cfg(unix)]
    fn fifo(path: &Path) {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let p = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: a NUL-terminated pathname in this test's uniquely owned temporary directory.
        assert_eq!(unsafe { libc::mkfifo(p.as_ptr(), 0o600) }, 0);
    }
    #[cfg(unix)]
    #[test]
    fn writerless_fifo_socket_device_and_symlink_rejected_before_read() {
        use std::os::unix::{fs::symlink, net::UnixListener};
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("fifo");
        fifo(&f);
        let socket = d.path().join("socket");
        let _listener = UnixListener::bind(&socket).unwrap();
        let target = d.path().join("regular");
        std::fs::write(&target, b"ok").unwrap();
        let link = d.path().join("link");
        symlink(target, &link).unwrap();
        for path in [&f, &socket, &link, Path::new("/dev/null")] {
            assert!(read(path, 8, "TEST")
                .unwrap_err()
                .starts_with("TEST_INPUT_NOT_REGULAR"));
        }
    }
    #[cfg(unix)]
    #[test]
    fn replacement_between_metadata_and_open_cannot_block_or_follow_link() {
        use std::os::unix::fs::symlink;
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("input");
        std::fs::write(&f, b"ok").unwrap();
        let err = read_after_preflight(&f, 8, "TEST", || {
            std::fs::rename(&f, d.path().join("prior")).unwrap();
            fifo(&f);
        })
        .unwrap_err();
        assert!(err.starts_with("TEST_INPUT_NOT_REGULAR"));
        let g = d.path().join("second");
        std::fs::write(&g, b"ok").unwrap();
        let err = read_after_preflight(&g, 8, "TEST", || {
            std::fs::rename(&g, d.path().join("second-prior")).unwrap();
            symlink(d.path().join("prior"), &g).unwrap();
        })
        .unwrap_err();
        assert!(err.starts_with("TEST_INPUT_IO"));
    }
    #[cfg(windows)]
    #[test]
    fn device_pipe_remote_and_stream_names_rejected_before_metadata() {
        for p in [
            r"\\.\pipe\writerless",
            r"\\server\share\model",
            r"\\?\UNC\server\share\a",
            "NUL.json",
            "CON",
            r"C:\model.json:hidden",
            r"C:model.json",
            r"C:\A\COM1.sysml",
        ] {
            // Assert the pure preflight first: a regression must fail this test
            // without attempting metadata or open on any remote/device path.
            assert!(
                validate_path(Path::new(p), "TEST")
                    .unwrap_err()
                    .starts_with("TEST_INPUT_PATH"),
                "{p}"
            );
            assert!(
                read(Path::new(p), 16, "TEST")
                    .unwrap_err()
                    .starts_with("TEST_INPUT_PATH"),
                "{p}"
            );
        }
    }

    #[test]
    fn windows_lexical_namespace_and_stream_classification_never_accesses_filesystem() {
        for path in [
            r"\\server\share\model",
            r"//server/share/model",
            r"\\?\UNC\server\share\a",
            r"\\?\unc\server\share\a",
            r"//?/UNC/server/share/a",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1\a",
            r"\\?\Volume{00000000-0000-0000-0000-000000000000}\a",
            r"\\?\pipe\writerless",
            r"\\?\relative",
            r"\\?\C:relative",
            r"\\?\C",
            r"\\.\pipe\writerless",
            r"\\.\C:\model",
            r"\rooted\model",
            r"C:model.json",
            r"1:\model.json",
            r"C:\model.json:hidden",
            r"\\?\C:\model.json:hidden",
            r"relative\model.json:hidden",
            r"C:\A\COM1.sysml",
            r"\\?\C:\A\LPT².sysml",
            "NUL.json",
            "CON",
            "model\0.json",
        ] {
            assert!(
                validate_windows_text(path, "LEXICAL")
                    .unwrap_err()
                    .starts_with("LEXICAL_INPUT_PATH"),
                "{path:?}"
            );
        }
        for path in [
            r"C:\models\model.json",
            r"C:/models/model.json",
            r"\\?\C:\models\model.json",
            r"\\?\z:\models\model.json",
            r"relative\model.json",
            r"relative/model.json",
            r"UNC\server\share\a",
            r"Volume{local-folder}\a",
            r"C:\A\COM10.sysml",
            r".\NULish.sysml",
        ] {
            assert_eq!(validate_windows_text(path, "LEXICAL"), Ok(()), "{path}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn native_windows_prefix_kinds_only_admit_disk_namespaces_without_metadata() {
        use std::path::{Component, Prefix};
        let prefix = |text: &'static str| match Path::new(text).components().next() {
            Some(Component::Prefix(prefix)) => prefix.kind(),
            other => panic!("Expected Windows namespace prefix for {text:?}, got {other:?}"),
        };
        assert!(matches!(prefix(r"C:\models\a"), Prefix::Disk(_)));
        assert!(matches!(
            prefix(r"\\?\C:\models\a"),
            Prefix::VerbatimDisk(_)
        ));
        for path in [r"C:\models\a", r"\\?\C:\models\a", r"relative\a"] {
            assert_eq!(validate_path(Path::new(path), "PREFIX"), Ok(()), "{path}");
        }
        assert!(matches!(prefix(r"\\server\share\a"), Prefix::UNC(_, _)));
        assert!(matches!(
            prefix(r"\\?\UNC\server\share\a"),
            Prefix::VerbatimUNC(_, _)
        ));
        assert!(matches!(
            prefix(r"\\.\pipe\writerless"),
            Prefix::DeviceNS(_)
        ));
        assert!(matches!(
            prefix(r"\\?\GLOBALROOT\Device\a"),
            Prefix::Verbatim(_)
        ));
        for path in [
            r"\\server\share\a",
            r"\\?\UNC\server\share\a",
            r"\\.\pipe\writerless",
            r"\\?\GLOBALROOT\Device\a",
        ] {
            assert!(
                validate_path(Path::new(path), "PREFIX")
                    .unwrap_err()
                    .starts_with("PREFIX_INPUT_PATH"),
                "{path}"
            );
        }
    }
}
