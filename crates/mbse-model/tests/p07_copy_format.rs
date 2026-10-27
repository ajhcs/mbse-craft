//! Actual decoder/IO acceptance for P07. Constructed provenance is QA input,
//! never evidence that live presentation-copy Apply exists.
use mbse_model::{
    native_views::{self as v, Geometry, Layout, Views},
    native_views_copy::CopyOrigin,
    native_views_inspect::Inspection,
    native_views_io::{self as io, Io, Kind},
    stage1_io::{self, Session},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

struct Fixture {
    _area: tempfile::TempDir,
    session: Session,
    views: Views,
    io: Io,
    source: PathBuf,
    native: PathBuf,
    original: Vec<u8>,
    v2: Value,
}
impl Fixture {
    fn new() -> Self {
        let area = tempfile::Builder::new()
            .prefix("p07-")
            .tempdir_in(std::env::var_os("TMPDIR").expect("workspace TMPDIR"))
            .unwrap();
        let mut session = Session::new(&area.path().join("state")).unwrap();
        let native = area.path().join("native.kernspan");
        let job = session.begin_save(&native).unwrap();
        assert!(session.complete(stage1_io::execute_write(job)));
        let views = Views::new(&session.editor).unwrap();
        let source = area.path().join("v1.mbse-views.json");
        let mut io = Io::default();
        let job = io.begin(&session, &views, &source, Kind::Bound).unwrap();
        assert!(io.complete(&session, &views, io::execute(job)));
        let original = std::fs::read(&source).unwrap();
        let mut v2: Value = serde_json::from_slice(&original).unwrap();
        v2["profile"] = v::COPY_PROFILE.into();
        v2["views"]["2"]["copy_origin"] = json!({
            "profile":"mbse-craft.presentation-copy-origin/1",
            "source_sha256":format!("{:x}",Sha256::digest(&original)),
            "source_profile":v::PROFILE,
            "source_document_id":v2["document_id"],
            "source_sidecar_id":v2["sidecar_id"],
            "source_view":"1",
            "declared_native_binding":v2["binding"],
            "occurrences":{"3":{"source_record":"1","expected_kind":"System","outcome":{"state":"Mapped","target_occurrence":"4","target_record":"1"}}},
            "edges":{}
        });
        Self {
            _area: area,
            session,
            views,
            io,
            source,
            native,
            original,
            v2,
        }
    }
    fn decode(&self, value: &Value) -> Result<Views, String> {
        Views::decode(
            &serde_json::to_vec(value).unwrap(),
            &self.session.editor,
            &self.session.native_checkpoint().unwrap().unwrap(),
        )
    }
    fn state(&self) -> (String, String, (usize, usize), u64, Vec<u8>, Vec<u8>) {
        (
            self.session.editor.snapshot.digest(),
            self.views.document().digest().unwrap(),
            self.views.history_usage(),
            self.views.document().next_id(),
            std::fs::read(&self.source).unwrap(),
            std::fs::read(&self.native).unwrap(),
        )
    }
}

#[test]
fn v1_bound_and_recovery_remain_readable_without_migration_or_rewrite() {
    let f = Fixture::new();
    let before = f.state();
    let loaded = f
        .decode(&serde_json::from_slice(&f.original).unwrap())
        .unwrap();
    assert_eq!(loaded.document().profile, v::PROFILE);
    assert_eq!(loaded.document().encoded().unwrap(), f.original);
    assert!(loaded
        .document()
        .views
        .values()
        .all(|view| view.copy_origin.is_none()));
    let recovery = loaded.recovery(&f.session.editor).unwrap();
    assert_eq!(recovery.profile, v::RECOVERY_PROFILE);
    assert!(Views::restore(&recovery.encoded().unwrap(), &f.session.editor).is_ok());
    assert_eq!(f.state(), before);
}

#[test]
fn v2_bound_recovery_save_and_reopen_preserve_inert_provenance() {
    let f = Fixture::new();
    let before = f.state();
    let loaded = f.decode(&f.v2).unwrap();
    let origin = loaded
        .document()
        .views
        .values()
        .find_map(|view| view.copy_origin.as_ref())
        .unwrap()
        .clone();
    origin
        .verify_source(&Inspection::open(&f.source).unwrap())
        .unwrap();
    let recovery = loaded.recovery(&f.session.editor).unwrap();
    assert_eq!(recovery.profile, v::COPY_RECOVERY_PROFILE);
    let restored = Views::restore(&recovery.encoded().unwrap(), &f.session.editor).unwrap();
    assert_eq!(restored.history_usage(), (0, 0));
    assert_eq!(
        restored
            .document()
            .views
            .values()
            .find_map(|v| v.copy_origin.as_ref()),
        Some(&origin)
    );
    let target = f._area.path().join("fresh-v2.mbse-views.json");
    let mut io = Io::default();
    let job = io.begin(&f.session, &loaded, &target, Kind::Bound).unwrap();
    assert!(io.complete(&f.session, &loaded, io::execute(job)));
    io.verify_preserved(&f.session, &loaded, Kind::Bound)
        .unwrap();
    let mut reopened = Views::new(&f.session.editor).unwrap();
    let mut reopened_io = Io::default();
    reopened_io
        .open(&f.session, &mut reopened, &target, false)
        .unwrap();
    assert_eq!(reopened.document(), loaded.document());
    assert_eq!(f.state(), before);
}

#[test]
fn version_change_requires_fresh_save_as_and_leaves_owned_v1_exact() {
    let mut f = Fixture::new();
    let loaded = f.decode(&f.v2).unwrap();
    let before = f.state();
    let error =
        f.io.begin(&f.session, &loaded, &f.source, Kind::Bound)
            .err()
            .unwrap();
    assert!(error.contains("FORMAT_SAVE_AS_REQUIRED"), "{error}");
    assert!(!f.io.pending());
    assert_eq!(f.state(), before);
    assert!(f
        .io
        .begin(
            &f.session,
            &loaded,
            &f._area.path().join("fresh.mbse-views.json"),
            Kind::Bound
        )
        .is_ok());
}

#[test]
fn future_profiles_unknown_fields_and_malformed_mappings_are_retained_without_mutation() {
    let mut f = Fixture::new();
    let before = f.state();
    let mut cases = Vec::new();
    let mut future = f.v2.clone();
    future["profile"] = "mbse-craft.native-views/999".into();
    cases.push(future);
    let mut unknown = f.v2.clone();
    unknown["views"]["2"]["copy_origin"]["future_authority"] = true.into();
    cases.push(unknown);
    let mut wrong = f.v2.clone();
    wrong["views"]["2"]["copy_origin"]["occurrences"]["3"]["outcome"]["target_occurrence"] =
        "3".into();
    cases.push(wrong);
    for (index, value) in cases.into_iter().enumerate() {
        let path = f
            ._area
            .path()
            .join(format!("refused-{index}.mbse-views.json"));
        let bytes = serde_json::to_vec(&value).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(f.io.open(&f.session, &mut f.views, &path, false).is_err());
        assert_eq!(f.io.retained.last().unwrap().bytes, bytes);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(f.state(), before);
        assert!(f
            .io
            .begin(&f.session, &f.views, &path, Kind::Bound)
            .err()
            .unwrap()
            .contains("PROTECTED_INPUT"));
    }
}

#[test]
fn v1_rejects_provenance_even_null_and_v2_rejects_duplicate_or_noncanonical_ids() {
    let f = Fixture::new();
    let before = f.state();
    let mut value: Value = serde_json::from_slice(&f.original).unwrap();
    value["views"]["2"]["copy_origin"] = Value::Null;
    assert!(f.decode(&value).err().unwrap().contains("REQUIRES_V2"));
    for id in [
        json!(4),
        json!("04"),
        json!("0"),
        json!("18446744073709551616"),
    ] {
        let mut value = f.v2.clone();
        value["views"]["2"]["copy_origin"]["occurrences"]["3"]["outcome"]["target_occurrence"] = id;
        assert!(f.decode(&value).is_err());
    }
    let bytes = serde_json::to_string(&f.v2).unwrap().replace(
        "\"source_view\":\"1\"",
        "\"source_view\":\"1\",\"source_view\":\"1\"",
    );
    assert_ne!(bytes, serde_json::to_string(&f.v2).unwrap());
    assert!(Views::decode(
        bytes.as_bytes(),
        &f.session.editor,
        &f.session.native_checkpoint().unwrap().unwrap()
    )
    .is_err());
    assert_eq!(f.state(), before);
}

#[test]
fn wrong_kind_duplicate_targets_and_unmapped_edge_endpoints_refuse_atomically() {
    let f = Fixture::new();
    let before = f.state();
    let mut value = f.v2.clone();
    value["views"]["2"]["copy_origin"]["occurrences"]["3"]["expected_kind"] = "Mission".into();
    assert!(f.decode(&value).is_err());
    let mut value = f.v2.clone();
    value["views"]["2"]["copy_origin"]["occurrences"]["5"] =
        value["views"]["2"]["copy_origin"]["occurrences"]["3"].clone();
    assert!(f.decode(&value).err().unwrap().contains("TARGET_MAPPING"));
    let mut value = f.v2.clone();
    value["views"]["2"]["copy_origin"]["occurrences"]["5"] = json!({"source_record":"1","expected_kind":"System","outcome":{"state":"Skipped","reason":"Explicitly skipped"}});
    value["views"]["2"]["copy_origin"]["edges"]["6"] = json!({"source_occurrence":"3","target_occurrence":"5","outcome":{"state":"Mapped","target_edge":"7"}});
    assert!(f
        .decode(&value)
        .err()
        .unwrap()
        .contains("UNMAPPED_ENDPOINT"));
    assert_eq!(f.state(), before);
}

#[test]
fn provenance_item_byte_and_utf8_reason_limits_refuse_before_target_mutation() {
    let f = Fixture::new();
    let before = f.state();
    let mut value = f.v2.clone();
    let origin = &mut value["views"]["2"]["copy_origin"];
    origin["occurrences"] = json!((3..3+v::MAX_PER_VIEW+1).map(|id|(id.to_string(),json!({"source_record":"1","expected_kind":"System","outcome":{"state":"Skipped","reason":"Skipped"}}))).collect::<serde_json::Map<_,_>>());
    assert!(f.decode(&value).err().unwrap().contains("MAPPING_BUDGET"));
    let mut value = f.v2.clone();
    value["views"]["2"]["copy_origin"]["occurrences"]["3"]["outcome"] =
        json!({"state":"Skipped","reason":"界".repeat(400)});
    assert!(f.decode(&value).err().unwrap().contains("SKIP_REASON"));
    let origin = json!({"profile":"mbse-craft.presentation-copy-origin/1","source_sha256":"a".repeat(64),"source_profile":v::RECOVERY_PROFILE,"source_document_id":"source","source_sidecar_id":"views:source","source_view":"1","declared_native_binding":null,"occurrences":(3..3+v::MAX_PER_VIEW).map(|id|(id.to_string(),json!({"source_record":"1","expected_kind":"System","outcome":{"state":"Skipped","reason":"r".repeat(1024)}}))).collect::<serde_json::Map<_,_>>(),"edges":{}});
    let mut value = f.v2.clone();
    value["views"]["1"]["copy_origin"] = origin.clone();
    value["views"]["2"]["copy_origin"] = origin;
    assert!(serde_json::to_vec(&value).unwrap().len() < v::MAX_BYTES);
    assert!(f
        .decode(&value)
        .err()
        .unwrap()
        .contains("PROVENANCE_BUDGET"));
    assert_eq!(f.state(), before);
}

#[test]
fn source_hash_identity_coverage_and_changed_bytes_require_explicit_recheck() {
    let f = Fixture::new();
    let before = f.state();
    let source = Inspection::open(&f.source).unwrap();
    let origin: CopyOrigin =
        serde_json::from_value(f.v2["views"]["2"]["copy_origin"].clone()).unwrap();
    origin.verify_source(&source).unwrap();
    let mut wrong = origin.clone();
    wrong.source_sha256 = "a".repeat(64);
    assert!(wrong.verify_source(&source).is_err());
    let mut wrong = origin.clone();
    wrong.occurrences.clear();
    assert!(wrong
        .verify_source(&source)
        .err()
        .unwrap()
        .contains("COVERAGE"));
    let mut wrong = origin.clone();
    wrong
        .occurrences
        .values_mut()
        .next()
        .unwrap()
        .source_record
        .0 = 999;
    assert!(wrong.verify_source(&source).is_err());
    assert_eq!(f.state(), before);
    let mut changed = f.original.clone();
    changed.push(b'\n');
    std::fs::write(&f.source, &changed).unwrap();
    assert!(origin
        .verify_source(&source)
        .err()
        .unwrap()
        .contains("SOURCE_CHANGED"));
    assert_eq!(std::fs::read(&f.source).unwrap(), changed);
}

#[test]
fn provenance_is_inert_through_layout_history_and_cannot_reuse_runtime_authority() {
    let f = Fixture::new();
    let mut loaded = f.decode(&f.v2).unwrap();
    let stamp = loaded.stamp(&f.session.editor);
    let origin = loaded
        .document()
        .views
        .values()
        .find_map(|v| v.copy_origin.clone())
        .unwrap();
    let occurrence = loaded
        .document()
        .occurrences
        .values()
        .find(|o| o.view.get() == 2)
        .unwrap()
        .id;
    loaded
        .apply(&f.session.editor, &stamp, Layout::Remove(occurrence))
        .unwrap();
    let restored = Views::restore(
        &loaded
            .recovery(&f.session.editor)
            .unwrap()
            .encoded()
            .unwrap(),
        &f.session.editor,
    )
    .unwrap();
    assert_eq!(restored.history_usage(), (0, 0));
    assert_eq!(
        restored
            .document()
            .views
            .values()
            .find_map(|v| v.copy_origin.clone()),
        Some(origin)
    );
    assert!(loaded
        .undo(&f.session.editor, &loaded.stamp(&f.session.editor), false)
        .unwrap());
    assert!(loaded.document().occurrences.contains_key(&occurrence));
    let fresh = f.decode(&f.v2).unwrap();
    let primary = fresh.document().primary;
    assert!(loaded
        .apply(
            &f.session.editor,
            &fresh.stamp(&f.session.editor),
            Layout::Camera(primary, v::Camera::default())
        )
        .is_err());
    assert_eq!(f.session.editor.snapshot.digest(), f.state().0);
}

#[test]
fn declared_source_binding_cannot_authorize_a_wrong_outer_native_binding() {
    let f = Fixture::new();
    let before = f.state();
    let mut value = f.v2.clone();
    value["binding"]["artifact_sha256"] = "a".repeat(64).into();
    assert!(f
        .decode(&value)
        .err()
        .unwrap()
        .contains("EXACT_NATIVE_BINDING"));
    value = f.v2.clone();
    value["views"]["2"]["copy_origin"]["declared_native_binding"]["document_id"] = "foreign".into();
    assert!(f.decode(&value).err().unwrap().contains("DECLARED_BINDING"));
    assert_eq!(f.state(), before);
}

#[test]
fn authored_edges_keep_exact_mapped_occurrence_endpoints() {
    use mbse_model::{
        native_views::{SemanticKey, SemanticRole},
        stage1::{Editor, RecordId, Snapshot},
    };
    let mut f = Fixture::new();
    let snapshot = Snapshot::from_fixture(
        f.session.editor.snapshot.document_id.clone(),
        &serde_json::from_str(include_str!("../fixtures/stage1/inspection.json")).unwrap(),
    )
    .unwrap();
    f.session.editor = Editor::new(snapshot, f.session.editor.freshness().instance_id).unwrap();
    let native = f._area.path().join("edge-native.kernspan");
    let job = f.session.begin_save(&native).unwrap();
    assert!(f.session.complete(stage1_io::execute_write(job)));
    let mut views = Views::new(&f.session.editor).unwrap();
    let primary = views.document().primary;
    let review = *views
        .document()
        .views
        .keys()
        .find(|id| **id != primary)
        .unwrap();
    for id in [100, 200] {
        views
            .apply(
                &f.session.editor,
                &views.stamp(&f.session.editor),
                Layout::Reuse {
                    view: review,
                    record: RecordId(id),
                    geometry: Geometry::at(id as f64, 30.),
                },
            )
            .unwrap();
    }
    let occurrence = |views: &Views, view, record| {
        views
            .document()
            .occurrences
            .values()
            .find(|o| o.view == view && o.record == RecordId(record))
            .unwrap()
            .id
    };
    for view in [primary, review] {
        let key = SemanticKey::checked(
            &f.session.editor.snapshot,
            RecordId(200),
            SemanticRole::CapabilityContributesToMission,
            RecordId(100),
        )
        .unwrap();
        views
            .apply(
                &f.session.editor,
                &views.stamp(&f.session.editor),
                Layout::Route {
                    view,
                    key,
                    source: occurrence(&views, view, 200),
                    target: occurrence(&views, view, 100),
                },
            )
            .unwrap();
    }
    let source = f._area.path().join("edge-source.mbse-views.json");
    let mut io = Io::default();
    let job = io.begin(&f.session, &views, &source, Kind::Bound).unwrap();
    assert!(io.complete(&f.session, &views, io::execute(job)));
    let bytes = std::fs::read(&source).unwrap();
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["profile"] = v::COPY_PROFILE.into();
    let mappings = views.document().occurrences.values().filter(|o|o.view==primary).map(|o| {
        let target = views.document().occurrences.values().find(|t|t.view==review && t.record==o.record);
        let outcome = target.map(|t|json!({"state":"Mapped","target_occurrence":t.id,"target_record":t.record.0.to_string()})).unwrap_or_else(||json!({"state":"Skipped","reason":"Not selected for the QA copy"}));
        (o.id.get().to_string(),json!({"source_record":o.record.0.to_string(),"expected_kind":o.expected_kind,"outcome":outcome}))
    }).collect::<serde_json::Map<_,_>>();
    let source_edge = views
        .document()
        .edges
        .values()
        .find(|e| e.view == primary)
        .unwrap();
    let target_edge = views
        .document()
        .edges
        .values()
        .find(|e| e.view == review)
        .unwrap();
    let edge_mappings = serde_json::Map::from_iter([(
        source_edge.id.get().to_string(),
        json!({"source_occurrence":source_edge.source,"target_occurrence":source_edge.target,"outcome":{"state":"Mapped","target_edge":target_edge.id}}),
    )]);
    value["views"][review.get().to_string()]["copy_origin"] = json!({"profile":"mbse-craft.presentation-copy-origin/1","source_sha256":format!("{:x}",Sha256::digest(&bytes)),"source_profile":v::PROFILE,"source_document_id":value["document_id"],"source_sidecar_id":value["sidecar_id"],"source_view":primary,"declared_native_binding":value["binding"],"occurrences":mappings,"edges":edge_mappings});
    let loaded = f.decode(&value).unwrap();
    loaded.document().views[&review]
        .copy_origin
        .as_ref()
        .unwrap()
        .verify_source(&Inspection::open(&source).unwrap())
        .unwrap();
    let before = (
        f.session.editor.snapshot.digest(),
        views.document().digest().unwrap(),
        views.history_usage(),
        views.document().next_id(),
    );
    value["views"][review.get().to_string()]["copy_origin"]["edges"]
        [source_edge.id.get().to_string()]["source_occurrence"] =
        serde_json::to_value(source_edge.target).unwrap();
    assert!(f.decode(&value).err().unwrap().contains("EDGE_IDENTITY"));
    assert_eq!(
        (
            f.session.editor.snapshot.digest(),
            views.document().digest().unwrap(),
            views.history_usage(),
            views.document().next_id()
        ),
        before
    );
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
}
