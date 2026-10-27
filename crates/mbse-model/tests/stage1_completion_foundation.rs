use serde_json::{json, Value};
use std::collections::BTreeMap;
use mbse_model::stage1::{Command, DocumentId, Editor, Record, RecordId, RecordKind, Snapshot};
use mbse_model::stage1_io::{execute_write, SaveStatus, Session};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/stage1/inspection.json")).unwrap()
}
fn editor() -> Editor {
    Editor::new(
        Snapshot::from_fixture(DocumentId("completion-qa".into()), &fixture()).unwrap(),
        "instance-qa".into(),
    )
    .unwrap()
}

#[test]
fn measure_relink_is_one_undoable_edit_and_preserves_partial_criterion() {
    let mut e = editor();
    let raw = e.snapshot.drafts.clone();
    let before = e.freshness();
    let target = e
        .command(
            &before,
            Command::Add {
                kind: RecordKind::Objective,
                owner: RecordId(1),
                name: "Second objective".into(),
            },
        )
        .unwrap()
        .created
        .unwrap();
    let baseline = e.snapshot.records.clone();
    e.command(
        &e.freshness(),
        Command::SetForObjective {
            measure: RecordId(301),
            objective: target,
        },
    )
    .unwrap();
    assert_eq!(
        e.snapshot.records[&RecordId(301)].fields["forObjective"],
        json!(target.0)
    );
    assert_eq!(e.snapshot.drafts, raw);
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(e.snapshot.records, baseline);
    assert_eq!(e.snapshot.drafts, raw);
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert_eq!(
        e.snapshot.records[&RecordId(301)].fields["forObjective"],
        json!(target.0)
    );
    assert_eq!(e.snapshot.drafts, raw);
}

#[test]
fn wrong_kind_missing_and_cross_system_relinks_preserve_exact_document_and_history() {
    let mut e = editor();
    e.snapshot.records.insert(
        RecordId(2),
        Record {
            id: RecordId(2),
            kind: RecordKind::System,
            name: "Other system".into(),
            scope: RecordId(2),
            fields: BTreeMap::new(),
        },
    );
    e.snapshot.records.insert(
        RecordId(3),
        Record {
            id: RecordId(3),
            kind: RecordKind::Objective,
            name: "Other objective".into(),
            scope: RecordId(2),
            fields: BTreeMap::new(),
        },
    );
    e.snapshot.validate().unwrap();
    let snapshot = e.snapshot.clone();
    for target in [RecordId(200), RecordId(9999), RecordId(3)] {
        assert!(e
            .command(
                &e.freshness(),
                Command::SetForObjective {
                    measure: RecordId(301),
                    objective: target
                }
            )
            .is_err());
        assert_eq!(e.snapshot, snapshot);
        assert!(!e.can_undo());
        assert!(!e.can_redo());
    }
}

fn saved_v1(dir: &std::path::Path, extra: Option<(&str, Value)>) -> (std::path::PathBuf, Vec<u8>) {
    let state = dir.join("write-state");
    let path = dir.join("legacy.kernspan");
    let mut session = Session::from_fixture(&state, &fixture()).unwrap();
    let job = session.begin_save(&path).unwrap();
    let result = execute_write(job);
    assert!(result.succeeded());
    assert!(session.complete(result));
    let mut value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["profile"], "kernspan.stage1/2");
    assert_eq!(value["schema_version"], 2);
    if let Some((key, extra)) = extra {
        value["snapshot"]["records"]["301"]["fields"]
            .as_object_mut()
            .unwrap()
            .insert(key.into(), extra);
    }
    value["profile"] = "kernspan.stage1/1".into();
    value["schema_version"] = 1.into();
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    (path, bytes)
}

