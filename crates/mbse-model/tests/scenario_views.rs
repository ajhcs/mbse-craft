use mbse_model::{
    native_views::{self as v, Layout, NativeAction, OccurrenceId, ViewId, Views, scenarios as p},
    stage1::*,
    stage1_io::{Session, execute_write},
    types::{
        self as t, Id,
        scenarios::{self as s, Action, Form, MessageKind as K, ParticipantKind as P},
    },
};
use serde_json::{Value, json};
fn author(e: &mut Editor, f: Form) -> Id {
    let id = f.id().unwrap_or(Id(e.snapshot.next_id));
    let key = f.key();
    for a in [Action::Set(f), Action::Apply(key)] {
        e.command(&e.freshness(), Command::Typed(t::Action::Scenario(a)))
            .unwrap();
    }
    id
}
fn line(e: &mut Editor, a: Id, target: Id, expected: P) -> Id {
    author(
        e,
        Form::Lifeline {
            scenario: a,
            id: None,
            name: format!("Lifeline{}", target.0),
            participant: target.0.to_string(),
            expected,
        },
    )
}
fn message(
    e: &mut Editor,
    a: Id,
    x: Id,
    y: Id,
    order: u32,
    operand: Option<Id>,
    kind: K,
    reply: Option<Id>,
) -> Id {
    author(
        e,
        Form::Message {
            scenario: a,
            id: None,
            name: "Review exchange".into(),
            source: x.0.to_string(),
            target: y.0.to_string(),
            order: order.to_string(),
            operand: operand.map_or(String::new(), |i| i.0.to_string()),
            message_kind: kind,
            reply_to: reply.map_or(String::new(), |i| i.0.to_string()),
        },
    )
}
struct Graph {
    e: Editor,
    a: Id,
    x: Id,
    y: Id,
    z: Id,
    actor: RecordId,
    component: Id,
    call: Id,
    reply: Id,
    alt: Id,
    yes: Id,
    no: Id,
    yes_msg: Id,
    no_msg: Id,
}
fn graph() -> Graph {
    let root: Value = serde_json::from_slice(include_bytes!(
        "../../../docs/parity/p33b/verification/native-02/run/state-execution-cold-copy.kernspan"
    ))
    .unwrap();
    let mut e = Editor::new(
        serde_json::from_value(root["snapshot"].clone()).unwrap(),
        "sequence:runtime".into(),
    )
    .unwrap();
    let component = Id(e.snapshot.next_id);
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::AddDefinition {
            system: RecordId(1),
            name: "Scenario participant".into(),
            kind: t::Kind::Component,
        }),
    )
    .unwrap();
    let actor = e
        .command(
            &e.freshness(),
            Command::Add {
                kind: RecordKind::Actor,
                owner: RecordId(1),
                name: "Reviewer".into(),
            },
        )
        .unwrap()
        .created
        .unwrap();
    let a = author(
        &mut e,
        Form::Scenario {
            id: None,
            system: "1".into(),
            name: "Review sequence".into(),
        },
    );
    let x = line(&mut e, a, component, P::Component);
    let y = line(&mut e, a, Id(1009), P::Usage(t::UsageKind::Part));
    let z = line(&mut e, a, Id(actor.0), P::Native(RecordKind::Actor));
    let call = message(&mut e, a, x, y, 1, None, K::Synchronous, None);
    let reply = message(&mut e, a, y, x, 2, None, K::Reply, Some(call));
    let alt = author(
        &mut e,
        Form::Alternative {
            scenario: a,
            id: None,
            name: "Review outcome".into(),
            order: "3".into(),
        },
    );
    let yes = author(
        &mut e,
        Form::Operand {
            scenario: a,
            id: None,
            name: "Approve".into(),
            alternative: alt.0.to_string(),
            order: "1".into(),
            guard: "temperature > 50 C".into(),
        },
    );
    let no = author(
        &mut e,
        Form::Operand {
            scenario: a,
            id: None,
            name: "Decline".into(),
            alternative: alt.0.to_string(),
            order: "2".into(),
            guard: "reviewer declines".into(),
        },
    );
    let yes_msg = message(&mut e, a, y, z, 1, Some(yes), K::Asynchronous, None);
    let no_msg = message(&mut e, a, z, x, 1, Some(no), K::Asynchronous, None);
    Graph {
        e,
        a,
        x,
        y,
        z,
        actor,
        component,
        call,
        reply,
        alt,
        yes,
        no,
        yes_msg,
        no_msg,
    }
}
fn views(e: &Editor) -> Views {
    let mut d:v::Document=serde_json::from_slice(include_bytes!("../../../docs/parity/p33b/verification/native-02/run/state-execution-cold-copy.mbse-views.json")).unwrap();
    d.profile = v::state_machines::RECOVERY_PROFILE.into();
    d.binding = None;
    Views::restore(&d.encoded().unwrap(), e).unwrap()
}
fn vid(n: u64) -> ViewId {
    serde_json::from_str(&format!("\"{n}\"")).unwrap()
}
fn layout(e: &Editor, v: &mut Views, a: p::Layout) -> Result<(), String> {
    let stamp = v.stamp(e);
    v.apply(e, &stamp, Layout::Scenario(a))
}
fn diagram(e: &Editor, v: &mut Views, a: Id) -> ViewId {
    let id = vid(v.document().next_id());
    layout(
        e,
        v,
        p::Layout::Create {
            scenario: a,
            title: "Independent sequence".into(),
        },
    )
    .unwrap();
    id
}
fn occurrence(v: &Views, view: ViewId, l: Id) -> OccurrenceId {
    v.document().scenario_diagrams[&view]
        .occurrences
        .values()
        .find(|o| o.element == l)
        .unwrap()
        .id
}
fn native(e: &mut Editor, v: &mut Views, a: NativeAction) {
    let stamp = v.stamp(e);
    v.dispatch(e, &stamp, a).unwrap();
}
fn minimal() -> (Editor, Views, Id, Id, ViewId) {
    let mut n = Snapshot::new(DocumentId("sequence:minimal".into()));
    n.extensions = Some(Default::default());
    let mut e = Editor::new(n, "sequence:minimal-runtime".into()).unwrap();
    let a = author(
        &mut e,
        Form::Scenario {
            id: None,
            system: "1".into(),
            name: "Minimal sequence".into(),
        },
    );
    let l = line(&mut e, a, Id(1), P::Native(RecordKind::System));
    let mut v = Views::new(&e).unwrap();
    let view = diagram(&e, &mut v, a);
    (e, v, a, l, view)
}
#[test]
fn sequence_views_derive_exact_authored_rows_replies_and_guarded_operands_with_independent_geometry_cameras()
 {
    let g = graph();
    let mut v = views(&g.e);
    let before = g.e.snapshot.clone();
    let first = diagram(&g.e, &mut v, g.a);
    let second = diagram(&g.e, &mut v, g.a);
    let projection = p::project(&v.document().scenario_diagrams[&first], &g.e.snapshot).unwrap();
    assert_eq!(
        projection.rows,
        vec![
            p::Row::Message { message: g.call },
            p::Row::Message { message: g.reply },
            p::Row::AlternativeStart { alternative: g.alt },
            p::Row::OperandStart {
                alternative: g.alt,
                operand: g.yes
            },
            p::Row::Message { message: g.yes_msg },
            p::Row::OperandStart {
                alternative: g.alt,
                operand: g.no
            },
            p::Row::Message { message: g.no_msg },
            p::Row::AlternativeEnd { alternative: g.alt }
        ]
    );
    assert_eq!(
        projection
            .routes
            .iter()
            .map(|r| (r.message, r.row, r.kind, r.operand, r.reply_to))
            .collect::<Vec<_>>(),
        vec![
            (g.call, 0, K::Synchronous, None, None),
            (g.reply, 1, K::Reply, None, Some(g.call)),
            (g.yes_msg, 4, K::Asynchronous, Some(g.yes), None),
            (g.no_msg, 6, K::Asynchronous, Some(g.no), None)
        ]
    );
    assert_eq!(projection.unshown_messages, 0);
    assert!(projection.unresolved_lifelines.is_empty());
    assert_eq!(
        p::bounds(&v.document().scenario_diagrams[&second], &g.e.snapshot).unwrap(),
        Some([25., 24., 883., 904.])
    );
    let id = occurrence(&v, first, g.x);
    let old = v.document().scenario_diagrams[&first].occurrences[&id]
        .geometry
        .clone();
    layout(
        &g.e,
        &mut v,
        p::Layout::Move {
            view: first,
            occurrence: id,
            geometry: v::Geometry::at(777., 222.),
        },
    )
    .unwrap();
    let stamp = v.stamp(&g.e);
    v.apply(
        &g.e,
        &stamp,
        Layout::Camera(first, v::Camera::checked(64., 32., 1.25).unwrap()),
    )
    .unwrap();
    assert_eq!(
        v.document().scenario_diagrams[&second]
            .occurrences
            .values()
            .find(|o| o.element == g.x)
            .unwrap()
            .geometry,
        old
    );
    assert_eq!(v.document().views[&second].camera, v::Camera::default());
    let stamp = v.stamp(&g.e);
    v.undo(&g.e, &stamp, false).unwrap();
    let stamp = v.stamp(&g.e);
    v.undo(&g.e, &stamp, false).unwrap();
    assert_eq!(
        v.document().scenario_diagrams[&first].occurrences[&id].geometry,
        old
    );
    let stamp = v.stamp(&g.e);
    v.undo(&g.e, &stamp, true).unwrap();
    assert_eq!(g.e.snapshot, before);
}
#[test]
fn sequence_visibility_duplicate_endpoints_pin_and_stale_layout_guard_never_choose_an_occurrence_or_change_semantics()
 {
    let mut g = graph();
    let mut v = views(&g.e);
    let view = diagram(&g.e, &mut v, g.a);
    let id = occurrence(&v, view, g.x);
    let before = g.e.snapshot.clone();
    layout(
        &g.e,
        &mut v,
        p::Layout::Reuse {
            view,
            element: g.x,
            geometry: v::Geometry::at(1200., 24.),
        },
    )
    .unwrap();
    let duplicate = *v.document().scenario_diagrams[&view]
        .occurrences
        .keys()
        .next_back()
        .unwrap();
    let project = p::project(&v.document().scenario_diagrams[&view], &g.e.snapshot).unwrap();
    assert_eq!(project.routes.len(), 1);
    assert_eq!(project.routes[0].message, g.yes_msg);
    assert_eq!(project.unshown_messages, 3);
    layout(
        &g.e,
        &mut v,
        p::Layout::Visible {
            view,
            occurrence: duplicate,
            visible: false,
        },
    )
    .unwrap();
    assert_eq!(
        p::project(&v.document().scenario_diagrams[&view], &g.e.snapshot)
            .unwrap()
            .routes
            .len(),
        4
    );
    layout(
        &g.e,
        &mut v,
        p::Layout::Pin {
            view,
            occurrence: id,
            pinned: true,
        },
    )
    .unwrap();
    let original = v.document().clone();
    assert!(
        layout(
            &g.e,
            &mut v,
            p::Layout::Move {
                view,
                occurrence: id,
                geometry: v::Geometry::at(800., 30.)
            }
        )
        .unwrap_err()
        .contains("PINNED")
    );
    assert_eq!(v.document(), &original);
    assert_eq!(g.e.snapshot, before);
    let stamp = v.stamp(&g.e);
    g.e.command(
        &g.e.freshness(),
        Command::Typed(t::Action::Scenario(Action::Set(Form::Scenario {
            id: Some(g.a),
            system: "1".into(),
            name: "Unapplied source".into(),
        }))),
    )
    .unwrap();
    assert!(
        v.apply(&g.e, &stamp, Layout::Camera(view, v::Camera::default()))
            .is_err()
    );
    assert_eq!(v.document(), &original);
}
#[test]
fn sequence_root_removal_and_observed_model_inverse_detach_restore_exact_occurrences_and_newer_raw()
{
    let mut g = graph();
    let mut v = views(&g.e);
    let view = diagram(&g.e, &mut v, g.a);
    let original = v.document().scenario_diagrams.clone();
    native(
        &mut g.e,
        &mut v,
        NativeAction::Command(Command::Typed(t::Action::Scenario(Action::Set(
            Form::Message {
                scenario: g.a,
                id: Some(g.yes_msg),
                name: "Newer raw retained".into(),
                source: "?".into(),
                target: "".into(),
                order: "not ordered".into(),
                operand: g.yes.0.to_string(),
                message_kind: K::Asynchronous,
                reply_to: String::new(),
            },
        )))),
    );
    let raw = s::forms(&g.e.snapshot).unwrap();
    native(
        &mut g.e,
        &mut v,
        NativeAction::Command(Command::Typed(t::Action::Scenario(Action::Remove {
            scenario: g.a,
            element: g.a,
        }))),
    );
    assert!(
        v.document().scenario_diagrams[&view]
            .occurrences
            .values()
            .all(|o| !o.presence.active())
    );
    assert!(
        !p::project(&v.document().scenario_diagrams[&view], &g.e.snapshot)
            .unwrap()
            .scenario_resolved
    );
    native(&mut g.e, &mut v, NativeAction::ModelUndo);
    assert_eq!(v.document().scenario_diagrams, original);
    assert_eq!(s::forms(&g.e.snapshot).unwrap(), raw);
    assert_eq!(
        p::project(&v.document().scenario_diagrams[&view], &g.e.snapshot)
            .unwrap()
            .routes
            .len(),
        4
    );
    native(&mut g.e, &mut v, NativeAction::ModelRedo);
    assert!(
        v.document().scenario_diagrams[&view]
            .occurrences
            .values()
            .all(|o| !o.presence.active())
    );
    native(&mut g.e, &mut v, NativeAction::ModelUndo);
    assert_eq!(v.document().scenario_diagrams, original);
}
#[test]
fn sequence_cold_detached_occurrences_cannot_invent_inverse_proof_but_explicit_reuse_keeps_old_geometry()
 {
    let mut g = graph();
    let mut v = views(&g.e);
    let view = diagram(&g.e, &mut v, g.a);
    let before = g.e.snapshot.clone();
    let old = v.document().scenario_diagrams[&view].occurrences.clone();
    native(
        &mut g.e,
        &mut v,
        NativeAction::Command(Command::Typed(t::Action::Scenario(Action::Remove {
            scenario: g.a,
            element: g.a,
        }))),
    );
    let bytes = v.recovery(&g.e).unwrap().encoded().unwrap();
    let mut cold = Editor::new(before, "sequence:cold-runtime".into()).unwrap();
    let mut restored = Views::restore(&bytes, &cold).unwrap();
    native(&mut cold, &mut restored, NativeAction::ModelUndo);
    let p = p::project(
        &restored.document().scenario_diagrams[&view],
        &cold.snapshot,
    )
    .unwrap();
    assert_eq!(p.routes.len(), 0);
    assert_eq!(p.unshown_messages, 4);
    let native_before = cold.snapshot.clone();
    for l in [g.x, g.y, g.z] {
        layout(
            &cold,
            &mut restored,
            p::Layout::Reuse {
                view,
                element: l,
                geometry: v::Geometry::at(30. + l.0 as f64 % 100., 40.),
            },
        )
        .unwrap();
    }
    assert_eq!(
        p::project(
            &restored.document().scenario_diagrams[&view],
            &cold.snapshot
        )
        .unwrap()
        .routes
        .len(),
        4
    );
    for (id, o) in old {
        let retained = &restored.document().scenario_diagrams[&view].occurrences[&id];
        assert!(!retained.presence.active());
        assert_eq!(retained.geometry, o.geometry);
    }
    assert_eq!(cold.snapshot, native_before);
}
#[test]
fn sequence_missing_changed_and_foreign_participants_keep_headers_and_rows_but_suppress_exact_affected_arrows()
 {
    let mut g = graph();
    let mut v = views(&g.e);
    let view = diagram(&g.e, &mut v, g.a);
    let original = v.document().scenario_diagrams.clone();
    let actor = g.e.snapshot.records.remove(&g.actor).unwrap();
    g.e.snapshot.validate().unwrap();
    let p1 = p::project(&v.document().scenario_diagrams[&view], &g.e.snapshot).unwrap();
    assert_eq!(p1.unresolved_lifelines, vec![g.z]);
    assert_eq!(p1.routes.len(), 2);
    assert_eq!(p1.rows.len(), 8);
    let mut other = g.e.snapshot.records[&RecordId(1)].clone();
    other.id = RecordId(20000);
    other.scope = RecordId(20000);
    other.name = "Foreign system".into();
    g.e.snapshot.records.insert(other.id, other);
    let mut actor = actor;
    actor.scope = RecordId(20000);
    g.e.snapshot.records.insert(actor.id, actor);
    g.e.snapshot.next_id = 20100;
    g.e.snapshot.validate().unwrap();
    assert_eq!(
        p::project(&v.document().scenario_diagrams[&view], &g.e.snapshot)
            .unwrap()
            .unresolved_lifelines,
        vec![g.z]
    );
    g.e.snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .get_mut(t::NAMESPACE)
        .unwrap()["definitions"][g.component.0.to_string()]["kind"] = json!({"Value":"Real"});
    g.e.snapshot.validate().unwrap();
    let before = g.e.snapshot.clone();
    let projection = p::project(&v.document().scenario_diagrams[&view], &g.e.snapshot).unwrap();
    assert_eq!(projection.unresolved_lifelines, vec![g.x, g.z]);
    assert_eq!(projection.routes.len(), 0);
    assert_eq!(projection.unshown_messages, 4);
    assert_eq!(v.document().scenario_diagrams, original);
    assert!(
        v.document().scenario_diagrams[&view]
            .occurrences
            .values()
            .all(|o| o.presence.active())
    );
    assert_eq!(g.e.snapshot, before);
}
#[test]
fn sequence_profile8_wire_and_exclusive_lifeline_views_reject_legacy_fields_noncanonical_ids_and_mixed_payloads()
 {
    let (e, v, a, l, view) = minimal();
    let original = v.recovery(&e).unwrap().encoded().unwrap();
    for version in 1..8 {
        let mut value: Value = serde_json::from_slice(&original).unwrap();
        value["profile"] = json!(format!("mbse-craft.native-views-recovery/{version}"));
        assert!(
            Views::restore(&serde_json::to_vec(&value).unwrap(), &e)
                .err()
                .unwrap()
                .contains("SCENARIO_REQUIRES_V8")
        );
    }
    let occurrence = occurrence(&v, view, l);
    for mode in 0..12 {
        let mut value: Value = serde_json::from_slice(&original).unwrap();
        let root = &mut value["scenario_diagrams"][view.get().to_string()];
        match mode {
            0 => root["source_document"] = json!("foreign:document"),
            1 => {
                root["occurrences"][occurrence.get().to_string()]["element"] =
                    json!(a.0.to_string())
            }
            2 => root["occurrences"][occurrence.get().to_string()]["element"] = json!(l.0),
            3 => {
                root["occurrences"][occurrence.get().to_string()]["element"] =
                    json!(format!("0{}", l.0))
            }
            4 => root["occurrences"][occurrence.get().to_string()]["expected"] = json!("Message"),
            5 => root["occurrences"][occurrence.get().to_string()]["geometry"]["width"] = json!(0),
            6 => root["occurrences"][occurrence.get().to_string()]["geometry"]["x"] = json!(1e9),
            7 => {
                let d = value["scenario_diagrams"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&view.get().to_string())
                    .unwrap();
                value["scenario_diagrams"]["9999"] = d;
            }
            8 => {
                let mut d = root["occurrences"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&occurrence.get().to_string())
                    .unwrap();
                let native_id = v
                    .document()
                    .occurrences
                    .keys()
                    .next()
                    .unwrap()
                    .get()
                    .to_string();
                d["id"] = json!(native_id);
                root["occurrences"][native_id] = d;
            }
            9 => {
                let row = value["occurrences"]
                    .as_object_mut()
                    .unwrap()
                    .values_mut()
                    .next()
                    .unwrap();
                row["view"] = json!(view.get().to_string());
            }
            10 => {
                value["activity_diagrams"][view.get().to_string()] = json!({"source_document":e.snapshot.document_id,"activity":"1000","occurrences":{}})
            }
            11 => {
                value["state_machine_diagrams"][view.get().to_string()] = json!({"source_document":e.snapshot.document_id,"machine":"1000","occurrences":{}})
            }
            _ => unreachable!(),
        };
        assert!(
            Views::restore(&serde_json::to_vec(&value).unwrap(), &e).is_err(),
            "mode{mode}"
        );
    }
    assert_eq!(v.recovery(&e).unwrap().encoded().unwrap(), original);
}

