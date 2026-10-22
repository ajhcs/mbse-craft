// MIT Kernspan test helper extracted from pinned c80037d2 build.rs.
// No build script is registered or executed by this test-only module.
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Relative source paths and all file/directory rebuild observations.
/// Public so the existing foundation target can exercise temporary fixtures.
#[derive(Debug, Clone)]
pub struct SourceInputs {
    pub files: Vec<PathBuf>,
    pub watches: Vec<PathBuf>,
}

pub fn source_inputs(
    root: &Path,
    files: &[PathBuf],
    trees: &[(&str, bool)],
) -> io::Result<SourceInputs> {
    fn visit(root: &Path, relative: &Path, all: bool, inputs: &mut SourceInputs) -> io::Result<()> {
        inputs.watches.push(relative.to_owned());
        for entry in fs::read_dir(root.join(relative))? {
            let path = relative.join(entry?.file_name());
            if root.join(&path).is_dir() {
                visit(root, &path, all, inputs)?;
            } else if all || path.extension().is_some_and(|extension| extension == "rs") {
                inputs.files.push(path);
            }
        }
        Ok(())
    }
    let mut inputs = SourceInputs {
        files: files.to_vec(),
        watches: Vec::new(),
    };
    for (path, all) in trees {
        visit(root, Path::new(path), *all, &mut inputs)?;
    }
    inputs.files.sort();
    inputs.watches.extend(inputs.files.iter().cloned());
    inputs.watches.sort();
    inputs.watches.dedup();
    Ok(inputs)
}

/// Preserve the established normalized-path/length/bytes encoding exactly.
pub fn source_id(root: &Path, inputs: &SourceInputs) -> io::Result<String> {
    let mut hash = Sha256::new();
    for path in &inputs.files {
        let name = path.to_string_lossy().replace('\\', "/");
        let bytes = fs::read(root.join(path))?;
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    Ok(format!("{:x}", hash.finalize()))
}

