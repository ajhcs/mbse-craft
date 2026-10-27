use mbse_model::{
    extensions::Extensions,
    stage1::*,
    stage1_io::{execute_write, Session},
    types::{
        self as t,
        state_machines::{self as s, Action, Form, StateKind},
        Id,
    },
};
use serde_json::json;
fn editor() -> Editor {
    let mut snapshot = Snapshot::new(DocumentId("state-machine:source".into()));
    snapshot.extensions = Some(Extensions::default());
    Editor::new(snapshot, "state-machine:runtime".into()).unwrap()
}
fn run(e: &mut Editor, a: Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(t::Action::StateMachine(a)))
        .map(|_| ())
}
fn set(e: &mut Editor, f: Form) -> String {
    let key = f.key();
    run(e, Action::Set(f)).unwrap();
    key
}
fn apply(e: &mut Editor, f: Form) -> Id {
    let id = f.id().unwrap_or(Id(e.snapshot.next_id));
    let key = set(e, f);
    run(e, Action::Apply(key)).unwrap();
    id
}
fn machine(e: &mut Editor) -> Id {
    apply(
        e,
        Form::Machine {
            id: None,
            system: "1".into(),
            name: "Controller".into(),
        },
    )
}
fn state(e: &mut Editor, machine: Id, kind: StateKind) -> Id {
    apply(
        e,
        Form::State {
            machine,
            id: None,
            name: format!("{kind:?}"),
            state_kind: kind,
        },
    )
}
fn event(e: &mut Editor, machine: Id) -> Id {
    apply(
        e,
        Form::Event {
            machine,
            id: None,
            name: "Go".into(),
        },
    )
}
fn transition(
    e: &mut Editor,
    machine: Id,
    source: Id,
    target: Id,
    event: Option<Id>,
    guard: &str,
) -> Id {
    apply(
        e,
        Form::Transition {
            machine,
            id: None,
            name: "Transition".into(),
            source: source.0.to_string(),
            target: target.0.to_string(),
            event: event.map_or(String::new(), |id| id.0.to_string()),
            guard: guard.into(),
        },
    )
}
fn graph() -> (Editor, Id, Id, Id, Id, Id, Id) {
    let mut e = editor();
    let m = machine(&mut e);
    let start = state(&mut e, m, StateKind::Initial);
    let waiting = state(&mut e, m, StateKind::State);
    let end = state(&mut e, m, StateKind::Final);
    let go = event(&mut e, m);
    transition(&mut e, m, start, waiting, None, "");
    let edge = transition(&mut e, m, waiting, end, Some(go), "true");
    (e, m, start, waiting, end, go, edge)
}
#[test]
fn state_machine_authoring_uses_shared_global_ids_exact_states_events_and_literal_transitions() {
    let (e, m, start, waiting, end, go, edge) = graph();
    let a = &t::load(&e.snapshot).unwrap().state_machines[&m];
    assert_eq!(a.system, RecordId(1));
    assert_eq!(a.states[&start].kind, StateKind::Initial);
    assert_eq!(a.states[&waiting].kind, StateKind::State);
    assert_eq!(a.states[&end].kind, StateKind::Final);
    assert_eq!(a.transitions[&edge].event, Some(go));
    assert_eq!(a.transitions[&edge].guard, Some(s::Guard::Boolean(true)));
    assert_eq!(s::ids(&t::load(&e.snapshot).unwrap()).count(), 7);
    assert_eq!(e.snapshot.next_id, 1007);
    assert!(s::diagnostics(&t::load(&e.snapshot).unwrap(), m)
        .unwrap()
        .is_empty());
    e.snapshot.validate().unwrap();
}
#[test]
fn state_machine_invalid_target_guard_event_initial_direction_and_second_initial_refuse_atomically()
{
    let (mut e, m, start, waiting, end, go, _) = graph();
    for (source, target, event, guard) in [
        (
            waiting.0.to_string(),
            "999".into(),
            go.0.to_string(),
            "true",
        ),
        (
            waiting.0.to_string(),
            end.0.to_string(),
            "999".into(),
            "true",
        ),
        (
            waiting.0.to_string(),
            end.0.to_string(),
            go.0.to_string(),
            "x >",
        ),
        (end.0.to_string(), waiting.0.to_string(), "".into(), ""),
        (waiting.0.to_string(), start.0.to_string(), "".into(), ""),
        (
            start.0.to_string(),
            waiting.0.to_string(),
            go.0.to_string(),
            "",
        ),
        (waiting.0.to_string(), "01003".into(), "".into(), ""),
    ] {
        let key = set(
            &mut e,
            Form::Transition {
                machine: m,
                id: None,
                name: "Invalid proposal".into(),
                source,
                target,
                event,
                guard: guard.into(),
            },
        );
        let before = e.snapshot.clone();
        assert!(run(&mut e, Action::Apply(key.clone())).is_err());
        assert_eq!(e.snapshot, before);
        assert!(s::forms(&e.snapshot).unwrap().contains_key(&key));
    }
    let key = set(
        &mut e,
        Form::State {
            machine: m,
            id: None,
            name: "Second start".into(),
            state_kind: StateKind::Initial,
        },
    );
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::Apply(key))
        .unwrap_err()
        .contains("MULTIPLE_INITIAL"));
    assert_eq!(e.snapshot, before);
}
#[test]
fn state_machine_ambiguous_and_duplicate_event_definitions_expose_exact_diagnostics_without_policy()
{
    let (mut e, m, _, waiting, end, go, first) = graph();
    let second = transition(&mut e, m, waiting, end, Some(go), "");
    let e1 = transition(&mut e, m, waiting, end, None, "else");
    let e2 = transition(&mut e, m, waiting, end, None, "else");
    let duplicate = event(&mut e, m);
    let d = s::diagnostics(&t::load(&e.snapshot).unwrap(), m).unwrap();
    assert!(d
        .iter()
        .any(|d| d.code == "SM_AMBIGUOUS_TRANSITIONS" && d.elements == vec![first, second]));
    assert!(d
        .iter()
        .any(|d| d.code == "SM_AMBIGUOUS_ELSE" && d.elements == vec![e1, e2]));
    assert!(d
        .iter()
        .any(|d| d.code == "SM_DUPLICATE_EVENT_NAME" && d.elements == vec![go, duplicate]));
    assert_eq!(
        t::load(&e.snapshot).unwrap().state_machines[&m]
            .transitions
            .len(),
        5
    );
    let other = machine(&mut e);
    let d = s::diagnostics(&t::load(&e.snapshot).unwrap(), other).unwrap();
    assert!(d.iter().any(|d| d.code == "SM_INITIAL_MISSING"));
    assert!(d.iter().any(|d| d.code == "SM_FINAL_MISSING"));
}
#[test]
fn state_machine_owned_foreign_endpoints_events_and_cross_family_draft_id_collisions_refuse() {
    let (mut e, m, _, waiting, end, _, _) = graph();
    let other = machine(&mut e);
    let foreign = state(&mut e, other, StateKind::State);
    let foreign_event = event(&mut e, other);
    for (target, event) in [
        (foreign.0.to_string(), "".into()),
        (end.0.to_string(), foreign_event.0.to_string()),
    ] {
        let key = set(
            &mut e,
            Form::Transition {
                machine: m,
                id: None,
                name: "Foreign".into(),
                source: waiting.0.to_string(),
                target,
                event,
                guard: "".into(),
            },
        );
        let before = e.snapshot.clone();
        assert!(run(&mut e, Action::Apply(key)).is_err());
        assert_eq!(e.snapshot, before);
    }
    for id in [Id(1), waiting] {
        let before = e.snapshot.clone();
        assert!(run(
            &mut e,
            Action::Set(Form::Event {
                machine: m,
                id: Some(id),
                name: "Wrong family".into()
            })
        )
        .is_err());
        assert_eq!(e.snapshot, before);
    }
    let before = e.snapshot.clone();
    assert!(e
        .command(
            &e.freshness(),
            Command::Typed(t::Action::Activity(t::activities::Action::Form(
                t::activities::forms::Action::Set(t::activities::forms::Form::Activity {
                    id: Some(m),
                    system: "1".into(),
                    name: "Foreign activity slot".into()
                })
            )))
        )
        .is_err());
    assert_eq!(e.snapshot, before);
}
#[test]
fn state_machine_distinct_invalid_proposals_survive_model_history_removal_and_selected_discard() {
    let (mut e, m, _, waiting, end, go, first) = graph();
    let second = transition(&mut e, m, waiting, end, Some(go), "else");
    for (id, guard) in [(first, "x >"), (second, "else ?")] {
        set(
            &mut e,
            Form::Transition {
                machine: m,
                id: Some(id),
                name: "Invalid guard".into(),
                source: waiting.0.to_string(),
                target: end.0.to_string(),
                event: go.0.to_string(),
                guard: guard.into(),
            },
        );
    }
    let raw = s::forms(&e.snapshot).unwrap();
    run(
        &mut e,
        Action::Remove {
            machine: m,
            element: m,
        },
    )
    .unwrap();
    assert!(t::load(&e.snapshot).unwrap().state_machines.is_empty());
    e.undo_model(&e.freshness()).unwrap();
    assert_eq!(s::forms(&e.snapshot).unwrap(), raw);
    assert_eq!(
        t::load(&e.snapshot).unwrap().state_machines[&m]
            .transitions
            .len(),
        3
    );
    e.redo_model(&e.freshness()).unwrap();
    assert_eq!(s::forms(&e.snapshot).unwrap(), raw);
    assert!(run(
        &mut e,
        Action::Apply(format!("transition:{}:{}", m.0, first.0))
    )
    .is_err());
    run(
        &mut e,
        Action::Discard(format!("transition:{}:{}", m.0, first.0)),
    )
    .unwrap();
    let after = s::forms(&e.snapshot).unwrap();
    assert_eq!(after.len(), raw.len() - 1);
    assert!(after.contains_key(&format!("transition:{}:{}", m.0, second.0)));
    e.undo_model(&e.freshness()).unwrap();
    assert_eq!(s::forms(&e.snapshot).unwrap(), after);
}
#[test]
fn state_machine_in_use_state_or_event_removal_refuses_without_deleting_proposals_or_meaning() {
    let (mut e, m, _, waiting, _, go, edge) = graph();
    for id in [waiting, go] {
        let before = e.snapshot.clone();
        assert!(run(
            &mut e,
            Action::Remove {
                machine: m,
                element: id
            }
        )
        .is_err());
        assert_eq!(e.snapshot, before);
    }
    run(
        &mut e,
        Action::Remove {
            machine: m,
            element: edge,
        },
    )
    .unwrap();
    run(
        &mut e,
        Action::Remove {
            machine: m,
            element: go,
        },
    )
    .unwrap();
    assert!(!t::load(&e.snapshot).unwrap().state_machines[&m]
        .events
        .contains_key(&go));
    assert!(s::forms(&e.snapshot)
        .unwrap()
        .contains_key(&format!("event:{}:{}", m.0, go.0)));
}
#[test]
fn state_machine_form_byte_count_and_quota_apply_reuse_are_bounded_and_atomic() {
    let mut e = editor();
    let before = e.snapshot.clone();
    assert!(run(
        &mut e,
        Action::Set(Form::Machine {
            id: None,
            system: "1".into(),
            name: "é".repeat(2049)
        })
    )
    .is_err());
    assert_eq!(e.snapshot, before);
    e.snapshot.next_id = 10000;
    for id in 2000..2127 {
        set(
            &mut e,
            Form::Machine {
                id: Some(Id(id)),
                system: "?".into(),
                name: format!("Detached {id}"),
            },
        );
    }
    let key = set(
        &mut e,
        Form::Machine {
            id: None,
            system: "1".into(),
            name: "New at quota".into(),
        },
    );
    let raw = s::forms(&e.snapshot).unwrap();
    assert_eq!(raw.len(), 128);
    run(&mut e, Action::Apply(key.clone())).unwrap();
    let after = s::forms(&e.snapshot).unwrap();
    assert_eq!(after.len(), 128);
    assert_eq!(e.snapshot.next_id, 10001);
    assert!(!after.contains_key(&key));
    for (k, v) in raw {
        if k != key {
            assert_eq!(after[&k], v);
        }
    }
    let before = e.snapshot.clone();
    assert!(run(
        &mut e,
        Action::Set(Form::Event {
            machine: Id(10000),
            id: None,
            name: "129th".into()
        })
    )
    .is_err());
    assert_eq!(e.snapshot, before);
}
#[test]
fn state_machine_strict_map_schema_native_system_and_global_identity_admission_refuse() {
    let (e, m, _, waiting, _, _, _) = graph();
    for mutation in [0, 1, 2, 3] {
        let mut snapshot = e.snapshot.clone();
        let v = snapshot
            .extensions
            .as_mut()
            .unwrap()
            .namespaces
            .get_mut(t::NAMESPACE)
            .unwrap();
        match mutation {
            0 => {
                let a = v["state_machines"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&m.0.to_string())
                    .unwrap();
                v["state_machines"][format!("0{}", m.0)] = a;
            }
            1 => v["state_machines"][m.0.to_string()]["unknown"] = json!(true),
            2 => v["state_machines"][m.0.to_string()]["system"] = json!(999),
            _ => {
                v["state_machines"][m.0.to_string()]["states"][waiting.0.to_string()]["id"] =
                    json!(1)
            }
        };
        assert!(Editor::new(snapshot, "wrong:admission".into()).is_err());
    }
}
#[test]
fn state_machine_native_save_cold_recovery_preserve_prior_activity_raw_opaque_and_views_source() {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let root: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p31b/verification/native-01/cold/execution-cold-copy.kernspan"
    ))
    .unwrap();
    let mut input: Snapshot = serde_json::from_value(root["snapshot"].clone()).unwrap();
    input.extensions.as_mut().unwrap().namespaces.insert(
        "fixture.uninterpreted/1".into(),
        json!({"unknownMeaning":[null,true,{"foreignId":"uninterpreted:17","text":"retain exactly"}]}),
    );
    session.editor = Editor::new(input, session.editor.freshness().instance_id).unwrap();
    let before = session.editor.snapshot.clone();
    let prior = t::load(&before).unwrap();
    let m = machine(&mut session.editor);
    state(&mut session.editor, m, StateKind::State);
    set(
        &mut session.editor,
        Form::Transition {
            machine: m,
            id: None,
            name: "Incomplete".into(),
            source: "?".into(),
            target: "".into(),
            event: "".into(),
            guard: "x >".into(),
        },
    );
    let expected = session.editor.snapshot.clone();
    assert_eq!(expected.records, before.records);
    assert_eq!(expected.drafts, before.drafts);
    assert_eq!(expected.creation_drafts, before.creation_drafts);
    assert_eq!(
        expected.extensions.as_ref().unwrap().namespaces["fixture.uninterpreted/1"],
        before.extensions.as_ref().unwrap().namespaces["fixture.uninterpreted/1"]
    );
    let mut accepted = t::load(&expected).unwrap();
    accepted.state_machines.clear();
    assert_eq!(accepted, prior);
    let path = area.path().join("state-machine.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let cold = Session::open(area.path(), &path).unwrap();
    assert_eq!(cold.editor.snapshot, expected);
    assert_eq!(
        s::forms(&cold.editor.snapshot).unwrap(),
        s::forms(&expected).unwrap()
    );
    let job = session.begin_recovery().unwrap();
    assert!(session.complete(execute_write(job)));
    let recovered = Session::restore(area.path(), session.last_recovery().unwrap()).unwrap();
    let mut recovered_snapshot = recovered.editor.snapshot.clone();
    assert_ne!(recovered_snapshot.document_id, expected.document_id);
    recovered_snapshot.document_id = expected.document_id.clone();
    assert_eq!(recovered_snapshot, expected);
}
#[test]
fn state_machine_legacy_deletion_does_not_claim_complete_reference_inventory() {
    use mbse_model::{native_views::Views, types::deletion};
    let (mut e, m, _, _, _, _, _) = graph();
    let views = Views::new(&e).unwrap();
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
    let prepared = deletion::prepare(&e, &views).unwrap();
    assert!(!prepared.summary().complete_supported_reference_inventory);
    assert!(prepared.plan().is_none());
    let summary = deletion::unknown_impact(&e);
    assert!(serde_json::to_string(&summary)
        .unwrap()
        .contains("StateMachine"));
    run(
        &mut e,
        Action::Remove {
            machine: m,
            element: m,
        },
    )
    .unwrap();
    let summary = deletion::unknown_impact(&e);
    assert!(serde_json::to_string(&summary)
        .unwrap()
        .contains("state-machine proposals"));
}

