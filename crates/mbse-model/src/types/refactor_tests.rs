use super::*;
use crate::{
    native_views::{Geometry, Layout, NativeAction, Views, connectors as vc, ports as vp},
    stage1::{Command, Editor},
};
use refactor::{Action as RA, Form as RF, Mode};
fn editor() -> Editor {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/parity/p20/fixtures/values.kernspan"
    ))
    .unwrap();
    Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "refactor:test".into(),
    )
    .unwrap()
}
fn run(e: &mut Editor, a: Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(a)).map(|_| ())
}
fn clone_form(root: u64) -> RF {
    RF {
        mode: Mode::CloneDefinition,
        definition: root.to_string(),
        name: "accepted copy".into(),
        usages: String::new(),
        owner: String::new(),
        keep_external: true,
    }
}
fn move_form(ids: &str, owner: u64) -> RF {
    RF {
        mode: Mode::MoveUsages,
        definition: "1004".into(),
        name: "preserved future name".into(),
        usages: ids.into(),
        owner: owner.to_string(),
        keep_external: true,
    }
}
fn prepare(e: &mut Editor, f: RF) -> refactor::Prepared {
    run(e, Action::Refactor(RA::SetForm(f))).unwrap();
    refactor::prepare(e).unwrap()
}
fn apply(e: &mut Editor, p: &refactor::Prepared) {
    run(
        e,
        Action::Refactor(RA::Apply(Box::new(p.plan().unwrap().clone()))),
    )
    .unwrap();
}
#[test]
fn clone_remaps_owned_usages_connectors_and_override_ids_keeps_external_targets_and_source_drafts()
{
    let mut e = editor();
    let p = prepare(&mut e, clone_form(1004));
    let before = e.snapshot.clone();
    let store = load(&before).unwrap();
    let j = journal(&before).unwrap();
    assert_eq!(p.summary().id_mapping.len(), 6);
    assert_eq!(p.summary().id_mapping[&Id(1004)], Id(1021));
    assert_eq!(e.snapshot, before);
    assert!(
        p.summary()
            .incoming_targets_retained
            .iter()
            .any(|r| r.owner == Id(1011) && r.target == Id(1004))
    );
    assert!(!p.summary().exhaustive_global_reference_verdict);
    assert!(
        p.summary()
            .uninterpreted_locations
            .iter()
            .any(|s| s.contains("opaque.p14.future"))
    );
    let ids = p.summary().id_mapping.clone();
    apply(&mut e, &p);
    let after = load(&e.snapshot).unwrap();
    assert_eq!(after.definitions[&ids[&Id(1004)]].name, "accepted copy");
    assert_eq!(after.properties[&ids[&Id(1006)]].owner, ids[&Id(1004)]);
    assert_eq!(after.usages[&ids[&Id(1009)]].target, Id(1007));
    let c = &after.connectors[&ids[&Id(1018)]];
    assert_eq!(c.owner, ids[&Id(1004)]);
    assert_eq!(c.source.usage, ids[&Id(1009)]);
    assert_eq!(c.target.usage, ids[&Id(1010)]);
    assert_eq!(
        (c.source.port, c.target.port, c.conveyed),
        (Id(1017), Id(1015), Id(1005))
    );
    assert_eq!(after.overrides[&ids[&Id(1009)]][&Id(1020)].text, "23");
    assert_eq!(after.usages[&Id(1011)], store.usages[&Id(1011)]);
    assert_eq!(journal(&e.snapshot).unwrap(), j);
    assert_eq!(e.snapshot.records, before.records);
    assert_eq!(e.snapshot.next_id, before.next_id + 6);
}
#[test]
fn clone_component_copies_owned_property_defaults_ports_and_reference_without_retargeting_incoming_parts()
 {
    let mut e = editor();
    let p = prepare(&mut e, clone_form(1007));
    let ids = p.summary().id_mapping.clone();
    let before = load(&e.snapshot).unwrap();
    apply(&mut e, &p);
    let s = load(&e.snapshot).unwrap();
    assert_eq!(s.defaults[&ids[&Id(1020)]], before.defaults[&Id(1020)]);
    assert_eq!(s.ports[&ids[&Id(1017)]].owner, ids[&Id(1007)]);
    assert_eq!(s.ports[&ids[&Id(1017)]].contract, Id(1014));
    assert_eq!(s.usages[&ids[&Id(1011)]].owner, ids[&Id(1007)]);
    assert_eq!(s.usages[&ids[&Id(1011)]].target, Id(1004));
    assert_eq!(s.usages[&Id(1009)], before.usages[&Id(1009)]);
    assert_eq!(s.overrides[&Id(1009)], before.overrides[&Id(1009)]);
}
#[test]
fn partial_move_policy_cycle_and_invalid_selection_previews_never_mutate_or_allocate_live_state() {
    let mut e = editor();
    let p = prepare(&mut e, move_form("1009", 1008));
    assert!(p.plan().is_none());
    assert_eq!(p.summary().boundary_connectors, vec![Id(1018), Id(1019)]);
    let before = e.snapshot.clone();
    assert_eq!(e.snapshot.next_id, 1021);
    drop(p);
    assert_eq!(e.snapshot, before);
    let p = prepare(&mut e, move_form("1009,1010", 1007));
    assert!(p.plan().is_none());
    assert!(p.summary().refusal.as_ref().unwrap().contains("CYCLE"));
    let mut f = clone_form(1004);
    f.keep_external = false;
    let p = prepare(&mut e, f);
    assert!(p.plan().is_none());
    assert_eq!(e.snapshot.next_id, 1021);
    for text in ["1009,1009", "1009,", "01009", "1005"] {
        let f = move_form(text, 1008);
        run(&mut e, Action::Refactor(RA::SetForm(f.clone()))).unwrap();
        let before = e.snapshot.clone();
        assert!(refactor::prepare(&e).is_err());
        assert_eq!(e.snapshot, before);
        assert_eq!(refactor::form(&e.snapshot).unwrap(), Some(f));
    }
}
fn dispatch(v: &mut Views, e: &mut Editor, a: NativeAction) {
    v.dispatch(e, &v.stamp(e), a).unwrap();
}
#[test]
fn move_connected_parts_updates_enclosed_connectors_with_same_ids_and_keeps_routes_and_values_through_undo()
 {
    let mut e = editor();
    let mut v = Views::new(&e).unwrap();
    let views: Vec<_> = v.document().views.keys().copied().collect();
    for view in views {
        let mut ids = Vec::new();
        for (usage, port, x) in [(1009, 1017, 25.), (1010, 1015, 400.)] {
            let id = v.document().next_id();
            v.apply(
                &e,
                &v.stamp(&e),
                Layout::Port(vp::Layout::ReuseInUsage {
                    view,
                    usage: Id(usage),
                    port: Id(port),
                    geometry: Geometry::at(x, 200.),
                }),
            )
            .unwrap();
            ids.push(serde_json::from_str(&format!("\"{id}\"")).unwrap());
        }
        v.apply(
            &e,
            &v.stamp(&e),
            Layout::Connector(vc::Layout::Route {
                view,
                connector: Id(1018),
                source: ids[0],
                target: ids[1],
            }),
        )
        .unwrap();
    }
    let ports = v.document().typed_ports.clone();
    let edges = v.document().connector_edges.clone();
    let old = load(&e.snapshot).unwrap();
    let p = prepare(&mut e, move_form("1009,1010", 1008));
    let counter = e.snapshot.next_id;
    assert_eq!(p.summary().moved_connectors, vec![Id(1018), Id(1019)]);
    dispatch(
        &mut v,
        &mut e,
        NativeAction::Command(Command::Typed(Action::Refactor(RA::Apply(Box::new(
            p.plan().unwrap().clone(),
        ))))),
    );
    let s = load(&e.snapshot).unwrap();
    for id in [1009, 1010] {
        assert_eq!(s.usages[&Id(id)].owner, Id(1008));
    }
    for id in [1018, 1019] {
        assert_eq!(s.connectors[&Id(id)].owner, Id(1008));
    }
    assert_eq!(s.overrides, old.overrides);
    assert_eq!(e.snapshot.next_id, counter);
    assert_eq!(v.document().typed_ports, ports);
    assert_eq!(v.document().connector_edges, edges);
    dispatch(&mut v, &mut e, NativeAction::ModelUndo);
    assert_eq!(load(&e.snapshot).unwrap(), old);
    assert_eq!(v.document().connector_edges, edges);
    dispatch(&mut v, &mut e, NativeAction::ModelRedo);
    assert_eq!(load(&e.snapshot).unwrap(), s);
    assert_eq!(v.document().typed_ports, ports);
    assert_eq!(v.document().connector_edges, edges);
}
#[test]
fn stale_foreign_and_replayed_plans_refuse_and_clone_undo_keeps_counter_and_newer_raw_form() {
    let mut e = editor();
    let p = prepare(&mut e, clone_form(1004));
    let plan = p.plan().unwrap().clone();
    let mut foreign = Editor::new(e.snapshot.clone(), "refactor:foreign".into()).unwrap();
    let before = foreign.snapshot.clone();
    assert!(
        run(
            &mut foreign,
            Action::Refactor(RA::Apply(Box::new(plan.clone())))
        )
        .is_err()
    );
    assert_eq!(foreign.snapshot, before);
    let original = load(&e.snapshot).unwrap();
    apply(&mut e, &p);
    let cloned = load(&e.snapshot).unwrap();
    let counter = e.snapshot.next_id;
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::Refactor(RA::Apply(Box::new(plan.clone())))).is_err());
    assert_eq!(e.snapshot, before);
    let mut newer = clone_form(1004);
    newer.name = "newer unfinished name".into();
    run(&mut e, Action::Refactor(RA::SetForm(newer.clone()))).unwrap();
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(load(&e.snapshot).unwrap(), original);
    assert_eq!(refactor::form(&e.snapshot).unwrap(), Some(newer.clone()));
    assert_eq!(e.snapshot.next_id, counter);
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert_eq!(load(&e.snapshot).unwrap(), cloned);
    assert_eq!(refactor::form(&e.snapshot).unwrap(), Some(newer));
    assert_eq!(e.snapshot.next_id, counter);
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::Refactor(RA::Apply(Box::new(plan)))).is_err());
    assert_eq!(e.snapshot, before);
}
#[test]
fn accepted_clone_member_budget_refuses_before_live_allocation() {
    let mut e = editor();
    let mut store = load(&e.snapshot).unwrap();
    for _ in 0..129 {
        let id = allocate(&mut e.snapshot).unwrap();
        store.properties.insert(
            id,
            Property {
                id,
                owner: Id(1007),
                name: format!("property{}", id.0),
                value_type: Id(1005),
                multiplicity: Multiplicity {
                    lower: 1,
                    upper: Some(1),
                },
            },
        );
    }
    put(&mut e.snapshot, NAMESPACE, &store).unwrap();
    e.snapshot.validate().unwrap();
    run(&mut e, Action::Refactor(RA::SetForm(clone_form(1007)))).unwrap();
    let before = e.snapshot.clone();
    assert!(refactor::prepare(&e).unwrap_err().contains("MEMBER_LIMIT"));
    assert_eq!(e.snapshot, before);
}
#[test]
fn captured_native_views4_cold_reopen_preserves_cloned_moved_ids_source_forms_and_rejects_old_preview_authority()
 {
    use crate::{
        native_views_io::{Io, Kind, accepted_native},
        stage1_io::{Session, execute_write},
    };
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let input = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/parity/p20/fixtures/values.kernspan"
    ));
    let mut session = Session::open(area.path(), input).unwrap();
    let mut views = Views::decode(
        &std::fs::read(input.with_extension("mbse-views.json")).unwrap(),
        &session.editor,
        &accepted_native(&session).unwrap(),
    )
    .unwrap();
    let ports = views.document().typed_ports.clone();
    let edges = views.document().connector_edges.clone();
    let original_forms = values::forms(&session.editor.snapshot).unwrap();
    let clone = prepare(&mut session.editor, clone_form(1004));
    dispatch(
        &mut views,
        &mut session.editor,
        NativeAction::Command(Command::Typed(Action::Refactor(RA::Apply(Box::new(
            clone.plan().unwrap().clone(),
        ))))),
    );
    let moved = prepare(&mut session.editor, move_form("1009,1010", 1008));
    dispatch(
        &mut views,
        &mut session.editor,
        NativeAction::Command(Command::Typed(Action::Refactor(RA::Apply(Box::new(
            moved.plan().unwrap().clone(),
        ))))),
    );
    let unexecuted = prepare(&mut session.editor, clone_form(1004));
    let expected = session.editor.snapshot.clone();
    let native = area.path().join("refactor.kernspan");
    let side = area.path().join("refactor.mbse-views.json");
    let mut io = Io::default();
    let dest = io
        .freeze_destination(&session, &views, &side, Kind::Bound)
        .unwrap();
    let capture = views.capture(&session.editor);
    let job = session.begin_save(&native).unwrap();
    assert!(session.complete(execute_write(job)));
    let job = io.begin_captured(&session, &views, capture, dest).unwrap();
    assert!(io.complete(&session, &views, crate::native_views_io::execute(job)));
    let mut cold = Session::open(area.path(), &native).unwrap();
    assert_eq!(cold.editor.snapshot, expected);
    assert_eq!(
        values::forms(&cold.editor.snapshot).unwrap(),
        original_forms
    );
    let reopened = Views::decode(
        &std::fs::read(side).unwrap(),
        &cold.editor,
        &accepted_native(&cold).unwrap(),
    )
    .unwrap();
    assert_eq!(reopened.document().typed_ports, ports);
    assert_eq!(reopened.document().connector_edges, edges);
    let before = cold.editor.snapshot.clone();
    assert!(
        run(
            &mut cold.editor,
            Action::Refactor(RA::Apply(Box::new(unexecuted.plan().unwrap().clone())))
        )
        .is_err()
    );
    assert_eq!(cold.editor.snapshot, before);
}
