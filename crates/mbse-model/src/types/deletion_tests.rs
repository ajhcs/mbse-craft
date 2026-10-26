use super::*;
use crate::{
    native_views::{Camera, Geometry, Layout, NativeAction, Views, connectors as vc, ports as vp},
    native_views_io::{Io, Kind, accepted_native},
    stage1::{Command, Editor},
    stage1_io::{Session, execute_write},
};
use deletion::{Action as DA, Form};
fn input() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/parity/p21a/fixtures/refactored.kernspan")
}
fn snapshot() -> Snapshot {
    let v: Value = serde_json::from_slice(&std::fs::read(input()).unwrap()).unwrap();
    serde_json::from_value(v["snapshot"].clone()).unwrap()
}
fn editor(clean: bool) -> Editor {
    let mut s = snapshot();
    if clean {
        s.records
            .get_mut(&RecordId(1))
            .unwrap()
            .fields
            .remove("opaque.p14.future");
    }
    Editor::new(s, "delete:test".into()).unwrap()
}
fn command(v: &mut Views, e: &mut Editor, a: NativeAction) -> Result<(), String> {
    v.dispatch(e, &v.stamp(e), a).map(|_| ())
}
fn form(v: &mut Views, e: &mut Editor, ids: &str, cascade: bool) {
    command(
        v,
        e,
        NativeAction::Command(Command::Typed(Action::Deletion(DA::SetForm(Form {
            ids: ids.into(),
            cascade,
        })))),
    )
    .unwrap();
}
fn prepared(v: &mut Views, e: &mut Editor, ids: &str) -> deletion::Prepared {
    form(v, e, ids, true);
    deletion::prepare(e, v).unwrap()
}
fn apply(v: &mut Views, e: &mut Editor, p: &deletion::Prepared) -> Result<(), String> {
    command(
        v,
        e,
        NativeAction::Command(Command::Typed(Action::Deletion(DA::Apply(Box::new(
            p.plan().unwrap().clone(),
        ))))),
    )
}
fn clone_views(e: &Editor) -> Views {
    let mut v = Views::new(e).unwrap();
    let views: Vec<_> = v.document().views.keys().copied().collect();
    for view in views {
        let mut ids = Vec::new();
        for (usage, port, x) in [(1023, 1017, 20.), (1024, 1015, 410.)] {
            let id = serde_json::from_value(serde_json::json!(v.document().next_id().to_string()))
                .unwrap();
            v.apply(
                e,
                &v.stamp(e),
                Layout::Port(vp::Layout::ReuseInUsage {
                    view,
                    usage: Id(usage),
                    port: Id(port),
                    geometry: Geometry::at(x, 270.),
                }),
            )
            .unwrap();
            ids.push(id);
        }
        v.apply(
            e,
            &v.stamp(e),
            Layout::Connector(vc::Layout::Route {
                view,
                connector: Id(1025),
                source: ids[0],
                target: ids[1],
            }),
        )
        .unwrap();
    }
    v
}
#[test]
fn original_unknown_metadata_and_unknown_raw_or_namespace_block_without_mutation() {
    let mut e = editor(false);
    let mut v = Views::new(&e).unwrap();
    let p = prepared(&mut v, &mut e, "1021");
    assert!(p.plan().is_none());
    assert!(!p.summary().complete_supported_reference_inventory);
    assert!(
        p.summary()
            .unknown
            .locations
            .iter()
            .any(|s| s.contains("opaque.p14.future"))
    );
    let before = e.snapshot.clone();
    drop(p);
    assert_eq!(e.snapshot, before);
    for case in 0..3 {
        let mut e = editor(true);
        match case {
            0 => {
                e.snapshot
                    .extensions
                    .as_mut()
                    .unwrap()
                    .namespaces
                    .insert("opaque.future".into(), serde_json::json!({"target":1021}));
            }
            1 => {
                let raw = e.snapshot.drafts.first_mut().unwrap();
                raw.key.field = "unknown.reference".into();
            }
            _ => {
                e.snapshot
                    .creation_drafts
                    .values_mut()
                    .next()
                    .unwrap()
                    .fields
                    .insert("unknown.reference".into(), "1021".into());
            }
        }
        e.snapshot.validate().unwrap();
        let mut v = Views::new(&e).unwrap();
        let p = prepared(&mut v, &mut e, "1021");
        assert!(p.plan().is_none());
        assert!(p.summary().unknown.count > 0);
    }
}
#[test]
fn clone_delete_cascades_only_owned_records_and_values_preserves_raw_forms_and_allocator() {
    let mut e = editor(true);
    let mut v = Views::new(&e).unwrap();
    let p = prepared(&mut v, &mut e, "1021");
    assert_eq!(
        p.summary()
            .removed
            .iter()
            .map(|r| r.id.0)
            .collect::<Vec<_>>(),
        vec![1021, 1022, 1023, 1024, 1025, 1026]
    );
    assert_eq!(p.summary().removed_values.len(), 1);
    assert_eq!(p.summary().removed_values[0].usage, Some(Id(1023)));
    let before = e.snapshot.clone();
    let old = load(&before).unwrap();
    let j = journal(&before).unwrap();
    assert!(p.summary().complete_supported_reference_inventory);
    apply(&mut v, &mut e, &p).unwrap();
    let s = load(&e.snapshot).unwrap();
    assert!(!s.definitions.contains_key(&Id(1021)));
    assert!(!s.overrides.contains_key(&Id(1023)));
    assert_eq!(s.overrides[&Id(1009)], old.overrides[&Id(1009)]);
    assert_eq!(s.defaults, old.defaults);
    assert_eq!(s.connectors[&Id(1018)], old.connectors[&Id(1018)]);
    assert_eq!(e.snapshot.records, before.records);
    assert_eq!(e.snapshot.drafts, before.drafts);
    assert_eq!(journal(&e.snapshot).unwrap(), j);
    assert_eq!(e.snapshot.next_id, 1027);
}
#[test]
fn payload_delete_closes_all_accepted_incoming_dependencies_without_deleting_part_targets() {
    let mut e = editor(true);
    let mut v = Views::new(&e).unwrap();
    let p = prepared(&mut v, &mut e, "1005");
    assert!(p.plan().is_some(), "{:?}", p.summary());
    assert!(p.summary().removed.iter().any(|r| r.id == Id(1020)));
    apply(&mut v, &mut e, &p).unwrap();
    let s = load(&e.snapshot).unwrap();
    assert!(s.properties.is_empty());
    assert!(s.interfaces.is_empty());
    assert!(s.ports.is_empty());
    assert!(s.connectors.is_empty());
    assert!(s.defaults.is_empty());
    assert!(s.overrides.is_empty());
    assert!(s.usages.contains_key(&Id(1009)));
    assert!(s.definitions.contains_key(&Id(1007)));
    s.validate().unwrap();
}
#[test]
fn deleting_authored_property_removes_defaults_overrides_and_retains_every_part() {
    let mut e = editor(true);
    let mut v = Views::new(&e).unwrap();
    let p = prepared(&mut v, &mut e, "1020");
    assert_eq!(p.summary().removed.len(), 1);
    assert_eq!(p.summary().removed_values.len(), 3);
    let before = load(&e.snapshot).unwrap();
    apply(&mut v, &mut e, &p).unwrap();
    let s = load(&e.snapshot).unwrap();
    assert!(s.defaults.is_empty());
    assert!(s.overrides.is_empty());
    assert_eq!(s.usages, before.usages);
    assert_eq!(s.connectors, before.connectors);
}
#[test]
fn policy_noncanonical_duplicate_missing_native_and_review_budget_refuse_atomically() {
    let mut e = editor(true);
    let mut v = Views::new(&e).unwrap();
    form(&mut v, &mut e, "1021", false);
    assert!(deletion::prepare(&e, &v).unwrap().plan().is_none());
    for ids in ["1021,1021", "01021", "1021,", "0", "9999", "1003"] {
        form(&mut v, &mut e, ids, true);
        let before = e.snapshot.clone();
        assert!(deletion::prepare(&e, &v).is_err(), "{ids}");
        assert_eq!(e.snapshot, before);
    }
    let mut s = load(&e.snapshot).unwrap();
    for _ in 0..129 {
        let id = allocate(&mut e.snapshot).unwrap();
        s.properties.insert(
            id,
            Property {
                id,
                owner: Id(1007),
                name: format!("p{}", id.0),
                value_type: Id(1005),
                multiplicity: Multiplicity {
                    lower: 1,
                    upper: Some(1),
                },
            },
        );
    }
    put(&mut e.snapshot, NAMESPACE, &s).unwrap();
    let mut v = Views::new(&e).unwrap();
    let p = prepared(&mut v, &mut e, "1007");
    assert!(p.plan().is_none());
    assert!(
        p.summary()
            .refusal
            .as_ref()
            .unwrap()
            .contains("review limit")
    );
}
#[test]
fn sealed_preview_requires_coupled_origin_and_rejects_model_raw_or_presentation_changes() {
    let mut e = editor(true);
    let mut v = Views::new(&e).unwrap();
    let p = prepared(&mut v, &mut e, "1021");
    let before = e.snapshot.clone();
    let action = Action::Deletion(DA::Apply(Box::new(p.plan().unwrap().clone())));
    assert!(
        e.command(&e.freshness(), Command::Typed(action.clone()))
            .unwrap_err()
            .contains("COUPLED")
    );
    let mut pure = before.clone();
    assert!(super::apply(&mut pure, action).is_err());
    assert_eq!(pure, before);
    let mut foreign = Views::new(&e).unwrap();
    assert!(apply(&mut foreign, &mut e, &p).is_err());
    assert_eq!(e.snapshot, before);
    let view = v.document().primary;
    v.apply(
        &e,
        &v.stamp(&e),
        Layout::Camera(view, Camera::checked(10., 20., 1.).unwrap()),
    )
    .unwrap();
    assert!(apply(&mut v, &mut e, &p).is_err());
    let p = prepared(&mut v, &mut e, "1021");
    form(&mut v, &mut e, "1021?", true);
    let before = e.snapshot.clone();
    assert!(apply(&mut v, &mut e, &p).is_err());
    assert_eq!(e.snapshot, before);
    let foreign = Editor::new(before, "other-instance".into()).unwrap();
    assert!(p.plan().unwrap().check(&foreign, &v).is_err());
}
#[test]
fn delete_undo_redo_restores_exact_port_and_route_identities_and_keeps_newer_invalid_form() {
    let mut e = editor(true);
    let mut v = clone_views(&e);
    let p = prepared(&mut v, &mut e, "1021");
    assert_eq!(p.summary().port_occurrences.len(), 4);
    assert_eq!(p.summary().connector_routes.len(), 2);
    let ports = v.document().typed_ports.clone();
    let edges = v.document().connector_edges.clone();
    let s = load(&e.snapshot).unwrap();
    apply(&mut v, &mut e, &p).unwrap();
    assert!(
        v.document()
            .typed_ports
            .values()
            .all(|o| !o.presence.active())
    );
    assert!(
        v.document()
            .connector_edges
            .values()
            .all(|o| !o.presence.active())
    );
    form(&mut v, &mut e, "1021?", true);
    for _ in 0..2 {
        command(&mut v, &mut e, NativeAction::ModelUndo).unwrap();
        assert_eq!(load(&e.snapshot).unwrap(), s);
        assert_eq!(v.document().typed_ports, ports);
        assert_eq!(v.document().connector_edges, edges);
        assert_eq!(deletion::form(&e.snapshot).unwrap().unwrap().ids, "1021?");
        command(&mut v, &mut e, NativeAction::ModelRedo).unwrap();
        assert!(
            v.document()
                .typed_ports
                .values()
                .all(|o| !o.presence.active())
        );
        assert!(
            v.document()
                .connector_edges
                .values()
                .all(|o| !o.presence.active())
        );
    }
    assert_eq!(e.snapshot.next_id, 1027);
}
#[test]
fn cold_recovery_has_no_inverse_history_authority_even_when_editor_history_is_available() {
    let mut e = editor(true);
    let mut v = clone_views(&e);
    let p = prepared(&mut v, &mut e, "1021");
    apply(&mut v, &mut e, &p).unwrap();
    let recovery = v.recovery(&e).unwrap().encoded().unwrap();
    let mut cold = Views::restore(&recovery, &e).unwrap();
    command(&mut cold, &mut e, NativeAction::ModelUndo).unwrap();
    assert!(
        load(&e.snapshot)
            .unwrap()
            .definitions
            .contains_key(&Id(1021))
    );
    assert!(
        cold.document()
            .typed_ports
            .values()
            .all(|o| !o.presence.active())
    );
    assert!(
        cold.document()
            .connector_edges
            .values()
            .all(|o| !o.presence.active())
    );
}
fn save(session: &mut Session, views: &Views, native: &std::path::Path) {
    let side = native.with_extension("mbse-views.json");
    let mut io = Io::default();
    let dest = io
        .freeze_destination(session, views, &side, Kind::Bound)
        .unwrap();
    let capture = views.capture(&session.editor);
    let job = session.begin_save(native).unwrap();
    assert!(session.complete(execute_write(job)));
    let job = io.begin_captured(session, views, capture, dest).unwrap();
    assert!(io.complete(session, views, crate::native_views_io::execute(job)));
}
#[test]
fn controlled_derivative_has_fresh_native_binding_and_actual_deleted_checkpoint_survives_cold_reopen()
 {
    use sha2::{Digest, Sha256};
    let sha = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let original_native = std::fs::read(input()).unwrap();
    let original_views = std::fs::read(input().with_extension("mbse-views.json")).unwrap();
    let source = Session::open(area.path(), &input()).unwrap();
    let original = Views::decode(
        &original_views,
        &source.editor,
        &accepted_native(&source).unwrap(),
    )
    .unwrap();
    let explicit_unbound = original
        .recovery(&source.editor)
        .unwrap()
        .encoded()
        .unwrap();
    let mut clean = Session::new(area.path()).unwrap();
    let mut snapshot = source.editor.snapshot.clone();
    let removed = snapshot
        .records
        .get_mut(&RecordId(1))
        .unwrap()
        .fields
        .remove("opaque.p14.future")
        .unwrap();
    clean.editor = Editor::new(snapshot, clean.editor.freshness().instance_id).unwrap();
    let mut views = Views::restore(&explicit_unbound, &clean.editor).unwrap();
    let output = std::env::var_os("MBSE_P21_FIXTURE_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| area.path().join("controlled"));
    std::fs::create_dir(&output).unwrap();
    let native = output.join("controlled.kernspan");
    save(&mut clean, &views, &native);
    let cold = Session::open(area.path(), &native).unwrap();
    let bound = Views::decode(
        &std::fs::read(native.with_extension("mbse-views.json")).unwrap(),
        &cold.editor,
        &accepted_native(&cold).unwrap(),
    )
    .unwrap();
    assert_eq!(cold.editor.snapshot, clean.editor.snapshot);
    assert_eq!(
        bound.document().typed_ports,
        original.document().typed_ports
    );
    assert_eq!(
        bound.document().connector_edges,
        original.document().connector_edges
    );
    assert!(deletion::unknown_impact(&cold.editor).locations.is_empty());
    let provenance = serde_json::json!({"schema":"mbse-craft.controlled-fixture/1","purpose":"Explicit synthetic derivative for positive deletion checks; original opaque metadata remains a blocking case. No source acceptance or repair claim.","source_native_sha256":sha(&original_native),"source_views_sha256":sha(&original_views),"removed_field":"records[1].fields.opaque.p14.future","removed_value":removed,"handoff":"explicit unbound recovery/4 followed by new session and actual native checkpoint plus captured bound views/4 Save","unbound_recovery_sha256":sha(&explicit_unbound),"controlled_native_sha256":sha(&std::fs::read(&native).unwrap()),"controlled_views_sha256":sha(&std::fs::read(native.with_extension("mbse-views.json")).unwrap())});
    std::fs::write(
        output.join("PROVENANCE.json"),
        serde_json::to_vec_pretty(&provenance).unwrap(),
    )
    .unwrap();
    let p = prepared(&mut views, &mut clean.editor, "1021");
    apply(&mut views, &mut clean.editor, &p).unwrap();
    form(&mut views, &mut clean.editor, "1021?", true);
    let expected = clean.editor.snapshot.clone();
    let deleted = area.path().join("deleted.kernspan");
    save(&mut clean, &views, &deleted);
    let reopened = Session::open(area.path(), &deleted).unwrap();
    assert_eq!(reopened.editor.snapshot, expected);
    assert!(!reopened.editor.can_undo());
    let loaded = Views::decode(
        &std::fs::read(deleted.with_extension("mbse-views.json")).unwrap(),
        &reopened.editor,
        &accepted_native(&reopened).unwrap(),
    )
    .unwrap();
    assert_eq!(loaded.document().typed_ports, views.document().typed_ports);
    assert_eq!(
        loaded.document().connector_edges,
        views.document().connector_edges
    );
    assert_eq!(std::fs::read(input()).unwrap(), original_native);
    assert_eq!(
        std::fs::read(input().with_extension("mbse-views.json")).unwrap(),
        original_views
    );
}
#[test]
fn old_journal_keeps_absent_deletion_field_and_round_trips_raw_detached_forms() {
    let e = editor(true);
    let j = journal(&e.snapshot).unwrap();
    assert!(j.deletion.is_none());
    let wire = serde_json::to_value(&j).unwrap();
    assert!(wire.get("deletion").is_none());
    assert_eq!(serde_json::from_value::<Journal>(wire).unwrap(), j);
}
