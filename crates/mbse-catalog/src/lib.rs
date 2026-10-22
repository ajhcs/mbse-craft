//! Offline, read-only external library folder admission. No native model projection.
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fmt, fs,
    path::{Component, Path, PathBuf},
};
pub const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FILES: usize = 128;
pub const MAX_NODES: usize = 256;
pub const MAX_DEPTH: usize = 8;
#[derive(Clone, Debug)]
pub struct FrozenFile {
    pub path: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct RawMetadata {
    pub bytes: Vec<u8>,
    pub value: Value,
}
#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub path: String,
    pub file_sha256: String,
}
#[derive(Clone, Debug)]
pub struct Catalog {
    pub root: PathBuf,
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub declared_license: String,
    pub metamodel: String,
    pub project: RawMetadata,
    pub metadata: RawMetadata,
    pub files: Vec<FrozenFile>,
    pub symbols: Vec<Symbol>,
    pub source_sha256: String,
    pub diagnostics: Vec<String>,
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, x: bool) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_i64<E: de::Error>(self, x: i64) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_u64<E: de::Error>(self, x: u64) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_f64<E: de::Error>(self, x: f64) -> Result<Strict, E> {
                serde_json::Number::from_f64(x)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite JSON"))
            }
            fn visit_str<E: de::Error>(self, x: &str) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_string<E: de::Error>(self, x: String) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = vec![];
                while let Some(Strict(x)) = a.next_element()? {
                    v.push(x)
                }
                Ok(Strict(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = serde_json::Map::new();
                while let Some((k, Strict(x))) = a.next_entry::<String, Strict>()? {
                    if v.contains_key(&k) {
                        return Err(de::Error::custom(format!("duplicate key {k}")));
                    }
                    v.insert(k, x);
                }
                Ok(Strict(Value::Object(v)))
            }
        }
        d.deserialize_any(V)
    }
}
fn metadata(bytes: &[u8]) -> Result<RawMetadata, String> {
    if bytes.len() > 256 * 1024 {
        return Err("CATALOG_METADATA_LIMIT".into());
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let Strict(value) = Strict::deserialize(&mut d).map_err(|e| format!("CATALOG_JSON: {e}"))?;
    d.end().map_err(|e| format!("CATALOG_JSON: {e}"))?;
    if !value.is_object() {
        return Err("CATALOG_METADATA_OBJECT_REQUIRED".into());
    }
    Ok(RawMetadata {
        bytes: bytes.to_vec(),
        value,
    })
}
fn safe_relative(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.contains(['\\', ':'])
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
        || !matches!(
            craft_docpath::classify(value),
            craft_docpath::PathKind::Local
        )
        || Path::new(value)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("CATALOG_UNSAFE_PATH: {value}"));
    }
    Ok(())
}
fn walk(
    root: &Path,
    dir: &Path,
    depth: usize,
    nodes: &mut usize,
    total: &mut usize,
    files: &mut Vec<FrozenFile>,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("CATALOG_DEPTH_LIMIT".into());
    }
    for item in fs::read_dir(dir).map_err(|e| format!("CATALOG_DIRECTORY: {e}"))? {
        *nodes += 1;
        if *nodes > MAX_NODES {
            return Err("CATALOG_NODE_LIMIT".into());
        }
        let entry = item.map_err(|e| e.to_string())?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "CATALOG_OUTSIDE_ROOT")?
            .to_str()
            .ok_or("CATALOG_NON_UTF8_PATH")?
            .replace(std::path::MAIN_SEPARATOR, "/");
        safe_relative(&relative)?;
        if kind.is_symlink() {
            return Err(format!("CATALOG_SYMLINK: {relative}"));
        }
        if kind.is_dir() {
            walk(root, &path, depth + 1, nodes, total, files)?;
            continue;
        }
        if !kind.is_file() {
            return Err(format!("CATALOG_NONREGULAR: {relative}"));
        }
        if files.len() >= MAX_FILES {
            return Err("CATALOG_FILE_COUNT_LIMIT".into());
        }
        let bytes = mbse_model::local_input::read(&path, MAX_FILE_BYTES, "CATALOG")?;
        *total = total
            .checked_add(bytes.len())
            .ok_or("CATALOG_TOTAL_LIMIT")?;
        if *total > MAX_TOTAL_BYTES {
            return Err("CATALOG_TOTAL_LIMIT".into());
        }
        if matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("sysml" | "kerml")
        ) {
            std::str::from_utf8(&bytes).map_err(|_| "CATALOG_SOURCE_ENCODING")?;
        }
        files.push(FrozenFile {
            path: relative,
            sha256: hash(&bytes),
            bytes,
        });
    }
    Ok(())
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("CATALOG_REQUIRED_TEXT: {key}"))
}
pub fn open(root: &Path) -> Result<Catalog, String> {
    if !root.is_absolute() {
        return Err("CATALOG_ABSOLUTE_DIRECTORY_REQUIRED".into());
    }
    if !fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_dir()
    {
        return Err("CATALOG_UNSAFE_ROOT".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut files = vec![];
    walk(&root, &root, 0, &mut 0, &mut 0, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let get = |name: &str| {
        files
            .iter()
            .find(|f| f.path == name)
            .ok_or_else(|| format!("CATALOG_MISSING_FILE: {name}"))
    };
    let project = metadata(&get(".project.json")?.bytes)?;
    let meta = metadata(&get(".meta.json")?.bytes)?;
    let name = text(&project.value, "name")?.to_owned();
    let publisher = text(&project.value, "publisher")?.to_owned();
    let version = text(&project.value, "version")?.to_owned();
    let declared_license = project
        .value
        .get("license")
        .and_then(Value::as_str)
        .unwrap_or("Unspecified; no redistribution clearance")
        .to_owned();
    let metamodel = meta
        .value
        .get("metamodel")
        .and_then(Value::as_str)
        .unwrap_or("unspecified")
        .to_owned();
    let checks = meta
        .value
        .get("checksum")
        .and_then(Value::as_object)
        .ok_or("CATALOG_CHECKSUM_TABLE_REQUIRED")?;
    for (path, descriptor) in checks {
        safe_relative(path)?;
        let source = get(path)?;
        if descriptor.get("algorithm").and_then(Value::as_str) != Some("SHA256") {
            return Err(format!("CATALOG_UNSUPPORTED_CHECKSUM: {path}"));
        }
        if descriptor.get("value").and_then(Value::as_str) != Some(source.sha256.as_str()) {
            return Err(format!("CATALOG_CHECKSUM_MISMATCH: {path}"));
        }
    }
    let index = meta
        .value
        .get("index")
        .and_then(Value::as_object)
        .ok_or("CATALOG_INDEX_REQUIRED")?;
    let mut symbols = vec![];
    let mut indexed = BTreeSet::new();
    for (name, path) in index {
        if name.is_empty() {
            return Err("CATALOG_EMPTY_SYMBOL".into());
        }
        let path = path.as_str().ok_or("CATALOG_INDEX_PATH_TYPE")?;
        safe_relative(path)?;
        if !path.ends_with(".sysml") && !path.ends_with(".kerml") {
            return Err(format!("CATALOG_INDEX_NOT_MODEL_SOURCE: {path}"));
        }
        let file = get(path)?;
        if !checks.contains_key(path) {
            return Err(format!("CATALOG_UNPINNED_INDEX_SOURCE: {path}"));
        }
        indexed.insert(path.to_owned());
        symbols.push(Symbol {
            name: name.clone(),
            path: path.into(),
            file_sha256: file.sha256.clone(),
        });
    }
    let mut digest = Sha256::new();
    for f in &files {
        digest.update((f.path.len() as u64).to_le_bytes());
        digest.update(f.path.as_bytes());
        digest.update((f.bytes.len() as u64).to_le_bytes());
        digest.update(&f.bytes)
    }
    let mut diagnostics=vec!["Indexed external source only; syntax, semantic import and execution not evaluated.".into(),"Dependencies are retained as metadata, unresolved; no network requests or implicit standard-library installation.".into(),"Declared license is provenance, not automatic redistribution permission. No package export enabled.".into()];
    if metamodel != "https://www.omg.org/spec/SysML/20250201"
        && metamodel != "https://www.omg.org/spec/KerML/20250201"
    {
        diagnostics.push(format!(
            "Unrecognized metamodel {metamodel}; no semantic support claimed."
        ))
    }
    for f in &files {
        if (f.path.ends_with(".sysml") || f.path.ends_with(".kerml")) && !indexed.contains(&f.path)
        {
            diagnostics.push(format!("Unindexed model source retained: {}", f.path))
        }
    }
    Ok(Catalog {
        root,
        name,
        publisher,
        version,
        declared_license,
        metamodel,
        project,
        metadata: meta,
        files,
        symbols,
        source_sha256: format!("{:x}", digest.finalize()),
        diagnostics,
    })
}
impl Catalog {
    pub fn source(&self, path: &str) -> Option<&FrozenFile> {
        self.files.iter().find(|f| f.path == path)
    }
    pub fn reverify(&self) -> Result<(), String> {
        let now = open(&self.root)?;
        if now.source_sha256 != self.source_sha256 {
            return Err("CATALOG_SOURCE_CHANGED: retain frozen preview; reopen explicitly".into());
        }
        Ok(())
    }
}
