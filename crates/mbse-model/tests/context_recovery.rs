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
    let area = area();
    let state = area.path().join("state");
    let native = area.path().join("contexts.kernspan");
    let mut session = Session::new(&state).unwrap();
    session.editor = e.clone();
    let job = session.begin_save(&native).unwrap();
    assert!(session.complete(execute_write(job)));
    let reopened = Session::open(&state, &native).unwrap();
    assert_eq!(reopened.editor.snapshot.records, e.snapshot.records);
    assert_eq!(reopened.editor.snapshot.drafts, e.snapshot.drafts);
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

#[test]
fn recovery_inventory_refuses_file_symlink_and_native_save_envelope() {
    let area = area();
    let state = area.path().join("state");
    let mut session = Session::new(&state).unwrap();
    let native = area.path().join("native.kernspan");
    let job = session.begin_save(&native).unwrap();
    assert!(session.complete(execute_write(job)));
    let copied = state.join("stage1-session-save");
    std::fs::create_dir(&copied).unwrap();
    let copied_path = copied.join("recovery.kernspan");
    std::fs::copy(&native, &copied_path).unwrap();
    #[cfg(unix)]
    let link_path = {
        let dir = state.join("stage1-session-file-link");
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("recovery.kernspan");
        std::os::unix::fs::symlink(&native, &path).unwrap();
        path
    };
    let inventory = discover_recoveries(&state).unwrap();
    assert!(inventory.entries.is_empty());
    assert!(inventory
        .rejected
        .iter()
        .any(|r| r.path == copied_path && r.reason == "STAGE1_NOT_RECOVERY"));
    #[cfg(unix)]
    assert!(inventory
        .rejected
        .iter()
        .any(|r| r.path == link_path && r.reason == "RECOVERY_UNSAFE_FILE"));
}

#[test]
fn recovery_inventory_file_count_and_aggregate_byte_caps_are_independent() {
    let area = area();
    let count_root = area.path().join("count");
    std::fs::create_dir(&count_root).unwrap();
    for i in 0..=mbse_model::recovery::MAX_RECOVERY_FILES {
        let dir = count_root.join(format!("stage1-session-{i}"));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("recovery.kernspan"), b"{").unwrap();
    }
    let count = discover_recoveries(&count_root).unwrap();
    assert!(count.truncated);
    assert_eq!(
        count.rejected.len(),
        mbse_model::recovery::MAX_RECOVERY_FILES
    );
    assert!(count.entries.is_empty());
    let bytes_root = area.path().join("bytes");
    std::fs::create_dir(&bytes_root).unwrap();
    let per_file = mbse_model::stage1_io::MAX_BYTES as u64;
    let admitted_count = mbse_model::recovery::MAX_TOTAL_BYTES / per_file;
    for i in 0..=admitted_count {
        let dir = bytes_root.join(format!("stage1-session-{i}"));
        std::fs::create_dir(&dir).unwrap();
        // Sparse invalid files exercise actual acquisition/decode without large fixture blobs.
        let file = std::fs::File::create(dir.join("recovery.kernspan")).unwrap();
        file.set_len(per_file).unwrap();
    }
    let bytes = discover_recoveries(&bytes_root).unwrap();
    assert!(bytes.truncated);
    assert_eq!(bytes.rejected.len(), admitted_count as usize);
    assert!(bytes.entries.is_empty());
}

#[test]
fn native_context_name_retains_unsupported_exchange_text_through_undo_and_reopen() {
    let mut e = editor();
    // Remove unrelated unsupported actors so the exchange refusal has one cause.
    e.snapshot
        .records
        .get_mut(&RecordId(10))
        .unwrap()
        .fields
        .insert("actors".into(), json!([]));
    let id = e
        .command(
            &e.freshness(),
            Command::AddContext {
                environment: RecordId(20),
                name: "Original context".into(),
                kind: ContextKind::Description,
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
    let text = "Retained\u{0001}name";
    e.command(
        &e.freshness(),
        Command::EditText {
            key: key.clone(),
            text: text.into(),
        },
    )
    .unwrap();
    assert_eq!(e.display_text(&key), text);
    assert_eq!(e.raw(&key).unwrap().text, text);
    assert!(e.undo_text(&e.freshness(), &key).unwrap());
    assert_eq!(e.display_text(&key), "Original context");
    assert!(e.redo_text(&e.freshness(), &key).unwrap());
    assert_eq!(e.display_text(&key), text);
    let before = e.snapshot.clone();
    let wrong = RawKey {
        record: RecordId(11),
        ..key.clone()
    };
    assert!(e
        .command(
            &e.freshness(),
            Command::EditText {
                key: wrong,
                text: "wrong owner".into()
            }
        )
        .is_err());
    assert_eq!(e.snapshot, before);
    let refusal = mbse_model::stage1_sysml::from_records(&e.snapshot.records).unwrap_err();
    assert_eq!(
        refusal.code,
        mbse_model::stage1_sysml::ExchangeCode::UnsupportedField
    );
    assert!(refusal.message.contains("control character"));
    let area = area();
    let state = area.path().join("state");
    let native = area.path().join("name.kernspan");
    let mut session = Session::new(&state).unwrap();
    session.editor = e;
    let job = session.begin_save(&native).unwrap();
    assert!(session.complete(execute_write(job)));
    let reopened = Session::open(&state, &native).unwrap();
    assert_eq!(reopened.editor.display_text(&key), text);
    assert_eq!(
        reopened.editor.snapshot.records[&RecordId(20)].fields["context"][0]["name"],
        text
    );
    assert_eq!(reopened.editor.raw(&key).unwrap().text, text);
}