#[test]
fn actual_v1_open_preserves_unknown_properties_raw_and_native_saved_until_explicit_v2_write() {
    let dir = tempfile::tempdir().unwrap();
    let (path, original) = saved_v1(
        dir.path(),
        Some((
            "externalOpaque",
            json!({"nested":[0,false,null,"retained"]}),
        )),
    );
    let mut reopened = Session::open(&dir.path().join("read-state"), &path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(reopened.status(), SaveStatus::Saved);
    assert_eq!(
        reopened.editor.snapshot.records[&RecordId(301)].fields["externalOpaque"],
        json!({"nested":[0,false,null,"retained"]})
    );
    assert!(reopened
        .editor
        .snapshot
        .drafts
        .iter()
        .any(|d| d.text == "9e"));
    let snapshot = reopened.editor.snapshot.clone();
    let result = execute_write(reopened.begin_save(&path).unwrap());
    assert!(result.succeeded());
    assert!(reopened.complete(result));
    let written: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(written["profile"], "kernspan.stage1/2");
    assert_eq!(written["schema_version"], 2);
    let again = Session::open(&dir.path().join("second-read"), &path).unwrap();
    assert_eq!(again.editor.snapshot, snapshot);
}

#[test]
fn v1_reserved_typed_key_collisions_are_not_promoted_and_original_files_are_unchanged() {
    for key in [
        "calculation",
        "_kernspanSysmlSource",
        "_kernspanCalculationRawOwners",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (path, original) = saved_v1(dir.path(), Some((key, json!({"legacyOpaque":"kept"}))));
        let error = match Session::open(&dir.path().join("read-state"), &path) {
            Ok(_) => panic!("opaque legacy key was promoted"),
            Err(e) => e,
        };
        assert!(error.contains("V1_TYPED_KEY_COLLISION"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}

use mbse_model::stage1::{CreationDraft, RawKey, Validity};
use mbse_model::stage1_calc as calc;

fn definition(
    measure: RecordId,
    value: &str,
    kind: calc::QuantityKind,
    unit: calc::UnitId,
) -> calc::Definition {
    calc::Definition {
        id: calc::CalculationId(measure.0),
        owner: measure,
        bindings: BTreeMap::from([(
            "x".into(),
            calc::Input::Literal(calc::Literal {
                decimal: value.into(),
                quantity: calc::QuantityType { kind, unit },
            }),
        )]),
        expression: calc::parse("x", &calc::Limits::default()).unwrap(),
        output: calc::QuantityType { kind, unit },
    }
}
fn input_key() -> RawKey {
    RawKey {
        record: RecordId(301),
        field: "calc.input.x".into(),
        item: None,
    }
}
fn put(e: &mut Editor, def: Option<calc::Definition>) {
    e.command(
        &e.freshness(),
        Command::PutCalculation {
            measure: RecordId(301),
            definition: def,
        },
    )
    .unwrap();
}
fn edit(e: &mut Editor, key: &RawKey, value: &str) {
    e.command(
        &e.freshness(),
        Command::EditText {
            key: key.clone(),
            text: value.into(),
        },
    )
    .unwrap();
}
fn value(e: &Editor) -> f64 {
    match &e.evaluate_calculations().unwrap().outcomes[&calc::CalculationId(301)] {
        calc::Outcome::Value(v) => v.value,
        other => panic!("Expected value, got {other:?}"),
    }
}

#[test]
fn removed_and_recreated_binding_does_not_adopt_old_raw_and_model_undo_restores_exact_generation() {
    let mut e = editor();
    let key = input_key();
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "3",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    edit(&mut e, &key, "7e");
    let old = e.raw(&key).unwrap().clone();
    assert_eq!(old.validity, Validity::Incomplete);
    put(&mut e, None);
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "11",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    assert!(e.raw(&key).is_none());
    assert_eq!(e.display_text(&key), "11");
    assert_eq!(value(&e), 11.0);
    assert!(e.snapshot.drafts.contains(&old));
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert!(e.snapshot.calculations().unwrap().is_empty());
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(e.raw(&key), Some(&old));
    assert!(matches!(
        e.evaluate_calculations().unwrap().outcomes[&calc::CalculationId(301)],
        calc::Outcome::Failure(_)
    ));
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert!(e.raw(&key).is_none());
    assert_eq!(value(&e), 11.0);
    assert!(e.snapshot.drafts.contains(&old));
}

#[test]
fn text_undo_is_scoped_to_binding_generation_and_keeps_previous_journal() {
    let mut e = editor();
    let key = input_key();
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "3",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    edit(&mut e, &key, "4");
    let previous = e.raw(&key).unwrap().clone();
    put(&mut e, None);
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "11",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    assert!(!e.undo_text(&e.freshness(), &key).unwrap());
    assert_eq!(value(&e), 11.0);
    edit(&mut e, &key, "12");
    assert_eq!(value(&e), 12.0);
    assert!(e.undo_text(&e.freshness(), &key).unwrap());
    assert_eq!(value(&e), 11.0);
    assert!(e.snapshot.drafts.contains(&previous));
    assert_ne!(
        e.raw(&key).unwrap().owner_generation,
        previous.owner_generation
    );
}

#[test]
fn compatible_literal_unit_conversion_and_model_history_preserve_physical_value_and_raw_context() {
    let mut e = editor();
    let key = input_key();
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "1",
            calc::QuantityKind::Length,
            calc::UnitId::Kilometre,
        )),
    );
    edit(&mut e, &key, "2");
    let generation = e.raw(&key).unwrap().owner_generation;
    e.command(
        &e.freshness(),
        Command::SetCalculationInputUnit {
            measure: RecordId(301),
            binding: "x".into(),
            unit: calc::UnitId::Metre,
        },
    )
    .unwrap();
    assert_eq!(e.raw(&key).unwrap().unit_context.as_deref(), Some("Metre"));
    assert_eq!(e.raw(&key).unwrap().owner_generation, generation);
    assert_eq!(e.display_text(&key), "2000");
    assert_eq!(value(&e), 2.0);
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(e.display_text(&key), "2");
    assert_eq!(
        e.raw(&key).unwrap().unit_context.as_deref(),
        Some("Kilometre")
    );
    assert_eq!(value(&e), 2.0);
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert_eq!(e.display_text(&key), "2000");
    assert_eq!(value(&e), 2.0);
    edit(&mut e, &key, "2e");
    let before = e.snapshot.clone();
    assert!(e
        .command(
            &e.freshness(),
            Command::SetCalculationInputUnit {
                measure: RecordId(301),
                binding: "x".into(),
                unit: calc::UnitId::Kilometre
            }
        )
        .is_err());
    assert_eq!(e.snapshot, before);
}

#[test]
fn calculation_definition_and_incomplete_raw_survive_actual_save_reopen_and_independent_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("calculation.kernspan");
    let mut session = Session::from_fixture(&dir.path().join("writer"), &fixture()).unwrap();
    let key = input_key();
    put(
        &mut session.editor,
        Some(definition(
            RecordId(301),
            "3",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    edit(&mut session.editor, &key, "9e");
    let snapshot = session.editor.snapshot.clone();
    let result = execute_write(session.begin_save(&path).unwrap());
    assert!(result.succeeded());
    assert!(session.complete(result));
    let original = std::fs::read(&path).unwrap();
    let reopened = Session::open(&dir.path().join("reader"), &path).unwrap();
    assert_eq!(reopened.editor.snapshot, snapshot);
    assert!(!reopened.editor.can_undo());
    assert_eq!(reopened.editor.raw(&key).unwrap().text, "9e");
    assert!(matches!(
        reopened.editor.evaluate_calculations().unwrap().outcomes[&calc::CalculationId(301)],
        calc::Outcome::Failure(_)
    ));
    let result = execute_write(session.begin_recovery().unwrap());
    assert!(result.succeeded());
    assert!(session.complete(result));
    let recovered = Session::restore(
        &dir.path().join("recover"),
        session.last_recovery().unwrap(),
    )
    .unwrap();
    assert_eq!(recovered.editor.snapshot.records, snapshot.records);
    assert_eq!(recovered.editor.snapshot.drafts, snapshot.drafts);
    assert_ne!(recovered.editor.snapshot.document_id, snapshot.document_id);
    assert_eq!(recovered.status(), SaveStatus::Unsaved);
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn noncanonical_literal_spelling_returns_exactly_after_unit_model_undo_and_redo() {
    let mut e = editor();
    let key = input_key();
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "1",
            calc::QuantityKind::Length,
            calc::UnitId::Kilometre,
        )),
    );
    edit(&mut e, &key, "+002.00");
    let generation = e.raw(&key).unwrap().owner_generation;
    e.command(
        &e.freshness(),
        Command::SetCalculationInputUnit {
            measure: RecordId(301),
            binding: "x".into(),
            unit: calc::UnitId::Metre,
        },
    )
    .unwrap();
    assert_eq!(value(&e), 2.0);
    assert_eq!(e.display_text(&key), "2000");
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(value(&e), 2.0);
    assert_eq!(e.display_text(&key), "+002.00");
    assert_eq!(
        e.raw(&key).unwrap().unit_context.as_deref(),
        Some("Kilometre")
    );
    assert_eq!(e.raw(&key).unwrap().owner_generation, generation);
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert_eq!(value(&e), 2.0);
    assert_eq!(e.display_text(&key), "2000");
}

#[test]
fn unit_model_undo_never_overwrites_a_later_raw_value_with_restored_accepted_spelling() {
    let mut e = editor();
    let key = input_key();
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "1",
            calc::QuantityKind::Length,
            calc::UnitId::Kilometre,
        )),
    );
    edit(&mut e, &key, "+002.00");
    e.command(
        &e.freshness(),
        Command::SetCalculationInputUnit {
            measure: RecordId(301),
            binding: "x".into(),
            unit: calc::UnitId::Metre,
        },
    )
    .unwrap();
    edit(&mut e, &key, "3000");
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(e.display_text(&key), "3000");
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(e.display_text(&key), "3");
    assert_eq!(
        e.raw(&key).unwrap().unit_context.as_deref(),
        Some("Kilometre")
    );
    let def = e
        .snapshot
        .calculations()
        .unwrap()
        .remove(&calc::CalculationId(301))
        .unwrap();
    assert!(matches!(&def.bindings["x"],calc::Input::Literal(lit) if lit.decimal=="+002.00"));
    assert!(matches!(
        e.evaluate_calculations().unwrap().outcomes[&calc::CalculationId(301)],
        calc::Outcome::Failure(_)
    ));
}

