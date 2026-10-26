use super::*;
use crate::stage1::{Command, DocumentId, Editor};
fn editor() -> Editor {
    let mut s = Snapshot::new(DocumentId("native:decomposition-fixture".into()));
    s.extensions = Some(Extensions::default());
    Editor::new(s, "parts:test".into()).unwrap()
}
fn run(e: &mut Editor, a: Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(a)).map(|_| ())
}
fn component(e: &mut Editor, name: &str) -> Id {
    let id = Id(e.snapshot.next_id);
    run(
        e,
        Action::AddDefinition {
            system: RecordId(1),
            name: name.into(),
            kind: Kind::Component,
        },
    )
    .unwrap();
    id
}
fn draft(e: &mut Editor, id: Option<Id>, owner: Id, target: Id, kind: UsageKind) {
    run(
        e,
        Action::SetUsageForm {
            form: UsageForm {
                id,
                name: "subassembly".into(),
                owner: owner.0.to_string(),
                target: target.0.to_string(),
                kind,
            },
        },
    )
    .unwrap()
}
fn add(e: &mut Editor, owner: Id, target: Id, kind: UsageKind) -> Id {
    draft(e, None, owner, target, kind);
    let id = Id(e.snapshot.next_id);
    run(e, Action::ApplyUsage).unwrap();
    id
}
#[test]
fn direct_and_indirect_containment_cycles_refuse_atomically_but_references_do_not_contain() {
    let mut e = editor();
    let a = component(&mut e, "A");
    let b = component(&mut e, "B");
    let c = component(&mut e, "C");
    add(&mut e, a, b, UsageKind::Part);
    add(&mut e, b, c, UsageKind::Part);
    for (owner, target) in [(c, a), (c, c)] {
        draft(&mut e, None, owner, target, UsageKind::Part);
        let before = e.snapshot.clone();
        assert!(
            preview_usage(&e.snapshot)
                .unwrap_err()
                .contains("CONTAINMENT_CYCLE")
        );
        assert!(run(&mut e, Action::ApplyUsage).is_err());
        assert_eq!(e.snapshot, before);
    }
    let reference = add(&mut e, c, a, UsageKind::Reference);
    assert_eq!(
        load(&e.snapshot).unwrap().usages[&reference].kind,
        UsageKind::Reference
    );
}
#[test]
fn moving_owned_usage_keeps_identity_and_target_through_undo_redo_and_retains_newer_raw_text() {
    let mut e = editor();
    let a = component(&mut e, "A");
    let b = component(&mut e, "B");
    let c = component(&mut e, "C");
    let id = add(&mut e, a, c, UsageKind::Part);
    let next = e.snapshot.next_id;
    draft(&mut e, Some(id), b, c, UsageKind::Part);
    run(&mut e, Action::ApplyUsage).unwrap();
    assert_eq!(load(&e.snapshot).unwrap().usages[&id].owner, b);
    assert_eq!(e.snapshot.next_id, next);
    let mut invalid = usage_form(&e.snapshot).unwrap().unwrap();
    invalid.target = "incomplete#".into();
    run(
        &mut e,
        Action::SetUsageForm {
            form: invalid.clone(),
        },
    )
    .unwrap();
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(load(&e.snapshot).unwrap().usages[&id].owner, a);
    assert_eq!(usage_form(&e.snapshot).unwrap(), Some(invalid.clone()));
    assert!(e.redo_model(&e.freshness()).unwrap());
    let u = &load(&e.snapshot).unwrap().usages[&id];
    assert_eq!((u.id, u.owner, u.target), (id, b, c));
    assert_eq!(usage_form(&e.snapshot).unwrap(), Some(invalid));
}
#[test]
fn reusable_part_types_create_distinct_usage_ids_and_wrong_kind_cross_system_targets_refuse() {
    let mut e = editor();
    let a = component(&mut e, "A");
    let b = component(&mut e, "B");
    let first = add(&mut e, a, b, UsageKind::Part);
    let second = add(&mut e, a, b, UsageKind::Part);
    assert_ne!(first, second);
    let value = Id(e.snapshot.next_id);
    run(
        &mut e,
        Action::AddDefinition {
            system: RecordId(1),
            name: "Real".into(),
            kind: Kind::Value(Scalar::Real),
        },
    )
    .unwrap();
    for (owner, target) in [(a, value), (value, a)] {
        draft(&mut e, None, owner, target, UsageKind::Reference);
        let before = e.snapshot.clone();
        assert!(
            run(&mut e, Action::ApplyUsage)
                .unwrap_err()
                .contains("WRONG_KIND")
        );
        assert_eq!(e.snapshot, before);
    }
    let system = RecordId(e.snapshot.next_id);
    e.snapshot.next_id += 1;
    let mut other = e.snapshot.records[&RecordId(1)].clone();
    other.id = system;
    other.scope = system;
    other.name = "Other System".into();
    e.snapshot.records.insert(system, other);
    let target = Id(e.snapshot.next_id);
    run(
        &mut e,
        Action::AddDefinition {
            system,
            name: "Other component".into(),
            kind: Kind::Component,
        },
    )
    .unwrap();
    draft(&mut e, None, a, target, UsageKind::Part);
    assert!(
        run(&mut e, Action::ApplyUsage)
            .unwrap_err()
            .contains("CROSS_SYSTEM")
    );
}
#[test]
fn actual_native_roundtrip_preserves_usages_and_invalid_form_beside_p15_definition_and_property_data()
 {
    use crate::stage1_io::{Session, execute_write};
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    session.editor = editor();
    let e = &mut session.editor;
    let a = component(e, "Subsystem");
    let b = component(e, "Device");
    let id = add(e, a, b, UsageKind::Part);
    let definition = DefinitionForm {
        system: RecordId(1),
        name: "Pending definition".into(),
        value: true,
        scalar: Scalar::Integer,
    };
    run(
        e,
        Action::SetDefinitionForm {
            form: definition.clone(),
        },
    )
    .unwrap();
    let mut form = usage_form(&e.snapshot).unwrap().unwrap();
    form.owner = "unfinished".into();
    form.target = "18446744073709551615".into();
    run(e, Action::SetUsageForm { form: form.clone() }).unwrap();
    assert!(preview_usage(&e.snapshot).is_err());
    let expected = e.snapshot.clone();
    let path = area.path().join("decomposition.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let reopened = Session::open(area.path(), &path).unwrap();
    assert_eq!(reopened.editor.snapshot, expected);
    assert_eq!(
        load(&reopened.editor.snapshot).unwrap().usages[&id].target,
        b
    );
    assert_eq!(usage_form(&reopened.editor.snapshot).unwrap(), Some(form));
    assert_eq!(
        definition_form(&reopened.editor.snapshot).unwrap(),
        Some(definition)
    );
    assert!(!reopened.editor.can_undo());
}
#[test]
fn detached_usage_drafts_do_not_recreate_identities_and_identical_apply_is_noop() {
    let mut e = editor();
    let a = component(&mut e, "A");
    let b = component(&mut e, "B");
    let id = add(&mut e, a, b, UsageKind::Part);
    let before = e.snapshot.clone();
    run(&mut e, Action::ApplyUsage).unwrap();
    assert_eq!(e.snapshot, before);
    let ticket = e.freshness();
    let form = usage_form(&e.snapshot).unwrap().unwrap();
    run(&mut e, Action::SetUsageForm { form }).unwrap();
    assert_eq!(e.snapshot, before);
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert!(!load(&e.snapshot).unwrap().usages.contains_key(&id));
    let before = e.snapshot.clone();
    assert!(
        run(&mut e, Action::ApplyUsage)
            .unwrap_err()
            .contains("DETACHED_DRAFT")
    );
    assert_eq!(e.snapshot, before);
    assert!(
        e.command(&ticket, Command::Typed(Action::ApplyUsage))
            .is_err()
    );
}
#[test]
fn serialized_usage_identity_collisions_noncanonical_keys_and_invalid_graphs_refuse() {
    let mut e = editor();
    let a = component(&mut e, "A");
    let b = component(&mut e, "B");
    let id = add(&mut e, a, b, UsageKind::Part);
    for mode in 0..3 {
        let mut s = e.snapshot.clone();
        let data = s
            .extensions
            .as_mut()
            .unwrap()
            .namespaces
            .get_mut(NAMESPACE)
            .unwrap();
        match mode {
            0 => {
                let mut u = data["usages"][id.0.to_string()].clone();
                u["id"] = serde_json::json!(a.0);
                data["usages"]
                    .as_object_mut()
                    .unwrap()
                    .insert(a.0.to_string(), u);
            }
            1 => {
                let u = data["usages"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&id.0.to_string())
                    .unwrap();
                data["usages"]
                    .as_object_mut()
                    .unwrap()
                    .insert(format!("0{}", id.0), u);
            }
            _ => data["usages"][id.0.to_string()]["target"] = serde_json::json!(a.0),
        }
        assert!(s.validate().is_err());
    }
}
