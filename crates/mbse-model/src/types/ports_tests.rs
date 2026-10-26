use super::*;
use crate::stage1::{Command, DocumentId, Editor};
use ports::{Action as PA, Direction, InterfaceForm, PortForm};
fn editor() -> Editor {
    let mut s = Snapshot::new(DocumentId("native:port-fixture".into()));
    s.extensions = Some(Extensions::default());
    Editor::new(s, "ports:test".into()).unwrap()
}
fn run(e: &mut Editor, action: Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(action))
        .map(|_| ())
}
fn define(e: &mut Editor, label: &str, kind: Kind) -> Id {
    let id = Id(e.snapshot.next_id);
    run(
        e,
        Action::AddDefinition {
            system: RecordId(1),
            name: label.into(),
            kind,
        },
    )
    .unwrap();
    id
}
fn interface(e: &mut Editor, payload: Id, direction: Direction) -> Id {
    run(
        e,
        Action::Port(PA::SetInterfaceForm(InterfaceForm {
            system: RecordId(1),
            name: "Transfer".into(),
            payload: payload.0.to_string(),
            direction,
        })),
    )
    .unwrap();
    let id = Id(e.snapshot.next_id);
    run(e, Action::Port(PA::AddInterface)).unwrap();
    id
}
fn port(e: &mut Editor, owner: Id, contract: Id, assignment: Id) -> Id {
    let id = Id(e.snapshot.next_id);
    run(e, Action::Port(PA::BeginPort { owner })).unwrap();
    run(
        e,
        Action::Port(PA::SetPortForm {
            id,
            form: PortForm {
                owner,
                name: "input".into(),
                contract: contract.0.to_string(),
                assignment: assignment.0.to_string(),
            },
        }),
    )
    .unwrap();
    run(e, Action::Port(PA::ApplyPort { id })).unwrap();
    id
}
#[test]
fn distinct_interface_ids_assign_when_exact_payload_direction_and_system_agree() {
    let mut e = editor();
    let owner = define(&mut e, "Device", Kind::Component);
    let value = define(&mut e, "Distance", Kind::Value(Scalar::Real));
    let contract = interface(&mut e, value, Direction::In);
    let assignment = interface(&mut e, value, Direction::In);
    assert_ne!(contract, assignment);
    let id = port(&mut e, owner, contract, assignment);
    let p = &load(&e.snapshot).unwrap().ports[&id];
    assert_eq!(
        (p.id, p.owner, p.contract, p.assignment),
        (id, owner, contract, Some(assignment))
    );
}
#[test]
fn incompatible_payload_direction_missing_and_wrong_kind_assignments_refuse_atomically() {
    let mut e = editor();
    let owner = define(&mut e, "Device", Kind::Component);
    let real = define(&mut e, "Distance", Kind::Value(Scalar::Real));
    let other_real = define(
        &mut e,
        "Different value identity",
        Kind::Value(Scalar::Real),
    );
    let input = interface(&mut e, real, Direction::In);
    let output = interface(&mut e, real, Direction::Out);
    let different = interface(&mut e, other_real, Direction::In);
    let id = port(&mut e, owner, input, input);
    for assignment in [output, different, owner, Id(9000)] {
        let mut f = ports::forms(&e.snapshot).unwrap()[&id].clone();
        f.assignment = assignment.0.to_string();
        run(&mut e, Action::Port(PA::SetPortForm { id, form: f })).unwrap();
        let before = e.snapshot.clone();
        assert!(ports::preview(&e.snapshot, id).is_err());
        assert!(run(&mut e, Action::Port(PA::ApplyPort { id })).is_err());
        assert_eq!(e.snapshot, before);
    }
}
#[test]
fn wrong_payload_and_port_owner_and_cross_system_contract_refuse() {
    let mut e = editor();
    let owner = define(&mut e, "Device", Kind::Component);
    let value = define(&mut e, "Real", Kind::Value(Scalar::Real));
    run(
        &mut e,
        Action::Port(PA::SetInterfaceForm(InterfaceForm {
            system: RecordId(1),
            name: "Wrong payload".into(),
            payload: owner.0.to_string(),
            direction: Direction::In,
        })),
    )
    .unwrap();
    let before = e.snapshot.clone();
    assert!(
        run(&mut e, Action::Port(PA::AddInterface))
            .unwrap_err()
            .contains("WRONG_KIND")
    );
    assert_eq!(e.snapshot, before);
    assert!(run(&mut e, Action::Port(PA::BeginPort { owner: value })).is_err());
    let contract = interface(&mut e, value, Direction::In);
    let system = RecordId(e.snapshot.next_id);
    e.snapshot.next_id += 1;
    let mut other = e.snapshot.records[&RecordId(1)].clone();
    other.id = system;
    other.scope = system;
    other.name = "Other System".into();
    e.snapshot.records.insert(system, other);
    let other_owner = Id(e.snapshot.next_id);
    run(
        &mut e,
        Action::AddDefinition {
            system,
            name: "Other device".into(),
            kind: Kind::Component,
        },
    )
    .unwrap();
    let id = Id(e.snapshot.next_id);
    run(&mut e, Action::Port(PA::BeginPort { owner: other_owner })).unwrap();
    run(
        &mut e,
        Action::Port(PA::SetPortForm {
            id,
            form: PortForm {
                owner: other_owner,
                name: "Cross".into(),
                contract: contract.0.to_string(),
                assignment: String::new(),
            },
        }),
    )
    .unwrap();
    assert!(
        run(&mut e, Action::Port(PA::ApplyPort { id }))
            .unwrap_err()
            .contains("CROSS_SYSTEM")
    );
}
#[test]
fn port_model_undo_redo_retains_newer_invalid_text_and_repeated_apply_is_noop() {
    let mut e = editor();
    let owner = define(&mut e, "Device", Kind::Component);
    let value = define(&mut e, "Real", Kind::Value(Scalar::Real));
    let contract = interface(&mut e, value, Direction::In);
    let id = port(&mut e, owner, contract, contract);
    let before = e.snapshot.clone();
    run(&mut e, Action::Port(PA::ApplyPort { id })).unwrap();
    assert_eq!(e.snapshot, before);
    let mut f = ports::forms(&e.snapshot).unwrap()[&id].clone();
    f.assignment = "pending#".into();
    run(
        &mut e,
        Action::Port(PA::SetPortForm {
            id,
            form: f.clone(),
        }),
    )
    .unwrap();
    let next = e.snapshot.next_id;
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert!(!load(&e.snapshot).unwrap().ports.contains_key(&id));
    assert_eq!(ports::forms(&e.snapshot).unwrap()[&id], f);
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert_eq!(
        load(&e.snapshot).unwrap().ports[&id].assignment,
        Some(contract)
    );
    assert_eq!(ports::forms(&e.snapshot).unwrap()[&id], f);
    assert_eq!(e.snapshot.next_id, next);
}
#[test]
fn native_save_cold_reopen_keeps_port_contracts_both_invalid_forms_and_prior_p16_data() {
    use crate::stage1_io::{Session, execute_write};
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/parity/p17a/fixtures/structure.kernspan"
    ))
    .unwrap();
    let snapshot: Snapshot = serde_json::from_value(source["snapshot"].clone()).unwrap();
    let prior = load(&snapshot).unwrap();
    let prior_forms = forms(&snapshot).unwrap();
    let prior_usage = usage_form(&snapshot).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    session.editor = Editor::new(snapshot, "ports:reopen".into()).unwrap();
    let contract = interface(&mut session.editor, Id(1005), Direction::In);
    let id = port(&mut session.editor, Id(1004), contract, contract);
    let mut f = ports::forms(&session.editor.snapshot).unwrap()[&id].clone();
    f.assignment = "incomplete#".into();
    run(
        &mut session.editor,
        Action::Port(PA::SetPortForm {
            id,
            form: f.clone(),
        }),
    )
    .unwrap();
    let unfinished = InterfaceForm {
        system: RecordId(1),
        name: "Pending interface".into(),
        payload: "1005?".into(),
        direction: Direction::InOut,
    };
    run(
        &mut session.editor,
        Action::Port(PA::SetInterfaceForm(unfinished.clone())),
    )
    .unwrap();
    let expected = session.editor.snapshot.clone();
    let path = area.path().join("ports.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let reopened = Session::open(area.path(), &path).unwrap();
    assert_eq!(reopened.editor.snapshot, expected);
    let accepted = load(&reopened.editor.snapshot).unwrap();
    assert_eq!(accepted.definitions, prior.definitions);
    assert_eq!(accepted.properties, prior.properties);
    assert_eq!(accepted.usages, prior.usages);
    assert_eq!(forms(&reopened.editor.snapshot).unwrap(), prior_forms);
    assert_eq!(usage_form(&reopened.editor.snapshot).unwrap(), prior_usage);
    assert_eq!(ports::forms(&reopened.editor.snapshot).unwrap()[&id], f);
    assert_eq!(
        ports::interface_form(&reopened.editor.snapshot).unwrap(),
        Some(unfinished)
    );
    assert!(!reopened.editor.can_undo());
}
#[test]
fn serialized_port_identity_collisions_and_noncanonical_interface_keys_refuse() {
    let mut e = editor();
    let owner = define(&mut e, "Device", Kind::Component);
    let value = define(&mut e, "Real", Kind::Value(Scalar::Real));
    let contract = interface(&mut e, value, Direction::In);
    let id = port(&mut e, owner, contract, contract);
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
                let mut p = data["ports"][id.0.to_string()].clone();
                p["id"] = serde_json::json!(owner.0);
                data["ports"]
                    .as_object_mut()
                    .unwrap()
                    .insert(owner.0.to_string(), p);
            }
            1 => {
                let i = data["interfaces"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&contract.0.to_string())
                    .unwrap();
                data["interfaces"]
                    .as_object_mut()
                    .unwrap()
                    .insert(format!("0{}", contract.0), i);
            }
            _ => {
                data["ports"][id.0.to_string()]["assignment"] = serde_json::json!(owner.0);
            }
        }
        assert!(s.validate().is_err());
    }
}
#[test]
fn p16_values_serialize_without_empty_port_or_interface_fields() {
    let raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/parity/p17a/fixtures/structure.kernspan"
    ))
    .unwrap();
    let snapshot: Snapshot = serde_json::from_value(raw["snapshot"].clone()).unwrap();
    snapshot.validate().unwrap();
    let ext = snapshot.extensions.as_ref().unwrap();
    assert_eq!(
        serde_json::to_value(load(&snapshot).unwrap()).unwrap(),
        ext.namespaces[NAMESPACE]
    );
    assert_eq!(
        serde_json::to_value(journal(&snapshot).unwrap()).unwrap(),
        ext.namespaces[DRAFTS]
    );
}
