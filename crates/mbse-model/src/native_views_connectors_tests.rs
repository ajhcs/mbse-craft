use super::*;
use crate::types::{
    self, Id,
    connectors::{Action as CA, Form},
};
fn editor() -> Editor {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p18b/fixtures/connectors.kernspan"
    ))
    .unwrap();
    Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "connector-views:test".into(),
    )
    .unwrap()
}
fn apply(v: &mut Views, e: &Editor, a: Layout) -> Result<(), String> {
    v.apply(e, &v.stamp(e), a)
}
fn reuse(v: &mut Views, e: &Editor, view: ViewId, usage: u64, port: u64, x: f64) -> OccurrenceId {
    let id = OccurrenceId(v.document.next_id());
    apply(
        v,
        e,
        Layout::Port(ports::Layout::ReuseInUsage {
            view,
            usage: Id(usage),
            port: Id(port),
            geometry: Geometry::at(x, 200.),
        }),
    )
    .unwrap();
    id
}
fn pair(v: &mut Views, e: &Editor, view: ViewId) -> (OccurrenceId, OccurrenceId) {
    (
        reuse(v, e, view, 1009, 1017, 25.),
        reuse(v, e, view, 1010, 1015, 400.),
    )
}
fn route(
    v: &mut Views,
    e: &Editor,
    view: ViewId,
    a: OccurrenceId,
    b: OccurrenceId,
) -> VisualEdgeId {
    let id = VisualEdgeId(v.document.next_id());
    apply(
        v,
        e,
        Layout::Connector(connectors::Layout::Route {
            view,
            connector: Id(1018),
            source: a,
            target: b,
        }),
    )
    .unwrap();
    id
}
fn command(v: &mut Views, e: &mut Editor, a: NativeAction) {
    v.dispatch(e, &v.stamp(e), a).unwrap();
}
#[test]
fn two_views_route_one_connector_through_distinct_qualified_occurrences_without_model_mutation() {
    let e = editor();
    let before = e.snapshot.clone();
    let mut v = Views::new(&e).unwrap();
    let views: Vec<_> = v.document.views.keys().copied().collect();
    let (a, b) = pair(&mut v, &e, views[0]);
    let (c, d) = pair(&mut v, &e, views[1]);
    let x = route(&mut v, &e, views[0], a, b);
    let y = route(&mut v, &e, views[1], c, d);
    assert_ne!(x, y);
    assert_eq!(v.document.profile, connectors::PROFILE);
    assert_eq!(
        v.document.connector_edges[&x].connector,
        v.document.connector_edges[&y].connector
    );
    assert_eq!(v.document.typed_ports[&a].usage, Some(Id(1009)));
    assert_eq!(v.document.typed_ports[&b].usage, Some(Id(1010)));
    assert_eq!(e.snapshot, before);
    let frozen = v.recovery(&e).unwrap();
    assert_eq!(frozen.profile, connectors::RECOVERY_PROFILE);
    assert_eq!(
        Views::restore(&frozen.encoded().unwrap(), &e)
            .unwrap()
            .document
            .typed_ports,
        v.document.typed_ports
    );
}
#[test]
fn wrong_reference_unqualified_reversed_cross_view_duplicate_and_stale_routes_refuse_atomically() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let view = v.document.primary;
    let (a, b) = pair(&mut v, &e, view);
    let other = *v.document.views.keys().find(|id| **id != view).unwrap();
    let wrong = reuse(&mut v, &e, view, 1009, 1015, 800.);
    let cross = reuse(&mut v, &e, other, 1010, 1015, 400.);
    let plain = OccurrenceId(v.document.next_id());
    apply(
        &mut v,
        &e,
        Layout::Port(ports::Layout::Reuse {
            view,
            port: Id(1017),
            geometry: Geometry::at(0., 0.),
        }),
    )
    .unwrap();
    for action in [
        Layout::Port(ports::Layout::ReuseInUsage {
            view,
            usage: Id(1011),
            port: Id(1017),
            geometry: Geometry::at(0., 0.),
        }),
        Layout::Port(ports::Layout::ReuseInUsage {
            view,
            usage: Id(1009),
            port: Id(1004),
            geometry: Geometry::at(0., 0.),
        }),
        Layout::Connector(connectors::Layout::Route {
            view,
            connector: Id(1018),
            source: b,
            target: a,
        }),
        Layout::Connector(connectors::Layout::Route {
            view,
            connector: Id(1018),
            source: a,
            target: wrong,
        }),
        Layout::Connector(connectors::Layout::Route {
            view,
            connector: Id(1018),
            source: a,
            target: cross,
        }),
        Layout::Connector(connectors::Layout::Route {
            view,
            connector: Id(1018),
            source: plain,
            target: b,
        }),
    ] {
        let before = v.document.clone();
        assert!(apply(&mut v, &e, action).is_err());
        assert_eq!(v.document, before);
    }
    let stale = v.stamp(&e);
    let edge = route(&mut v, &e, view, a, b);
    let before = v.document.clone();
    assert!(
        apply(
            &mut v,
            &e,
            Layout::Connector(connectors::Layout::Route {
                view,
                connector: Id(1018),
                source: a,
                target: b
            })
        )
        .is_err()
    );
    assert!(
        v.apply(
            &e,
            &stale,
            Layout::Connector(connectors::Layout::Remove(edge))
        )
        .is_err()
    );
    assert_eq!(v.document, before);
}
#[test]
fn remove_visual_route_or_endpoint_and_layout_undo_keep_semantic_connector_and_other_view_exact() {
    let e = editor();
    let before = e.snapshot.clone();
    let mut v = Views::new(&e).unwrap();
    let ids: Vec<_> = v.document.views.keys().copied().collect();
    let (a, b) = pair(&mut v, &e, ids[0]);
    let (c, d) = pair(&mut v, &e, ids[1]);
    let x = route(&mut v, &e, ids[0], a, b);
    let y = route(&mut v, &e, ids[1], c, d);
    let saved = v.document.clone();
    apply(&mut v, &e, Layout::Connector(connectors::Layout::Remove(x))).unwrap();
    assert!(!v.document.connector_edges.contains_key(&x));
    assert_eq!(e.snapshot, before);
    v.undo(&e, &v.stamp(&e), false).unwrap();
    assert_eq!(v.document.connector_edges, saved.connector_edges);
    apply(&mut v, &e, Layout::Port(ports::Layout::Remove(a))).unwrap();
    assert!(!v.document.connector_edges.contains_key(&x));
    assert_eq!(v.document.connector_edges[&y], saved.connector_edges[&y]);
    v.undo(&e, &v.stamp(&e), false).unwrap();
    assert_eq!(v.document.connector_edges, saved.connector_edges);
    assert_eq!(v.document.typed_ports, saved.typed_ports);
    assert_eq!(e.snapshot, before);
}
#[test]
fn semantic_undo_detaches_both_routes_redo_restores_same_runtime_proof_and_newer_text() {
    let mut e = editor();
    let ext = e.snapshot.extensions.as_mut().unwrap();
    ext.namespaces
        .get_mut(types::NAMESPACE)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("connectors");
    let mut f: Form =
        serde_json::from_value(ext.namespaces[types::DRAFTS]["connectors"]["1018"].clone())
            .unwrap();
    f.conveyed = "1005".into();
    let mut v = Views::new(&e).unwrap();
    command(
        &mut v,
        &mut e,
        NativeAction::Command(Command::Typed(types::Action::Connector(CA::Set {
            id: Id(1018),
            form: f.clone(),
        }))),
    );
    command(
        &mut v,
        &mut e,
        NativeAction::Command(Command::Typed(types::Action::Connector(CA::Apply {
            id: Id(1018),
        }))),
    );
    let ids: Vec<_> = v.document.views.keys().copied().collect();
    for view in ids {
        let (a, b) = pair(&mut v, &e, view);
        route(&mut v, &e, view, a, b);
    }
    f.conveyed = "1005?".into();
    command(
        &mut v,
        &mut e,
        NativeAction::Command(Command::Typed(types::Action::Connector(CA::Set {
            id: Id(1018),
            form: f.clone(),
        }))),
    );
    let accepted = v.document.connector_edges.clone();
    command(&mut v, &mut e, NativeAction::ModelUndo);
    assert!(
        v.document
            .connector_edges
            .values()
            .all(|x| !x.presence.active())
    );
    assert!(v.document.typed_ports.values().all(|x| x.presence.active()));
    assert_eq!(types::connectors::forms(&e.snapshot).unwrap()[&Id(1018)], f);
    let recovery = v.recovery(&e).unwrap();
    let mut cold = Views::restore(&recovery.encoded().unwrap(), &e).unwrap();
    let mut clone = e.clone();
    command(&mut cold, &mut clone, NativeAction::ModelRedo);
    assert!(
        cold.document
            .connector_edges
            .values()
            .all(|x| !x.presence.active())
    );
    command(&mut v, &mut e, NativeAction::ModelRedo);
    assert_eq!(v.document.connector_edges, accepted);
    assert_eq!(types::connectors::forms(&e.snapshot).unwrap()[&Id(1018)], f);
}
#[test]
fn captured_v4_native_views_save_and_cold_reopen_preserve_contexts_routes_forms_and_core() {
    use crate::{
        native_views_io::{Io, Kind, accepted_native},
        stage1_io::{Session, execute_write},
    };
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let instance = session.editor.freshness().instance_id;
    session.editor = Editor::new(editor().snapshot, instance).unwrap();
    let expected = session.editor.snapshot.clone();
    let mut v = Views::new(&session.editor).unwrap();
    let core = v.document.occurrences.clone();
    let ids: Vec<_> = v.document.views.keys().copied().collect();
    for view in ids {
        let (a, b) = pair(&mut v, &session.editor, view);
        route(&mut v, &session.editor, view, a, b);
    }
    let side = area.path().join("connectors.mbse-views.json");
    let mut io = Io::default();
    let dest = io
        .freeze_destination(&session, &v, &side, Kind::Bound)
        .unwrap();
    let capture = v.capture(&session.editor);
    assert_eq!(
        capture.document(&session.editor, None).unwrap().profile,
        connectors::RECOVERY_PROFILE
    );
    let native = area.path().join("connectors.kernspan");
    let job = session.begin_save(&native).unwrap();
    assert!(session.complete(execute_write(job)));
    let job = io.begin_captured(&session, &v, capture, dest).unwrap();
    assert!(io.complete(&session, &v, crate::native_views_io::execute(job)));
    assert!(io.is_saved(&session, &v));
    let frozen = v
        .freeze(&session.editor, &accepted_native(&session).unwrap())
        .unwrap();
    assert_eq!(frozen.profile, connectors::PROFILE);
    let cold = Session::open(area.path(), &native).unwrap();
    let views = Views::decode(
        &std::fs::read(side).unwrap(),
        &cold.editor,
        &accepted_native(&cold).unwrap(),
    )
    .unwrap();
    assert_eq!(views.document, frozen);
    assert_eq!(views.document.occurrences, core);
    assert_eq!(cold.editor.snapshot, expected);
    assert_eq!(views.history_usage(), (0, 0));
}
#[test]
fn malformed_v4_profile_context_wire_and_shared_identity_collision_refuse() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let view = v.document.primary;
    let (a, b) = pair(&mut v, &e, view);
    let edge = route(&mut v, &e, view, a, b);
    let good = v.document.clone();
    for mode in 0..5 {
        let mut doc = good.clone();
        match mode {
            0 => doc.profile = ports::PROFILE.into(),
            1 => doc.typed_ports.get_mut(&a).unwrap().usage = Some(Id(1011)),
            2 => doc.connector_edges.get_mut(&edge).unwrap().source = b,
            3 => {
                let mut x = doc.connector_edges.remove(&edge).unwrap();
                x.id = VisualEdgeId(a.0);
                doc.connector_edges.insert(x.id, x);
            }
            _ => doc.connector_edges.get_mut(&edge).unwrap().connector = Id(1017),
        }
        assert!(doc.validate(&e.snapshot).is_err(), "mode{mode}");
    }
    let mut wire = serde_json::to_value(&good).unwrap();
    wire["typed_ports"][a.0.to_string()]["usage"] = serde_json::json!("01009");
    assert!(serde_json::from_value::<Document>(wire).is_err());
}
#[test]
fn v3_and_older_wire_values_remain_exact_without_qualified_fields() {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p18b/fixtures/connectors.mbse-views.json"
    ))
    .unwrap();
    let doc: Document = serde_json::from_value(v.clone()).unwrap();
    doc.validate(&editor().snapshot).unwrap();
    assert_eq!(serde_json::to_value(doc).unwrap(), v);
}