#[test]
fn state_machine_joint_type_activity_machine_element_cap_rejects_next_creation_atomically() {
    let mut snapshot = editor().snapshot;
    snapshot.next_id = 10000;
    let mut store = t::Store::default();
    let states = (1001..1460)
        .map(|n| {
            (
                Id(n),
                s::State {
                    id: Id(n),
                    name: format!("State {n}"),
                    kind: StateKind::State,
                },
            )
        })
        .collect();
    store.state_machines.insert(
        Id(1000),
        s::Machine {
            id: Id(1000),
            system: RecordId(1),
            name: "Boundary machine".into(),
            states,
            events: Default::default(),
            transitions: Default::default(),
        },
    );
    for n in 2000..2051 {
        store.definitions.insert(
            Id(n),
            t::Definition {
                id: Id(n),
                system: RecordId(1),
                name: format!("Value {n}"),
                kind: t::Kind::Value(t::Scalar::Real),
            },
        );
    }
    store.activities.insert(
        Id(3000),
        t::activities::Activity {
            id: Id(3000),
            system: RecordId(1),
            name: "Retained activity".into(),
            partitions: Default::default(),
            nodes: Default::default(),
            pins: Default::default(),
            flows: Default::default(),
        },
    );
    assert_eq!(
        s::ids(&store).count() + store.definitions.len() + t::activities::ids(&store).count(),
        512
    );
    let ext = snapshot.extensions.as_mut().unwrap();
    ext.required_namespaces.push(t::NAMESPACE.into());
    ext.namespaces
        .insert(t::NAMESPACE.into(), serde_json::to_value(store).unwrap());
    let mut e = Editor::new(snapshot, "state-machine:joint-boundary".into()).unwrap();
    let key = set(
        &mut e,
        Form::State {
            machine: Id(1000),
            id: None,
            name: "513th shared element".into(),
            state_kind: StateKind::State,
        },
    );
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::Apply(key))
        .unwrap_err()
        .contains("TYPE_VERSION_OR_LIMIT"));
    assert_eq!(e.snapshot, before);
}
