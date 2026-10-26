use super::*;
use crate::stage1::{Command, Editor};
use values::{Action as VA, Form as VF, Origin, Target};
fn run(e: &mut Editor, a: Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(a)).map(|_| ())
}
fn add_property(e: &mut Editor, value_type: Id) -> Id {
    let id = Id(e.snapshot.next_id);
    run(e, Action::BeginProperty { owner: Id(1007) }).unwrap();
    run(
        e,
        Action::SetForm {
            id,
            form: Form {
                owner: Id(1007),
                name: "instance gain".into(),
                value_type: value_type.0.to_string(),
                lower: "1".into(),
                upper: "1".into(),
            },
        },
    )
    .unwrap();
    run(e, Action::ApplyProperty { id }).unwrap();
    id
}
fn editor() -> Editor {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/parity/p19/fixtures/connectors.kernspan"
    ))
    .unwrap();
    let mut e = Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "values:test".into(),
    )
    .unwrap();
    assert_eq!(add_property(&mut e, Id(1005)), Id(1020));
    e
}
fn default() -> Target {
    Target::Default { property: Id(1020) }
}
fn part(id: u64) -> Target {
    Target::Override {
        usage: Id(id),
        property: Id(1020),
    }
}
fn set(e: &mut Editor, target: Target, text: &str) {
    run(e, Action::Value(VA::Begin(target))).unwrap();
    let mut f = values::forms(&e.snapshot).unwrap()[&target.key()].clone();
    f.text = text.into();
    run(e, Action::Value(VA::Set(f))).unwrap();
}
fn accept(e: &mut Editor, target: Target, text: &str) {
    set(e, target, text);
    run(e, Action::Value(VA::Apply(target))).unwrap();
}
#[test]
fn defaults_update_only_inheriting_parts_and_clear_override_returns_to_inheritance() {
    let mut e = editor();
    accept(&mut e, default(), "12.5");
    accept(&mut e, part(1009), "23");
    let before = load(&e.snapshot).unwrap().overrides;
    accept(&mut e, default(), "17");
    let s = load(&e.snapshot).unwrap();
    assert_eq!(s.overrides, before);
    let explicit = values::effective(&s, part(1009)).unwrap();
    let inherited = values::effective(&s, part(1010)).unwrap();
    assert_eq!(
        (explicit.value.unwrap().text, explicit.origin),
        ("23".into(), Origin::Override)
    );
    assert_eq!(
        (inherited.value.unwrap().text, inherited.origin),
        ("17".into(), Origin::Default)
    );
    run(&mut e, Action::Value(VA::Clear(part(1009)))).unwrap();
    assert_eq!(
        values::effective(&load(&e.snapshot).unwrap(), part(1009))
            .unwrap()
            .origin,
        Origin::Default
    );
    run(&mut e, Action::Value(VA::Clear(default()))).unwrap();
    assert_eq!(
        values::effective(&load(&e.snapshot).unwrap(), part(1010))
            .unwrap()
            .origin,
        Origin::Unset
    );
}
#[test]
fn bad_literals_wrong_instance_context_and_multivalued_property_changes_refuse_atomically() {
    let mut e = editor();
    accept(&mut e, default(), "12.5");
    for text in ["2e+", "NaN", "inf", "true", "1e999"] {
        set(&mut e, default(), text);
        let before = e.snapshot.clone();
        assert!(values::preview(&e.snapshot, default()).is_err());
        assert!(run(&mut e, Action::Value(VA::Apply(default()))).is_err());
        assert_eq!(e.snapshot, before);
        assert_eq!(
            values::forms(&e.snapshot).unwrap()[&default().key()].text,
            text
        );
    }
    for target in [
        part(1011),
        Target::Override {
            usage: Id(1009),
            property: Id(1006),
        },
        Target::Default { property: Id(1005) },
    ] {
        let before = e.snapshot.clone();
        assert!(run(&mut e, Action::Value(VA::Begin(target))).is_err());
        assert_eq!(e.snapshot, before);
    }
    let mut f = forms(&e.snapshot).unwrap()[&Id(1020)].clone();
    f.upper = "2".into();
    run(
        &mut e,
        Action::SetForm {
            id: Id(1020),
            form: f,
        },
    )
    .unwrap();
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::ApplyProperty { id: Id(1020) }).is_err());
    assert_eq!(e.snapshot, before);
}
#[test]
fn model_undo_redo_clear_and_stale_apply_preserve_newer_invalid_value_expression_text() {
    let mut e = editor();
    accept(&mut e, default(), "12.5");
    accept(&mut e, part(1009), "23");
    let before = e.snapshot.clone();
    run(&mut e, Action::Value(VA::Apply(part(1009)))).unwrap();
    assert_eq!(e.snapshot, before);
    let stale = e.freshness();
    let f = VF {
        target: part(1009),
        text: "2e+".into(),
        expression: "x*".into(),
    };
    run(&mut e, Action::Value(VA::Set(f.clone()))).unwrap();
    let raw = e.snapshot.clone();
    assert!(
        e.command(&stale, Command::Typed(Action::Value(VA::Apply(part(1009)))))
            .is_err()
    );
    assert_eq!(e.snapshot, raw);
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(
        values::effective(&load(&e.snapshot).unwrap(), part(1009))
            .unwrap()
            .origin,
        Origin::Default
    );
    assert_eq!(values::forms(&e.snapshot).unwrap()[&part(1009).key()], f);
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert_eq!(
        values::effective(&load(&e.snapshot).unwrap(), part(1009))
            .unwrap()
            .value
            .unwrap()
            .text,
        "23"
    );
    run(&mut e, Action::Value(VA::Clear(part(1009)))).unwrap();
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(values::forms(&e.snapshot).unwrap()[&part(1009).key()], f);
    assert_eq!(
        values::effective(&load(&e.snapshot).unwrap(), part(1009))
            .unwrap()
            .origin,
        Origin::Override
    );
}
#[test]
fn derived_previews_use_accepted_effective_values_are_pure_and_invalidate_on_input_or_instance_change()
 {
    let mut e = editor();
    accept(&mut e, default(), "12.5");
    accept(&mut e, part(1009), "23");
    set(&mut e, part(1009), "unfinished");
    let before = e.snapshot.clone();
    let report = values::derive(&e, part(1009), "x * 2 + 1").unwrap();
    assert_eq!(report.value(), Some(47.));
    assert_eq!(report.effective().origin, Origin::Override);
    assert!(report.current(&e));
    assert_eq!(e.snapshot, before);
    let other = Editor::new(before.clone(), "values:other-instance".into()).unwrap();
    assert!(!report.current(&other));
    accept(&mut e, default(), "17");
    assert!(!report.current(&e));
    assert_eq!(
        values::derive(&e, part(1009), "x * 2 + 1").unwrap().value(),
        Some(47.)
    );
    assert_eq!(
        values::derive(&e, part(1010), "x * 2 + 1").unwrap().value(),
        Some(35.)
    );
    let before = e.snapshot.clone();
    let failed = values::derive(&e, part(1010), "x / 0").unwrap();
    assert!(failed.value().is_none() && failed.diagnostic().is_some());
    assert!(values::derive(&e, part(1010), "system(x)").is_err());
    assert_eq!(e.snapshot, before);
}
#[test]
fn boolean_integer_text_literals_keep_types_and_large_integer_results_refuse_precision_loss() {
    let mut e = editor();
    for (kind, text, invalid) in [
        (Scalar::Boolean, "true", "yes"),
        (Scalar::Integer, "9223372036854775807", "1.5"),
        (Scalar::Text, "  literal text  ", ""),
    ] {
        let value = Id(e.snapshot.next_id);
        run(
            &mut e,
            Action::AddDefinition {
                system: RecordId(1),
                name: format!("{kind:?}"),
                kind: Kind::Value(kind),
            },
        )
        .unwrap();
        let p = add_property(&mut e, value);
        let target = Target::Default { property: p };
        accept(&mut e, target, text);
        let literal = load(&e.snapshot).unwrap().defaults[&p].clone();
        assert_eq!(literal.kind, kind);
        assert_eq!(literal.text, text);
        assert!(values::derive(&e, target, "x").is_err());
        if kind != Scalar::Text {
            set(&mut e, target, invalid);
            let before = e.snapshot.clone();
            assert!(run(&mut e, Action::Value(VA::Apply(target))).is_err());
            assert_eq!(e.snapshot, before);
        }
    }
}
#[test]
fn serialized_values_refuse_noncanonical_wrong_context_and_type_without_changing_legacy_wire() {
    let raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/parity/p19/fixtures/connectors.kernspan"
    ))
    .unwrap();
    let old: Snapshot = serde_json::from_value(raw["snapshot"].clone()).unwrap();
    let ext = old.extensions.as_ref().unwrap();
    assert_eq!(
        serde_json::to_value(load(&old).unwrap()).unwrap(),
        ext.namespaces[NAMESPACE]
    );
    assert_eq!(
        serde_json::to_value(journal(&old).unwrap()).unwrap(),
        ext.namespaces[DRAFTS]
    );
    let mut e = editor();
    accept(&mut e, default(), "12.5");
    accept(&mut e, part(1009), "23");
    for mode in 0..4 {
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
                let v = data["defaults"]
                    .as_object_mut()
                    .unwrap()
                    .remove("1020")
                    .unwrap();
                data["defaults"]
                    .as_object_mut()
                    .unwrap()
                    .insert("01020".into(), v);
            }
            1 => data["defaults"]["1020"]["kind"] = serde_json::json!("Integer"),
            2 => {
                let v = data["overrides"]
                    .as_object_mut()
                    .unwrap()
                    .remove("1009")
                    .unwrap();
                data["overrides"]
                    .as_object_mut()
                    .unwrap()
                    .insert("1011".into(), v);
            }
            _ => {
                let v = data["overrides"]["1009"]
                    .as_object_mut()
                    .unwrap()
                    .remove("1020")
                    .unwrap();
                data["overrides"]["1009"]
                    .as_object_mut()
                    .unwrap()
                    .insert("1006".into(), v);
            }
        }
        assert!(s.validate().is_err(), "mode{mode}");
    }
}
#[test]
fn captured_native_views4_save_cold_reopen_retains_defaults_overrides_drafts_routes_and_no_derived_writes()
 {
    use crate::{
        native_views::Views,
        native_views_io::{Io, Kind, accepted_native},
        stage1_io::{Session, execute_write},
    };
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let native = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/parity/p19/fixtures/connectors.kernspan"
    ));
    let source_views = native.with_extension("mbse-views.json");
    let mut session = Session::open(area.path(), native).unwrap();
    let views = Views::decode(
        &std::fs::read(&source_views).unwrap(),
        &session.editor,
        &accepted_native(&session).unwrap(),
    )
    .unwrap();
    let core = views.document().occurrences.clone();
    let ports = views.document().typed_ports.clone();
    let routes = views.document().connector_edges.clone();
    add_property(&mut session.editor, Id(1005));
    accept(&mut session.editor, default(), "17");
    accept(&mut session.editor, part(1009), "23");
    set(&mut session.editor, part(1009), "2e+");
    let expected = session.editor.snapshot.clone();
    let derived = values::derive(&session.editor, part(1009), "x*2").unwrap();
    assert_eq!(derived.value(), Some(46.));
    assert_eq!(session.editor.snapshot, expected);
    let target = area.path().join("values.kernspan");
    let side = area.path().join("values.mbse-views.json");
    let mut io = Io::default();
    let dest = io
        .freeze_destination(&session, &views, &side, Kind::Bound)
        .unwrap();
    let capture = views.capture(&session.editor);
    let job = session.begin_save(&target).unwrap();
    assert!(session.complete(execute_write(job)));
    let job = io.begin_captured(&session, &views, capture, dest).unwrap();
    assert!(io.complete(&session, &views, crate::native_views_io::execute(job)));
    let cold = Session::open(area.path(), &target).unwrap();
    assert_eq!(cold.editor.snapshot, expected);
    let reopened = Views::decode(
        &std::fs::read(side).unwrap(),
        &cold.editor,
        &accepted_native(&cold).unwrap(),
    )
    .unwrap();
    assert_eq!(
        reopened.document().profile,
        crate::native_views::connectors::PROFILE
    );
    assert_eq!(reopened.document().occurrences, core);
    assert_eq!(reopened.document().typed_ports, ports);
    assert_eq!(reopened.document().connector_edges, routes);
    assert!(!derived.current(&cold.editor));
    assert_eq!(
        values::forms(&cold.editor.snapshot).unwrap()[&part(1009).key()].text,
        "2e+"
    );
}