#[test]
fn omitted_literal_context_on_open_is_captured_before_history_and_never_reinterpreted_after_conversion(
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("omitted-context.kernspan");
    let key = input_key();
    let mut s = Session::from_fixture(&dir.path().join("writer"), &fixture()).unwrap();
    put(
        &mut s.editor,
        Some(definition(
            RecordId(301),
            "1",
            calc::QuantityKind::Length,
            calc::UnitId::Kilometre,
        )),
    );
    s.editor
        .snapshot
        .drafts
        .push(mbse_model::stage1::RawDraft {
            key: key.clone(),
            text: "1".into(),
            version: s.editor.snapshot.raw_version,
            validity: Validity::Valid,
            unit_context: None,
            owner_generation: Some(s.editor.snapshot.raw_version),
        });
    s.editor.snapshot.validate().unwrap();
    let result = execute_write(s.begin_save(&path).unwrap());
    assert!(result.succeeded());
    assert!(s.complete(result));
    let original = std::fs::read(&path).unwrap();
    let mut opened = Session::open(&dir.path().join("reader"), &path).unwrap();
    let e = &mut opened.editor;
    assert!(e.raw(&key).unwrap().unit_context.is_none());
    assert_eq!(value(e), 1.0);
    edit(e, &key, "2");
    e.command(
        &e.freshness(),
        Command::SetCalculationInputUnit {
            measure: RecordId(301),
            binding: "x".into(),
            unit: calc::UnitId::Metre,
        },
    )
    .unwrap();
    assert!(e.undo_text(&e.freshness(), &key).unwrap());
    assert!(e.undo_text(&e.freshness(), &key).unwrap());
    assert_eq!(e.display_text(&key), "1000");
    assert_eq!(e.raw(&key).unwrap().unit_context.as_deref(), Some("Metre"));
    assert_eq!(value(e), 1.0);
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn rename_and_same_name_output_rebind_keep_old_drafts_inactive_and_undo_restores_their_owner() {
    let mut e = editor();
    let key = input_key();
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "4",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    edit(&mut e, &key, "9e");
    let old = e.raw(&key).unwrap().clone();
    let mut renamed = definition(
        RecordId(301),
        "5",
        calc::QuantityKind::Count,
        calc::UnitId::Count,
    );
    let input = renamed.bindings.remove("x").unwrap();
    renamed.bindings.insert("y".into(), input);
    renamed.expression = calc::parse("y", &calc::Limits::default()).unwrap();
    put(&mut e, Some(renamed));
    assert!(e.raw(&key).is_none());
    assert_eq!(value(&e), 5.0);
    assert!(e.snapshot.drafts.contains(&old));
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(e.raw(&key), Some(&old));
    assert!(e.redo_model(&e.freshness()).unwrap());
    let other = e
        .command(
            &e.freshness(),
            Command::Add {
                kind: RecordKind::Measure,
                owner: RecordId(300),
                name: "Explicit source output".into(),
            },
        )
        .unwrap()
        .created
        .unwrap();
    e.command(
        &e.freshness(),
        Command::PutCalculation {
            measure: other,
            definition: Some(definition(
                other,
                "8",
                calc::QuantityKind::Count,
                calc::UnitId::Count,
            )),
        },
    )
    .unwrap();
    let mut rebound = definition(
        RecordId(301),
        "4",
        calc::QuantityKind::Count,
        calc::UnitId::Count,
    );
    rebound.bindings.insert(
        "x".into(),
        calc::Input::Output(calc::CalculationId(other.0)),
    );
    put(&mut e, Some(rebound));
    assert!(e.raw(&key).is_none());
    assert_eq!(value(&e), 8.0);
    assert!(e.snapshot.drafts.contains(&old));
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "4",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    assert!(e.raw(&key).is_none());
    assert_eq!(value(&e), 4.0);
    assert!(e.snapshot.drafts.contains(&old));
}

#[test]
fn genuine_import_is_independent_dirty_and_exact_source_survives_native_recovery_generations() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("original.sysml");
    let bytes = include_bytes!("../fixtures/stage1-interchange/supported-minimal.sysml");
    std::fs::write(&source, bytes).unwrap();
    let mut imported = Session::import_sysml(&dir.path().join("import"), &source).unwrap();
    assert_eq!(imported.status(), SaveStatus::Unsaved);
    assert!(imported.path.is_none());
    assert!(!imported.editor.can_undo());
    assert!(imported.editor.snapshot.drafts.is_empty());
    let archive = imported
        .editor
        .snapshot
        .records
        .values()
        .find_map(|r| r.fields.get("_kernspanSysmlSource"))
        .unwrap()
        .clone();
    assert_eq!(archive["source_utf8"].as_str().unwrap().as_bytes(), bytes);
    assert!(imported
        .begin_export_sysml(&source)
        .unwrap_err()
        .contains("PROTECTED_SOURCE"));
    let native = dir.path().join("imported.kernspan");
    let result = execute_write(imported.begin_save(&native).unwrap());
    assert!(result.succeeded());
    assert!(imported.complete(result));
    let mut reopened = Session::open(&dir.path().join("reopened"), &native).unwrap();
    let recovery = execute_write(reopened.begin_recovery().unwrap());
    assert!(reopened.complete(recovery));
    let recovered = Session::restore(
        &dir.path().join("recovered"),
        reopened.last_recovery().unwrap(),
    )
    .unwrap();
    assert_eq!(
        recovered
            .editor
            .snapshot
            .records
            .values()
            .find_map(|r| r.fields.get("_kernspanSysmlSource")),
        Some(&archive)
    );
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
    let mut recovered = recovered;
    assert!(recovered.begin_export_sysml(&source).is_err());
    assert_ne!(
        imported.editor.snapshot.document_id,
        recovered.editor.snapshot.document_id
    );
}

