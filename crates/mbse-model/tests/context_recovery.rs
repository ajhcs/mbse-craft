use mbse_model::{
    recovery::discover_recoveries,
    stage1::{
        AffiliationKind, Command, ContextKind, DocumentId, Editor, RawKey, RecordId, Snapshot,
    },
    stage1_io::{execute_write, Session},
};
use serde_json::json;
fn editor() -> Editor {
    Editor::new(Snapshot::from_fixture(DocumentId("context-test".into()), &json!({"records":[
        {"id":1,"type":"System","name":"One","scope":1},
        {"id":2,"type":"System","name":"Two","scope":2},
        {"id":10,"type":"Stakeholder","name":"Reviewer","scope":1,"affiliations":{},"actors":[{"opaque":true}]},
        {"id":11,"type":"Organization","name":"Team","scope":1},
        {"id":12,"type":"Organization","name":"Foreign","scope":2},
        {"id":20,"type":"Environment","name":"Bench","scope":1,"context":[],"inputs":[]}
    ]})).unwrap(), "instance".into()).unwrap()
}
#[test]
fn context_and_affiliation_are_typed_undoable_preserve_opaque_and_roundtrip() {
    let mut e = editor();
    e.command(
        &e.freshness(),
        Command::EditText {
            key: RawKey {
                record: RecordId(10),
                field: "interest".into(),
                item: None,
            },
            text: "Review evidence".into(),
        },
    )
    .unwrap();
    for organization in [RecordId(12), RecordId(20)] {
        let before = e.snapshot.clone();
        assert!(e
            .command(
                &e.freshness(),
                Command::SetAffiliation {
                    stakeholder: RecordId(10),
                    organization,
                    relationship: Some(AffiliationKind::Internal)
                }
            )
            .is_err());
        assert_eq!(e.snapshot, before);
    }
    e.command(
        &e.freshness(),
        Command::SetAffiliation {
            stakeholder: RecordId(10),
            organization: RecordId(11),
            relationship: Some(AffiliationKind::Internal),
        },
    )
    .unwrap();
    assert_eq!(
        e.snapshot.records[&RecordId(10)].fields["affiliations"]["11"],
        "Internal"
    );
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert!(e.snapshot.records[&RecordId(10)].fields["affiliations"]
        .as_object()
        .unwrap()
        .is_empty());
    assert!(e.redo_model(&e.freshness()).unwrap());
    let id = e
        .command(
            &e.freshness(),
            Command::AddContext {
                environment: RecordId(20),
                name: "Operating assumption".into(),
                kind: ContextKind::Assumption,
            },
        )
        .unwrap()
        .created
        .unwrap();
    let key = RawKey {
        record: RecordId(20),
        field: "contextName".into(),
        item: Some(id),
    };
    e.command(
        &e.freshness(),
        Command::EditText {
            key: key.clone(),
            text: "Named assumption".into(),
        },
    )
    .unwrap();
    assert_eq!(e.display_text(&key), "Named assumption");
    let key = RawKey {
        field: "contextText".into(),
        ..key
    };
    e.command(
        &e.freshness(),
        Command::EditText {
            key: key.clone(),
            text: "Reviewer available".into(),
        },
    )
    .unwrap();
    e.command(
        &e.freshness(),
        Command::SetContextKind {
            environment: RecordId(20),
            item: id,
            kind: ContextKind::Description,
        },
    )
    .unwrap();
    assert_eq!(
        e.snapshot.records[&RecordId(20)].fields["context"][0]["kind"],
        "Description"
    );
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(
        e.snapshot.records[&RecordId(20)].fields["context"][0]["kind"],
        "Assumption"
    );
    assert_eq!(
        e.snapshot.records[&RecordId(10)].fields["actors"],
        json!([{"opaque":true}])
    );
    assert!(mbse_model::stage1_sysml::from_records(&e.snapshot.records).is_err());
    e.snapshot
        .records
        .get_mut(&RecordId(10))
        .unwrap()
        .fields
        .insert("actors".into(), json!([]));
    let model = mbse_model::stage1_sysml::from_records(&e.snapshot.records).unwrap();
    let exported = mbse_model::stage1_sysml::export(&model).unwrap();
    let imported =
        mbse_model::stage1_sysml::import("copy.sysml", exported.utf8.as_bytes()).unwrap();
    assert_eq!(
        imported.semantics.records[&RecordId(20)].fields["context"],
        e.snapshot.records[&RecordId(20)].fields["context"]
    );
}
fn area() -> tempfile::TempDir {
    let root = std::env::var_os("TMPDIR").expect("explicit workspace TMPDIR");
    assert!(std::path::Path::new(&root).starts_with("/workspace"));
    tempfile::tempdir_in(root).unwrap()
}
#[test]
fn restart_inventory_refuses_corrupt_main_unknown_and_symlink_and_restores_copy() {
    let area = area();
    let state = area.path().join("state");
    let mut s = Session::new(&state).unwrap();
    let job = s.begin_recovery().unwrap();
    let path = job.ticket.destination.clone();
    assert!(s.complete(execute_write(job)));
    let expected = s.editor.snapshot.digest();
    drop(s);
    let bad = state.join("stage1-session-corrupt");
    std::fs::create_dir(&bad).unwrap();
    std::fs::write(bad.join("recovery.kernspan"), b"{truncated").unwrap();
    let foreign = state.join("stage1-session-foreign");
    std::fs::create_dir(&foreign).unwrap();
    std::fs::write(
        foreign.join("recovery.kernspan"),
        b"{\"profile\":\"foreign\"}",
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(path.parent().unwrap(), state.join("stage1-session-link")).unwrap();
    let inventory = discover_recoveries(&state).unwrap();
    assert_eq!(inventory.entries.len(), 1);
    assert_eq!(inventory.entries[0].snapshot_hash, expected);
    assert_eq!(inventory.entries[0].path, path);
    assert!(inventory.rejected.len() >= 2);
    let restored = Session::restore(&state, &path).unwrap();
    assert!(restored.is_dirty());
    assert!(restored.path.is_none());
}
#[test]
fn inventory_is_explicitly_bounded() {
    let area = area();
    let state = area.path().join("state");
    std::fs::create_dir(&state).unwrap();
    for i in 0..mbse_model::recovery::MAX_SCAN_ENTRIES + 1 {
        std::fs::create_dir(state.join(format!("unrelated-{i}"))).unwrap();
    }
    let inventory = discover_recoveries(&state).unwrap();
    assert!(inventory.truncated);
    assert!(inventory.entries.is_empty());
}
