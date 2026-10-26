use super::*;
use crate::stage1::{Command, Editor};
use connectors::{Action as CA, Form};
fn run(e: &mut Editor, a: Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(a)).map(|_| ())
}
fn editor() -> Editor {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/parity/p18a/fixtures/ports.kernspan"
    ))
    .unwrap();
    let mut e = Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "connector:test".into(),
    )
    .unwrap();
    run(
        &mut e,
        Action::Port(ports::Action::BeginPort { owner: Id(1007) }),
    )
    .unwrap();
    run(
        &mut e,
        Action::Port(ports::Action::SetPortForm {
            id: Id(1017),
            form: ports::PortForm {
                owner: Id(1007),
                name: "output".into(),
                contract: "1014".into(),
                assignment: String::new(),
            },
        }),
    )
    .unwrap();
    run(
        &mut e,
        Action::Port(ports::Action::ApplyPort { id: Id(1017) }),
    )
    .unwrap();
    e
}
fn form() -> Form {
    Form {
        owner: Id(1004),
        name: "Measurement transfer".into(),
        source_usage: "1009".into(),
        source_port: "1017".into(),
        target_usage: "1010".into(),
        target_port: "1015".into(),
        conveyed: "1005".into(),
    }
}
fn draft(e: &mut Editor) -> Id {
    let id = Id(e.snapshot.next_id);
    run(e, Action::Connector(CA::Begin { owner: Id(1004) })).unwrap();
    run(e, Action::Connector(CA::Set { id, form: form() })).unwrap();
    id
}
fn accept(e: &mut Editor) -> Id {
    let id = draft(e);
    run(e, Action::Connector(CA::Apply { id })).unwrap();
    id
}
#[test]
fn connector_qualifies_two_instances_of_same_type_and_uses_effective_interface() {
    let mut e = editor();
    let id = accept(&mut e);
    let s = load(&e.snapshot).unwrap();
    let c = &s.connectors[&id];
    assert_ne!(c.source.usage, c.target.usage);
    assert_eq!(
        s.usages[&c.source.usage].target,
        s.usages[&c.target.usage].target
    );
    assert_eq!(
        connectors::endpoint_interface(&s, c.owner, c.target)
            .unwrap()
            .id,
        Id(1013)
    );
    assert_eq!(c.conveyed, Id(1005));
    assert_eq!(c.source.port, Id(1017));
    assert_eq!(c.target.port, Id(1015));
}
#[test]
fn invalid_endpoint_kind_owner_port_direction_and_exact_payload_refuse_atomically() {
    let mut e = editor();
    let id = draft(&mut e);
    for mode in 0..10 {
        let mut f = form();
        match mode {
            0 => f.source_usage = "1011".into(),
            1 => f.source_usage = "1004".into(),
            2 => f.owner = Id(1008),
            3 => f.source_port = "1005".into(),
            4 => f.source_port = "1015".into(),
            5 => f.target_port = "1017".into(),
            6 => {
                f.target_usage = f.source_usage.clone();
                f.target_port = f.source_port.clone();
            }
            7 => f.conveyed = "1004".into(),
            8 => f.conveyed = "01005".into(),
            _ => f.conveyed = "1005?".into(),
        };
        run(
            &mut e,
            Action::Connector(CA::Set {
                id,
                form: f.clone(),
            }),
        )
        .unwrap();
        let before = e.snapshot.clone();
        assert!(connectors::preview(&e.snapshot, id).is_err(), "mode{mode}");
        assert!(
            run(&mut e, Action::Connector(CA::Apply { id })).is_err(),
            "mode{mode}"
        );
        assert_eq!(e.snapshot, before);
        assert_eq!(connectors::forms(&e.snapshot).unwrap()[&id], f);
    }
    // Matching scalar shape does not make distinct Value definitions compatible.
    let other = Id(e.snapshot.next_id);
    run(
        &mut e,
        Action::AddDefinition {
            system: RecordId(1),
            name: "Another real".into(),
            kind: Kind::Value(Scalar::Real),
        },
    )
    .unwrap();
    let mut f = form();
    f.conveyed = other.0.to_string();
    run(&mut e, Action::Connector(CA::Set { id, form: f })).unwrap();
    assert!(
        connectors::preview(&e.snapshot, id)
            .unwrap_err()
            .contains("INCOMPATIBLE")
    );
    // Existing port on another Component cannot be used on this Part's type.
    let mut s = load(&e.snapshot).unwrap();
    s.ports.get_mut(&Id(1017)).unwrap().owner = Id(1008);
    put(&mut e.snapshot, NAMESPACE, &s).unwrap();
    run(&mut e, Action::Connector(CA::Set { id, form: form() })).unwrap();
    assert!(
        connectors::preview(&e.snapshot, id)
            .unwrap_err()
            .contains("NOT_ON_PART_TYPE")
    );
}
#[test]
fn undo_redo_keeps_newer_invalid_connector_text_monotonic_ids_and_stale_refusal() {
    let mut e = editor();
    let id = accept(&mut e);
    let before = e.snapshot.clone();
    run(&mut e, Action::Connector(CA::Apply { id })).unwrap();
    assert_eq!(before, e.snapshot);
    let stale = e.freshness();
    let mut f = form();
    f.conveyed = "1005?".into();
    run(
        &mut e,
        Action::Connector(CA::Set {
            id,
            form: f.clone(),
        }),
    )
    .unwrap();
    let raw = e.snapshot.clone();
    assert!(
        e.command(&stale, Command::Typed(Action::Connector(CA::Apply { id })))
            .is_err()
    );
    assert_eq!(e.snapshot, raw);
    let next = e.snapshot.next_id;
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert!(!load(&e.snapshot).unwrap().connectors.contains_key(&id));
    assert_eq!(connectors::forms(&e.snapshot).unwrap()[&id], f);
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert_eq!(
        load(&e.snapshot).unwrap().connectors[&id].conveyed,
        Id(1005)
    );
    assert_eq!(connectors::forms(&e.snapshot).unwrap()[&id], f);
    assert_eq!(e.snapshot.next_id, next);
}
#[test]
fn moving_connected_part_out_of_owner_refuses_until_connection_undone() {
    let mut e = editor();
    let id = accept(&mut e);
    let u = load(&e.snapshot).unwrap().usages[&Id(1009)].clone();
    let moved = UsageForm {
        id: Some(u.id),
        owner: "1008".into(),
        name: u.name.clone(),
        target: u.target.0.to_string(),
        kind: u.kind,
    };
    run(
        &mut e,
        Action::SetUsageForm {
            form: moved.clone(),
        },
    )
    .unwrap();
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::ApplyUsage).is_err());
    assert_eq!(e.snapshot, before);
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert!(!load(&e.snapshot).unwrap().connectors.contains_key(&id));
    run(&mut e, Action::ApplyUsage).unwrap();
    assert_eq!(load(&e.snapshot).unwrap().usages[&Id(1009)].owner, Id(1008));
}
#[test]
fn malformed_connector_and_cross_journal_id_collisions_refuse() {
    let mut e = editor();
    let id = accept(&mut e);
    for mode in 0..5 {
        let mut s = e.snapshot.clone();
        let ext = s.extensions.as_mut().unwrap();
        if mode == 4 {
            let j = ext.namespaces.get_mut(DRAFTS).unwrap();
            let f = j["connectors"][id.0.to_string()].clone();
            j["connectors"]
                .as_object_mut()
                .unwrap()
                .insert("1015".into(), f);
        } else {
            let d = ext.namespaces.get_mut(NAMESPACE).unwrap();
            match mode {
                0 => d["connectors"][id.0.to_string()]["source"]["usage"] = serde_json::json!(1011),
                1 => {
                    let c = d["connectors"]
                        .as_object_mut()
                        .unwrap()
                        .remove(&id.0.to_string())
                        .unwrap();
                    d["connectors"]
                        .as_object_mut()
                        .unwrap()
                        .insert(format!("0{}", id.0), c);
                }
                2 => {
                    let mut c = d["connectors"][id.0.to_string()].clone();
                    c["id"] = serde_json::json!(1004);
                    d["connectors"]
                        .as_object_mut()
                        .unwrap()
                        .insert("1004".into(), c);
                }
                _ => d["connectors"][id.0.to_string()]["conveyed"] = serde_json::json!(1007),
            }
        }
        assert!(s.validate().is_err(), "mode{mode}");
    }
}
#[test]
fn captured_native_save_cold_reopen_retains_connector_raw_forms_prior_semantics_and_views() {
    use crate::{
        native_views::{Views, ports as vp},
        native_views_io::{Io, Kind as WriteKind, accepted_native},
        stage1_io::{Session, execute_write},
    };
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let instance = session.editor.freshness().instance_id;
    session.editor = Editor::new(editor().snapshot, instance).unwrap();
    let prior = load(&session.editor.snapshot).unwrap();
    let prior_j = journal(&session.editor.snapshot).unwrap();
    let id = accept(&mut session.editor);
    let mut f = form();
    f.target_port = "unfinished#".into();
    run(
        &mut session.editor,
        Action::Connector(CA::Set {
            id,
            form: f.clone(),
        }),
    )
    .unwrap();
    let expected = session.editor.snapshot.clone();
    let mut v = Views::new(&session.editor).unwrap();
    let view = *v.document.views.keys().next().unwrap();
    v.apply(
        &session.editor,
        &v.stamp(&session.editor),
        crate::native_views::Layout::Port(vp::Layout::Reuse {
            view,
            port: Id(1016),
            geometry: crate::native_views::Geometry::at(20., 30.),
        }),
    )
    .unwrap();
    let core = v.document.occurrences.clone();
    let ports = v.document.typed_ports.clone();
    let path = area.path().join("connectors.kernspan");
    let side = area.path().join("connectors.mbse-views.json");
    let mut io = Io::default();
    let dest = io
        .freeze_destination(&session, &v, &side, WriteKind::Bound)
        .unwrap();
    let capture = v.capture(&session.editor);
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let job = io.begin_captured(&session, &v, capture, dest).unwrap();
    assert!(io.complete(&session, &v, crate::native_views_io::execute(job)));
    assert!(io.is_saved(&session, &v));
    let cold = Session::open(area.path(), &path).unwrap();
    assert_eq!(cold.editor.snapshot, expected);
    assert!(!cold.editor.can_undo());
    let after = load(&cold.editor.snapshot).unwrap();
    assert_eq!(after.definitions, prior.definitions);
    assert_eq!(after.properties, prior.properties);
    assert_eq!(after.usages, prior.usages);
    assert_eq!(after.ports, prior.ports);
    let j = journal(&cold.editor.snapshot).unwrap();
    assert_eq!(j.forms, prior_j.forms);
    assert_eq!(j.usage, prior_j.usage);
    assert_eq!(j.ports, prior_j.ports);
    assert_eq!(j.interface, prior_j.interface);
    assert_eq!(j.definition, prior_j.definition);
    assert_eq!(j.connectors[&id], f);
    let views = Views::decode(
        &std::fs::read(side).unwrap(),
        &cold.editor,
        &accepted_native(&cold).unwrap(),
    )
    .unwrap();
    assert_eq!(views.document.occurrences, core);
    assert_eq!(views.document.typed_ports, ports);
}
#[test]
fn old_type_store_and_journal_wire_values_do_not_gain_empty_connector_fields() {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/parity/p18a/fixtures/ports.kernspan"
    ))
    .unwrap();
    let s: Snapshot = serde_json::from_value(v["snapshot"].clone()).unwrap();
    s.validate().unwrap();
    let ext = s.extensions.as_ref().unwrap();
    assert_eq!(
        serde_json::to_value(load(&s).unwrap()).unwrap(),
        ext.namespaces[NAMESPACE]
    );
    assert_eq!(
        serde_json::to_value(journal(&s).unwrap()).unwrap(),
        ext.namespaces[DRAFTS]
    );
}
