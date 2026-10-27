use mbse_model::{
    extensions::Extensions,
    native_views::{
        self as v, Layout, NativeAction, OccurrenceId, ViewId, Views, activities as va,
    },
    native_views_inspect::Inspection,
    native_views_plan::Plan,
    stage1::*,
    stage1_io::{Session, execute_write},
    types::{
        self as t, Id,
        activities::{
            self as a,
            forms::{self as f, Form},
        },
    },
};
use serde_json::{Value, json};
fn editor() -> Editor {
    let mut s = Snapshot::new(DocumentId("activity:forms-source".into()));
    s.extensions = Some(Extensions::default());
    Editor::new(s, "activity:forms-runtime".into()).unwrap()
}
fn run(e: &mut Editor, action: a::Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(t::Action::Activity(action)))
        .map(|_| ())
}
fn set(e: &mut Editor, form: Form) -> String {
    let key = form.key();
    run(e, a::Action::Form(f::Action::Set(form))).unwrap();
    key
}
fn apply(e: &mut Editor, key: &str) -> Result<(), String> {
    run(e, a::Action::Form(f::Action::Apply(key.into())))
}
fn activity(e: &mut Editor) -> Id {
    let id = Id(e.snapshot.next_id);
    let key = set(
        e,
        Form::Activity {
            id: None,
            system: "1".into(),
            name: "Inspect".into(),
        },
    );
    apply(e, &key).unwrap();
    id
}
fn node(e: &mut Editor, activity: Id, kind: a::NodeKind) -> Id {
    let id = Id(e.snapshot.next_id);
    let key = set(
        e,
        Form::Node {
            activity,
            id: None,
            name: format!("{kind:?}"),
            node_kind: kind,
            partition: "".into(),
        },
    );
    apply(e, &key).unwrap();
    id
}
fn graph() -> (Editor, Id, Id, Id, Id) {
    let mut e = editor();
    let a = activity(&mut e);
    let start = node(&mut e, a, a::NodeKind::Initial);
    let action = node(&mut e, a, a::NodeKind::Action);
    let end = node(&mut e, a, a::NodeKind::Final);
    for (source, target) in [(start, action), (action, end)] {
        let key = set(
            &mut e,
            Form::Flow {
                form: a::FlowForm {
                    activity: a,
                    id: None,
                    name: "Control".into(),
                    kind: a::FlowKind::Control,
                    source: source.0.to_string(),
                    target: target.0.to_string(),
                    guard: "".into(),
                },
            },
        );
        apply(&mut e, &key).unwrap();
    }
    (e, a, start, action, end)
}
fn layout(e: &Editor, views: &mut Views, action: va::Layout) -> Result<(), String> {
    views.apply(e, &views.stamp(e), Layout::Activity(action))
}
fn diagram(e: &Editor, views: &mut Views, a: Id) -> ViewId {
    let id: ViewId = serde_json::from_str(&format!("\"{}\"", views.document().next_id())).unwrap();
    layout(
        e,
        views,
        va::Layout::Create {
            activity: a,
            title: "Activity diagram".into(),
        },
    )
    .unwrap();
    id
}
fn occ(views: &Views, view: ViewId, id: Id) -> OccurrenceId {
    views.document().activity_diagrams[&view]
        .occurrences
        .values()
        .find(|o| o.element == id)
        .unwrap()
        .id
}
#[test]
fn keyed_activity_proposals_survive_switching_apply_refusal_model_history_and_explicit_single_discard()
 {
    let (mut e, a, _, action, _) = graph();
    let before = t::load(&e.snapshot).unwrap();
    let mut keys = Vec::new();
    for form in [
        Form::Activity {
            id: None,
            system: "1?".into(),
            name: "Unfinished activity".into(),
        },
        Form::Node {
            activity: a,
            id: Some(action),
            name: "Unfinished node".into(),
            node_kind: a::NodeKind::Action,
            partition: "2?".into(),
        },
        Form::Flow {
            form: a::FlowForm {
                activity: a,
                id: None,
                name: "Unfinished flow".into(),
                kind: a::FlowKind::Control,
                source: "x".into(),
                target: "y".into(),
                guard: "x >".into(),
            },
        },
    ] {
        keys.push(set(&mut e, form));
    }
    let raw = f::load(&e.snapshot).unwrap();
    for key in &keys {
        let snapshot = e.snapshot.clone();
        assert!(apply(&mut e, key).is_err());
        assert_eq!(e.snapshot, snapshot);
    }
    assert_eq!(t::load(&e.snapshot).unwrap(), before);
    e.undo_model(&e.freshness()).unwrap();
    assert_eq!(f::load(&e.snapshot).unwrap(), raw);
    e.redo_model(&e.freshness()).unwrap();
    assert_eq!(f::load(&e.snapshot).unwrap(), raw);
    run(&mut e, a::Action::Form(f::Action::Discard(keys[1].clone()))).unwrap();
    let now = f::load(&e.snapshot).unwrap();
    assert!(!now.contains_key(&keys[1]));
    assert!(now.contains_key(&keys[0]) && now.contains_key(&keys[2]));
    assert_eq!(t::load(&e.snapshot).unwrap(), before);
}
#[test]
fn legacy_flow_proposal_migrates_exactly_without_dropping_invalid_text_or_other_forms() {
    let (mut e, a, start, action, _) = graph();
    let old = a::FlowForm {
        activity: a,
        id: None,
        name: "Original old proposal".into(),
        kind: a::FlowKind::Control,
        source: start.0.to_string(),
        target: action.0.to_string(),
        guard: "incomplete ?".into(),
    };
    run(&mut e, a::Action::SetFlowForm(old.clone())).unwrap();
    let legacy = Form::Flow { form: old.clone() };
    let native = e.snapshot.records.clone();
    set(
        &mut e,
        Form::Activity {
            id: None,
            system: "1?".into(),
            name: "Other creation".into(),
        },
    );
    assert!(a::flow_form(&e.snapshot).unwrap().is_none());
    assert_eq!(f::load(&e.snapshot).unwrap()[&legacy.key()], legacy);
    assert_eq!(e.snapshot.records, native);
    let raw = e.snapshot.extensions.as_ref().unwrap().namespaces[t::DRAFTS].clone();
    assert!(raw.get("activity_flow").is_none());
    assert!(raw["activity_forms"].as_object().unwrap().len() >= 2);
}
#[test]
fn creation_and_edit_forms_keep_stable_slot_ids_and_native_allocator_while_invalid_pin_type_refuses()
 {
    let mut e = editor();
    let a = activity(&mut e);
    let p = Id(e.snapshot.next_id);
    let k = set(
        &mut e,
        Form::Partition {
            activity: a,
            id: None,
            name: "Review team".into(),
        },
    );
    apply(&mut e, &k).unwrap();
    let n = Id(e.snapshot.next_id);
    let k = set(
        &mut e,
        Form::Node {
            activity: a,
            id: None,
            name: "Review".into(),
            node_kind: a::NodeKind::Action,
            partition: p.0.to_string(),
        },
    );
    apply(&mut e, &k).unwrap();
    let key = set(
        &mut e,
        Form::Pin {
            activity: a,
            id: None,
            owner: n.0.to_string(),
            name: "Input".into(),
            direction: a::Direction::In,
            value_type: "1?".into(),
        },
    );
    let before = e.snapshot.clone();
    assert!(apply(&mut e, &key).is_err());
    assert_eq!(e.snapshot, before);
    assert!(
        f::load(&e.snapshot)
            .unwrap()
            .keys()
            .any(|k| k == &format!("node:{}:{}", a.0, n.0))
    );
    assert!(t::load(&e.snapshot).unwrap().activities[&a].nodes[&n].partition == Some(p));
}
#[test]
fn form_alias_foreign_identity_conflicting_legacy_and_bounds_refuse_atomically() {
    let (mut e, a, _, n, _) = graph();
    let before = e.snapshot.clone();
    let bad = Form::Activity {
        id: Some(n),
        system: "1".into(),
        name: "Foreign ID".into(),
    };
    assert!(run(&mut e, a::Action::Form(f::Action::Set(bad))).is_err());
    assert_eq!(e.snapshot, before);
    let form = Form::Activity {
        id: None,
        system: "?".into(),
        name: "x".repeat(4097),
    };
    assert!(run(&mut e, a::Action::Form(f::Action::Set(form))).is_err());
    assert_eq!(e.snapshot, before);
    let mut wire = e.snapshot.clone();
    let j = wire
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .get_mut(t::DRAFTS)
        .unwrap();
    let map = j["activity_forms"].as_object_mut().unwrap();
    let key = map.keys().next().unwrap().clone();
    let value = map.remove(&key).unwrap();
    map.insert(format!("alias:{key}"), value);
    assert!(wire.validate().is_err());
    let old = a::FlowForm {
        activity: a,
        id: None,
        name: "Old".into(),
        kind: a::FlowKind::Control,
        source: "1?".into(),
        target: "2?".into(),
        guard: "?".into(),
    };
    run(&mut e, a::Action::SetFlowForm(old.clone())).unwrap();
    let mut wire = e.snapshot.clone();
    let j = wire
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .get_mut(t::DRAFTS)
        .unwrap();
    j["activity_forms"][Form::Flow { form: old }.key()] = json!({"kind":"Flow","form":{"activity":a.0,"id":null,"name":"conflict","kind":"Control","source":"different","target":"2?","guard":"?"}});
    assert!(wire.validate().is_err());
}
#[test]
fn two_activity_diagrams_keep_independent_geometry_visibility_pinning_camera_and_layout_history() {
    let (e, a, _, n, _) = graph();
    let native = e.snapshot.clone();
    let mut v = Views::new(&e).unwrap();
    let first = diagram(&e, &mut v, a);
    let second = diagram(&e, &mut v, a);
    let stamp = v.stamp(&e);
    v.apply(
        &e,
        &stamp,
        Layout::Camera(first, v::Camera::checked(80., 40., 1.5).unwrap()),
    )
    .unwrap();
    let original = v.document().clone();
    let id = occ(&v, first, n);
    layout(
        &e,
        &mut v,
        va::Layout::Move {
            view: first,
            occurrence: id,
            geometry: v::Geometry::at(900., 700.),
        },
    )
    .unwrap();
    layout(
        &e,
        &mut v,
        va::Layout::Visible {
            view: first,
            occurrence: id,
            visible: false,
        },
    )
    .unwrap();
    layout(
        &e,
        &mut v,
        va::Layout::Pin {
            view: first,
            occurrence: id,
            pinned: true,
        },
    )
    .unwrap();
    let saved = v.document().clone();
    assert!(
        layout(
            &e,
            &mut v,
            va::Layout::Move {
                view: first,
                occurrence: id,
                geometry: v::Geometry::at(1., 2.)
            }
        )
        .is_err()
    );
    assert_eq!(v.document(), &saved);
    assert_eq!(
        v.document().activity_diagrams[&second],
        original.activity_diagrams[&second]
    );
    v.undo(&e, &v.stamp(&e), false).unwrap();
    assert!(!v.document().activity_diagrams[&first].occurrences[&id].pinned);
    v.undo(&e, &v.stamp(&e), true).unwrap();
    assert!(v.document().activity_diagrams[&first].occurrences[&id].pinned);
    assert_eq!(
        v.document().views[&second].camera,
        original.views[&second].camera
    );
    assert_eq!(e.snapshot, native);
}
#[test]
fn activity_flow_projection_requires_unique_visible_active_exact_endpoints_without_semantic_edits()
{
    let (e, a, start, n, _) = graph();
    let mut v = Views::new(&e).unwrap();
    let view = diagram(&e, &mut v, a);
    let native = e.snapshot.clone();
    assert_eq!(
        va::project(&v.document().activity_diagrams[&view], &e.snapshot)
            .unwrap()
            .routes
            .len(),
        2
    );
    layout(
        &e,
        &mut v,
        va::Layout::Reuse {
            view,
            element: start,
            geometry: v::Geometry::at(100., 500.),
        },
    )
    .unwrap();
    let p = va::project(&v.document().activity_diagrams[&view], &e.snapshot).unwrap();
    assert_eq!((p.routes.len(), p.unshown_flows), (1, 1));
    let id = occ(&v, view, n);
    layout(
        &e,
        &mut v,
        va::Layout::Visible {
            view,
            occurrence: id,
            visible: false,
        },
    )
    .unwrap();
    let p = va::project(&v.document().activity_diagrams[&view], &e.snapshot).unwrap();
    assert_eq!((p.routes.len(), p.unshown_flows), (0, 2));
    assert_eq!(e.snapshot, native);
}
#[test]
fn native_activity_removal_detaches_and_observed_inverse_reactivates_but_cold_data_has_no_proof() {
    let (mut e, a, _, n, _) = graph();
    let mut v = Views::new(&e).unwrap();
    let view = diagram(&e, &mut v, a);
    let id = occ(&v, view, n);
    {
        let stamp = v.stamp(&e);
        v.dispatch(
            &mut e,
            &stamp,
            NativeAction::Command(Command::Typed(t::Action::Activity(a::Action::Remove {
                activity: a,
                element: a,
            }))),
        )
        .unwrap();
    }
    assert!(
        !v.document().activity_diagrams[&view].occurrences[&id]
            .presence
            .active()
    );
    let bytes = v.recovery(&e).unwrap().encoded().unwrap();
    let mut cold = Views::restore(&bytes, &e).unwrap();
    {
        let stamp = cold.stamp(&e);
        cold.dispatch(&mut e, &stamp, NativeAction::ModelUndo)
            .unwrap();
    }
    assert!(
        !cold.document().activity_diagrams[&view].occurrences[&id]
            .presence
            .active()
    );
    {
        let stamp = v.stamp(&e);
        v.dispatch(&mut e, &stamp, NativeAction::ModelUndo).unwrap();
        assert!(
            !v.document().activity_diagrams[&view].occurrences[&id]
                .presence
                .active()
        );
    }
    let (mut e, a, _, n, _) = graph();
    let mut v = Views::new(&e).unwrap();
    let view = diagram(&e, &mut v, a);
    let id = occ(&v, view, n);
    let stamp = v.stamp(&e);
    v.dispatch(
        &mut e,
        &stamp,
        NativeAction::Command(Command::Typed(t::Action::Activity(a::Action::Remove {
            activity: a,
            element: a,
        }))),
    )
    .unwrap();
    let stamp = v.stamp(&e);
    v.dispatch(&mut e, &stamp, NativeAction::ModelUndo).unwrap();
    assert!(
        v.document().activity_diagrams[&view].occurrences[&id]
            .presence
            .active()
    );
}
#[test]
fn activity_profile6_cold_forms_diagrams_and_viewpoint_actions_preserve_all_maps_and_native_meaning()
 {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let (mut e, a, _, n, _) = graph();
    e = Editor::new(e.snapshot.clone(), session.editor.freshness().instance_id).unwrap();
    let mut v = Views::new(&e).unwrap();
    let view = diagram(&e, &mut v, a);
    let mut proposal = v::viewpoints::Form::new(&e);
    proposal.name = "Activity reviewer".into();
    v.apply(
        &e,
        &v.stamp(&e),
        Layout::Viewpoint(view, v::viewpoints::Action::SetForm(proposal)),
    )
    .unwrap();
    assert_eq!(v.document().profile, va::PROFILE);
    set(
        &mut e,
        Form::Node {
            activity: a,
            id: Some(n),
            name: "Unfinished node".into(),
            node_kind: a::NodeKind::Action,
            partition: "?".into(),
        },
    );
    let expected = e.snapshot.clone();
    let recovery = v.recovery(&e).unwrap();
    assert_eq!(recovery.profile, va::RECOVERY_PROFILE);
    session.editor = e;
    let path = area.path().join("native.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let cold = Session::open(area.path(), &path).unwrap();
    assert_eq!(cold.editor.snapshot, expected);
    let checkpoint = mbse_model::native_views_io::accepted_native(&session).unwrap();
    let bound = v.freeze(&session.editor, &checkpoint).unwrap();
    assert_eq!(bound.profile, va::PROFILE);
    let decoded = Views::decode(&bound.encoded().unwrap(), &session.editor, &checkpoint).unwrap();
    assert_eq!(decoded.document(), &bound);
    let restored = Views::restore(&recovery.encoded().unwrap(), &cold.editor).unwrap();
    assert_eq!(restored.document(), &recovery);
    assert_eq!(
        restored.document().activity_diagrams,
        v.document().activity_diagrams
    );
    assert!(!restored.can_layout_history(false));
}
#[test]
fn activity_view_wire_legacy_alias_foreign_active_missing_kind_and_copy_routes_refuse() {
    let (e, a, _, n, _) = graph();
    let mut v = Views::new(&e).unwrap();
    let view = diagram(&e, &mut v, a);
    let id = occ(&v, view, n);
    let d = v.recovery(&e).unwrap();
    for mode in 0..5 {
        let mut value = serde_json::to_value(&d).unwrap();
        match mode {
            0 => value["profile"] = json!(v::RECOVERY_PROFILE),
            1 => {
                value["activity_diagrams"][view.get().to_string()]["source_document"] =
                    json!("foreign")
            }
            2 => {
                value["activity_diagrams"][view.get().to_string()]["occurrences"]
                    [id.get().to_string()]["element"] = json!("01")
            }
            3 => {
                value["activity_diagrams"][view.get().to_string()]["occurrences"]
                    [id.get().to_string()]["expected"] = json!({"Node":"Decision"})
            }
            _ => {
                value["activity_diagrams"][view.get().to_string()]["provider"] =
                    json!("unsupported")
            }
        };
        assert!(Views::restore(&serde_json::to_vec(&value).unwrap(), &e).is_err());
    }
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let path = area.path().join("source.mbse-views.json");
    std::fs::write(&path, d.encoded().unwrap()).unwrap();
    let original = std::fs::read(&path).unwrap();
    assert!(Inspection::open(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let target = Views::new(&e).unwrap();
    let native = target.recovery(&e).unwrap();
    let path2 = area.path().join("native.mbse-views.json");
    std::fs::write(&path2, native.encoded().unwrap()).unwrap();
    let source2 = Inspection::open(&path2).unwrap();
    assert!(Plan::new(&source2, native.primary, &e, &v).is_err());
}
#[test]
fn distinct_existing_flow_invalid_guards_survive_selection_and_exact_native_cold_open() {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let (mut e, a, _, _, _) = graph();
    let accepted = t::load(&e.snapshot).unwrap();
    let mut keys = Vec::new();
    for flow in accepted.activities[&a].flows.values() {
        keys.push(set(
            &mut e,
            Form::Flow {
                form: a::FlowForm {
                    activity: a,
                    id: Some(flow.id),
                    name: flow.name.clone(),
                    kind: flow.kind,
                    source: flow.source.0.to_string(),
                    target: flow.target.0.to_string(),
                    guard: format!("unfinished guard {} ?", flow.id.0),
                },
            },
        ));
    }
    let raw = f::load(&e.snapshot).unwrap();
    for key in keys {
        let before = e.snapshot.clone();
        assert!(apply(&mut e, &key).is_err());
        assert_eq!(e.snapshot, before);
    }
    assert_eq!(t::load(&e.snapshot).unwrap(), accepted);
    let mut session = Session::new(area.path()).unwrap();
    session.editor = e;
    let expected = session.editor.snapshot.clone();
    let path = area.path().join("two-flow-raw.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let cold = Session::open(area.path(), &path).unwrap();
    assert_eq!(cold.editor.snapshot, expected);
    assert_eq!(f::load(&cold.editor.snapshot).unwrap(), raw);
}
#[test]
fn keyed_form_count_limit_refuses_new_text_without_losing_existing_detached_proposals() {
    let mut e = editor();
    e.snapshot.next_id = 10000;
    for n in 2000..2128 {
        set(
            &mut e,
            Form::Activity {
                id: Some(Id(n)),
                system: "?".into(),
                name: format!("Retained detached proposal {n}"),
            },
        );
    }
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            a::Action::Form(f::Action::Set(Form::Activity {
                id: Some(Id(2128)),
                system: "?".into(),
                name: "over bound".into()
            }))
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    assert_eq!(f::load(&e.snapshot).unwrap().len(), 128);
}
#[test]
fn activity_v6_coexists_with_typed_ports_connectors_and_viewpoints_without_downgrade() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p28/fixtures/native.kernspan"
    ))
    .unwrap();
    let mut e = Editor::new(
        serde_json::from_value(fixture["snapshot"].clone()).unwrap(),
        "activity:typed".into(),
    )
    .unwrap();
    let a = activity(&mut e);
    node(&mut e, a, a::NodeKind::Action);
    let mut doc: v::Document = serde_json::from_str(include_str!(
        "../../../docs/parity/p28/fixtures/native.mbse-views.json"
    ))
    .unwrap();
    doc.profile = v::connectors::RECOVERY_PROFILE.into();
    doc.binding = None;
    let mut v = Views::restore(&doc.encoded().unwrap(), &e).unwrap();
    diagram(&e, &mut v, a);
    let original = v.document().activity_diagrams.clone();
    let native = e.snapshot.clone();
    let view = v.document().primary;
    for (port, usage, x) in [(1017, 1009, 700.), (1015, 1010, 900.)] {
        v.apply(
            &e,
            &v.stamp(&e),
            Layout::Port(v::ports::Layout::ReuseInUsage {
                view,
                port: Id(port),
                usage: Id(usage),
                geometry: v::Geometry::at(x, 700.),
            }),
        )
        .unwrap();
        assert_eq!(v.document().profile, va::RECOVERY_PROFILE);
    }
    let ids: Vec<_> = v
        .document()
        .typed_ports
        .keys()
        .rev()
        .take(2)
        .copied()
        .collect();
    v.apply(
        &e,
        &v.stamp(&e),
        Layout::Connector(v::connectors::Layout::Route {
            view,
            connector: Id(1018),
            source: ids[1],
            target: ids[0],
        }),
    )
    .unwrap();
    let mut form = v::viewpoints::Form::new(&e);
    form.name = "Typed reviewer".into();
    for action in [
        v::viewpoints::Action::SetForm(form),
        v::viewpoints::Action::Apply,
    ] {
        v.apply(&e, &v.stamp(&e), Layout::Viewpoint(view, action))
            .unwrap();
        assert_eq!(v.document().profile, va::RECOVERY_PROFILE);
    }
    assert_eq!(v.document().activity_diagrams, original);
    assert_eq!(e.snapshot, native);
    let recovered = v.recovery(&e).unwrap();
    assert_eq!(
        Views::restore(&recovered.encoded().unwrap(), &e)
            .unwrap()
            .document(),
        &recovered
    );
}
#[test]
fn applying_creation_at_form_quota_reuses_its_slot_without_transient_overflow_or_raw_loss() {
    let (mut e, a, start, _, end) = graph();
    e.snapshot.next_id = 10000;
    let initial = f::load(&e.snapshot).unwrap().len();
    for n in 2000..2000 + (127 - initial) as u64 {
        set(
            &mut e,
            Form::Activity {
                id: Some(Id(n)),
                system: "?".into(),
                name: format!("Retained detached {n}"),
            },
        );
    }
    let key = set(
        &mut e,
        Form::Flow {
            form: a::FlowForm {
                activity: a,
                id: None,
                name: "Direct control".into(),
                kind: a::FlowKind::Control,
                source: start.0.to_string(),
                target: end.0.to_string(),
                guard: "".into(),
            },
        },
    );
    let before = f::load(&e.snapshot).unwrap();
    assert_eq!(before.len(), 128);
    let created = Id(e.snapshot.next_id);
    apply(&mut e, &key).unwrap();
    let after = f::load(&e.snapshot).unwrap();
    assert_eq!(after.len(), 128);
    assert!(!after.contains_key(&key));
    for (k, v) in before {
        if k != key {
            assert_eq!(after[&k], v);
        }
    }
    assert!(
        t::load(&e.snapshot).unwrap().activities[&a]
            .flows
            .contains_key(&created)
    );
    assert_eq!(e.snapshot.next_id, created.0 + 1);
}