#[test]
fn actual_export_has_separate_receipt_never_acknowledges_save_or_live_edits_and_refuses_existing_copy(
) {
    use mbse_model::stage1_io::execute_export;
    use mbse_model::stage1_sysml as exchange;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("copy.sysml");
    let mut session = Session::from_fixture(&dir.path().join("state"), &fixture()).unwrap();
    let original = session.editor.snapshot.clone();
    let expected =
        exchange::semantic_digest(&exchange::from_records(&original.records).unwrap()).unwrap();
    let job = session.begin_export_sysml(&source).unwrap();
    assert!(!session.pending());
    assert!(session.export_pending());
    assert!(session.last_export().is_none());
    edit(
        &mut session.editor,
        &RawKey {
            record: RecordId(1),
            field: "purpose".into(),
            item: None,
        },
        "Later purpose",
    );
    let live = session.editor.snapshot.clone();
    let result = execute_export(job);
    assert!(result.outcome().is_ok());
    let duplicate = result.clone();
    let mut other = Session::new(&dir.path().join("other")).unwrap();
    assert!(!other.complete_export(result.clone()).unwrap());
    assert!(session.complete_export(result).unwrap());
    assert!(!session.complete_export(duplicate).unwrap());
    assert_eq!(session.status(), SaveStatus::Unsaved);
    assert!(session.path.is_none());
    assert!(!session.recovery_current());
    let receipt = session.last_export().unwrap();
    assert_eq!(receipt.semantic_digest, expected);
    assert_eq!(receipt.ticket.snapshot_hash, original.digest());
    assert_ne!(
        receipt.ticket.snapshot_hash,
        session.editor.snapshot.digest()
    );
    let actual = std::fs::read(&source).unwrap();
    let parsed = exchange::import("copy.sysml", &actual).unwrap();
    assert_eq!(
        exchange::semantic_digest(&parsed.semantics).unwrap(),
        expected
    );
    assert_eq!(session.editor.snapshot, live);
    assert!(original
        .drafts
        .iter()
        .all(|draft| session.editor.snapshot.drafts.contains(draft)));
    assert!(session.begin_export_sysml(&source).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), actual);
}

#[test]
fn exported_copy_completion_does_not_consume_pending_native_save_and_disk_conflicts_never_approve_export(
) {
    use mbse_model::stage1_io::execute_export;
    let dir = tempfile::tempdir().unwrap();
    let native = dir.path().join("model.kernspan");
    let source = dir.path().join("copy.sysml");
    let mut session = Session::from_fixture(&dir.path().join("state"), &fixture()).unwrap();
    let save = session.begin_save(&native).unwrap();
    let native_ticket = session.pending_ticket().cloned().unwrap();
    let export = execute_export(session.begin_export_sysml(&source).unwrap());
    assert!(export.outcome().is_ok());
    std::fs::write(&source, b"externally changed").unwrap();
    assert!(session
        .complete_export(export)
        .unwrap_err()
        .contains("DISK_CONFLICT"));
    assert_eq!(session.pending_ticket(), Some(&native_ticket));
    assert!(session.last_export().is_none());
    assert!(session.complete(execute_write(save)));
    assert_eq!(session.status(), SaveStatus::Saved);
}

