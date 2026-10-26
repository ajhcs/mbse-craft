use super::*;
use crate::{
    native_views::{NativeAction, Views},
    native_views_io::{Io, Kind as SaveKind, accepted_native},
    stage1::{Command, Editor},
    stage1_io::{Session, execute_write},
};
use tags::{Action as TA, Form as TF, TargetKind as TK};
fn input() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/parity/p22/fixtures/native-typed.kernspan")
}
fn editor() -> Editor {
    let v: Value = serde_json::from_slice(&std::fs::read(input()).unwrap()).unwrap();
    Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "profile:test".into(),
    )
    .unwrap()
}
fn profile() -> tags::Profile {
    tags::Profile {
        identity: tags::Identity {
            namespace: "urn:mbse-craft:qa".into(),
            version: "1.0".into(),
        },
        stereotypes: BTreeMap::from([(
            "Valve".into(),
            tags::Stereotype {
                applies_to: BTreeSet::from([TK::Measure, TK::ComponentDefinition, TK::Part]),
                tags: BTreeMap::from([("label".into(), Scalar::Text)]),
            },
        )]),
    }
}
fn form(id: u64) -> TF {
    TF {
        profile_json: serde_json::to_string(&profile()).unwrap(),
        target: id.to_string(),
        namespace: "urn:mbse-craft:qa".into(),
        version: "1.0".into(),
        stereotype: "Valve".into(),
        values_json: r#"{"label":{"kind":"Text","text":"prototype"}}"#.into(),
    }
}
fn run(e: &mut Editor, a: TA) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(Action::ProfileTag(a)))
        .map(|_| ())
}
fn set(e: &mut Editor, f: TF) {
    run(e, TA::SetForm(f)).unwrap();
}
fn register(e: &mut Editor) {
    set(e, form(1003));
    run(e, TA::Register).unwrap();
}
fn apply(e: &mut Editor, id: u64) {
    set(e, form(id));
    run(e, TA::Apply).unwrap();
}
fn dispatch(v: &mut Views, e: &mut Editor, a: NativeAction) {
    v.dispatch(e, &v.stamp(e), a).unwrap();
}
#[test]
fn exact_namespace_version_stereotype_and_record_kind_admission_never_reclassifies() {
    let mut e = editor();
    register(&mut e);
    let native = e.snapshot.records.clone();
    for (namespace, version, stereotype) in [
        ("URN:mbse-craft:qa", "1.0", "Valve"),
        ("urn:mbse-craft:qa", "1.0.0", "Valve"),
        ("urn:mbse-craft:qa", "1.0", "valve"),
    ] {
        let mut f = form(1003);
        f.namespace = namespace.into();
        f.version = version.into();
        f.stereotype = stereotype.into();
        set(&mut e, f);
        let before = e.snapshot.clone();
        assert!(run(&mut e, TA::Apply).is_err());
        assert_eq!(e.snapshot, before);
    }
    apply(&mut e, 1003);
    assert_eq!(e.snapshot.records, native);
    assert_eq!(
        load(&e.snapshot)
            .unwrap()
            .profile_applications
            .values()
            .next()
            .unwrap()
            .expected,
        TK::Measure
    );
    set(&mut e, form(1000));
    let before = e.snapshot.clone();
    assert!(run(&mut e, TA::Apply).unwrap_err().contains("KIND"));
    assert_eq!(e.snapshot, before);
}
#[test]
fn registered_identity_is_immutable_and_idempotent_without_aliasing_distinct_versions() {
    let mut e = editor();
    register(&mut e);
    let rev = e.snapshot.model_revision;
    run(&mut e, TA::Register).unwrap();
    assert_eq!(e.snapshot.model_revision, rev);
    let mut p = profile();
    p.stereotypes
        .get_mut("Valve")
        .unwrap()
        .tags
        .insert("extra".into(), Scalar::Boolean);
    let mut f = form(1003);
    f.profile_json = serde_json::to_string(&p).unwrap();
    set(&mut e, f.clone());
    let before = e.snapshot.clone();
    assert!(run(&mut e, TA::Register).unwrap_err().contains("IMMUTABLE"));
    assert_eq!(e.snapshot, before);
    p.identity.version = "1.0.0".into();
    f.profile_json = serde_json::to_string(&p).unwrap();
    set(&mut e, f);
    run(&mut e, TA::Register).unwrap();
    let s = load(&e.snapshot).unwrap();
    assert_eq!(s.profiles.len(), 2);
    assert_eq!(s.profiles[&profile().identity.key()], profile());
}
#[test]
fn primitive_literal_schema_exact_keys_numeric_spelling_unicode_and_duplicate_json_checks() {
    let mut e = editor();
    let mut p = profile();
    p.stereotypes.get_mut("Valve").unwrap().tags = BTreeMap::from([
        ("flag".into(), Scalar::Boolean),
        ("integer".into(), Scalar::Integer),
        ("real".into(), Scalar::Real),
        ("text".into(), Scalar::Text),
    ]);
    let mut f = form(1003);
    f.profile_json = serde_json::to_string(&p).unwrap();
    f.values_json=r#"{"flag":{"kind":"Boolean","text":"true"},"integer":{"kind":"Integer","text":"0017"},"real":{"kind":"Real","text":"1.700e+1"},"text":{"kind":"Text","text":"μ 1003 is literal text"}}"#.into();
    set(&mut e, f.clone());
    run(&mut e, TA::Register).unwrap();
    run(&mut e, TA::Apply).unwrap();
    let old = load(&e.snapshot).unwrap();
    let a = old.profile_applications.values().next().unwrap();
    assert_eq!(a.values["integer"].text, "0017");
    assert_eq!(a.values["real"].text, "1.700e+1");
    assert_eq!(a.values["text"].text, "μ 1003 is literal text");
    for bad in [
        f.values_json.replace("true", "True"),
        f.values_json.replace("1.700e+1", "NaN"),
        f.values_json.replace("0017", " 17"),
        f.values_json.replace("\"integer\":", "\"renamed\":"),
        f.values_json.replacen('{', "{\"text\":{},", 1),
    ] {
        f.values_json = bad;
        set(&mut e, f.clone());
        let before = e.snapshot.clone();
        assert!(run(&mut e, TA::Apply).is_err());
        assert_eq!(e.snapshot, before);
        assert_eq!(load(&e.snapshot).unwrap(), old);
    }
}
#[test]
fn duplicate_profile_keys_unknown_domain_kind_and_unknown_fields_refuse_with_exact_raw_form() {
    let mut e = editor();
    for json in [
        r#"{"identity":{"namespace":"urn:a","namespace":"urn:b","version":"1"},"stereotypes":{}}"#,
        r#"{"identity":{"namespace":"urn:a","version":"1"},"stereotypes":{"Valve":{"applies_to":["CustomValve"],"tags":{}}}}"#,
        r#"{"identity":{"namespace":"urn:a","version":"1","future":true},"stereotypes":{}}"#,
    ] {
        let mut f = form(1003);
        f.profile_json = json.into();
        set(&mut e, f.clone());
        let before = e.snapshot.clone();
        assert!(run(&mut e, TA::Register).is_err());
        assert_eq!(e.snapshot, before);
        assert_eq!(tags::form(&e.snapshot).unwrap(), Some(f));
        assert!(load(&e.snapshot).unwrap().profiles.is_empty());
    }
}
#[test]
fn metadata_remove_undo_redo_keeps_newer_invalid_form_and_accepted_native_kind() {
    let mut e = editor();
    register(&mut e);
    apply(&mut e, 1003);
    let accepted = load(&e.snapshot).unwrap().profile_applications;
    run(&mut e, TA::Remove).unwrap();
    let mut f = form(1003);
    f.target = "1003?".into();
    f.version = "unknown-version".into();
    set(&mut e, f.clone());
    e.undo_model(&e.freshness()).unwrap();
    assert_eq!(load(&e.snapshot).unwrap().profile_applications, accepted);
    e.redo_model(&e.freshness()).unwrap();
    assert!(load(&e.snapshot).unwrap().profile_applications.is_empty());
    assert_eq!(tags::form(&e.snapshot).unwrap(), Some(f));
    assert_eq!(
        e.snapshot.records[&RecordId(1003)].kind,
        RecordKind::Measure
    );
}
#[test]
fn typed_clone_copies_only_owned_profile_targets_to_fresh_ids_shared_registry_exact() {
    let mut e = editor();
    register(&mut e);
    for id in [1003, 1021, 1023] {
        apply(&mut e, id);
    }
    let before = load(&e.snapshot).unwrap();
    let f = refactor::Form {
        mode: refactor::Mode::CloneDefinition,
        definition: "1021".into(),
        name: "tagged clone".into(),
        usages: String::new(),
        owner: String::new(),
        keep_external: true,
    };
    e.command(
        &e.freshness(),
        Command::Typed(Action::Refactor(refactor::Action::SetForm(f))),
    )
    .unwrap();
    let p = refactor::prepare(&e).unwrap();
    assert_eq!(p.summary().copied_profile_applications.len(), 2);
    let map = p.summary().id_mapping.clone();
    e.command(
        &e.freshness(),
        Command::Typed(Action::Refactor(refactor::Action::Apply(Box::new(
            p.plan().unwrap().clone(),
        )))),
    )
    .unwrap();
    let s = load(&e.snapshot).unwrap();
    assert_eq!(s.profiles, before.profiles);
    assert_eq!(s.profile_applications.len(), 5);
    for id in [1021, 1023] {
        let expected = map[&Id(id)];
        assert!(
            s.profile_applications
                .values()
                .any(|a| a.target == expected)
        );
    }
    for (key, a) in before.profile_applications {
        assert_eq!(s.profile_applications[&key], a);
    }
    assert_eq!(
        e.snapshot.records[&RecordId(1003)].kind,
        RecordKind::Measure
    );
}
#[test]
fn native_and_typed_delete_preview_lists_application_cleanup_and_undo_restores_exact_metadata() {
    for root in [1003, 1021] {
        let mut e = editor();
        register(&mut e);
        for id in [1003, 1021, 1023] {
            apply(&mut e, id);
        }
        let mut v = Views::new(&e).unwrap();
        dispatch(
            &mut v,
            &mut e,
            NativeAction::Command(Command::Typed(Action::Deletion(deletion::Action::SetForm(
                deletion::Form {
                    ids: root.to_string(),
                    cascade: true,
                },
            )))),
        );
        let p = deletion::prepare(&e, &v).unwrap();
        assert_eq!(
            p.summary().removed_profile_applications.len(),
            if root == 1003 { 1 } else { 2 }
        );
        let old = load(&e.snapshot).unwrap();
        dispatch(
            &mut v,
            &mut e,
            NativeAction::Command(Command::Typed(Action::Deletion(deletion::Action::Apply(
                Box::new(p.plan().unwrap().clone()),
            )))),
        );
        let mut f = form(root);
        f.target = format!("{root}?");
        set(&mut e, f.clone());
        dispatch(&mut v, &mut e, NativeAction::ModelUndo);
        assert_eq!(load(&e.snapshot).unwrap(), old);
        assert_eq!(tags::form(&e.snapshot).unwrap(), Some(f));
    }
}
#[test]
fn persisted_identity_key_namespace_version_and_target_kind_tampering_refuse() {
    let mut e = editor();
    register(&mut e);
    apply(&mut e, 1003);
    let s = load(&e.snapshot).unwrap();
    for case in 0..3 {
        let mut bad = s.clone();
        match case {
            0 => bad.profiles.values_mut().next().unwrap().identity.version = "9".into(),
            1 => {
                bad.profile_applications
                    .values_mut()
                    .next()
                    .unwrap()
                    .profile
                    .namespace = "urn:other".into()
            }
            _ => {
                bad.profile_applications
                    .values_mut()
                    .next()
                    .unwrap()
                    .expected = TK::System
            }
        };
        let mut snapshot = e.snapshot.clone();
        put(&mut snapshot, NAMESPACE, &bad).unwrap();
        assert!(snapshot.validate().is_err());
    }
}
#[test]
fn legacy_wire_omits_empty_profile_fields_and_opaque_optional_profile_namespace_is_not_interpreted()
{
    let mut e = editor();
    let wire = e.snapshot.extensions.as_ref().unwrap().namespaces[NAMESPACE].clone();
    let s = load(&e.snapshot).unwrap();
    assert_eq!(serde_json::to_value(&s).unwrap(), wire);
    assert!(wire.get("profiles").is_none() && wire.get("profile_applications").is_none());
    assert!(
        serde_json::to_value(journal(&e.snapshot).unwrap())
            .unwrap()
            .get("profile_tags")
            .is_none()
    );
    let opaque =
        serde_json::json!({"namespace":"urn:vendor","version":"unknown","retained":[1,null]});
    e.snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert("vendor.profile/9".into(), opaque.clone());
    register(&mut e);
    apply(&mut e, 1003);
    assert_eq!(
        e.snapshot.extensions.as_ref().unwrap().namespaces["vendor.profile/9"],
        opaque
    );
    let mut v = Views::new(&e).unwrap();
    dispatch(
        &mut v,
        &mut e,
        NativeAction::Command(Command::Typed(Action::Deletion(deletion::Action::SetForm(
            deletion::Form {
                ids: "1003".into(),
                cascade: true,
            },
        )))),
    );
    assert!(deletion::prepare(&e, &v).unwrap().plan().is_none());
}
#[test]
fn actual_native_views4_capture_and_cold_reopen_preserve_tags_raw_identity_and_all_prior_meaning() {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let original = std::fs::read(input()).unwrap();
    let original_side = std::fs::read(input().with_extension("mbse-views.json")).unwrap();
    let mut s = Session::open(area.path(), &input()).unwrap();
    let v = Views::decode(&original_side, &s.editor, &accepted_native(&s).unwrap()).unwrap();
    let before = s.editor.snapshot.clone();
    register(&mut s.editor);
    apply(&mut s.editor, 1003);
    apply(&mut s.editor, 1021);
    let mut raw = form(1021);
    raw.namespace = "urn:unknown".into();
    raw.values_json = "{incomplete".into();
    set(&mut s.editor, raw.clone());
    let expected = s.editor.snapshot.clone();
    let native = area.path().join("profile-tags.kernspan");
    let mut io = Io::default();
    let dest = io
        .freeze_destination(
            &s,
            &v,
            &native.with_extension("mbse-views.json"),
            SaveKind::Bound,
        )
        .unwrap();
    let capture = v.capture(&s.editor);
    let job = s.begin_save(&native).unwrap();
    assert!(s.complete(execute_write(job)));
    let job = io.begin_captured(&s, &v, capture, dest).unwrap();
    assert!(io.complete(&s, &v, crate::native_views_io::execute(job)));
    let cold = Session::open(area.path(), &native).unwrap();
    assert_eq!(cold.editor.snapshot, expected);
    assert_eq!(tags::form(&cold.editor.snapshot).unwrap(), Some(raw));
    assert!(tags::preview(&cold.editor.snapshot, TA::Apply).is_err());
    assert!(!cold.editor.can_undo());
    assert_eq!(cold.editor.snapshot.records, before.records);
    assert_eq!(cold.editor.snapshot.drafts, before.drafts);
    let reopened = Views::decode(
        &std::fs::read(native.with_extension("mbse-views.json")).unwrap(),
        &cold.editor,
        &accepted_native(&cold).unwrap(),
    )
    .unwrap();
    assert_eq!(reopened.document().occurrences, v.document().occurrences);
    assert_eq!(reopened.document().edges, v.document().edges);
    assert_eq!(reopened.document().typed_ports, v.document().typed_ports);
    assert_eq!(
        reopened.document().connector_edges,
        v.document().connector_edges
    );
    assert_eq!(std::fs::read(input()).unwrap(), original);
    assert_eq!(
        std::fs::read(input().with_extension("mbse-views.json")).unwrap(),
        original_side
    );
}
