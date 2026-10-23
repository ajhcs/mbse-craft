//! Explicit bounded read-only presentation inspection. Nothing here can admit
//! source claims into a native model, replace target views or mint a checkpoint.
use crate::{
    local_input,
    native_views::{self as v, Document, Origin},
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub struct Inspection {
    path: PathBuf,
    bytes: Vec<u8>,
    sha256: String,
    declared: Document,
    _origin: Origin,
}
impl Inspection {
    pub fn open(path: &Path) -> Result<Self, String> {
        let bytes = local_input::read(path, v::MAX_BYTES, "VIEWS_SOURCE_INSPECT")?;
        let declared = v::inspect_document(&bytes)?;
        // The existing8MiB raw/encoded limits bound two retained payloads,
        // not total parser allocations or RSS. No target/history is cloned.
        Ok(Self {
            path: path.to_owned(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes,
            declared,
            _origin: v::new_origin()?,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn source_sha256(&self) -> &str {
        &self.sha256
    }
    pub fn source_bytes(&self) -> usize {
        self.bytes.len()
    }
    /// All identifiers, native bindings and relationship keys remain source
    /// declarations. Structural validation does not validate native semantics.
    pub fn declared(&self) -> &Document {
        &self.declared
    }
    pub fn verify_unchanged(&self) -> Result<(), String> {
        let bytes = local_input::read(&self.path, v::MAX_BYTES, "VIEWS_SOURCE_RECHECK")?;
        if bytes != self.bytes {
            return Err(
                "VIEWS_SOURCE_CHANGED: inspect the source again; previous preview retained".into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        native_views::{Geometry, Layout, Views},
        stage1::{DocumentId, Editor, RecordId, Snapshot},
    };
    fn fixture() -> (tempfile::TempDir, PathBuf, Editor, Views) {
        let tmp = std::env::var_os("TMPDIR").expect("workspace TMPDIR");
        assert!(Path::new(&tmp).starts_with("/workspace") || cfg!(windows));
        let area = tempfile::Builder::new()
            .prefix("views-source-")
            .tempdir_in(tmp)
            .unwrap();
        let editor = Editor::new(
            Snapshot::new(DocumentId("source-document".into())),
            "source-runtime".into(),
        )
        .unwrap();
        let views = Views::new(&editor).unwrap();
        let path = area.path().join("source.json");
        std::fs::write(&path, views.recovery(&editor).unwrap().encoded().unwrap()).unwrap();
        (area, path, editor, views)
    }
    #[test]
    fn explicit_source_inspection_retains_bytes_and_never_changes_target() {
        let (_area, path, editor, mut target) = fixture();
        let id = *target.document().occurrences.keys().next().unwrap();
        let stamp = target.stamp(&editor);
        target
            .apply(&editor, &stamp, Layout::Move(id, Geometry::at(400., 300.)))
            .unwrap();
        let before = (
            editor.snapshot.digest(),
            target.document().digest().unwrap(),
            target.history_usage(),
            target.document().next_id(),
        );
        let bytes = std::fs::read(&path).unwrap();
        let preview = Inspection::open(&path).unwrap();
        assert_eq!(preview.source_bytes(), bytes.len());
        preview.verify_unchanged().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(
            (
                editor.snapshot.digest(),
                target.document().digest().unwrap(),
                target.history_usage(),
                target.document().next_id()
            ),
            before
        );
        let reopened = Inspection::open(&path).unwrap();
        assert_ne!(preview._origin, reopened._origin);
    }
    #[test]
    fn source_claims_are_not_target_admission_or_native_binding_proof() {
        let (_area, path, editor, views) = fixture();
        let mut declared = views.recovery(&editor).unwrap();
        let id = *declared.occurrences.keys().next().unwrap();
        declared.occurrences.get_mut(&id).unwrap().record = RecordId(999999);
        let bytes = declared.encoded().unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(Inspection::open(&path).is_ok()); // structurally bounded source claim
        assert!(Views::restore(&bytes, &editor).is_err()); // native admission still refuses
        declared.profile = v::PROFILE.into();
        declared.binding = Some(v::NativeBinding {
            document_id: declared.document_id.clone(),
            artifact_sha256: "a".repeat(64),
            snapshot_sha256: "b".repeat(64),
        });
        std::fs::write(&path, declared.encoded().unwrap()).unwrap();
        let preview = Inspection::open(&path).unwrap();
        assert_eq!(
            preview.declared().binding.as_ref().unwrap().artifact_sha256,
            "a".repeat(64)
        );
    }
    #[test]
    fn changed_or_missing_source_refuses_recheck_without_replacing_preview() {
        let (_area, path, _, _) = fixture();
        let preview = Inspection::open(&path).unwrap();
        let hash = preview.source_sha256().to_owned();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.push(b'\n');
        std::fs::write(&path, &bytes).unwrap();
        assert!(preview
            .verify_unchanged()
            .unwrap_err()
            .contains("SOURCE_CHANGED"));
        assert_eq!(preview.source_sha256(), hash);
        let other = Inspection::open(&path).unwrap();
        assert_ne!(preview.source_sha256(), other.source_sha256());
        std::fs::remove_file(&path).unwrap();
        assert!(preview.verify_unchanged().is_err());
        assert_eq!(preview.source_sha256(), hash);
    }
    #[test]
    fn future_unknown_duplicate_and_corrupt_source_preserve_original_bytes() {
        let (_area, path, _, _) = fixture();
        let original = std::fs::read(&path).unwrap();
        let text = String::from_utf8(original).unwrap();
        for candidate in [
            text.replace(v::RECOVERY_PROFILE, "mbse-craft.native-views-recovery/999"),
            text.replacen('{', "{\"unknown\":true,", 1),
            text.replacen('{', "{\"revision\":\"1\",", 1),
            "{".into(),
        ] {
            std::fs::write(&path, candidate.as_bytes()).unwrap();
            assert!(Inspection::open(&path).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), candidate.as_bytes());
        }
    }
    #[test]
    fn structurally_invalid_geometry_and_profile_binding_are_refused() {
        let (_area, path, editor, views) = fixture();
        let mut d = views.recovery(&editor).unwrap();
        let id = *d.occurrences.keys().next().unwrap();
        d.occurrences.get_mut(&id).unwrap().geometry.width = -1.;
        std::fs::write(&path, d.encoded().unwrap()).unwrap();
        assert!(Inspection::open(&path).is_err());
        let mut d = views.recovery(&editor).unwrap();
        d.profile = v::PROFILE.into();
        std::fs::write(&path, d.encoded().unwrap()).unwrap();
        assert!(Inspection::open(&path).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn explicit_source_reader_refuses_symlink_without_following_it() {
        let (area, path, _, _) = fixture();
        let link = area.path().join("link.json");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(Inspection::open(&link).is_err());
        assert!(Inspection::open(&path).is_ok());
    }
    #[test]
    fn raw_boundary_and_collection_budget_are_enforced_before_typed_preview() {
        let (_area, path, _, _) = fixture();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.resize(v::MAX_BYTES, b' ');
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(
            Inspection::open(&path).unwrap().source_bytes(),
            v::MAX_BYTES
        );
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len((v::MAX_BYTES + 1) as u64)
            .unwrap();
        assert!(Inspection::open(&path).is_err());
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            (v::MAX_BYTES + 1) as u64
        );
        let (_other, other, _, _) = fixture();
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&other).unwrap()).unwrap();
        let template = value["views"]["1"].clone();
        let map = value["views"].as_object_mut().unwrap();
        for n in 1..=v::MAX_VIEWS + 1 {
            map.insert(n.to_string(), template.clone());
        }
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(Inspection::open(&path)
            .err()
            .unwrap()
            .contains("COUNT_BUDGET"));
    }
    #[test]
    fn nested_duplicates_invalid_endpoints_and_allocator_never_admit_a_preview() {
        let (_area, path, _, _) = fixture();
        let original = std::fs::read_to_string(&path).unwrap();
        let duplicate = original.replacen("\"geometry\": {", "\"geometry\": {\"x\":0,", 1);
        assert_ne!(duplicate, original);
        std::fs::write(&path, &duplicate).unwrap();
        assert!(Inspection::open(&path).is_err());
        let mut value: serde_json::Value = serde_json::from_str(&original).unwrap();
        value["next_id"] = serde_json::json!("3");
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(Inspection::open(&path).err().unwrap().contains("ALLOCATOR"));
        let mut value: serde_json::Value = serde_json::from_str(&original).unwrap();
        value["next_id"] = serde_json::json!("6");
        value["edges"]["5"] = serde_json::json!({"id":"5","view":"1","source":"9999","target":"3","key":{"owner":"1","role":"ObjectiveSubject","target":"1","target_kind":"System"},"presence":{"state":"Detached","reason":"source declaration"}});
        let bytes = serde_json::to_vec(&value).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(Inspection::open(&path)
            .err()
            .unwrap()
            .contains("OCCURRENCE_MISSING"));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    #[test]
    fn source_inspection_cannot_bless_a_forged_bound_checkpoint() {
        let (area, path, _, _) = fixture();
        let mut session = crate::stage1_io::Session::new(area.path()).unwrap();
        let native = session
            .begin_save(&area.path().join("native.kernspan"))
            .unwrap();
        assert!(session.complete(crate::stage1_io::execute_write(native)));
        let cp = session.native_checkpoint().unwrap().unwrap();
        let views = Views::new(&session.editor).unwrap();
        let mut d = views.freeze(&session.editor, &cp).unwrap();
        std::fs::write(&path, d.encoded().unwrap()).unwrap();
        assert!(Inspection::open(&path).is_ok());
        d.binding.as_mut().unwrap().artifact_sha256 = "a".repeat(64);
        let bytes = d.encoded().unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(Inspection::open(&path).is_ok());
        assert!(Views::decode(&bytes, &session.editor, &cp).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    #[test]
    fn encoded_payload_expansion_is_bounded_independently_of_raw_input() {
        let (_area, path, _, _) = fixture();
        let base: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let mut found = false;
        for length in (300..=800).step_by(25) {
            let mut value = base.clone();
            value["next_id"] = serde_json::json!((v::MAX_EDGES + 5).to_string());
            let edges = value["edges"].as_object_mut().unwrap();
            let template = serde_json::json!({"id":"5","view":"1","source":"3","target":"3","key":{"owner":"1","role":"ObjectiveSubject","target":"1","target_kind":"System"},"presence":{"state":"Detached","reason":"r".repeat(length)}});
            for n in 0..v::MAX_EDGES {
                let id = (n + 5).to_string();
                let mut edge = template.clone();
                edge["id"] = serde_json::json!(id);
                edges.insert(id, edge);
            }
            let raw = serde_json::to_vec(&value).unwrap();
            let pretty = serde_json::to_vec_pretty(&value).unwrap();
            if raw.len() <= v::MAX_BYTES && pretty.len() > v::MAX_BYTES {
                std::fs::write(&path, &raw).unwrap();
                assert!(Inspection::open(&path)
                    .err()
                    .unwrap()
                    .contains("ENCODED_BUDGET"));
                assert_eq!(std::fs::metadata(&path).unwrap().len(), raw.len() as u64);
                found = true;
                break;
            }
        }
        assert!(
            found,
            "fixture must straddle raw/encoded payload boundaries"
        );
    }
}
