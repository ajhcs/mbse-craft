use super::*;
use crate::types::{self, ports::Action as PA};
fn editor() -> Editor {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p17b/fixtures/ports.kernspan"
    ))
    .unwrap();
    Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "typed-views:test".into(),
    )
    .unwrap()
}
fn apply(v: &mut Views, e: &Editor, a: ports::Layout) {
    let stamp = v.stamp(e);
    v.apply(e, &stamp, Layout::Port(a)).unwrap();
}
fn reuse(v: &mut Views, e: &Editor, view: ViewId, port: types::Id, x: f64) -> OccurrenceId {
    let id = OccurrenceId(v.document.next_id());
    apply(
        v,
        e,
        ports::Layout::Reuse {
            view,
            port,
            geometry: Geometry::at(x, 20.),
        },
    );
    id
}
fn command(v: &mut Views, e: &mut Editor, action: types::Action) {
    let stamp = v.stamp(e);
    v.dispatch(e, &stamp, NativeAction::Command(Command::Typed(action)))
        .unwrap();
}
#[test]
fn two_views_reuse_one_semantic_port_with_separate_geometry_and_layout_history() {
    let e = editor();
    let baseline = e.snapshot.clone();
    let mut v = Views::new(&e).unwrap();
    let view_ids: Vec<_> = v.document.views.keys().copied().collect();
    let a = reuse(&mut v, &e, view_ids[0], types::Id(1015), 11.);
    let b = reuse(&mut v, &e, view_ids[1], types::Id(1015), 22.);
    assert_ne!(a, b);
    assert_eq!(v.document.profile, ports::PROFILE);
    apply(&mut v, &e, ports::Layout::Move(a, Geometry::at(90., 40.)));
    apply(&mut v, &e, ports::Layout::Visible(a, false));
    assert!(!v.document.typed_ports[&a].visible);
    assert_eq!(v.document.typed_ports[&b].geometry.x, 22.);
    apply(&mut v, &e, ports::Layout::Remove(a));
    assert!(
        types::load(&e.snapshot)
            .unwrap()
            .ports
            .contains_key(&types::Id(1015))
    );
    let next = v.document.next_id();
    let stamp = v.stamp(&e);
    assert!(v.undo(&e, &stamp, false).unwrap());
    assert!(!v.document.typed_ports[&a].visible);
    assert_eq!(v.document.typed_ports[&a].geometry.x, 90.);
    assert_eq!(v.document.next_id(), next);
    assert_eq!(e.snapshot, baseline);
}
#[test]
fn stale_pinned_wrong_kind_and_missing_view_reuse_refuse_without_mutation() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let view = v.document.primary;
    let stale = v.stamp(&e);
    let id = reuse(&mut v, &e, view, types::Id(1015), 10.);
    apply(&mut v, &e, ports::Layout::Pin(id, true));
    let before = (v.document.clone(), v.history_usage());
    let stamp = v.stamp(&e);
    for action in [
        ports::Layout::Move(id, Geometry::at(20., 30.)),
        ports::Layout::Reuse {
            view,
            port: types::Id(1004),
            geometry: Geometry::at(0., 0.),
        },
        ports::Layout::Reuse {
            view: ViewId(9999),
            port: types::Id(1015),
            geometry: Geometry::at(0., 0.),
        },
    ] {
        assert!(v.apply(&e, &stamp, Layout::Port(action)).is_err());
        assert_eq!((v.document.clone(), v.history_usage()), before);
    }
    assert!(
        v.apply(&e, &stale, Layout::Port(ports::Layout::Remove(id)))
            .is_err()
    );
    assert_eq!((v.document.clone(), v.history_usage()), before);
}
#[test]
fn coupled_model_undo_redo_detaches_and_restores_exact_ports_while_retaining_newer_raw_text() {
    let mut e = editor();
    let mut v = Views::new(&e).unwrap();
    let id = types::Id(e.snapshot.next_id);
    command(
        &mut v,
        &mut e,
        types::Action::Port(PA::BeginPort {
            owner: types::Id(1007),
        }),
    );
    let mut form = types::ports::PortForm {
        owner: types::Id(1007),
        name: "Additional input".into(),
        contract: "1012".into(),
        assignment: "1013".into(),
    };
    command(
        &mut v,
        &mut e,
        types::Action::Port(PA::SetPortForm {
            id,
            form: form.clone(),
        }),
    );
    command(&mut v, &mut e, types::Action::Port(PA::ApplyPort { id }));
    let accepted = e.snapshot.clone();
    let views: Vec<_> = v.document.views.keys().copied().collect();
    let a = reuse(&mut v, &e, views[0], id, 11.);
    let b = reuse(&mut v, &e, views[1], id, 22.);
    form.assignment = "new invalid text".into();
    command(
        &mut v,
        &mut e,
        types::Action::Port(PA::SetPortForm {
            id,
            form: form.clone(),
        }),
    );
    let stamp = v.stamp(&e);
    v.dispatch(&mut e, &stamp, NativeAction::ModelUndo).unwrap();
    assert!(!types::load(&e.snapshot).unwrap().ports.contains_key(&id));
    assert!(
        v.document
            .typed_ports
            .values()
            .all(|o| !o.presence.active())
    );
    let recovery = v.recovery(&e).unwrap().encoded().unwrap();
    let mut cold = Editor::new(accepted, "typed-views:cold".into()).unwrap();
    let mut restored = Views::restore(&recovery, &cold).unwrap();
    assert!(
        restored
            .document
            .typed_ports
            .values()
            .all(|o| !o.presence.active())
    );
    let stamp = restored.stamp(&cold);
    restored
        .dispatch(&mut cold, &stamp, NativeAction::ModelRedo)
        .unwrap();
    assert!(
        restored
            .document
            .typed_ports
            .values()
            .all(|o| !o.presence.active())
    );
    form.assignment = "even newer invalid text".into();
    command(
        &mut v,
        &mut e,
        types::Action::Port(PA::SetPortForm {
            id,
            form: form.clone(),
        }),
    );
    let stamp = v.stamp(&e);
    v.dispatch(&mut e, &stamp, NativeAction::ModelRedo).unwrap();
    assert!(v.document.typed_ports.values().all(|o| o.presence.active()));
    assert_eq!(
        (
            v.document.typed_ports[&a].geometry.x,
            v.document.typed_ports[&b].geometry.x
        ),
        (11., 22.)
    );
    assert_eq!(types::ports::forms(&e.snapshot).unwrap()[&id], form);
}
#[test]
fn actual_bound_native_save_and_views_cold_decode_preserve_both_port_occurrences_and_core_data() {
    use crate::{
        native_views_io::accepted_native,
        stage1_io::{Session, execute_write},
    };
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    session.editor = editor();
    let mut v = Views::new(&session.editor).unwrap();
    let core = v.document.occurrences.clone();
    let view_ids: Vec<_> = v.document.views.keys().copied().collect();
    reuse(&mut v, &session.editor, view_ids[0], types::Id(1015), 11.);
    reuse(&mut v, &session.editor, view_ids[1], types::Id(1015), 22.);
    let native = area.path().join("ports.kernspan");
    let job = session.begin_save(&native).unwrap();
    assert!(session.complete(execute_write(job)));
    let checkpoint = accepted_native(&session).unwrap();
    let frozen = v.freeze(&session.editor, &checkpoint).unwrap();
    assert_eq!(frozen.profile, ports::PROFILE);
    let bytes = frozen.encoded().unwrap();
    let sidecar = area.path().join("ports.mbse-views.json");
    std::fs::write(&sidecar, &bytes).unwrap();
    let reopened = Session::open(area.path(), &native).unwrap();
    let reopened_checkpoint = accepted_native(&reopened).unwrap();
    let cold = Views::decode(
        &std::fs::read(sidecar).unwrap(),
        &reopened.editor,
        &reopened_checkpoint,
    )
    .unwrap();
    assert_eq!(cold.document, frozen);
    assert_eq!(cold.document.occurrences, core);
    assert_eq!(reopened.editor.snapshot, session.editor.snapshot);
    assert_eq!(cold.history_usage(), (0, 0));
    let recovery = cold.recovery(&reopened.editor).unwrap();
    assert_eq!(recovery.profile, ports::RECOVERY_PROFILE);
    assert_eq!(
        Views::restore(&recovery.encoded().unwrap(), &reopened.editor)
            .unwrap()
            .document
            .typed_ports,
        frozen.typed_ports
    );
}
#[test]
fn malformed_profile_identity_count_and_target_inputs_refuse_instead_of_reclassifying_ports() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let view = v.document.primary;
    let id = reuse(&mut v, &e, view, types::Id(1015), 10.);
    let good = v.recovery(&e).unwrap();
    for mode in 0..4 {
        let mut d = good.clone();
        match mode {
            0 => d.profile = RECOVERY_PROFILE.into(),
            1 => {
                let mut o = d.typed_ports.remove(&id).unwrap();
                o.id = OccurrenceId(3);
                d.typed_ports.insert(o.id, o);
            }
            2 => d.typed_ports.get_mut(&id).unwrap().port = types::Id(1004),
            _ => {
                d.typed_ports.clear();
                for n in 0..MAX_PER_VIEW {
                    let mut o = good.typed_ports[&id].clone();
                    o.id = OccurrenceId(100 + n as u64);
                    d.typed_ports.insert(o.id, o);
                }
                d.next_id = Serial(100 + MAX_PER_VIEW as u64);
            }
        }
        assert!(Views::restore(&d.encoded().unwrap(), &e).is_err());
    }
    let mut wire: serde_json::Value = serde_json::from_slice(&good.encoded().unwrap()).unwrap();
    let old = wire["typed_ports"]
        .as_object_mut()
        .unwrap()
        .remove(&id.0.to_string())
        .unwrap();
    wire["typed_ports"]
        .as_object_mut()
        .unwrap()
        .insert(format!("0{}", id.0), old);
    assert!(Views::restore(&serde_json::to_vec(&wire).unwrap(), &e).is_err());
}
#[test]
fn typed_source_inspection_and_copy_into_typed_target_refuse_without_dropping_original_content() {
    use crate::{native_views_inspect::Inspection, native_views_plan::Plan};
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let e = editor();
    let mut target = Views::new(&e).unwrap();
    let view = target.document.primary;
    let core = target.recovery(&e).unwrap().encoded().unwrap();
    let source = area.path().join("core-views.json");
    std::fs::write(&source, &core).unwrap();
    let inspection = Inspection::open(&source).unwrap();
    reuse(&mut target, &e, view, types::Id(1015), 12.);
    let before = (
        e.snapshot.digest(),
        target.document.digest().unwrap(),
        target.history_usage(),
    );
    assert!(Plan::new(&inspection, inspection.declared().primary, &e, &target).is_err());
    assert_eq!(
        (
            e.snapshot.digest(),
            target.document.digest().unwrap(),
            target.history_usage()
        ),
        before
    );
    let bytes = target.recovery(&e).unwrap().encoded().unwrap();
    let typed = area.path().join("typed-views.json");
    std::fs::write(&typed, &bytes).unwrap();
    assert!(
        Inspection::open(&typed)
            .err()
            .unwrap()
            .contains("TYPED_SOURCE_INSPECTION_UNSUPPORTED")
    );
    assert_eq!(std::fs::read(typed).unwrap(), bytes);
    assert_eq!(std::fs::read(source).unwrap(), core);
}
#[test]
fn layout_undo_refuses_to_resurrect_missing_semantic_port_and_keeps_v3_after_removal() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let view = v.document.primary;
    let id = reuse(&mut v, &e, view, types::Id(1015), 12.);
    apply(&mut v, &e, ports::Layout::Remove(id));
    assert_eq!(v.document.profile, ports::PROFILE);
    let mut removed = e.snapshot.clone();
    removed
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .get_mut(types::NAMESPACE)
        .unwrap()["ports"]
        .as_object_mut()
        .unwrap()
        .remove("1015");
    let absent = Editor::new(removed, "typed-views:absent".into()).unwrap();
    let before = (v.document.clone(), v.history_usage());
    let stamp = v.stamp(&absent);
    assert!(v.undo(&absent, &stamp, false).is_err());
    assert_eq!((v.document.clone(), v.history_usage()), before);
}
#[test]
fn legacy_wire_values_do_not_gain_typed_fields_or_change_profile() {
    let raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p17b/fixtures/core-views.json"
    ))
    .unwrap();
    let doc = decode_document(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert!(doc.typed_ports.is_empty());
    assert_eq!(serde_json::to_value(doc).unwrap(), raw);
}
