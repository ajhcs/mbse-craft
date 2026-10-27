use mbse_model::{
    extensions::Extensions,
    stage1::*,
    stage1_io::{Session, execute_write},
    types::{self as t, Id, activities::*},
};
use serde_json::json;
fn editor() -> Editor {
    let mut s = Snapshot::new(DocumentId("activity:source".into()));
    s.extensions = Some(Extensions::default());
    Editor::new(s, "activity:runtime".into()).unwrap()
}
fn run(e: &mut Editor, a: Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(t::Action::Activity(a)))
        .map(|_| ())
}
fn activity(e: &mut Editor, name: &str) -> Id {
    let id = Id(e.snapshot.next_id);
    run(
        e,
        Action::Activity {
            id: None,
            system: RecordId(1),
            name: name.into(),
        },
    )
    .unwrap();
    id
}
fn node(e: &mut Editor, a: Id, kind: NodeKind) -> Id {
    let id = Id(e.snapshot.next_id);
    run(
        e,
        Action::Node {
            activity: a,
            id: None,
            name: format!("{kind:?}"),
            kind,
            partition: None,
        },
    )
    .unwrap();
    id
}
fn define(e: &mut Editor, kind: t::Kind) -> Id {
    let id = Id(e.snapshot.next_id);
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::AddDefinition {
            system: RecordId(1),
            name: "Payload".into(),
            kind,
        }),
    )
    .unwrap();
    id
}
fn pin(e: &mut Editor, a: Id, owner: Id, direction: Direction, ty: Id) -> Id {
    let id = Id(e.snapshot.next_id);
    run(
        e,
        Action::Pin {
            activity: a,
            id: None,
            owner,
            name: format!("{direction:?}"),
            direction,
            value_type: ty,
        },
    )
    .unwrap();
    id
}
fn form(a: Id, source: Id, target: Id, guard: &str) -> FlowForm {
    FlowForm {
        activity: a,
        id: None,
        name: "Flow".into(),
        kind: FlowKind::Control,
        source: source.0.to_string(),
        target: target.0.to_string(),
        guard: guard.into(),
    }
}
fn flow(e: &mut Editor, f: FlowForm) -> Id {
    run(e, Action::SetFlowForm(f)).unwrap();
    let id = Id(e.snapshot.next_id);
    run(e, Action::ApplyFlow).unwrap();
    id
}
#[test]
fn activity_actions_partitions_pins_and_both_flow_kinds_use_existing_allocator_and_native_history()
{
    let mut e = editor();
    let native = e.snapshot.records.clone();
    let a = activity(&mut e, "Inspect");
    let part = Id(e.snapshot.next_id);
    run(
        &mut e,
        Action::Partition {
            activity: a,
            id: None,
            name: "Reviewer".into(),
        },
    )
    .unwrap();
    let start = node(&mut e, a, NodeKind::Initial);
    let action = node(&mut e, a, NodeKind::Action);
    run(
        &mut e,
        Action::Node {
            activity: a,
            id: Some(action),
            name: "Review".into(),
            kind: NodeKind::Action,
            partition: Some(part),
        },
    )
    .unwrap();
    let end = node(&mut e, a, NodeKind::Final);
    flow(&mut e, form(a, start, action, ""));
    flow(&mut e, form(a, action, end, ""));
    let ty = define(&mut e, t::Kind::Value(t::Scalar::Text));
    let out = pin(&mut e, a, action, Direction::Out, ty);
    let input = pin(&mut e, a, action, Direction::In, ty);
    let mut f = form(a, out, input, "");
    f.kind = FlowKind::Object;
    let object = flow(&mut e, f);
    let accepted = t::load(&e.snapshot).unwrap();
    assert_eq!(
        accepted.activities[&a].flows[&object].kind,
        FlowKind::Object
    );
    assert!(diagnostics(&accepted, a).unwrap().is_empty());
    assert_eq!(e.snapshot.records, native);
    assert_eq!(
        ids(&accepted)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        10
    );
    let high = e.snapshot.next_id;
    e.undo_model(&e.freshness()).unwrap();
    assert!(
        !t::load(&e.snapshot).unwrap().activities[&a]
            .flows
            .contains_key(&object)
    );
    assert_eq!(e.snapshot.next_id, high);
    e.redo_model(&e.freshness()).unwrap();
    assert_eq!(t::load(&e.snapshot).unwrap(), accepted);
}
#[test]
fn invalid_guard_and_endpoint_proposals_remain_raw_while_prior_flow_and_allocation_stay_exact() {
    let mut e = editor();
    let a = activity(&mut e, "Inspect");
    let d = node(&mut e, a, NodeKind::Decision);
    let n = node(&mut e, a, NodeKind::Action);
    let id = flow(&mut e, form(a, d, n, "true"));
    let accepted = t::load(&e.snapshot).unwrap();
    for (source, guard) in [
        (d.0.to_string(), "x >"),
        (format!("0{}", d.0), "true"),
        (d.0.to_string(), "True"),
        ("999999".into(), "else"),
    ] {
        let mut f = form(a, d, n, guard);
        f.id = Some(id);
        f.source = source;
        run(&mut e, Action::SetFlowForm(f.clone())).unwrap();
        let before = e.snapshot.clone();
        assert!(run(&mut e, Action::ApplyFlow).is_err());
        assert_eq!(e.snapshot, before);
        assert_eq!(t::load(&e.snapshot).unwrap(), accepted);
        assert_eq!(flow_form(&e.snapshot).unwrap(), Some(f));
    }
}
#[test]
fn activity_foreign_ownership_wrong_pin_type_and_control_direction_refuse_atomically() {
    let mut e = editor();
    let a = activity(&mut e, "A");
    let b = activity(&mut e, "B");
    let start = node(&mut e, a, NodeKind::Initial);
    let end = node(&mut e, a, NodeKind::Final);
    let other = node(&mut e, b, NodeKind::Action);
    for f in [
        form(a, other, end, ""),
        form(a, end, start, ""),
        form(a, start, end, "true"),
    ] {
        run(&mut e, Action::SetFlowForm(f)).unwrap();
        let before = e.snapshot.clone();
        assert!(run(&mut e, Action::ApplyFlow).is_err());
        assert_eq!(e.snapshot, before);
    }
    let component = define(&mut e, t::Kind::Component);
    let action = node(&mut e, a, NodeKind::Action);
    for owner in [start, other, action] {
        let before = e.snapshot.clone();
        assert!(
            run(
                &mut e,
                Action::Pin {
                    activity: a,
                    id: None,
                    owner,
                    name: "bad".into(),
                    direction: Direction::In,
                    value_type: component
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot, before);
    }
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            Action::Node {
                activity: a,
                id: None,
                name: "another initial".into(),
                kind: NodeKind::Initial,
                partition: None
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
}
#[test]
fn object_payload_identity_direction_and_decision_duplicate_guard_rules_refuse() {
    let mut e = editor();
    let a = activity(&mut e, "A");
    let d = node(&mut e, a, NodeKind::Decision);
    let one = node(&mut e, a, NodeKind::Action);
    let two = node(&mut e, a, NodeKind::Action);
    flow(&mut e, form(a, d, one, "else"));
    for guard in ["else", ""] {
        run(&mut e, Action::SetFlowForm(form(a, d, two, guard))).unwrap();
        let before = e.snapshot.clone();
        assert!(run(&mut e, Action::ApplyFlow).is_err());
        assert_eq!(e.snapshot, before);
    }
    let ty = define(&mut e, t::Kind::Value(t::Scalar::Real));
    let other = define(&mut e, t::Kind::Value(t::Scalar::Real));
    let out = pin(&mut e, a, one, Direction::Out, ty);
    let input = pin(&mut e, a, two, Direction::In, other);
    for (source, target) in [(out, input), (input, out)] {
        let mut f = form(a, source, target, "");
        f.kind = FlowKind::Object;
        run(&mut e, Action::SetFlowForm(f)).unwrap();
        let before = e.snapshot.clone();
        assert!(run(&mut e, Action::ApplyFlow).is_err());
        assert_eq!(e.snapshot, before);
    }
}
#[test]
fn newer_activity_raw_guard_survives_model_undo_redo_removal_and_detached_refusal() {
    let mut e = editor();
    let a = activity(&mut e, "A");
    let d = node(&mut e, a, NodeKind::Decision);
    let n = node(&mut e, a, NodeKind::Action);
    let id = flow(&mut e, form(a, d, n, "true"));
    let mut bad = flow_form(&e.snapshot).unwrap().unwrap();
    bad.guard = "x >".into();
    run(&mut e, Action::SetFlowForm(bad.clone())).unwrap();
    let high = e.snapshot.next_id;
    e.undo_model(&e.freshness()).unwrap();
    assert_eq!(flow_form(&e.snapshot).unwrap(), Some(bad.clone()));
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::ApplyFlow).is_err());
    assert_eq!(e.snapshot, before);
    e.redo_model(&e.freshness()).unwrap();
    assert_eq!(flow_form(&e.snapshot).unwrap(), Some(bad));
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            Action::Remove {
                activity: a,
                element: n
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    run(
        &mut e,
        Action::Remove {
            activity: a,
            element: id,
        },
    )
    .unwrap();
    run(
        &mut e,
        Action::Remove {
            activity: a,
            element: n,
        },
    )
    .unwrap();
    assert_eq!(e.snapshot.next_id, high);
}
#[test]
fn activity_native_save_recovery_and_cold_open_preserve_exact_opaque_and_invalid_guard() {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    session.editor = editor();
    let a = activity(&mut session.editor, "A");
    let d = node(&mut session.editor, a, NodeKind::Decision);
    let n = node(&mut session.editor, a, NodeKind::Action);
    flow(&mut session.editor, form(a, d, n, "true"));
    let mut f = flow_form(&session.editor.snapshot).unwrap().unwrap();
    f.guard = "unparsed ?".into();
    run(&mut session.editor, Action::SetFlowForm(f.clone())).unwrap();
    session
        .editor
        .snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert(
            "opaque.optional/1".into(),
            json!({"original":[false,"payload",null]}),
        );
    let before = session.editor.snapshot.clone();
    let path = area.path().join("activity.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let cold = Session::open(area.path(), &path).unwrap();
    assert_eq!(cold.editor.snapshot, before);
    assert_eq!(flow_form(&cold.editor.snapshot).unwrap(), Some(f));
    assert!(!cold.editor.can_undo());
    let job = session.begin_recovery().unwrap();
    let result = execute_write(job);
    let recovery = result.ticket.destination.clone();
    assert!(session.complete(result));
    let cold = Session::restore(area.path(), &recovery).unwrap();
    assert_ne!(cold.editor.snapshot.document_id, before.document_id);
    let mut expected_recovery = before.clone();
    expected_recovery.document_id = cold.editor.snapshot.document_id.clone();
    assert_eq!(cold.editor.snapshot, expected_recovery);
}
#[test]
fn activity_wire_alias_unknown_fields_identity_and_legacy_empty_shapes_refuse_without_upgrade() {
    let e = editor();
    assert!(t::load(&e.snapshot).unwrap().activities.is_empty());
    let v = serde_json::to_value(t::Store::default()).unwrap();
    assert!(v.get("activities").is_none());
    let mut e = editor();
    let a = activity(&mut e, "A");
    let n = node(&mut e, a, NodeKind::Action);
    for mode in 0..5 {
        let mut s = e.snapshot.clone();
        let data = s
            .extensions
            .as_mut()
            .unwrap()
            .namespaces
            .get_mut(t::NAMESPACE)
            .unwrap();
        let key = a.0.to_string();
        match mode {
            0 => {
                let row = data["activities"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&key)
                    .unwrap();
                data["activities"]
                    .as_object_mut()
                    .unwrap()
                    .insert(format!("0{key}"), row);
            }
            1 => data["activities"][&key]["provider"] = json!("not supported"),
            2 => data["activities"][&key]["nodes"][n.0.to_string()]["id"] = json!(1),
            3 => data["activities"][&key]["system"] = json!(9999),
            _ => data["activities"][&key]["nodes"][n.0.to_string()]["kind"] = json!("ExecuteShell"),
        };
        assert!(s.validate().is_err());
    }
    let mut legacy = Snapshot::new(DocumentId("legacy".into()));
    let before = legacy.clone();
    assert!(
        t::apply(
            &mut legacy,
            t::Action::Activity(Action::Activity {
                id: None,
                system: RecordId(1),
                name: "A".into()
            })
        )
        .is_err()
    );
    assert_eq!(legacy, before);
}
#[test]
fn activity_global_count_and_byte_budgets_refuse_without_partial_allocation_or_raw_loss() {
    let mut e = editor();
    let a = activity(&mut e, "A");
    let mut s = t::load(&e.snapshot).unwrap();
    let row = s.activities.get_mut(&a).unwrap();
    for i in 0..511 {
        let id = Id(e.snapshot.next_id + i);
        row.nodes.insert(
            id,
            Node {
                id,
                name: "Action".into(),
                kind: NodeKind::Action,
                partition: None,
            },
        );
    }
    e.snapshot.next_id += 511;
    let ext = e.snapshot.extensions.as_mut().unwrap();
    ext.required_namespaces = vec![t::NAMESPACE.into()];
    ext.namespaces
        .insert(t::NAMESPACE.into(), serde_json::to_value(s).unwrap());
    e.snapshot.validate().unwrap();
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            Action::Node {
                activity: a,
                id: None,
                name: "over budget".into(),
                kind: NodeKind::Action,
                partition: None
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    let mut f = form(a, Id(1), Id(2), "true");
    f.guard = "x".repeat(4097);
    assert!(run(&mut e, Action::SetFlowForm(f)).is_err());
    assert_eq!(e.snapshot, before);
}

#[test]
fn activity_shared_extension_byte_budget_and_foreign_partition_edits_refuse_atomically() {
    let mut e = editor();
    let a = activity(&mut e, "A");
    let b = activity(&mut e, "B");
    let p = Id(e.snapshot.next_id);
    run(
        &mut e,
        Action::Partition {
            activity: b,
            id: None,
            name: "Foreign".into(),
        },
    )
    .unwrap();
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            Action::Node {
                activity: a,
                id: None,
                name: "wrong partition".into(),
                kind: NodeKind::Action,
                partition: Some(p)
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    e.snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert("opaque.padding/1".into(), json!(""));
    let remaining = mbse_model::extensions::MAX_BYTES
        - serde_json::to_vec(e.snapshot.extensions.as_ref().unwrap())
            .unwrap()
            .len();
    e.snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert("opaque.padding/1".into(), json!("x".repeat(remaining - 64)));
    e.snapshot.validate().unwrap();
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::SetFlowForm(form(a, Id(1), Id(2), "true"))).is_err());
    assert_eq!(e.snapshot, before);
}
