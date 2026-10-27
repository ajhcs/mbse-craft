use mbse_model::{
    extensions::Extensions,
    stage1::*,
    stage1_io::{Session, execute_write},
    types::{
        self as t, Id,
        scenarios::{self as s, Action, Form, MessageKind as K, ParticipantKind as P},
    },
};
use serde_json::{Value, json};
fn editor() -> Editor {
    let mut n = Snapshot::new(DocumentId("scenario:source".into()));
    n.extensions = Some(Extensions::default());
    Editor::new(n, "scenario:runtime".into()).unwrap()
}
fn run(e: &mut Editor, a: Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(t::Action::Scenario(a)))
        .map(|_| ())
}
fn set(e: &mut Editor, f: Form) -> String {
    let key = f.key();
    run(e, Action::Set(f)).unwrap();
    key
}
fn accept(e: &mut Editor, f: Form) -> Id {
    let id = f.id().unwrap_or(Id(e.snapshot.next_id));
    let key = set(e, f);
    run(e, Action::Apply(key)).unwrap();
    id
}
fn scenario(e: &mut Editor) -> Id {
    accept(
        e,
        Form::Scenario {
            id: None,
            system: "1".into(),
            name: "Inspection exchange".into(),
        },
    )
}
fn line(e: &mut Editor, scenario: Id, target: Id, expected: P) -> Id {
    accept(
        e,
        Form::Lifeline {
            scenario,
            id: None,
            name: format!("Participant{}", target.0),
            participant: target.0.to_string(),
            expected,
        },
    )
}
fn message(
    e: &mut Editor,
    scenario: Id,
    source: Id,
    target: Id,
    order: u32,
    operand: Option<Id>,
    kind: K,
    reply: Option<Id>,
) -> Id {
    accept(
        e,
        Form::Message {
            scenario,
            id: None,
            name: "Authored exchange".into(),
            source: source.0.to_string(),
            target: target.0.to_string(),
            order: order.to_string(),
            operand: operand.map_or(String::new(), |i| i.0.to_string()),
            message_kind: kind,
            reply_to: reply.map_or(String::new(), |i| i.0.to_string()),
        },
    )
}
fn alternative(e: &mut Editor, scenario: Id, order: u32) -> Id {
    accept(
        e,
        Form::Alternative {
            scenario,
            id: None,
            name: "Review result".into(),
            order: order.to_string(),
        },
    )
}
fn operand(e: &mut Editor, scenario: Id, alt: Id, order: u32, guard: &str) -> Id {
    accept(
        e,
        Form::Operand {
            scenario,
            id: None,
            name: "Guarded path".into(),
            alternative: alt.0.to_string(),
            order: order.to_string(),
            guard: guard.into(),
        },
    )
}
fn fixture() -> (Editor, Id, Id, Id, Id, Id, Id) {
    let mut e = editor();
    for name in ["Sensor", "Console"] {
        e.command(
            &e.freshness(),
            Command::Typed(t::Action::AddDefinition {
                system: RecordId(1),
                name: name.into(),
                kind: t::Kind::Component,
            }),
        )
        .unwrap();
    }
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::SetUsageForm {
            form: t::UsageForm {
                id: None,
                name: "sensor part".into(),
                owner: "1001".into(),
                target: "1000".into(),
                kind: t::UsageKind::Part,
            },
        }),
    )
    .unwrap();
    e.command(&e.freshness(), Command::Typed(t::Action::ApplyUsage))
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
    let a = scenario(&mut e);
    let x = line(&mut e, a, Id(1000), P::Component);
    let y = line(&mut e, a, Id(1002), P::Usage(t::UsageKind::Part));
    let z = line(&mut e, a, Id(actor.0), P::Native(RecordKind::Actor));
    (e, a, x, y, z, Id(actor.0), Id(1000))
}
#[test]
fn scenario_exact_owned_lifelines_ordered_calls_replies_and_guarded_alternatives_preserve_native_meaning()
 {
    let (mut e, a, x, y, z, _, _) = fixture();
    let native = e.snapshot.records.clone();
    let call = message(&mut e, a, x, y, 1, None, K::Synchronous, None);
    let reply = message(&mut e, a, y, x, 2, None, K::Reply, Some(call));
    let alt = alternative(&mut e, a, 3);
    let yes = operand(&mut e, a, alt, 1, "temperature > 50 C");
    let no = operand(&mut e, a, alt, 2, "otherwise / reviewer declined");
    let m1 = message(&mut e, a, y, z, 1, Some(yes), K::Asynchronous, None);
    let m2 = message(&mut e, a, z, x, 1, Some(no), K::Asynchronous, None);
    let store = t::load(&e.snapshot).unwrap();
    let graph = &store.scenarios[&a];
    assert_eq!(
        s::main_sequence(graph),
        vec![
            s::Fragment::Message(call),
            s::Fragment::Message(reply),
            s::Fragment::Alternative(alt)
        ]
    );
    assert_eq!(
        s::ordered_operands(&graph.alternatives[&alt])
            .iter()
            .map(|o| o.id)
            .collect::<Vec<_>>(),
        vec![yes, no]
    );
    assert_eq!(s::operand_sequence(graph, yes).unwrap(), vec![m1]);
    assert_eq!(s::operand_sequence(graph, no).unwrap(), vec![m2]);
    assert_eq!(
        graph.alternatives[&alt].operands[&yes].guard,
        "temperature > 50 C"
    );
    assert!(s::diagnostics(&e.snapshot, &store, a).unwrap().is_empty());
    assert_eq!(e.snapshot.records, native);
    assert_eq!(s::ids(&store).count(), 11);
    assert_eq!(graph.lifelines[&x].participant, Id(1000));
    assert_eq!(graph.lifelines[&y].participant, Id(1002));
}
#[test]
fn scenario_new_bindings_require_exact_existing_kind_and_system_without_name_based_replacement() {
    let (mut e, a, _, _, _, actor, _) = fixture();
    let mut other = e.snapshot.records[&RecordId(1)].clone();
    other.id = RecordId(2000);
    other.scope = RecordId(2000);
    other.name = "Foreign system".into();
    e.snapshot.records.insert(other.id, other);
    e.snapshot.next_id = 3000;
    e.snapshot.validate().unwrap();
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::AddDefinition {
            system: RecordId(2000),
            name: "Sensor".into(),
            kind: t::Kind::Component,
        }),
    )
    .unwrap();
    for (target, expected) in [
        (Id(9999), P::Component),
        (actor, P::Component),
        (Id(1000), P::Native(RecordKind::Actor)),
        (Id(1002), P::Usage(t::UsageKind::Reference)),
        (Id(3000), P::Component),
    ] {
        let key = set(
            &mut e,
            Form::Lifeline {
                scenario: a,
                id: None,
                name: "Refused participant".into(),
                participant: target.0.to_string(),
                expected,
            },
        );
        let before = e.snapshot.clone();
        assert!(run(&mut e, Action::Apply(key.clone())).is_err());
        assert_eq!(e.snapshot, before);
        assert!(s::forms(&e.snapshot).unwrap().contains_key(&key));
    }
}
#[test]
fn scenario_foreign_endpoint_operand_duplicate_order_and_bad_reply_refuse_with_raw_retained() {
    let (mut e, a, x, y, _, _, _) = fixture();
    let call = message(&mut e, a, x, y, 1, None, K::Synchronous, None);
    let alt = alternative(&mut e, a, 2);
    let op = operand(&mut e, a, alt, 1, "true");
    let other = scenario(&mut e);
    let foreign = line(&mut e, other, Id(1001), P::Component);
    let base = Form::Message {
        scenario: a,
        id: None,
        name: "Invalid route".into(),
        source: x.0.to_string(),
        target: y.0.to_string(),
        order: "3".into(),
        operand: String::new(),
        message_kind: K::Asynchronous,
        reply_to: String::new(),
    };
    for mode in 0..7 {
        let mut f = base.clone();
        if let Form::Message {
            source,
            target,
            order,
            operand,
            message_kind,
            reply_to,
            ..
        } = &mut f
        {
            match mode {
                0 => *source = foreign.0.to_string(),
                1 => *target = "9999".into(),
                2 => *order = "2".into(),
                3 => *operand = foreign.0.to_string(),
                4 => {
                    *message_kind = K::Reply;
                    *reply_to = call.0.to_string();
                }
                5 => *reply_to = call.0.to_string(),
                6 => {
                    *message_kind = K::Reply;
                    *source = y.0.to_string();
                    *target = x.0.to_string();
                    *reply_to = call.0.to_string();
                    *operand = op.0.to_string();
                }
                _ => unreachable!(),
            }
        }
        let key = set(&mut e, f);
        let before = e.snapshot.clone();
        assert!(
            run(&mut e, Action::Apply(key.clone())).is_err(),
            "mode{mode}"
        );
        assert_eq!(e.snapshot, before);
        assert!(s::forms(&e.snapshot).unwrap().contains_key(&key));
    }
}
#[test]
fn scenario_removed_wrong_kind_participants_and_duplicate_guards_produce_exact_diagnostics_without_rebinding()
 {
    let (mut e, a, x, y, z, actor, _) = fixture();
    let msg = message(&mut e, a, z, y, 1, None, K::Asynchronous, None);
    let alias = line(&mut e, a, Id(1000), P::Component);
    let alt = alternative(&mut e, a, 2);
    let first = operand(&mut e, a, alt, 1, "reviewer approves");
    let second = operand(&mut e, a, alt, 2, "reviewer approves");
    let mut wire = serde_json::to_value(t::load(&e.snapshot).unwrap()).unwrap();
    wire["definitions"]["1000"]["kind"] = json!({"Value":"Real"});
    wire["usages"].as_object_mut().unwrap().remove("1002");
    e.snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert(t::NAMESPACE.into(), wire);
    e.snapshot.records.remove(&RecordId(actor.0));
    e.snapshot.validate().unwrap();
    let replacement = e
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
    assert_ne!(replacement.0, actor.0);
    let before = e.snapshot.clone();
    let store = t::load(&e.snapshot).unwrap();
    let diags = s::diagnostics(&e.snapshot, &store, a).unwrap();
    assert!(
        diags
            .iter()
            .any(|d| d.code == "SCENARIO_PARTICIPANT_UNRESOLVED" && d.elements == vec![z, actor])
    );
    assert!(diags.iter().any(|d|d.code=="SCENARIO_PARTICIPANT_UNRESOLVED"&&d.elements==vec![x,Id(1000)]));
    assert!(
        diags
            .iter()
            .any(|d| d.code == "SCENARIO_MESSAGE_PARTICIPANT_UNRESOLVED"
                && d.elements == vec![msg, z, y])
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == "SCENARIO_DUPLICATE_GUARD" && d.elements == vec![first, second])
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == "SCENARIO_PARTICIPANT_ALIASES" && d.elements == vec![x, alias])
    );
    assert_eq!(store.scenarios[&a].lifelines[&z].participant, actor);
    assert_eq!(e.snapshot, before);
}
#[test]
fn scenario_distinct_invalid_proposals_survive_root_removal_model_undo_redo_and_selected_discard() {
    let (mut e, a, x, y, _, _, _) = fixture();
    let m1 = message(&mut e, a, x, y, 1, None, K::Asynchronous, None);
    let m2 = message(&mut e, a, y, x, 2, None, K::Asynchronous, None);
    let original = t::load(&e.snapshot).unwrap();
    let mut keys = Vec::new();
    for (id, bad) in [(m1, "missing source"), (m2, "foreign endpoint ?")] {
        keys.push(set(
            &mut e,
            Form::Message {
                scenario: a,
                id: Some(id),
                name: "Unapplied".into(),
                source: bad.into(),
                target: y.0.to_string(),
                order: "00".into(),
                operand: String::new(),
                message_kind: K::Asynchronous,
                reply_to: String::new(),
            },
        ));
    }
    let raw = s::forms(&e.snapshot).unwrap();
    run(
        &mut e,
        Action::Remove {
            scenario: a,
            element: a,
        },
    )
    .unwrap();
    assert!(!t::load(&e.snapshot).unwrap().scenarios.contains_key(&a));
    let high = e.snapshot.next_id;
    e.undo_model(&e.freshness()).unwrap();
    assert_eq!(t::load(&e.snapshot).unwrap(), original);
    assert_eq!(s::forms(&e.snapshot).unwrap(), raw);
    e.redo_model(&e.freshness()).unwrap();
    assert!(!t::load(&e.snapshot).unwrap().scenarios.contains_key(&a));
    e.undo_model(&e.freshness()).unwrap();
    assert_eq!(e.snapshot.next_id, high);
    run(&mut e, Action::Discard(keys[0].clone())).unwrap();
    let after = s::forms(&e.snapshot).unwrap();
    assert!(!after.contains_key(&keys[0]) && after[&keys[1]] == raw[&keys[1]]);
    assert_eq!(t::load(&e.snapshot).unwrap(), original);
}
#[test]
fn scenario_in_use_lifeline_call_operand_alternative_removal_refuses_without_partial_deletion() {
    let (mut e, a, x, y, _, _, _) = fixture();
    let call = message(&mut e, a, x, y, 1, None, K::Synchronous, None);
    message(&mut e, a, y, x, 2, None, K::Reply, Some(call));
    let alt = alternative(&mut e, a, 3);
    let op = operand(&mut e, a, alt, 1, "enabled");
    let msg = message(&mut e, a, x, y, 1, Some(op), K::Asynchronous, None);
    for id in [x, call, op, alt] {
        let before = e.snapshot.clone();
        assert!(
            run(
                &mut e,
                Action::Remove {
                    scenario: a,
                    element: id
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot, before);
    }
    run(
        &mut e,
        Action::Remove {
            scenario: a,
            element: msg,
        },
    )
    .unwrap();
    run(
        &mut e,
        Action::Remove {
            scenario: a,
            element: op,
        },
    )
    .unwrap();
    run(
        &mut e,
        Action::Remove {
            scenario: a,
            element: alt,
        },
    )
    .unwrap();
    assert!(
        s::forms(&e.snapshot)
            .unwrap()
            .values()
            .any(|f| f.id() == Some(op))
    );
}

#[test]
fn scenario_wire_canonical_maps_schema_order_reply_native_system_and_reserved_target_highwater_are_strict()
 {
    let (mut e, a, x, y, _, _, _) = fixture();
    let msg = message(&mut e, a, x, y, 1, None, K::Synchronous, None);
    let alt = alternative(&mut e, a, 2);
    let op = operand(&mut e, a, alt, 1, "guard text");
    for mode in 0..8 {
        let mut n = e.snapshot.clone();
        let v = n
            .extensions
            .as_mut()
            .unwrap()
            .namespaces
            .get_mut(t::NAMESPACE)
            .unwrap();
        let key = a.0.to_string();
        match mode {
            0 => {
                let row = v["scenarios"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&key)
                    .unwrap();
                v["scenarios"][format!("0{key}")] = row;
            }
            1 => v["scenarios"][&key]["lifelines"][x.0.to_string()]["id"] = json!(9999),
            2 => v["scenarios"][&key]["nested_fragments"] = json!({}),
            3 => {
                v["scenarios"][&key]["alternatives"][alt.0.to_string()]["operands"]
                    [op.0.to_string()]["guard"] = json!("")
            }
            4 => v["scenarios"][&key]["alternatives"][alt.0.to_string()]["order"] = json!(1),
            5 => {
                v["scenarios"][&key]["messages"][msg.0.to_string()]["kind"] = json!("Reply");
                v["scenarios"][&key]["messages"][msg.0.to_string()]["reply_to"] = json!(msg.0);
            }
            6 => v["scenarios"][&key]["lifelines"][x.0.to_string()]["participant"] = json!(0),
            7 => v["scenarios"][&key]["system"] = json!(1003),
            _ => unreachable!(),
        };
        assert!(n.validate().is_err(), "mode{mode}");
    }
    let mut n = e.snapshot.clone();
    n.extensions
        .as_mut()
        .unwrap()
        .namespaces
        .get_mut(t::NAMESPACE)
        .unwrap()["scenarios"][a.0.to_string()]["lifelines"][x.0.to_string()]["participant"] =
        json!(9000);
    n.next_id = 9000;
    assert!(n.validate().is_err());
    n.next_id = 9001;
    let mut reserved = Editor::new(n, "scenario:reserved-target".into()).unwrap();
    reserved
        .command(
            &reserved.freshness(),
            Command::Typed(t::Action::AddDefinition {
                system: RecordId(1),
                name: "Sensor".into(),
                kind: t::Kind::Component,
            }),
        )
        .unwrap();
    let store = t::load(&reserved.snapshot).unwrap();
    assert!(store.definitions.contains_key(&Id(9001)));
    assert_eq!(store.scenarios[&a].lifelines[&x].participant, Id(9000));
    assert!(
        s::diagnostics(&reserved.snapshot, &store, a)
            .unwrap()
            .iter()
            .any(|d| d.elements == vec![x, Id(9000)])
    );
}
#[test]
fn scenario_native_typed_other_family_and_detached_draft_ids_never_alias_or_rebind() {
    let (mut e, a, _, _, _, _, _) = fixture();
    for id in [Id(1), Id(1000), Id(1002)] {
        let before = e.snapshot.clone();
        assert!(
            run(
                &mut e,
                Action::Set(Form::Scenario {
                    id: Some(id),
                    system: "1".into(),
                    name: "Foreign identity".into()
                })
            )
            .is_err()
        );
        assert_eq!(e.snapshot, before);
    }
    let before = e.snapshot.clone();
    assert!(
        e.command(
            &e.freshness(),
            Command::Typed(t::Action::StateMachine(t::state_machines::Action::Set(
                t::state_machines::Form::Machine {
                    id: Some(a),
                    system: "1".into(),
                    name: "Conflicting machine draft".into()
                }
            )))
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    assert!(
        e.command(
            &e.freshness(),
            Command::Typed(t::Action::SetForm {
                id: a,
                form: t::Form {
                    owner: Id(1000),
                    name: "Conflicting property".into(),
                    value_type: "1000".into(),
                    lower: "1".into(),
                    upper: "1".into()
                }
            })
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    e.snapshot.next_id = 10000;
    let key = set(
        &mut e,
        Form::Scenario {
            id: Some(Id(4000)),
            system: "?".into(),
            name: "Detached".into(),
        },
    );
    let before = e.snapshot.clone();
    assert!(
        e.command(
            &e.freshness(),
            Command::Typed(t::Action::StateMachine(t::state_machines::Action::Set(
                t::state_machines::Form::Machine {
                    id: Some(Id(4000)),
                    system: "1".into(),
                    name: "Same detached ID".into()
                }
            )))
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    run(&mut e, Action::Discard(key)).unwrap();
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::StateMachine(t::state_machines::Action::Set(
            t::state_machines::Form::Machine {
                id: Some(Id(4000)),
                system: "1".into(),
                name: "Other family first".into(),
            },
        ))),
    )
    .unwrap();
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            Action::Set(Form::Scenario {
                id: Some(Id(4000)),
                system: "1".into(),
                name: "Same detached ID".into()
            })
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
}
#[test]
fn scenario_utf8_raw_bytes_family_quota_and_creation_slot_migration_are_atomic() {
    let mut e = editor();
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            Action::Set(Form::Scenario {
                id: None,
                system: "1".into(),
                name: "é".repeat(2049)
            })
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    set(
        &mut e,
        Form::Scenario {
            id: None,
            system: "1".into(),
            name: "é".repeat(2048),
        },
    );
    e.snapshot.next_id = 10000;
    for n in 2000..2127 {
        set(
            &mut e,
            Form::Scenario {
                id: Some(Id(n)),
                system: "?".into(),
                name: format!("Detached{n}"),
            },
        );
    }
    let key = set(
        &mut e,
        Form::Scenario {
            id: None,
            system: "1".into(),
            name: "New at quota".into(),
        },
    );
    let old = s::forms(&e.snapshot).unwrap();
    assert_eq!(old.len(), 128);
    run(&mut e, Action::Apply(key.clone())).unwrap();
    let raw = s::forms(&e.snapshot).unwrap();
    assert_eq!(raw.len(), 128);
    assert!(!raw.contains_key(&key) && raw.contains_key("scenario:10000"));
    for (k, v) in old {
        if k != key {
            assert_eq!(raw[&k], v);
        }
    }
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            Action::Set(Form::Lifeline {
                scenario: Id(10000),
                id: None,
                name: "129th proposal".into(),
                participant: "1".into(),
                expected: P::Native(RecordKind::System)
            })
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
}
#[test]
fn scenario_joint_type_activity_machine_and_scenario_cap_rejects_513th_without_consuming_id() {
    let mut n = editor().snapshot;
    n.next_id = 10000;
    let mut store = t::Store::default();
    for id in 1000..1400 {
        store.definitions.insert(
            Id(id),
            t::Definition {
                id: Id(id),
                system: RecordId(1),
                name: format!("Component{id}"),
                kind: t::Kind::Component,
            },
        );
    }
    store.activities.insert(
        Id(2000),
        t::activities::Activity {
            id: Id(2000),
            system: RecordId(1),
            name: "Activity".into(),
            partitions: Default::default(),
            nodes: Default::default(),
            pins: Default::default(),
            flows: Default::default(),
        },
    );
    store.state_machines.insert(
        Id(2010),
        t::state_machines::Machine {
            id: Id(2010),
            system: RecordId(1),
            name: "Machine".into(),
            states: (2011..2020)
                .map(|id| {
                    (
                        Id(id),
                        t::state_machines::State {
                            id: Id(id),
                            name: format!("State{id}"),
                            kind: t::state_machines::StateKind::State,
                        },
                    )
                })
                .collect(),
            events: Default::default(),
            transitions: Default::default(),
        },
    );
    store.scenarios.insert(
        Id(3000),
        s::Scenario {
            id: Id(3000),
            system: RecordId(1),
            name: "Scenario".into(),
            lifelines: (3001..3101)
                .map(|id| {
                    (
                        Id(id),
                        s::Lifeline {
                            id: Id(id),
                            name: format!("Alias{id}"),
                            participant: Id(1000),
                            expected: P::Component,
                        },
                    )
                })
                .collect(),
            messages: Default::default(),
            alternatives: Default::default(),
        },
    );
    assert_eq!(
        store.definitions.len()
            + t::activities::ids(&store).count()
            + t::state_machines::ids(&store).count()
            + s::ids(&store).count(),
        512
    );
    let ext = n.extensions.as_mut().unwrap();
    ext.required_namespaces.push(t::NAMESPACE.into());
    ext.namespaces
        .insert(t::NAMESPACE.into(), serde_json::to_value(store).unwrap());
    let mut e = Editor::new(n, "scenario:joint-budget".into()).unwrap();
    let key = set(
        &mut e,
        Form::Lifeline {
            scenario: Id(3000),
            id: None,
            name: "513th object".into(),
            participant: "1000".into(),
            expected: P::Component,
        },
    );
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::Apply(key)).is_err());
    assert_eq!(e.snapshot, before);
    assert_eq!(e.snapshot.next_id, 10000);
}
#[test]
fn scenario_native_bound_save_cold_and_recovery_keep_entire_original_native_raw_opaque_and_seven_views_maps()
 {
    use mbse_model::{
        native_views::{self as v, Views},
        native_views_io as io,
    };
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let root: Value = serde_json::from_slice(include_bytes!(
        "../../../docs/parity/p33b/verification/native-02/run/state-execution-cold-copy.kernspan"
    ))
    .unwrap();
    session.editor = Editor::new(
        serde_json::from_value(root["snapshot"].clone()).unwrap(),
        session.editor.freshness().instance_id,
    )
    .unwrap();
    let before = session.editor.snapshot.clone();
    let mut original_views:v::Document=serde_json::from_slice(include_bytes!("../../../docs/parity/p33b/verification/native-02/run/state-execution-cold-copy.mbse-views.json")).unwrap();
    original_views.profile = v::state_machines::RECOVERY_PROFILE.into();
    original_views.binding = None;
    let views = Views::restore(&original_views.encoded().unwrap(), &session.editor).unwrap();
    let prior = t::load(&before).unwrap();
    let a = scenario(&mut session.editor);
    let x = line(&mut session.editor, a, Id(1004), P::Component);
    let y = line(
        &mut session.editor,
        a,
        Id(1009),
        P::Usage(t::UsageKind::Part),
    );
    let msg = message(&mut session.editor, a, x, y, 1, None, K::Asynchronous, None);
    set(
        &mut session.editor,
        Form::Message {
            scenario: a,
            id: Some(msg),
            name: "Incomplete native proposal".into(),
            source: "?".into(),
            target: "".into(),
            order: "not ordered".into(),
            operand: String::new(),
            message_kind: K::Asynchronous,
            reply_to: String::new(),
        },
    );
    let expected = session.editor.snapshot.clone();
    assert_eq!(expected.records, before.records);
    assert_eq!(expected.drafts, before.drafts);
    assert_eq!(expected.creation_drafts, before.creation_drafts);
    let mut accepted = t::load(&expected).unwrap();
    accepted.scenarios.clear();
    assert_eq!(accepted, prior);
    let mut namespaces = expected.extensions.as_ref().unwrap().namespaces.clone();
    namespaces
        .get_mut(t::NAMESPACE)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("scenarios");
    namespaces
        .get_mut(t::DRAFTS)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("scenario_forms");
    assert_eq!(namespaces, before.extensions.as_ref().unwrap().namespaces);
    let path = area.path().join("scenario.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let mut writes = io::Io::default();
    let side = area.path().join("scenario.mbse-views.json");
    let job = writes
        .begin(&session, &views, &side, io::Kind::Bound)
        .unwrap();
    assert!(writes.complete(&session, &views, io::execute(job)));
    let cold = Session::open(area.path(), &path).unwrap();
    assert_eq!(cold.editor.snapshot, expected);
    let decoded = Views::decode(
        &std::fs::read(&side).unwrap(),
        &cold.editor,
        &cold.native_checkpoint().unwrap().unwrap(),
    )
    .unwrap();
    let old = serde_json::to_value(views.document()).unwrap();
    let new = serde_json::to_value(decoded.document()).unwrap();
    for key in [
        "views",
        "occurrences",
        "edges",
        "typed_ports",
        "connector_edges",
        "activity_diagrams",
        "state_machine_diagrams",
    ] {
        assert_eq!(new.get(key), old.get(key), "{key}");
    }
    assert!(!decoded.can_layout_history(false));
    let job = session.begin_recovery().unwrap();
    assert!(session.complete(execute_write(job)));
    let restored = Session::restore(area.path(), session.last_recovery().unwrap()).unwrap();
    let mut recovered = restored.editor.snapshot;
    assert_ne!(recovered.document_id, expected.document_id);
    recovered.document_id = expected.document_id.clone();
    assert_eq!(recovered, expected);
    let recovery_path = area.path().join("scenario-views-recovery.mbse-views.json");
    let job = writes
        .begin(&session, &views, &recovery_path, io::Kind::Recovery)
        .unwrap();
    assert!(writes.complete(&session, &views, io::execute(job)));
    let unbound = Views::restore(&std::fs::read(recovery_path).unwrap(), &session.editor).unwrap();
    assert_eq!(
        serde_json::to_value(unbound.document()).unwrap()["state_machine_diagrams"],
        old["state_machine_diagrams"]
    );
}
#[test]
fn scenario_legacy_deletion_and_refactor_disclose_uninterpreted_references_and_retain_authored_data()
 {
    use mbse_model::{
        native_views::Views,
        types::{deletion, refactor},
    };
    let (mut e, a, _, _, _, _, _) = fixture();
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
    assert!(
        !prepared.summary().complete_supported_reference_inventory && prepared.plan().is_none()
    );
    assert!(
        serde_json::to_string(&deletion::unknown_impact(&e))
            .unwrap()
            .contains("Scenario")
    );
    let original = t::load(&e.snapshot).unwrap();
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::Refactor(refactor::Action::SetForm(
            refactor::Form {
                mode: refactor::Mode::CloneDefinition,
                definition: "1000".into(),
                name: "Sensor copy".into(),
                usages: String::new(),
                owner: String::new(),
                keep_external: true,
            },
        ))),
    )
    .unwrap();
    let reviewed = refactor::prepare(&e).unwrap();
    assert!(!reviewed.summary().exhaustive_global_reference_verdict);
    assert!(
        reviewed
            .summary()
            .uninterpreted_locations
            .iter()
            .any(|s| s.contains("Scenario"))
    );
    assert_eq!(t::load(&e.snapshot).unwrap(), original);
    run(
        &mut e,
        Action::Remove {
            scenario: a,
            element: a,
        },
    )
    .unwrap();
    assert!(
        serde_json::to_string(&deletion::unknown_impact(&e))
            .unwrap()
            .contains("Retained scenario proposals")
    );
}