#[test]
fn unsupported_import_and_export_refuse_atomically_with_original_bytes_and_native_status_preserved()
{
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("unsupported.sysml");
    let original = include_bytes!("../fixtures/stage1-interchange/unsupported-behavior.sysml");
    std::fs::write(&source, original).unwrap();
    assert!(Session::import_sysml(&dir.path().join("state"), &source).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let mut session = Session::new(&dir.path().join("native")).unwrap();
    let native = dir.path().join("saved.kernspan");
    let result = execute_write(session.begin_save(&native).unwrap());
    assert!(session.complete(result));
    session
        .editor
        .snapshot
        .records
        .get_mut(&RecordId(1))
        .unwrap()
        .fields
        .insert("unsupportedMeaning".into(), json!("preserve"));
    let before = session.editor.snapshot.clone();
    let status = session.status();
    let target = dir.path().join("new.sysml");
    assert!(session.begin_export_sysml(&target).is_err());
    assert_eq!(session.editor.snapshot, before);
    assert_eq!(session.status(), status);
    assert!(!target.exists());
    assert!(!session.export_pending());
}

#[test]
fn desktop_import_refusal_retains_exact_bounded_invalid_bytes_independent_of_later_disk_change() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("rejected.sysml");
    let original = vec![0xff, 0x00, 0x22, 0x80];
    std::fs::write(&source, &original).unwrap();
    let failure = match Session::import_sysml_retained(&dir.path().join("state"), &source) {
        Ok(_) => panic!("invalid UTF8 import succeeded"),
        Err(failure) => failure,
    };
    std::fs::write(&source, b"later filesystem revision").unwrap();
    let asset = failure.source_asset.unwrap();
    assert_eq!(asset.original_bytes, Some(original.clone()));
    assert_eq!(asset.byte_len, original.len());
    assert!(failure.message.contains("Syntax"));
    assert_ne!(std::fs::read(&source).unwrap(), original);
}

fn config_key(id: &str, field: &str) -> RawKey {
    RawKey {
        record: RecordId(301),
        field: format!("{id}:{field}"),
        item: None,
    }
}

fn config_draft(e: &Editor, mode: &str, source: &str) -> CreationDraft {
    CreationDraft {
        owner: RecordId(301),
        kind: RecordKind::Measure,
        fields: BTreeMap::from([
            ("profile".into(), "kernspan.stage1.calc-config/1".into()),
            ("mode".into(), mode.into()),
            (
                "target_name".into(),
                if mode == "add" { "" } else { "x" }.into(),
            ),
            (
                "target_generation".into(),
                if mode == "add" {
                    String::new()
                } else {
                    e.snapshot
                        .calculation_binding_generation(RecordId(301), "x")
                        .unwrap()
                        .to_string()
                },
            ),
            ("name".into(), "unfinished_input".into()),
            ("decimal".into(), "1".into()),
            ("source_kind".into(), source.into()),
            ("kind".into(), "Length".into()),
            ("unit".into(), "Kilometre".into()),
            (
                "output_id".into(),
                if source == "output" { "9999" } else { "" }.into(),
            ),
            ("open".into(), "true".into()),
        ]),
    }
}

fn put_config(e: &mut Editor, id: &str, draft: CreationDraft) {
    e.command(
        &e.freshness(),
        Command::PutCreationDraft {
            id: id.into(),
            draft,
        },
    )
    .unwrap();
}

