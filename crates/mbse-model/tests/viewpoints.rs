use mbse_model::{
    native_views::{self as v, Camera, Geometry, Layout, ViewId, Views, viewpoints::*},
    native_views_inspect::Inspection,
    native_views_plan::{Choice, Plan},
    stage1::*,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn editor() -> Editor {
    let value: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p29a/fixtures/inspection.json"
    ))
    .unwrap();
    let mut e = Editor::new(
        Snapshot::from_fixture(DocumentId("viewpoint:source".into()), &value).unwrap(),
        "viewpoint:runtime".into(),
    )
    .unwrap();
    e.command(
        &e.freshness(),
        Command::EditText {
            key: RawKey {
                record: RecordId(301),
                field: "criterion".into(),
                item: None,
            },
            text: "9e".into(),
        },
    )
    .unwrap();
    e
}
fn form(e: &Editor) -> Form {
    let mut f = Form::new(e);
    f.name = "Quality review".into();
    f.stakeholder = "401".into();
    f.concern = "500".into();
    f.system = "1".into();
    f.kinds = BTreeSet::from([RecordKind::Measure]);
    f.name_contains = "RECORDED".into();
    f
}
fn action(v: &mut Views, e: &Editor, id: ViewId, a: Action) -> Result<(), String> {
    v.apply(e, &v.stamp(e), Layout::Viewpoint(id, a))
}
fn admit(v: &mut Views, e: &Editor, id: ViewId, f: Form) {
    action(v, e, id, Action::SetForm(f)).unwrap();
    action(v, e, id, Action::Apply).unwrap();
}
fn area() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap()
}
#[test]
fn viewpoint_projection_changes_only_presentation_and_uses_layout_undo_redo() {
    let e = editor();
    let native = e.snapshot.clone();
    let mut v = Views::new(&e).unwrap();
    let id = v.document().primary;
    let old = v.document().clone();
    admit(&mut v, &e, id, form(&e));
    let p = project(&v.document().views[&id], &e.snapshot);
    assert!(p.active() && p.context_resolved());
    assert_eq!(
        p.matched().iter().map(|id| id.0).collect::<Vec<_>>(),
        vec![301]
    );
    assert!(!p.allows(RecordId(401)));
    assert_eq!(e.snapshot, native);
    assert!(!e.can_undo());
    assert_eq!(v.document().occurrences, old.occurrences);
    assert_eq!(v.document().edges, old.edges);
    assert_eq!(v.document().views[&id].camera, old.views[&id].camera);
    let spec = v.document().views[&id].viewpoint.clone();
    v.undo(&e, &v.stamp(&e), false).unwrap();
    assert!(v.document().views[&id].viewpoint.is_none());
    assert_eq!(v.document().profile, PROFILE);
    v.undo(&e, &v.stamp(&e), true).unwrap();
    assert_eq!(v.document().views[&id].viewpoint, spec);
    assert_eq!(e.snapshot, native);
}
#[test]
fn invalid_viewpoint_form_survives_recovery_without_overwriting_accepted_scope() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let id = v.document().primary;
    admit(&mut v, &e, id, form(&e));
    let accepted = v.document().views[&id].viewpoint.clone();
    let mut bad = form(&e);
    bad.stakeholder = "401?".into();
    bad.name_contains = "  retained invalid raw  ".into();
    action(&mut v, &e, id, Action::SetForm(bad.clone())).unwrap();
    let before = v.document().clone();
    assert!(action(&mut v, &e, id, Action::Apply).is_err());
    assert_eq!(v.document(), &before);
    assert_eq!(v.document().views[&id].viewpoint, accepted);
    let bytes = v.recovery(&e).unwrap().encoded().unwrap();
    let cold = Views::restore(&bytes, &e).unwrap();
    assert_eq!(cold.document().profile, RECOVERY_PROFILE);
    assert_eq!(cold.document().views[&id].viewpoint_form, Some(bad));
    assert_eq!(cold.document().views[&id].viewpoint, accepted);
    assert!(project(&cold.document().views[&id], &e.snapshot).allows(RecordId(301)));
}
#[test]
fn concern_scope_mismatch_refuses_and_foreign_context_requires_explicit_rebind() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let id = v.document().primary;
    let mut wrong = form(&e);
    wrong.stakeholder = "400".into();
    action(&mut v, &e, id, Action::SetForm(wrong)).unwrap();
    let old = v.document().clone();
    assert!(
        action(&mut v, &e, id, Action::Apply)
            .unwrap_err()
            .contains("not expressed")
    );
    assert_eq!(v.document(), &old);
    admit(&mut v, &e, id, form(&e));
    let old_spec = v.document().views[&id].viewpoint.clone().unwrap();
    let mut foreign = e.snapshot.clone();
    foreign.document_id = DocumentId("viewpoint:foreign".into());
    let other = Editor::new(foreign, "viewpoint:foreign-runtime".into()).unwrap();
    let p = project(&v.document().views[&id], &other.snapshot);
    assert!(!p.active() && !p.context_resolved() && p.allows(RecordId(401)));
    let mut target = Views::new(&other).unwrap();
    let tid = target.document().primary;
    action(
        &mut target,
        &other,
        tid,
        Action::SetForm(Form::from_spec(&old_spec)),
    )
    .unwrap();
    assert!(action(&mut target, &other, tid, Action::Apply).is_err());
    action(&mut target, &other, tid, Action::RebindFormSource).unwrap();
    action(&mut target, &other, tid, Action::Apply).unwrap();
    assert_eq!(
        target.document().views[&tid]
            .viewpoint
            .as_ref()
            .unwrap()
            .source_document,
        other.snapshot.document_id
    );
    assert_eq!(old_spec.source_document, e.snapshot.document_id);
}
#[test]
fn unresolved_documentary_reference_fails_open_without_retargeting_or_changing_native_meaning() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let id = v.document().primary;
    let mut f = form(&e);
    f.stakeholder = "400".into();
    f.concern.clear();
    admit(&mut v, &e, id, f);
    let spec = v.document().views[&id].viewpoint.clone();
    let mut missing = e.snapshot.clone();
    missing.records.remove(&RecordId(400));
    missing.validate().unwrap();
    let p = project(&v.document().views[&id], &missing);
    assert!(!p.context_resolved() && !p.active());
    assert!(p.allows(RecordId(200)));
    assert!(
        p.limitations()
            .iter()
            .any(|s| s.contains("Detached Stakeholder #400"))
    );
    assert_eq!(v.document().views[&id].viewpoint, spec);
    assert_eq!(
        e.snapshot.records[&RecordId(401)].kind,
        RecordKind::Stakeholder
    );
}
#[test]
fn viewpoint_wire_ids_profiles_unknown_fields_and_bounds_refuse_without_implicit_upgrade() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let id = v.document().primary;
    let legacy = v.recovery(&e).unwrap().encoded().unwrap();
    assert!(!String::from_utf8_lossy(&legacy).contains("\"viewpoint\":"));
    let mut value: Value = serde_json::from_slice(&legacy).unwrap();
    value["views"][id.get().to_string()]["viewpoint"] = Value::Null;
    assert!(
        Views::restore(&serde_json::to_vec(&value).unwrap(), &e)
            .err().unwrap()
            .contains("VIEWPOINT_REQUIRES_V5")
    );
    admit(&mut v, &e, id, form(&e));
    let good: Value = serde_json::from_slice(&v.recovery(&e).unwrap().encoded().unwrap()).unwrap();
    assert_eq!(
        good["views"][id.get().to_string()]["viewpoint"]["stakeholder"],
        "401"
    );
    for bad in [json!(401), json!("0401"), json!("0"), json!("401?")] {
        let mut x = good.clone();
        x["views"][id.get().to_string()]["viewpoint"]["stakeholder"] = bad;
        assert!(Views::restore(&serde_json::to_vec(&x).unwrap(), &e).is_err());
    }
    let mut x = good.clone();
    x["views"][id.get().to_string()]["viewpoint"]["script"] = json!("never executes");
    assert!(Views::restore(&serde_json::to_vec(&x).unwrap(), &e).is_err());
    let before = v.document().clone();
    let mut f = form(&e);
    f.name = "x".repeat(4097);
    assert!(action(&mut v, &e, id, Action::SetForm(f)).is_err());
    assert_eq!(v.document(), &before);
}
#[test]
fn independent_view_filters_preserve_manual_visibility_geometry_cameras_and_cold_identity() {
    let e = editor();
    let mut v = Views::new(&e).unwrap();
    let primary = v.document().primary;
    let second = *v
        .document()
        .views
        .keys()
        .find(|id| **id != primary)
        .unwrap();
    v.apply(
        &e,
        &v.stamp(&e),
        Layout::Reuse {
            view: second,
            record: RecordId(200),
            geometry: Geometry::at(777., 333.),
        },
    )
    .unwrap();
    v.apply(
        &e,
        &v.stamp(&e),
        Layout::Camera(second, Camera::checked(30., 40., 0.6).unwrap()),
    )
    .unwrap();
    let hidden = v
        .document()
        .occurrences
        .values()
        .find(|o| o.view == primary && o.record == RecordId(301))
        .unwrap()
        .id;
    v.apply(&e, &v.stamp(&e), Layout::Visible(hidden, false))
        .unwrap();
    let old = v.document().clone();
    admit(&mut v, &e, primary, form(&e));
    let mut all = Form::new(&e);
    all.name = "All native kinds".into();
    admit(&mut v, &e, second, all);
    assert!(!project(&v.document().views[&primary], &e.snapshot).allows(RecordId(200)));
    assert!(project(&v.document().views[&second], &e.snapshot).allows(RecordId(200)));
    assert!(!v.document().occurrences[&hidden].visible);
    let cold = Views::restore(&v.recovery(&e).unwrap().encoded().unwrap(), &e).unwrap();
    assert_eq!(cold.document().occurrences, old.occurrences);
    for (id, view) in &old.views {
        assert_eq!(cold.document().views[id].camera, view.camera);
        assert_eq!(cold.document().views[id].title, view.title);
    }
    assert!(project(&cold.document().views[&second], &e.snapshot).allows(RecordId(401)));
}
#[test]
fn copied_view_keeps_exact_source_scope_and_raw_form_instead_of_rebinding_native_ids() {
    let e = editor();
    let mut source = Views::new(&e).unwrap();
    let id = source.document().primary;
    admit(&mut source, &e, id, form(&e));
    let mut bad = form(&e);
    bad.system = "1?".into();
    action(&mut source, &e, id, Action::SetForm(bad.clone())).unwrap();
    let dir = area();
    let path = dir.path().join("source.json");
    std::fs::write(&path, source.recovery(&e).unwrap().encoded().unwrap()).unwrap();
    let inspection = Inspection::open(&path).unwrap();
    let mut snap = e.snapshot.clone();
    snap.document_id = DocumentId("viewpoint:target".into());
    let target_e = Editor::new(snap, "viewpoint:target-runtime".into()).unwrap();
    let mut target = Views::new(&target_e).unwrap();
    let mut plan = Plan::new(&inspection, id, &target_e, &target).unwrap();
    for o in inspection
        .declared()
        .occurrences
        .values()
        .filter(|o| o.view == id)
    {
        plan.choose(
            &inspection,
            &target_e,
            &target,
            o.id,
            Choice::Target(o.record),
        )
        .unwrap();
    }
    let reviewed = plan
        .review(&inspection, &target_e, &target, "Copied declared scope")
        .unwrap();
    let copied = plan
        .apply_copy(&inspection, &target_e, &mut target, &reviewed)
        .unwrap();
    assert_eq!(target.document().profile, PROFILE);
    assert_eq!(
        target.document().views[&copied].viewpoint,
        source.document().views[&id].viewpoint
    );
    assert_eq!(target.document().views[&copied].viewpoint_form, Some(bad));
    let p = project(&target.document().views[&copied], &target_e.snapshot);
    assert!(!p.active() && !p.context_resolved() && p.allows(RecordId(401)));
}
#[test]
fn viewpoint_v5_survives_typed_port_connector_actions_recovery_and_stale_native_stamp_refusal() {
    let value: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p28/fixtures/native.kernspan"
    ))
    .unwrap();
    let e = Editor::new(
        serde_json::from_value(value["snapshot"].clone()).unwrap(),
        "viewpoint:typed".into(),
    )
    .unwrap();
    let mut doc: v::Document = serde_json::from_str(include_str!(
        "../../../docs/parity/p28/fixtures/native.mbse-views.json"
    ))
    .unwrap();
    doc.profile = v::connectors::RECOVERY_PROFILE.into();
    doc.binding = None;
    let mut v = Views::restore(&doc.encoded().unwrap(), &e).unwrap();
    let id = v.document().primary;
    let native = e.snapshot.clone();
    let mut f = Form::new(&e);
    f.name = "Typed native records".into();
    admit(&mut v, &e, id, f);
    let point = v.document().views[&id].viewpoint.clone();
    v.apply(
        &e,
        &v.stamp(&e),
        Layout::Port(v::ports::Layout::ReuseInUsage {
            view: id,
            port: mbse_model::types::Id(1017),
            usage: mbse_model::types::Id(1009),
            geometry: Geometry::at(700., 700.),
        }),
    )
    .unwrap();
    let a = *v.document().typed_ports.keys().last().unwrap();
    v.apply(
        &e,
        &v.stamp(&e),
        Layout::Port(v::ports::Layout::ReuseInUsage {
            view: id,
            port: mbse_model::types::Id(1015),
            usage: mbse_model::types::Id(1010),
            geometry: Geometry::at(900., 700.),
        }),
    )
    .unwrap();
    let b = *v.document().typed_ports.keys().last().unwrap();
    v.apply(
        &e,
        &v.stamp(&e),
        Layout::Connector(v::connectors::Layout::Route {
            view: id,
            connector: mbse_model::types::Id(1018),
            source: a,
            target: b,
        }),
    )
    .unwrap();
    assert_eq!(v.document().profile, PROFILE);
    assert_eq!(v.document().views[&id].viewpoint, point);
    assert_eq!(e.snapshot, native);
    let rec = v.recovery(&e).unwrap();
    assert_eq!(rec.profile, RECOVERY_PROFILE);
    let cold = Views::restore(&rec.encoded().unwrap(), &e).unwrap();
    assert_eq!(cold.document().views[&id].viewpoint, point);
    let stamp = v.stamp(&e);
    let mut changed = e.clone();
    changed
        .snapshot
        .records
        .get_mut(&RecordId(1002))
        .unwrap()
        .name = "Changed same-counter native payload".into();
    let before = v.document().clone();
    assert!(
        v.apply(&changed, &stamp, Layout::Viewpoint(id, Action::Clear))
            .is_err()
    );
    assert_eq!(v.document(), &before);
}
