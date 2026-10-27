use mbse_catalog::*;
use serde_json::json;
fn fixture() -> tempfile::TempDir {
    let area = tempfile::tempdir().unwrap();
    std::fs::write(area.path().join(".project.json"),b"{ \"name\":\"Example\",\"publisher\":\"synthetic\",\"version\":\"1.0.0\",\"license\":\"MIT\",\"vendor\":{\"unknown\":true} }").unwrap();
    std::fs::write(
        area.path().join("Example.sysml"),
        b"package Example { part def Sample; }\n",
    )
    .unwrap();
    std::fs::write(
        area.path().join("LICENSE.txt"),
        b"Synthetic fixture authored for MBSE-Craft; MIT.",
    )
    .unwrap();
    write_meta(area.path(), "Example.sysml");
    area
}
fn write_meta(root: &std::path::Path, path: &str) {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(root.join("Example.sysml")).unwrap();
    let v = json!({"metamodel":"https://www.omg.org/spec/SysML/20250201","index":{"Example":path},"checksum":{"Example.sysml":{"algorithm":"SHA256","value":format!("{:x}",Sha256::digest(bytes))}},"vendor":["retained"]});
    std::fs::write(root.join(".meta.json"), serde_json::to_vec(&v).unwrap()).unwrap()
}
#[test]
fn preview_retains_exact_unknown_metadata_and_sources() {
    let area = fixture();
    let before = std::fs::read(area.path().join(".project.json")).unwrap();
    let c = open(area.path()).unwrap();
    assert_eq!(c.project.bytes, before);
    assert_eq!(c.project.value["vendor"]["unknown"], true);
    assert_eq!(c.metadata.value["vendor"], json!(["retained"]));
    assert_eq!(c.symbols.len(), 1);
    assert_eq!(
        c.source("Example.sysml").unwrap().bytes,
        b"package Example { part def Sample; }\n"
    );
    c.reverify().unwrap();
    std::fs::write(area.path().join("LICENSE.txt"), b"changed").unwrap();
    assert!(c.reverify().unwrap_err().contains("SOURCE_CHANGED"));
    assert_eq!(c.project.bytes, before)
}
#[test]
fn changed_model_and_duplicate_metadata_are_refused() {
    let area = fixture();
    std::fs::write(area.path().join("Example.sysml"), b"changed source").unwrap();
    assert!(open(area.path()).unwrap_err().contains("CHECKSUM_MISMATCH"));
    let area = fixture();
    std::fs::write(
        area.path().join(".project.json"),
        b"{\"name\":\"one\",\"name\":\"two\"}",
    )
    .unwrap();
    assert!(open(area.path()).unwrap_err().contains("duplicate key"))
}
#[test]
fn untrusted_index_paths_never_escape() {
    for path in [
        "../Example.sysml",
        "/Example.sysml",
        "C:/Example.sysml",
        "dir\\Example.sysml",
        "./Example.sysml",
        "https://example.org/Example.sysml",
    ] {
        let area = fixture();
        write_meta(area.path(), path);
        assert!(
            open(area.path()).unwrap_err().contains("UNSAFE_PATH"),
            "{path}"
        )
    }
}
#[cfg(unix)]
#[test]
fn symlinks_are_not_source() {
    let area = fixture();
    std::os::unix::fs::symlink("Example.sysml", area.path().join("alias.sysml")).unwrap();
    assert!(open(area.path()).unwrap_err().contains("SYMLINK"))
}
#[test]
fn count_and_byte_limits_refuse_without_partial_success() {
    let area = fixture();
    for n in 0..MAX_FILES {
        std::fs::write(area.path().join(format!("extra-{n}")), b"x").unwrap()
    }
    assert!(open(area.path()).unwrap_err().contains("COUNT_LIMIT"));
    let area = fixture();
    std::fs::write(area.path().join("huge"), vec![0u8; MAX_FILE_BYTES + 1]).unwrap();
    assert!(open(area.path()).is_err())
}
#[test]
fn malformed_and_unsupported_checksum_are_explicit() {
    let area = fixture();
    std::fs::write(area.path().join(".meta.json"), b"{truncated").unwrap();
    assert!(open(area.path()).unwrap_err().contains("CATALOG_JSON"));
    let area = fixture();
    let p = area.path().join(".meta.json");
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    v["checksum"]["Example.sysml"]["algorithm"] = "MD5".into();
    std::fs::write(p, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(open(area.path())
        .unwrap_err()
        .contains("UNSUPPORTED_CHECKSUM"))
}
