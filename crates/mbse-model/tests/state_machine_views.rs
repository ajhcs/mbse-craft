use mbse_model::{
    native_views::{
        self as v, state_machines as p, Layout, NativeAction, OccurrenceId, ViewId, Views,
    },
    native_views_inspect::Inspection,
    native_views_plan::Plan,
    stage1::*,
    stage1_io::{execute_write, Session},
    types::{
        self as t,
        state_machines::{self as sm, Action, Form, StateKind},
        Id,
    },
};
fn editor() -> Editor {
    let root: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p31b/verification/native-01/cold/execution-cold-copy.kernspan"
    ))
    .unwrap();
    Editor::new(
        serde_json::from_value(root["snapshot"].clone()).unwrap(),
        "state-machine:views-runtime".into(),
    )
    .unwrap()
}
fn command(e: &mut Editor, a: Action) {
    e.command(&e.freshness(), Command::Typed(t::Action::StateMachine(a)))
        .unwrap();
}
fn author(e: &mut Editor, f: Form) -> Id {
    let id = f.id().unwrap_or(Id(e.snapshot.next_id));
    let key = f.key();
    command(e, Action::Set(f));
    command(e, Action::Apply(key));
    id
}
fn graph() -> (Editor, Id, Id, Id, Id, Id) {
    let mut e = editor();
    let m = author(
        &mut e,
        Form::Machine {
            id: None,
            name: "State controller".into(),
            system: "1".into(),
        },
    );
    let mut nodes = Vec::new();
    for kind in [StateKind::Initial, StateKind::State, StateKind::Final] {
        nodes.push(author(
            &mut e,
            Form::State {
                machine: m,
                id: None,
                name: format!("{kind:?}"),
                state_kind: kind,
            },
        ));
    }
    let event = author(
        &mut e,
        Form::Event {
            machine: m,
            id: None,
            name: "Go".into(),
        },
    );
    for (source, target, ev, guard) in [
        (nodes[0], nodes[1], "".into(), ""),
        (nodes[1], nodes[2], event.0.to_string(), "true"),
    ] {
        author(
            &mut e,
            Form::Transition {
                machine: m,
                id: None,
                name: "Transition".into(),
                source: source.0.to_string(),
                target: target.0.to_string(),
                event: ev,
                guard: guard.into(),
            },
        );
    }
    (e, m, nodes[0], nodes[1], nodes[2], event)
}
fn views(e: &Editor) -> Views {
    let mut d: v::Document = serde_json::from_str(include_str!(
        "../../../docs/parity/p31b/verification/native-01/cold/execution-cold-copy.mbse-views.json"
    ))
    .unwrap();
    d.profile = v::activities::RECOVERY_PROFILE.into();
    d.binding = None;
    Views::restore(&d.encoded().unwrap(), e).unwrap()
}
fn vid(n: u64) -> ViewId {
    serde_json::from_str(&format!("\"{n}\"")).unwrap()
}
fn oid(n: u64) -> OccurrenceId {
    serde_json::from_str(&format!("\"{n}\"")).unwrap()
}
fn layout(e: &Editor, v: &mut Views, a: p::Layout) -> Result<(), String> {
    v.apply(e, &v.stamp(e), Layout::StateMachine(a))
}
fn diagram(e: &Editor, v: &mut Views, m: Id) -> ViewId {
    let id = vid(v.document().next_id());
    layout(
        e,
        v,
        p::Layout::Create {
            machine: m,
            title: "State machine".into(),
        },
    )
    .unwrap();
    id
}
fn occurrence(v: &Views, view: ViewId, element: Id) -> OccurrenceId {
    v.document().state_machine_diagrams[&view]
        .occurrences
        .values()
        .find(|o| o.element == element)
        .unwrap()
        .id
}
#[test]
fn state_machine_diagrams_project_exact_events_guards_and_keep_independent_geometry_history_and_native(
) {
    let (e, m, _, state, _, event) = graph();
    let mut v = views(&e);
    let before = e.snapshot.clone();
    let original = v.document().clone();
    let a = diagram(&e, &mut v, m);
    let b = diagram(&e, &mut v, m);
    let id = occurrence(&v, b, state);
    let geometry = v.document().state_machine_diagrams[&b].occurrences[&id]
        .geometry
        .clone();
    let shifted = v::Geometry::at(800., 500.);
    layout(
        &e,
        &mut v,
        p::Layout::Move {
            view: b,
            occurrence: id,
            geometry: shifted.clone(),
        },
    )
    .unwrap();
    assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
    assert_eq!(
        v.document().state_machine_diagrams[&a]
            .occurrences
            .values()
            .find(|o| o.element == state)
            .unwrap()
            .geometry,
        geometry
    );
    let projection = p::project(&v.document().state_machine_diagrams[&b], &e.snapshot).unwrap();
    assert!(projection.machine_resolved);
    assert_eq!(projection.routes.len(), 2);
    assert!(projection
        .routes
        .iter()
        .any(|r| r.event == Some(event) && r.guard == Some(sm::Guard::Boolean(true))));
    v.undo(&e, &v.stamp(&e), false).unwrap();
    assert_eq!(
        v.document().state_machine_diagrams[&b].occurrences[&id].geometry,
        geometry
    );
    v.undo(&e, &v.stamp(&e), true).unwrap();
    assert_eq!(
        v.document().state_machine_diagrams[&b].occurrences[&id].geometry,
        shifted
    );
    assert_eq!(e.snapshot, before);
    assert_eq!(v.document().occurrences, original.occurrences);
    assert_eq!(v.document().activity_diagrams, original.activity_diagrams);
    assert_eq!(v.document().typed_ports, original.typed_ports);
    assert_eq!(v.document().connector_edges, original.connector_edges);
}
#[test]
fn state_machine_unique_visible_endpoint_projection_pin_visibility_and_stale_layout_refuse_without_native_write(
) {
    let (mut e, m, _, state, _, _) = graph();
    let mut v = views(&e);
    let view = diagram(&e, &mut v, m);
    let id = occurrence(&v, view, state);
    layout(
        &e,
        &mut v,
        p::Layout::Reuse {
            view,
            element: state,
            geometry: v::Geometry::at(10., 500.),
        },
    )
    .unwrap();
    assert!(
        p::project(&v.document().state_machine_diagrams[&view], &e.snapshot)
            .unwrap()
            .routes
            .is_empty()
    );
    let copy = *v.document().state_machine_diagrams[&view]
        .occurrences
        .keys()
        .next_back()
        .unwrap();
    layout(
        &e,
        &mut v,
        p::Layout::Visible {
            view,
            occurrence: copy,
            visible: false,
        },
    )
    .unwrap();
    assert_eq!(
        p::project(&v.document().state_machine_diagrams[&view], &e.snapshot)
            .unwrap()
            .routes
            .len(),
        2
    );
    layout(
        &e,
        &mut v,
        p::Layout::Pin {
            view,
            occurrence: id,
            pinned: true,
        },
    )
    .unwrap();
    let before = v.document().clone();
    assert!(layout(
        &e,
        &mut v,
        p::Layout::Move {
            view,
            occurrence: id,
            geometry: v::Geometry::at(0., 0.)
        }
    )
    .is_err());
    assert_eq!(v.document(), &before);
    let stale = v.stamp(&e);
    command(
        &mut e,
        Action::Set(Form::Machine {
            id: Some(m),
            system: "1".into(),
            name: "New raw".into(),
        }),
    );
    assert!(v
        .apply(
            &e,
            &stale,
            Layout::StateMachine(p::Layout::Visible {
                view,
                occurrence: id,
                visible: false
            })
        )
        .is_err());
    assert_eq!(v.document(), &before);
}
#[test]
fn state_machine_model_root_removal_detaches_and_observed_inverse_alone_restores_exact_occurrences()
{
    let (mut e, m, _, _, _, _) = graph();
    let mut v = views(&e);
    let view = diagram(&e, &mut v, m);
    let original = v.document().state_machine_diagrams[&view].clone();
    let raw = sm::forms(&e.snapshot).unwrap();
    let stamp = v.stamp(&e);
    v.dispatch(
        &mut e,
        &stamp,
        NativeAction::Command(Command::Typed(t::Action::StateMachine(Action::Remove {
            machine: m,
            element: m,
        }))),
    )
    .unwrap();
    assert!(v.document().state_machine_diagrams[&view]
        .occurrences
        .values()
        .all(|o| !o.presence.active()));
    assert!(
        !p::project(&v.document().state_machine_diagrams[&view], &e.snapshot)
            .unwrap()
            .machine_resolved
    );
    let stamp = v.stamp(&e);
    v.dispatch(&mut e, &stamp, NativeAction::ModelUndo).unwrap();
    assert_eq!(v.document().state_machine_diagrams[&view], original);
    assert_eq!(sm::forms(&e.snapshot).unwrap(), raw);
    let stamp = v.stamp(&e);
    v.dispatch(&mut e, &stamp, NativeAction::ModelRedo).unwrap();
    assert!(v.document().state_machine_diagrams[&view]
        .occurrences
        .values()
        .all(|o| !o.presence.active()));
}
#[test]
fn state_machine_kind_change_detaches_and_exact_inverse_reactivates_without_reclassifying_other_states(
) {
    let (mut e, m, _, _, _, _) = graph();
    let spare = author(
        &mut e,
        Form::State {
            machine: m,
            id: None,
            name: "Spare".into(),
            state_kind: StateKind::State,
        },
    );
    let mut v = views(&e);
    let view = diagram(&e, &mut v, m);
    let id = occurrence(&v, view, spare);
    let key = Form::State {
        machine: m,
        id: Some(spare),
        name: "Spare".into(),
        state_kind: StateKind::Final,
    };
    command(&mut e, Action::Set(key.clone()));
    let stamp = v.stamp(&e);
    v.dispatch(
        &mut e,
        &stamp,
        NativeAction::Command(Command::Typed(t::Action::StateMachine(Action::Apply(
            key.key(),
        )))),
    )
    .unwrap();
    assert!(!v.document().state_machine_diagrams[&view].occurrences[&id]
        .presence
        .active());
    assert_eq!(
        v.document().state_machine_diagrams[&view]
            .occurrences
            .values()
            .filter(|o| o.presence.active())
            .count(),
        3
    );
    let stamp = v.stamp(&e);
    v.dispatch(&mut e, &stamp, NativeAction::ModelUndo).unwrap();
    assert!(v.document().state_machine_diagrams[&view].occurrences[&id]
        .presence
        .active());
}
#[test]
fn state_machine_cold_detached_data_cannot_mint_inverse_history_authority() {
    let (mut e, m, _, _, _, _) = graph();
    let mut v = views(&e);
    let view = diagram(&e, &mut v, m);
    let stamp = v.stamp(&e);
    v.dispatch(
        &mut e,
        &stamp,
        NativeAction::Command(Command::Typed(t::Action::StateMachine(Action::Remove {
            machine: m,
            element: m,
        }))),
    )
    .unwrap();
    let data = v.recovery(&e).unwrap().encoded().unwrap();
    let mut cold = Views::restore(&data, &e).unwrap();
    let stamp = cold.stamp(&e);
    cold.dispatch(&mut e, &stamp, NativeAction::ModelUndo)
        .unwrap();
    assert!(t::load(&e.snapshot)
        .unwrap()
        .state_machines
        .contains_key(&m));
    assert!(cold.document().state_machine_diagrams[&view]
        .occurrences
        .values()
        .all(|o| !o.presence.active()));
    assert_eq!(
        p::project(&cold.document().state_machine_diagrams[&view], &e.snapshot)
            .unwrap()
            .unshown_transitions,
        2
    );
}
#[test]
fn state_machine_profile7_schema_ids_foreign_binding_mixed_shapes_and_legacy_protocols_refuse() {
    let (e, m, _, state, _, _) = graph();
    let mut v = views(&e);
    let view = diagram(&e, &mut v, m);
    let raw = serde_json::to_value(v.recovery(&e).unwrap()).unwrap();
    for version in 1..=6 {
        let mut value = raw.clone();
        value["profile"] = serde_json::json!(format!("mbse-craft.native-views-recovery/{version}"));
        assert!(Views::restore(&serde_json::to_vec(&value).unwrap(), &e).is_err());
    }
    let id = occurrence(&v, view, state);
    for mutation in 0..5 {
        let mut value = raw.clone();
        let d = &mut value["state_machine_diagrams"][view.get().to_string()];
        match mutation {
            0 => d["source_document"] = serde_json::json!("foreign:document"),
            1 => d["occurrences"][id.get().to_string()]["element"] = serde_json::json!("07000"),
            2 => {
                d["occurrences"][id.get().to_string()]["geometry"]["width"] = serde_json::json!(-1)
            }
            3 => d["unknown"] = serde_json::json!(true),
            _ => d["machine"] = serde_json::json!("1"),
        };
        assert!(Views::restore(&serde_json::to_vec(&value).unwrap(), &e).is_err());
    }
    let before = v.document().clone();
    assert!(v
        .apply(
            &e,
            &v.stamp(&e),
            Layout::Reuse {
                view,
                record: RecordId(1),
                geometry: v::Geometry::at(0., 0.)
            }
        )
        .is_err());
    assert_eq!(v.document(), &before);
    let mut value = raw.clone();
    let n = value["occurrences"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let stolen = value["state_machine_diagrams"][view.get().to_string()]["occurrences"]
        .as_object_mut()
        .unwrap()
        .remove(&id.get().to_string())
        .unwrap();
    value["state_machine_diagrams"][view.get().to_string()]["occurrences"][&n] = stolen;
    value["state_machine_diagrams"][view.get().to_string()]["occurrences"][&n]["id"] =
        serde_json::json!(n);
    assert!(Views::restore(&serde_json::to_vec(&value).unwrap(), &e).is_err());
}
#[test]
fn state_machine_profile7_survives_native_ports_connectors_viewpoints_activity_creation_and_clear()
{
    let (e, m, _, _, _, _) = graph();
    let mut v = views(&e);
    let view = diagram(&e, &mut v, m);
    let original = v.document().state_machine_diagrams.clone();
    let primary = v.document().primary;
    let native = e.snapshot.clone();
    for (port, usage, x) in [(1017, 1009, 700.), (1015, 1010, 900.)] {
        v.apply(
            &e,
            &v.stamp(&e),
            Layout::Port(v::ports::Layout::ReuseInUsage {
                view: primary,
                port: Id(port),
                usage: Id(usage),
                geometry: v::Geometry::at(x, 700.),
            }),
        )
        .unwrap();
        assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
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
            view: primary,
            connector: Id(1018),
            source: ids[1],
            target: ids[0],
        }),
    )
    .unwrap();
    let mut form = v::viewpoints::Form::new(&e);
    form.name = "State reviewer".into();
    for action in [
        v::viewpoints::Action::SetForm(form),
        v::viewpoints::Action::Apply,
    ] {
        v.apply(&e, &v.stamp(&e), Layout::Viewpoint(primary, action))
            .unwrap();
        assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
    }
    v.apply(
        &e,
        &v.stamp(&e),
        Layout::Activity(v::activities::Layout::Create {
            activity: Id(1033),
            title: "Another activity".into(),
        }),
    )
    .unwrap();
    assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
    assert_eq!(v.document().state_machine_diagrams, original);
    assert_eq!(e.snapshot, native);
    layout(&e, &mut v, p::Layout::Clear(view)).unwrap();
    assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
    v.undo(&e, &v.stamp(&e), false).unwrap();
    assert_eq!(v.document().state_machine_diagrams, original);
    v.apply(&e, &v.stamp(&e), Layout::CloseView(view)).unwrap();
    assert!(!v.document().state_machine_diagrams.contains_key(&view));
    assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
}
#[test]
fn state_machine_bound_native_checkpoint_and_unbound_capture_recovery_roundtrip_keep_all_presentation_maps(
) {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let (e, m, _, state, _, _) = graph();
    session.editor = Editor::new(e.snapshot, session.editor.freshness().instance_id).unwrap();
    let mut v = views(&session.editor);
    let view = diagram(&session.editor, &mut v, m);
    let id = occurrence(&v, view, state);
    layout(
        &session.editor,
        &mut v,
        p::Layout::Move {
            view,
            occurrence: id,
            geometry: v::Geometry::at(555., 333.),
        },
    )
    .unwrap();
    let native = session.editor.snapshot.clone();
    let native_path = area.path().join("state-diagram.kernspan");
    let write = session.begin_save(&native_path).unwrap();
    assert!(session.complete(execute_write(write)));
    let checkpoint = session.native_checkpoint().unwrap().unwrap();
    let bound = v.freeze(&session.editor, &checkpoint).unwrap();
    assert_eq!(bound.profile, p::PROFILE);
    let cold = Views::decode(&bound.encoded().unwrap(), &session.editor, &checkpoint).unwrap();
    assert_eq!(
        cold.document().state_machine_diagrams,
        v.document().state_machine_diagrams
    );
    assert_eq!(
        cold.document().activity_diagrams,
        v.document().activity_diagrams
    );
    assert_eq!(session.editor.snapshot, native);
    let mut io = mbse_model::native_views_io::Io::default();
    let recovery_path = area.path().join("state-views-recovery.json");
    let destination = io
        .freeze_destination(
            &session,
            &v,
            &recovery_path,
            mbse_model::native_views_io::Kind::Recovery,
        )
        .unwrap();
    let captured = v.capture(&session.editor);
    let write = io
        .begin_captured(&session, &v, captured, destination)
        .unwrap();
    assert!(io.complete(&session, &v, mbse_model::native_views_io::execute(write)));
    let recovered: v::Document =
        serde_json::from_slice(&std::fs::read(&recovery_path).unwrap()).unwrap();
    assert_eq!(recovered.profile, p::RECOVERY_PROFILE);
    assert_eq!(
        Views::restore(&recovered.encoded().unwrap(), &session.editor)
            .unwrap()
            .document(),
        &recovered
    );
    assert!(!cold.can_layout_history(false));
}
#[test]
fn state_machine_legacy_inspection_and_copy_refuse_before_partial_inventory_and_preserve_source_bytes(
) {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let (e, m, _, _, _, _) = graph();
    let mut target = views(&e);
    diagram(&e, &mut target, m);
    let path = area.path().join("machine-views.json");
    let raw = target.recovery(&e).unwrap().encoded().unwrap();
    std::fs::write(&path, &raw).unwrap();
    assert!(Inspection::open(&path)
        .err()
        .unwrap()
        .contains("STATE_MACHINE_SOURCE_INSPECTION_UNSUPPORTED"));
    assert_eq!(std::fs::read(&path).unwrap(), raw);
    let plain = Views::new(&e).unwrap();
    let source_path = area.path().join("plain.json");
    let original = plain.recovery(&e).unwrap().encoded().unwrap();
    std::fs::write(&source_path, &original).unwrap();
    let inspected = Inspection::open(&source_path).unwrap();
    assert!(Plan::new(&inspected, plain.document().primary, &e, &target)
        .err()
        .unwrap()
        .contains("STATE_MACHINE_COPY_UNSUPPORTED"));
    assert_eq!(std::fs::read(&source_path).unwrap(), original);
}
#[test]
fn state_machine_joint_presentation_and_per_view_limits_bound_duplicate_occurrences_before_admission(
) {
    let (e, m, _, state, _, _) = graph();
    let mut v = views(&e);
    let view = diagram(&e, &mut v, m);
    let mut d = v.recovery(&e).unwrap();
    let template = d.state_machine_diagrams[&view].clone();
    let original_occurrence = template.occurrences[&occurrence(&v, view, state)].clone();
    let count = d.occurrences.len()
        + d.typed_ports.len()
        + d.activity_diagrams
            .values()
            .map(|d| d.occurrences.len())
            .sum::<usize>()
        + d.state_machine_diagrams
            .values()
            .map(|d| d.occurrences.len())
            .sum::<usize>();
    let mut remaining = v::MAX_OCCURRENCES - count;
    let mut next = 2000;
    let mut added_views = Vec::new();
    for index in 0..4 {
        let id = vid(1000 + index);
        added_views.push(id);
        d.views.insert(
            id,
            v::View {
                id,
                title: "Duplicate-boundary".into(),
                camera: Default::default(),
                copy_origin: None,
                viewpoint: None,
                viewpoint_form: None,
            },
        );
        let mut diagram = template.clone();
        diagram.occurrences.clear();
        for _ in 0..remaining.min(v::MAX_PER_VIEW) {
            let mut o = original_occurrence.clone();
            o.id = oid(next);
            next += 1;
            diagram.occurrences.insert(o.id, o);
        }
        remaining = remaining.saturating_sub(v::MAX_PER_VIEW);
        d.state_machine_diagrams.insert(id, diagram);
    }
    assert_eq!(remaining, 0);
    d.next_id = serde_json::from_str("\"50000\"").unwrap();
    let encoded = d.encoded().unwrap();
    Views::restore(&encoded, &e).unwrap();
    let mut over = d.clone();
    let mut o = original_occurrence.clone();
    o.id = oid(next);
    over.state_machine_diagrams
        .get_mut(added_views.last().unwrap())
        .unwrap()
        .occurrences
        .insert(o.id, o);
    assert!(Views::restore(&over.encoded().unwrap(), &e).is_err());
    let mut per_view = d;
    let from = *added_views.last().unwrap();
    let transfer = per_view
        .state_machine_diagrams
        .get_mut(&from)
        .unwrap()
        .occurrences
        .pop_last()
        .unwrap();
    per_view
        .state_machine_diagrams
        .get_mut(&added_views[0])
        .unwrap()
        .occurrences
        .insert(transfer.0, transfer.1);
    assert!(Views::restore(&per_view.encoded().unwrap(), &e).is_err());
}