#[test]
fn sequence_profile8_stays_monotonic_through_ports_connectors_viewpoints_activity_state_and_native_layouts()
 {
    let g = graph();
    let mut v = views(&g.e);
    let view = diagram(&g.e, &mut v, g.a);
    let original = v.document().scenario_diagrams.clone();
    let before = g.e.snapshot.clone();
    let primary = v.document().primary;
    for (port, usage, x) in [(1017, 1009, 700.), (1015, 1010, 900.)] {
        let stamp = v.stamp(&g.e);
        v.apply(
            &g.e,
            &stamp,
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
    let stamp = v.stamp(&g.e);
    v.apply(
        &g.e,
        &stamp,
        Layout::Connector(v::connectors::Layout::Route {
            view: primary,
            connector: Id(1018),
            source: ids[1],
            target: ids[0],
        }),
    )
    .unwrap();
    let mut form = v::viewpoints::Form::new(&g.e);
    form.name = "Sequence reviewer".into();
    for a in [
        v::viewpoints::Action::SetForm(form),
        v::viewpoints::Action::Apply,
    ] {
        let stamp = v.stamp(&g.e);
        v.apply(&g.e, &stamp, Layout::Viewpoint(primary, a))
            .unwrap();
        assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
    }
    let stamp = v.stamp(&g.e);
    v.apply(
        &g.e,
        &stamp,
        Layout::Activity(v::activities::Layout::Create {
            activity: Id(1033),
            title: "Activity alongside sequence".into(),
        }),
    )
    .unwrap();
    assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
    let stamp = v.stamp(&g.e);
    v.apply(
        &g.e,
        &stamp,
        Layout::StateMachine(v::state_machines::Layout::Create {
            machine: Id(7000),
            title: "State alongside sequence".into(),
        }),
    )
    .unwrap();
    assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
    let stamp = v.stamp(&g.e);
    v.apply(&g.e, &stamp, Layout::AddView("Another native view".into()))
        .unwrap();
    assert_eq!(v.document().scenario_diagrams, original);
    layout(&g.e, &mut v, p::Layout::Clear(view)).unwrap();
    assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
    let stamp = v.stamp(&g.e);
    v.undo(&g.e, &stamp, false).unwrap();
    assert_eq!(v.document().scenario_diagrams, original);
    let stamp = v.stamp(&g.e);
    v.apply(&g.e, &stamp, Layout::CloseView(view)).unwrap();
    assert_eq!(v.document().profile, p::RECOVERY_PROFILE);
    assert!(!v.document().scenario_diagrams.contains_key(&view));
    assert_eq!(g.e.snapshot, before);
}
#[test]
fn sequence_actual_bound_native_cold_and_captured_recovery_preserve_all_eight_maps_and_entire_snapshot()
 {
    use mbse_model::native_views_io as io;
    let g = graph();
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    session.editor = Editor::new(g.e.snapshot, session.editor.freshness().instance_id).unwrap();
    let mut v = views(&session.editor);
    let view = diagram(&session.editor, &mut v, g.a);
    let id = occurrence(&v, view, g.x);
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
    native(
        &mut session.editor,
        &mut v,
        NativeAction::Command(Command::Typed(t::Action::Scenario(Action::Set(
            Form::Message {
                scenario: g.a,
                id: Some(g.call),
                name: "Retained invalid call".into(),
                source: "?".into(),
                target: "".into(),
                order: "invalid".into(),
                operand: String::new(),
                message_kind: K::Synchronous,
                reply_to: String::new(),
            },
        )))),
    );
    let before = session.editor.snapshot.clone();
    let path = area.path().join("sequence.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let mut writes = io::Io::default();
    let side = area.path().join("sequence.mbse-views.json");
    let job = writes.begin(&session, &v, &side, io::Kind::Bound).unwrap();
    assert!(writes.complete(&session, &v, io::execute(job)));
    let cold = Session::open(area.path(), &path).unwrap();
    assert_eq!(cold.editor.snapshot, before);
    let decoded = Views::decode(
        &std::fs::read(&side).unwrap(),
        &cold.editor,
        &cold.native_checkpoint().unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(decoded.document().profile, p::PROFILE);
    let old = serde_json::to_value(v.document()).unwrap();
    let data = serde_json::to_value(decoded.document()).unwrap();
    for k in [
        "views",
        "occurrences",
        "edges",
        "typed_ports",
        "connector_edges",
        "activity_diagrams",
        "state_machine_diagrams",
        "scenario_diagrams",
    ] {
        assert_eq!(old.get(k), data.get(k), "{k}");
    }
    assert!(!decoded.can_layout_history(false));
    let recovery = area.path().join("sequence-recovery.mbse-views.json");
    let destination = writes
        .freeze_destination(&session, &v, &recovery, io::Kind::Recovery)
        .unwrap();
    let capture = v.capture(&session.editor);
    let job = writes
        .begin_captured(&session, &v, capture, destination)
        .unwrap();
    assert!(writes.complete(&session, &v, io::execute(job)));
    let restored = Views::restore(&std::fs::read(recovery).unwrap(), &session.editor).unwrap();
    assert_eq!(restored.document().profile, p::RECOVERY_PROFILE);
    let recovered = serde_json::to_value(restored.document()).unwrap();
    for key in [
        "views",
        "occurrences",
        "edges",
        "typed_ports",
        "connector_edges",
        "activity_diagrams",
        "state_machine_diagrams",
        "scenario_diagrams",
    ] {
        assert_eq!(recovered.get(key), old.get(key), "captured recovery {key}");
    }
    assert_eq!(session.editor.snapshot, before);
}
#[test]
fn sequence_legacy_inspection_copy_refuse_before_partial_inventory_and_copy_origin_cannot_alias_sequence_ids()
 {
    use mbse_model::{
        native_views_inspect::Inspection,
        native_views_plan::{Choice, Plan},
    };
    let (e, v, a, l, view) = minimal();
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let path = area.path().join("sequence.mbse-views.json");
    let raw = v.recovery(&e).unwrap().encoded().unwrap();
    std::fs::write(&path, &raw).unwrap();
    assert!(
        Inspection::open(&path)
            .err()
            .unwrap()
            .contains("SCENARIO_SOURCE_INSPECTION_UNSUPPORTED")
    );
    assert_eq!(std::fs::read(&path).unwrap(), raw);
    let mut plain = Views::new(&e).unwrap();
    let source_path = area.path().join("plain.json");
    let source = plain.recovery(&e).unwrap().encoded().unwrap();
    std::fs::write(&source_path, &source).unwrap();
    let inspected = Inspection::open(&source_path).unwrap();
    assert!(
        Plan::new(&inspected, plain.document().primary, &e, &v)
            .err()
            .unwrap()
            .contains("SCENARIO_COPY_UNSUPPORTED")
    );
    let mut plan = Plan::new(&inspected, plain.document().primary, &e, &plain).unwrap();
    for id in inspected
        .declared()
        .occurrences
        .values()
        .filter(|o| o.view == inspected.declared().primary)
        .map(|o| &o.id)
    {
        plan.choose(&inspected, &e, &plain, *id, Choice::Target(RecordId(1)))
            .unwrap();
    }
    let reviewed = plan
        .review(&inspected, &e, &plain, "Copied native")
        .unwrap();
    let copied = plan
        .apply_copy(&inspected, &e, &mut plain, &reviewed)
        .unwrap();
    let sequence = diagram(&e, &mut plain, a);
    let target = occurrence(&plain, sequence, l);
    let mut value = serde_json::to_value(plain.recovery(&e).unwrap()).unwrap();
    let origin = &mut value["views"][copied.get().to_string()]["copy_origin"];
    let mappings = origin["occurrences"].as_object_mut().unwrap();
    let row = mappings.values_mut().next().unwrap();
    row["outcome"]["target_occurrence"] = json!(target.get().to_string());
    assert!(
        Views::restore(&serde_json::to_vec(&value).unwrap(), &e)
            .err()
            .unwrap()
            .contains("COPY_TARGET_MAPPING")
    );
    let mut closed = Views::restore(&raw, &e).unwrap();
    let stamp = closed.stamp(&e);
    closed.apply(&e, &stamp, Layout::CloseView(view)).unwrap();
    assert!(closed.document().scenario_diagrams.is_empty());
    assert_eq!(closed.document().profile, p::RECOVERY_PROFILE);
    let mut plan = Plan::new(&inspected, inspected.declared().primary, &e, &closed).unwrap();
    for id in inspected
        .declared()
        .occurrences
        .values()
        .filter(|o| o.view == inspected.declared().primary)
        .map(|o| &o.id)
    {
        plan.choose(&inspected, &e, &closed, *id, Choice::Target(RecordId(1)))
            .unwrap();
    }
    let review = plan
        .review(&inspected, &e, &closed, "Native copy after sequence close")
        .unwrap();
    plan.apply_copy(&inspected, &e, &mut closed, &review)
        .unwrap();
    assert_eq!(closed.document().profile, p::RECOVERY_PROFILE);
    assert!(closed.document().scenario_diagrams.is_empty());
    assert_eq!(std::fs::read(source_path).unwrap(), source);
    assert!(v.document().scenario_diagrams.contains_key(&view));
}
#[test]
fn sequence_joint_occurrence_per_view_and_view_count_limits_admit_exact_boundary_and_refuse_overflow_atomically()
 {
    let (e, mut v, a, l, first) = minimal();
    for _ in 0..3 {
        diagram(&e, &mut v, a);
    }
    let mut d = v.recovery(&e).unwrap();
    let ids: Vec<_> = d.scenario_diagrams.keys().copied().collect();
    let template = d.scenario_diagrams[&first]
        .occurrences
        .values()
        .next()
        .unwrap()
        .clone();
    let native = d.occurrences.len() + d.typed_ports.len();
    let mut remaining = v::MAX_OCCURRENCES - native;
    let mut next = 100000;
    for view in &ids {
        let map = &mut d.scenario_diagrams.get_mut(view).unwrap().occurrences;
        map.clear();
        for _ in 0..remaining.min(v::MAX_PER_VIEW) {
            let mut o = template.clone();
            o.id = serde_json::from_str(&format!("\"{next}\"")).unwrap();
            next += 1;
            map.insert(o.id, o);
        }
        remaining = remaining.saturating_sub(v::MAX_PER_VIEW);
    }
    assert_eq!(remaining, 0);
    d.next_id = serde_json::from_str("\"200000\"").unwrap();
    let mut full = Views::restore(&d.encoded().unwrap(), &e).unwrap();
    let original = full.document().clone();
    assert!(
        layout(
            &e,
            &mut full,
            p::Layout::Reuse {
                view: *ids.last().unwrap(),
                element: l,
                geometry: v::Geometry::at(30., 40.)
            }
        )
        .is_err()
    );
    assert_eq!(full.document(), &original);
    let mut over = d.clone();
    let mut o = template.clone();
    o.id = serde_json::from_str(&format!("\"{next}\"")).unwrap();
    over.scenario_diagrams
        .get_mut(ids.last().unwrap())
        .unwrap()
        .occurrences
        .insert(o.id, o);
    assert!(Views::restore(&over.encoded().unwrap(), &e).is_err());
    let transfer = d
        .scenario_diagrams
        .get_mut(ids.last().unwrap())
        .unwrap()
        .occurrences
        .pop_last()
        .unwrap();
    d.scenario_diagrams
        .get_mut(&ids[0])
        .unwrap()
        .occurrences
        .insert(transfer.0, transfer.1);
    assert!(Views::restore(&d.encoded().unwrap(), &e).is_err());
    let (e, mut limited, a, _, _) = minimal();
    while limited.document().views.len() < v::MAX_VIEWS {
        let stamp = limited.stamp(&e);
        limited
            .apply(&e, &stamp, Layout::AddView("Within view budget".into()))
            .unwrap();
    }
    let original = limited.document().clone();
    assert!(
        layout(
            &e,
            &mut limited,
            p::Layout::Create {
                scenario: a,
                title: "17th view".into()
            }
        )
        .is_err()
    );
    assert_eq!(limited.document(), &original);
}
#[test]
fn sequence_retained_detached_diagram_keeps_legacy_deletion_incomplete_after_all_semantics_and_raw_are_removed()
 {
    use mbse_model::types::deletion;
    let (mut e, mut v, a, _, view) = minimal();
    native(
        &mut e,
        &mut v,
        NativeAction::Command(Command::Typed(t::Action::Scenario(Action::Remove {
            scenario: a,
            element: a,
        }))),
    );
    for key in s::forms(&e.snapshot).unwrap().keys() {
        native(
            &mut e,
            &mut v,
            NativeAction::Command(Command::Typed(t::Action::Scenario(Action::Discard(
                key.clone(),
            )))),
        );
    }
    assert!(
        v.document().scenario_diagrams[&view]
            .occurrences
            .values()
            .all(|o| !o.presence.active())
    );
    assert!(s::forms(&e.snapshot).unwrap().is_empty());
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::Deletion(deletion::Action::SetForm(
            deletion::Form {
                ids: "1".into(),
                cascade: true,
            },
        ))),
    )
    .unwrap();
    let prepared = deletion::prepare(&e, &v).unwrap();
    assert!(
        !prepared.summary().complete_supported_reference_inventory && prepared.plan().is_none()
    );
    assert!(
        serde_json::to_string(prepared.summary())
            .unwrap()
            .contains("Retained scenario diagram")
    );
}
#[test]
fn sequence_same_counter_source_digest_foreign_document_and_invalid_native_source_fail_without_layout_or_native_writes()
 {
    let mut g = graph();
    let mut v = views(&g.e);
    let view = diagram(&g.e, &mut v, g.a);
    let original = v.document().clone();
    let stamp = v.stamp(&g.e);
    let mut changed = g.e.snapshot.clone();
    changed.records.get_mut(&RecordId(1)).unwrap().name = "Same counters changed source".into();
    let changed = Editor::new(changed, g.e.freshness().instance_id).unwrap();
    assert_eq!(changed.freshness(), g.e.freshness());
    assert!(
        v.apply(
            &changed,
            &stamp,
            Layout::Scenario(p::Layout::Visible {
                view,
                occurrence: occurrence(&v, view, g.x),
                visible: false
            })
        )
        .is_err()
    );
    assert_eq!(v.document(), &original);
    let mut foreign = v.document().scenario_diagrams[&view].clone();
    foreign.source_document = DocumentId("sequence:foreign".into());
    let projected = p::project(&foreign, &g.e.snapshot).unwrap();
    assert!(!projected.scenario_resolved && projected.routes.is_empty());
    g.e.snapshot.records.remove(&RecordId(1));
    assert!(g.e.snapshot.validate().is_err());
    let native_before = g.e.snapshot.clone();
    let moving = occurrence(&v, view, g.x);
    assert!(
        layout(
            &g.e,
            &mut v,
            p::Layout::Move {
                view,
                occurrence: moving,
                geometry: v::Geometry::at(70., 80.)
            }
        )
        .is_err()
    );
    assert!(p::project(&v.document().scenario_diagrams[&view], &g.e.snapshot).is_err());
    assert_eq!(v.document(), &original);
    assert_eq!(g.e.snapshot, native_before);
}

#[test]
fn sequence_individual_lifeline_removal_and_observed_inverse_preserve_remaining_order_and_all_other_geometry()
 {
    let mut g = graph();
    let mut v = views(&g.e);
    let view = diagram(&g.e, &mut v, g.a);
    let id = occurrence(&v, view, g.z);
    let original = v.document().scenario_diagrams.clone();
    for message in [g.yes_msg, g.no_msg] {
        native(
            &mut g.e,
            &mut v,
            NativeAction::Command(Command::Typed(t::Action::Scenario(Action::Remove {
                scenario: g.a,
                element: message,
            }))),
        );
    }
    let accepted = t::load(&g.e.snapshot).unwrap();
    let raw = s::forms(&g.e.snapshot).unwrap();
    let next = g.e.snapshot.next_id;
    native(
        &mut g.e,
        &mut v,
        NativeAction::Command(Command::Typed(t::Action::Scenario(Action::Remove {
            scenario: g.a,
            element: g.z,
        }))),
    );
    assert!(
        !v.document().scenario_diagrams[&view].occurrences[&id]
            .presence
            .active()
    );
    for (other, o) in &original[&view].occurrences {
        let current = &v.document().scenario_diagrams[&view].occurrences[other];
        assert_eq!(current.geometry, o.geometry);
        if *other != id {
            assert!(current.presence.active());
        }
    }
    let projection = p::project(&v.document().scenario_diagrams[&view], &g.e.snapshot).unwrap();
    assert_eq!(
        projection
            .routes
            .iter()
            .map(|r| r.message)
            .collect::<Vec<_>>(),
        vec![g.call, g.reply]
    );
    assert_eq!(projection.rows.len(), 6);
    native(&mut g.e, &mut v, NativeAction::ModelUndo);
    assert_eq!(v.document().scenario_diagrams, original);
    assert_eq!(t::load(&g.e.snapshot).unwrap(), accepted);
    assert_eq!(s::forms(&g.e.snapshot).unwrap(), raw);
    assert_eq!(g.e.snapshot.next_id, next);
    assert!(
        !t::load(&g.e.snapshot).unwrap().scenarios[&g.a]
            .messages
            .contains_key(&g.yes_msg)
    );
    native(&mut g.e, &mut v, NativeAction::ModelRedo);
    assert!(
        !v.document().scenario_diagrams[&view].occurrences[&id]
            .presence
            .active()
    );
    native(&mut g.e, &mut v, NativeAction::ModelUndo);
    assert_eq!(v.document().scenario_diagrams, original);
}