#[test]
fn unfinished_add_replace_rename_configuration_survives_actual_save_open_and_keep_draft_recovery() {
    for mode in ["add", "replace", "rename"] {
        for source in ["literal", "output"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("configuration.kernspan");
            let mut session =
                Session::from_fixture(&dir.path().join("writer"), &fixture()).unwrap();
            put(
                &mut session.editor,
                Some(definition(
                    RecordId(301),
                    "3",
                    calc::QuantityKind::Count,
                    calc::UnitId::Count,
                )),
            );
            let baseline_records = session.editor.snapshot.records.clone();
            let model_revision = session.editor.snapshot.model_revision;
            let id = session
                .editor
                .snapshot
                .calculation_config_id(RecordId(301))
                .unwrap();
            let draft = config_draft(&session.editor, mode, source);
            put_config(&mut session.editor, &id, draft);
            let name_key = config_key(&id, "name");
            let decimal_key = config_key(&id, "decimal");
            edit(&mut session.editor, &name_key, " unfinished name ");
            edit(&mut session.editor, &decimal_key, "9e");
            assert_eq!(
                session.editor.raw(&decimal_key).unwrap().owner_generation,
                None
            );
            assert_eq!(session.editor.raw(&decimal_key).unwrap().unit_context, None);
            // Validity describes retained scratch text, not a valid numeric input.
            assert_eq!(
                session.editor.raw(&decimal_key).unwrap().validity,
                Validity::Valid
            );
            assert_eq!(session.editor.snapshot.records, baseline_records);
            assert_eq!(session.editor.snapshot.model_revision, model_revision);
            assert_eq!(value(&session.editor), 3.0);
            let snapshot = session.editor.snapshot.clone();
            let result = execute_write(session.begin_save(&path).unwrap());
            assert!(result.succeeded());
            assert!(session.complete(result));
            let original = std::fs::read(&path).unwrap();
            let mut reopened = Session::open(&dir.path().join("reader"), &path).unwrap();
            assert_eq!(reopened.editor.snapshot, snapshot);
            assert_eq!(reopened.editor.display_text(&name_key), " unfinished name ");
            assert_eq!(reopened.editor.display_text(&decimal_key), "9e");
            assert!(!reopened.editor.can_undo());
            assert!(!reopened
                .editor
                .undo_text(&reopened.editor.freshness(), &decimal_key)
                .unwrap());
            assert_eq!(reopened.editor.snapshot, snapshot);
            // Cancel closes only the persisted panel; its unsubmitted text/options stay.
            let mut closed = reopened.editor.snapshot.creation_drafts[&id].clone();
            closed.fields.insert("open".into(), "false".into());
            put_config(&mut reopened.editor, &id, closed);
            let retained = reopened.editor.snapshot.clone();
            assert_eq!(reopened.status(), SaveStatus::Unsaved);
            let result = execute_write(reopened.begin_recovery().unwrap());
            assert!(result.succeeded());
            assert!(reopened.complete(result));
            let recovered = Session::restore(
                &dir.path().join("recover"),
                reopened.last_recovery().unwrap(),
            )
            .unwrap();
            assert_ne!(recovered.editor.snapshot.document_id, retained.document_id);
            assert_eq!(recovered.editor.snapshot.records, baseline_records);
            assert_eq!(recovered.editor.snapshot.drafts, retained.drafts);
            assert_eq!(
                recovered.editor.snapshot.creation_drafts,
                retained.creation_drafts
            );
            let fields = &recovered.editor.snapshot.creation_drafts[&id].fields;
            assert_eq!(fields["mode"], mode);
            assert_eq!(fields["source_kind"], source);
            assert_eq!(fields["kind"], "Length");
            assert_eq!(fields["unit"], "Kilometre");
            assert_eq!(fields["name"], " unfinished name ");
            assert_eq!(fields["decimal"], "9e");
            assert_eq!(fields["open"], "false");
            assert_eq!(
                fields["output_id"],
                if source == "output" { "9999" } else { "" }
            );
            assert_eq!(value(&recovered.editor), 3.0);
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
    }
}

#[test]
fn configuration_text_fallback_and_key_scoped_undo_redo_do_not_change_accepted_model() {
    let mut e = editor();
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "3",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    let id = e.snapshot.calculation_config_id(RecordId(301)).unwrap();
    let draft = config_draft(&e, "replace", "literal");
    put_config(&mut e, &id, draft);
    let baseline_records = e.snapshot.records.clone();
    let revision = e.snapshot.model_revision;
    let name = config_key(&id, "name");
    let decimal = config_key(&id, "decimal");
    assert_eq!(e.display_text(&name), "unfinished_input");
    assert_eq!(e.display_text(&decimal), "1");
    assert!(e.raw(&name).is_none());
    edit(&mut e, &decimal, "9");
    edit(&mut e, &decimal, "9e");
    edit(&mut e, &name, "renamed");
    let version = e.snapshot.raw_version;
    assert!(e.undo_text(&e.freshness(), &decimal).unwrap());
    assert_eq!(e.display_text(&decimal), "9");
    assert_eq!(e.display_text(&name), "renamed");
    assert_eq!(e.snapshot.creation_drafts[&id].fields["decimal"], "9");
    assert!(e.undo_text(&e.freshness(), &decimal).unwrap());
    assert_eq!(e.display_text(&decimal), "1");
    assert!(e.redo_text(&e.freshness(), &decimal).unwrap());
    assert!(e.redo_text(&e.freshness(), &decimal).unwrap());
    assert_eq!(e.display_text(&decimal), "9e");
    assert!(e.undo_text(&e.freshness(), &name).unwrap());
    assert_eq!(e.display_text(&name), "unfinished_input");
    assert!(e.redo_text(&e.freshness(), &name).unwrap());
    let mut options = e.snapshot.creation_drafts[&id].clone();
    options.fields.insert("source_kind".into(), "output".into());
    options.fields.insert("output_id".into(), "9999".into());
    options.fields.insert("open".into(), "false".into());
    put_config(&mut e, &id, options);
    assert_eq!(e.display_text(&decimal), "9e");
    assert_eq!(e.snapshot.creation_drafts[&id].fields["name"], "renamed");
    assert!(e.snapshot.raw_version > version);
    assert_eq!(e.snapshot.model_revision, revision);
    assert_eq!(e.snapshot.records, baseline_records);
    assert_eq!(value(&e), 3.0);
    e.snapshot.validate().unwrap();
}

#[test]
fn configuration_generation_removal_recreate_and_model_history_never_adopt_old_scratch() {
    let mut e = editor();
    assert_eq!(e.snapshot.calculation_config_id(RecordId(301)), None);
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "3",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    let old_id = e.snapshot.calculation_config_id(RecordId(301)).unwrap();
    let old_draft = config_draft(&e, "add", "literal");
    put_config(&mut e, &old_id, old_draft.clone());
    let old_key = config_key(&old_id, "decimal");
    edit(&mut e, &old_key, "9e");
    let old_raw = e.raw(&old_key).unwrap().clone();
    put(&mut e, None);
    assert_eq!(e.snapshot.calculation_config_id(RecordId(301)), None);
    assert!(e.raw(&old_key).is_none());
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "11",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    let new_id = e.snapshot.calculation_config_id(RecordId(301)).unwrap();
    assert_ne!(new_id, old_id);
    let new_key = config_key(&new_id, "decimal");
    let new_draft = config_draft(&e, "add", "output");
    put_config(&mut e, &new_id, new_draft);
    edit(&mut e, &new_key, "2");
    let retained = e.snapshot.clone();
    assert!(e
        .command(
            &e.freshness(),
            Command::EditText {
                key: old_key.clone(),
                text: "adopted".into()
            }
        )
        .is_err());
    assert!(e.undo_text(&e.freshness(), &old_key).is_err());
    assert!(e
        .command(
            &e.freshness(),
            Command::PutCreationDraft {
                id: old_id.clone(),
                draft: old_draft
            }
        )
        .is_err());
    assert_eq!(e.snapshot, retained);
    assert!(e.undo_text(&e.freshness(), &new_key).unwrap());
    assert_eq!(e.display_text(&new_key), "1");
    assert!(e.redo_text(&e.freshness(), &new_key).unwrap());
    let version = e.snapshot.raw_version;
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(e.snapshot.calculation_config_id(RecordId(301)), None);
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(
        e.snapshot.calculation_config_id(RecordId(301)),
        Some(old_id.clone())
    );
    assert_eq!(e.display_text(&old_key), "9e");
    assert_eq!(e.raw(&old_key), Some(&old_raw));
    assert!(e.raw(&new_key).is_none());
    assert_eq!(value(&e), 3.0);
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert_eq!(
        e.snapshot.calculation_config_id(RecordId(301)),
        Some(new_id)
    );
    assert_eq!(e.display_text(&new_key), "2");
    assert!(e.snapshot.drafts.contains(&old_raw));
    assert_eq!(e.snapshot.creation_drafts[&old_id].fields["decimal"], "9e");
    assert!(e.snapshot.raw_version >= version);
    let mut reopened = Editor::new(e.snapshot.clone(), "replacement".into()).unwrap();
    assert!(!reopened.undo_text(&reopened.freshness(), &new_key).unwrap());
    assert_eq!(reopened.display_text(&new_key), "2");
}

#[test]
fn configuration_collisions_and_noncurrent_field_bindings_refuse_without_mutation() {
    let mut e = editor();
    put(
        &mut e,
        Some(definition(
            RecordId(301),
            "3",
            calc::QuantityKind::Count,
            calc::UnitId::Count,
        )),
    );
    let id = e.snapshot.calculation_config_id(RecordId(301)).unwrap();
    let draft = config_draft(&e, "add", "literal");
    for mismatch in ["profile", "kind", "owner"] {
        let mut collided = e.clone();
        let mut opaque = draft.clone();
        match mismatch {
            "profile" => {
                opaque
                    .fields
                    .insert("profile".into(), "legacy.opaque/1".into());
            }
            "kind" => opaque.kind = RecordKind::Objective,
            _ => opaque.owner = RecordId(300),
        }
        collided.snapshot.creation_drafts.insert(id.clone(), opaque);
        let before = collided.snapshot.clone();
        assert!(collided
            .command(
                &collided.freshness(),
                Command::PutCreationDraft {
                    id: id.clone(),
                    draft: draft.clone()
                }
            )
            .is_err());
        assert!(collided
            .command(
                &collided.freshness(),
                Command::EditText {
                    key: config_key(&id, "decimal"),
                    text: "9e".into()
                }
            )
            .is_err());
        assert_eq!(collided.snapshot, before);
        assert!(collided.can_undo());
    }
    put_config(&mut e, &id, draft);
    let before = e.snapshot.clone();
    for key in [
        config_key(&id, "unit"),
        RawKey {
            record: RecordId(300),
            field: format!("{id}:name"),
            item: None,
        },
        RawKey {
            record: RecordId(301),
            field: format!("{id}:name"),
            item: Some(RecordId(301)),
        },
        config_key(&format!("{id}0"), "decimal"),
    ] {
        assert!(e
            .command(
                &e.freshness(),
                Command::EditText {
                    key,
                    text: "changed".into()
                }
            )
            .is_err());
        assert_eq!(e.snapshot, before);
    }
    edit(&mut e, &config_key(&id, "decimal"), "9e");
    let mut forged = e.snapshot.clone();
    let raw = forged
        .drafts
        .iter_mut()
        .find(|raw| raw.key == config_key(&id, "decimal"))
        .unwrap();
    raw.validity = Validity::Incomplete;
    assert!(forged.validate().is_err());
    let mut forged = e.snapshot.clone();
    let raw = forged
        .drafts
        .iter_mut()
        .find(|raw| raw.key == config_key(&id, "decimal"))
        .unwrap();
    raw.unit_context = Some("Kilometre".into());
    assert!(forged.validate().is_err());
}

#[test]
fn binding_generation_query_covers_output_targets_and_imported_zero_without_literal_raw_fields() {
    let mut e = editor();
    let literal = definition(
        RecordId(301),
        "3",
        calc::QuantityKind::Count,
        calc::UnitId::Count,
    );
    e.snapshot
        .records
        .get_mut(&RecordId(301))
        .unwrap()
        .fields
        .insert(
            "calculation".into(),
            serde_json::to_value(&literal).unwrap(),
        );
    e.snapshot.validate().unwrap();
    assert_eq!(
        e.snapshot.calculation_config_id(RecordId(301)).as_deref(),
        Some("calc-config:301:0")
    );
    assert_eq!(
        e.snapshot
            .calculation_binding_generation(RecordId(301), "x"),
        Some(0)
    );
    assert_eq!(
        e.snapshot
            .calculation_binding_generation(RecordId(301), "missing"),
        None
    );
    assert_eq!(
        e.snapshot
            .calculation_binding_generation(RecordId(300), "x"),
        None
    );
    let mut output = literal.clone();
    output
        .bindings
        .insert("x".into(), calc::Input::Output(calc::CalculationId(301)));
    put(&mut e, Some(output.clone()));
    let generation = e
        .snapshot
        .calculation_binding_generation(RecordId(301), "x")
        .unwrap();
    assert!(generation > 0);
    assert!(e.raw(&input_key()).is_none());
    assert!(e
        .command(
            &e.freshness(),
            Command::EditText {
                key: input_key(),
                text: "9e".into()
            }
        )
        .is_err());
    assert_eq!(
        e.snapshot.calculation_config_id(RecordId(301)).as_deref(),
        Some("calc-config:301:0")
    );
    output
        .bindings
        .insert("x".into(), calc::Input::Output(calc::CalculationId(9999)));
    put(&mut e, Some(output));
    let rebound = e
        .snapshot
        .calculation_binding_generation(RecordId(301), "x")
        .unwrap();
    assert_ne!(rebound, generation);
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(
        e.snapshot
            .calculation_binding_generation(RecordId(301), "x"),
        Some(generation)
    );
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert_eq!(
        e.snapshot
            .calculation_binding_generation(RecordId(301), "x"),
        Some(rebound)
    );
}

#[test]
fn schema1_configuration_journal_prefix_collisions_refuse_migration_unchanged() {
    for raw_collision in [false, true] {
        let area = tempfile::tempdir().unwrap();
        let (path, bytes) = saved_v1(area.path(), None);
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        if raw_collision {
            value["snapshot"]["raw_version"] = 2.into();
            value["snapshot"]["drafts"].as_array_mut().unwrap().push(json!({"key":{"record":301,"field":"calc-config:301:0:decimal","item":null},"text":"legacy9e","version":2,"validity":"Valid","unit_context":null}));
        } else {
            value["snapshot"]["creation_drafts"]["calc-config:301:0"] = json!({"owner":301,"kind":"Measure","fields":{"profile":"opaque-legacy","name":"retain exact"}});
        }
        let original = serde_json::to_vec_pretty(&value).unwrap();
        std::fs::write(&path, &original).unwrap();
        let failure = match Session::open(&area.path().join("read"), &path) {
            Ok(_) => panic!("legacy configuration collision was promoted"),
            Err(failure) => failure,
        };
        assert!(failure.contains("V1_TYPED_KEY_COLLISION"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}

// Exercise the retained source identity helper on owned fixture trees.
// This test-only helper does not register the original project build script.
#[allow(dead_code)]
#[path = "support/source_identity.rs"]
mod source_build;

#[test]
fn source_identity_binds_vendor_contents_membership_and_directory_watches() {
    use std::{fs, path::PathBuf};
    let area = tempfile::tempdir().unwrap();
    let root = area.path();
    fs::create_dir_all(root.join("vendor/accesskit/src")).unwrap();
    fs::create_dir_all(root.join("vendor/accesskit_windows/src")).unwrap();
    fs::write(root.join("Cargo.toml"), "fixture manifest").unwrap();
    fs::write(
        root.join("vendor/accesskit/src/lib.rs"),
        "initial common code",
    )
    .unwrap();
    fs::write(
        root.join("vendor/accesskit_windows/src/lib.rs"),
        "provider code",
    )
    .unwrap();
    fs::write(root.join("vendor/accesskit/LICENSE-MIT"), "retained notice").unwrap();
    let collect = || {
        source_build::source_inputs(root, &[PathBuf::from("Cargo.toml")], &[("vendor", true)])
            .unwrap()
    };
    let initial = collect();
    for directory in [
        "vendor",
        "vendor/accesskit",
        "vendor/accesskit/src",
        "vendor/accesskit_windows",
        "vendor/accesskit_windows/src",
    ] {
        assert!(
            initial.watches.iter().any(|path| path.ends_with(directory)),
            "new members under {directory} need a rebuild watch"
        );
    }
    let baseline = source_build::source_id(root, &initial).unwrap();
    fs::write(
        root.join("vendor/accesskit/src/lib.rs"),
        "patched common code",
    )
    .unwrap();
    let changed = source_build::source_id(root, &collect()).unwrap();
    assert_ne!(
        baseline, changed,
        "vendor code bytes must bind the compiled identity"
    );
    fs::write(
        root.join("vendor/accesskit/LICENSE-ADDITIONAL"),
        "additional retained notice",
    )
    .unwrap();
    let added = collect();
    assert!(added
        .files
        .iter()
        .any(|path| path.ends_with("vendor/accesskit/LICENSE-ADDITIONAL")));
    assert_ne!(
        changed,
        source_build::source_id(root, &added).unwrap(),
        "non-Rust vendor members must bind the identity too"
    );
    fs::remove_file(root.join("vendor/accesskit/LICENSE-ADDITIONAL")).unwrap();
    assert_eq!(
        changed,
        source_build::source_id(root, &collect()).unwrap(),
        "removing the same member must restore the prior inputs"
    );
    fs::write(root.join("vendor/accesskit/LICENSE-MIT"), "changed notice").unwrap();
    assert_ne!(
        changed,
        source_build::source_id(root, &collect()).unwrap(),
        "retained license contents must also bind the identity"
    );
}

#[test]
fn source_identity_is_portable_across_workspace_roots_and_creation_order() {
    use std::{fs, path::PathBuf};
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let files = [
        ("Cargo.toml", "fixture manifest"),
        ("vendor/accesskit/src/lib.rs", "common code"),
        ("vendor/accesskit/LICENSE-MIT", "retained notice"),
        ("vendor/accesskit_windows/src/lib.rs", "provider code"),
    ];
    for (root, reverse) in [(first.path(), false), (second.path(), true)] {
        let mut ordered = files.to_vec();
        if reverse {
            ordered.reverse();
        }
        for (name, bytes) in ordered {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }
    let identity = |root| {
        let inputs =
            source_build::source_inputs(root, &[PathBuf::from("Cargo.toml")], &[("vendor", true)])
                .unwrap();
        source_build::source_id(root, &inputs).unwrap()
    };
    assert_eq!(
        identity(first.path()),
        identity(second.path()),
        "equal relative source bytes must give the same cross-platform source identity"
    );
}
