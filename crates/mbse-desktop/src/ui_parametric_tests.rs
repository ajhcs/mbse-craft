use super::*;
fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p34c/verification/native-02/run/scenario-final-cold-copy.kernspan"
    ))
    .unwrap();
    session.editor = mbse_model::stage1::Editor::new(
        serde_json::from_value(value["snapshot"].clone()).unwrap(),
        session.editor.freshness().instance_id,
    )
    .unwrap();
    let ctx = egui::Context::default();
    let mut a = Workspace::new(
        session,
        None,
        None,
        craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
        &ctx,
    );
    a.dispatch_workspace(
        crate::command_catalog::Id::Parametrics,
        crate::command_catalog::Surface::Menu,
        ShortcutTarget::Model,
    );
    (area, a, ctx)
}
fn render(a: &mut Workspace, ctx: &egui::Context, events: Vec<egui::Event>) {
    a.audit.clear();
    a.activity_frame_start();
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(2200., 1600.),
            )),
            focused: true,
            events,
            ..Default::default()
        },
        |ui| {
            a.canvas_body(ui);
            a.pm_dialog(ctx);
        },
    )
    .drop_without_applying_deltas();
}
fn settle(a: &mut Workspace, ctx: &egui::Context) {
    for _ in 0..4 {
        render(a, ctx, vec![]);
    }
}
fn rect(a: &Workspace, key: &str) -> [f32; 4] {
    a.audit
        .get(key)
        .copied()
        .or_else(|| {
            a.views
                .toolbar_bounds()
                .find(|(k, _)| k == key)
                .map(|(_, r)| r)
        })
        .unwrap_or_else(|| panic!("missing {key}"))
}
fn click(a: &mut Workspace, ctx: &egui::Context, key: &str) {
    settle(a, ctx);
    let mut r = rect(a, key);
    for _ in 0..24 {
        if r[2] - r[0] > 5. && r[3] - r[1] >= 12. && r[1] >= 0. && r[3] <= 1600. {
            break;
        }
        let live: Vec<_> = a
            .audit
            .iter()
            .filter(|(k, v)| {
                k.starts_with("parametric:") && v[3] - v[1] >= 12. && v[1] >= 0. && v[3] <= 1600.
            })
            .map(|(_, r)| *r)
            .collect();
        let top = live.iter().map(|r| r[1]).fold(100., f32::min);
        let bottom = live.iter().map(|r| r[3]).fold(800., f32::max);
        let pos = egui::pos2(
            ((r[0] + r[2]) / 2.).clamp(40., 2100.),
            (bottom - 24.).clamp(60., 1530.),
        );
        let delta = if (r[1] + r[3]) / 2. > (top + bottom) / 2. {
            -320.
        } else {
            320.
        };
        render(
            a,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0., delta),
                    modifiers: egui::Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                },
            ],
        );
        settle(a, ctx);
        r = rect(a, key);
    }
    assert!(
        r[2] - r[0] > 5. && r[3] - r[1] >= 12.,
        "clipped {key}: {r:?}"
    );
    let pos = egui::pos2((r[0] + r[2]) / 2., (r[1] + r[3]) / 2.);
    for pressed in [true, false] {
        render(
            a,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    settle(a, ctx);
}
fn fill(a: &mut Workspace, ctx: &egui::Context, key: &str, text: &str) {
    click(a, ctx, &format!("parametric:field:{key}"));
    let mut events = vec![egui::Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    }];
    if text.is_empty() {
        events.push(egui::Event::Key {
            key: egui::Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
    } else {
        events.push(egui::Event::Text(text.into()));
    }
    render(a, ctx, events);
    settle(a, ctx);
}

fn pick(a: &mut Workspace, id: Id) {
    let root = a.parametrics.specification.unwrap();
    let f = super::accepted_form(
        &ty::load(&a.session.editor.snapshot).unwrap().parametrics[&root],
        id,
    )
    .unwrap();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.pm_pick(f, Some(&stamp)).unwrap();
}
fn seed(a: &mut Workspace) -> (Id, Id, Id, Id, Id) {
    use mbse_model::stage1_calc::UnitId::*;
    let root = Id(a.session.editor.snapshot.next_id);
    let x = Id(root.0 + 1);
    let x1 = Id(root.0 + 2);
    let b = Id(root.0 + 3);
    let b1 = Id(root.0 + 4);
    for f in [
        Form::Specification {
            id: None,
            system: "1".into(),
            name: "Distance relation".into(),
            calculation: "1003".into(),
        },
        Form::Parameter {
            specification: root,
            id: None,
            name: "x".into(),
            quantity: quantity(Metre),
        },
        Form::Parameter {
            specification: root,
            id: None,
            name: "x1".into(),
            quantity: quantity(Second),
        },
        Form::Binding {
            specification: root,
            id: None,
            parameter: x.0.to_string(),
            source: SourceForm::NativeInput {
                calculation: "1003".into(),
                name: "x".into(),
            },
        },
        Form::Binding {
            specification: root,
            id: None,
            parameter: x1.0.to_string(),
            source: SourceForm::NativeInput {
                calculation: "1003".into(),
                name: "x1".into(),
            },
        },
    ] {
        let stamp = a.views.views.stamp(&a.session.editor);
        a.pm_model(&stamp, pm::Action::Set(f.clone())).unwrap();
        let stamp = a.views.views.stamp(&a.session.editor);
        a.pm_model(&stamp, pm::Action::Apply(f.key())).unwrap();
    }
    a.parametrics.specification = Some(root);
    pick(a, root);
    (root, x, x1, b, b1)
}
fn source(a: &mut Workspace, ctx: &egui::Context, name: &str) {
    let key = format!("parametric:input:1003:{name}");
    if !a.audit.contains_key(&key) {
        click(a, ctx, "parametric:Browse sources");
    }
    click(a, ctx, &key);
}
#[test]
fn rendered_parametric_widgets_author_native_spec_parameters_bindings_and_complete_type_links_without_execution()
 {
    let (_d, mut a, ctx) = app();
    let records = a.session.editor.snapshot.records.clone();
    let layout = a.views.views.document().encoded().unwrap();
    fill(&mut a, &ctx, "name", "Distance relation");
    click(&mut a, &ctx, "parametric:Browse calculations");
    click(&mut a, &ctx, "parametric:calculation:1003");
    let root = Id(a.session.editor.snapshot.next_id);
    click(&mut a, &ctx, "parametric:Apply");
    assert_eq!(a.parametrics.specification, Some(root));
    let mut params = Vec::new();
    for name in ["x", "x1"] {
        click(&mut a, &ctx, "parametric:new:Parameter");
        click(&mut a, &ctx, &format!("parametric:declare:{name}"));
        let id = Id(a.session.editor.snapshot.next_id);
        click(&mut a, &ctx, "parametric:Apply");
        params.push(id);
    }
    assert_eq!(
        pm::report(&a.session.editor.snapshot, root)
            .unwrap()
            .unbound,
        params
    );
    for (id, name) in params.iter().zip(["x", "x1"]) {
        click(&mut a, &ctx, "parametric:new:Binding");
        fill(&mut a, &ctx, "parameter", &id.0.to_string());
        source(&mut a, &ctx, name);
        click(&mut a, &ctx, "parametric:Apply");
    }
    let r = pm::report(&a.session.editor.snapshot, root).unwrap();
    assert!(r.accepted_bindings_complete && !r.execution_performed);
    assert_eq!(a.session.editor.snapshot.records, records);
    assert_eq!(a.views.views.document().encoded().unwrap(), layout);
}
#[test]
fn rendered_parametric_dimension_refusal_duplicate_draft_and_global_history_preserve_newer_raw() {
    let (_d, mut a, ctx) = app();
    let (root, x, _, b, _) = seed(&mut a);
    pick(&mut a, b);
    fill(&mut a, &ctx, "source_name", "x1");
    let accepted = ty::load(&a.session.editor.snapshot).unwrap();
    click(&mut a, &ctx, "parametric:Apply");
    assert!(a.parametrics.note.contains("QUANTITY_KIND_MISMATCH"));
    assert_eq!(ty::load(&a.session.editor.snapshot).unwrap(), accepted);
    click(&mut a, &ctx, "parametric:new:Binding");
    fill(&mut a, &ctx, "parameter", &x.0.to_string());
    source(&mut a, &ctx, "x");
    click(&mut a, &ctx, "parametric:Apply");
    assert!(a.parametrics.note.contains("DUPLICATE_BINDING"));
    let forms = pm::forms(&a.session.editor.snapshot).unwrap();
    click(&mut a, &ctx, "parametric:Model Undo");
    assert_eq!(pm::forms(&a.session.editor.snapshot).unwrap(), forms);
    assert!(
        !pm::report(&a.session.editor.snapshot, root)
            .unwrap()
            .accepted_bindings_complete
    );
    click(&mut a, &ctx, "parametric:Model Redo");
    assert_eq!(ty::load(&a.session.editor.snapshot).unwrap(), accepted);
    assert_eq!(pm::forms(&a.session.editor.snapshot).unwrap(), forms);
}
#[test]
fn rendered_parametric_typed_default_and_part_override_units_keep_unset_and_wrong_dimension_visible()
 {
    let (_d, mut a, ctx) = app();
    let (root, _, _, b, _) = seed(&mut a);
    pick(&mut a, b);
    click(&mut a, &ctx, "parametric:source-kind:Typed value");
    fill(&mut a, &ctx, "property", "1006");
    click(&mut a, &ctx, "parametric:source-unit:Metre");
    click(&mut a, &ctx, "parametric:Apply");
    let r = pm::report(&a.session.editor.snapshot, root).unwrap();
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "PARAMETRIC_BOUND_VALUE_UNSET")
    );
    fill(&mut a, &ctx, "property", "1020");
    fill(&mut a, &ctx, "usage", "1009");
    click(&mut a, &ctx, "parametric:Apply");
    assert!(
        pm::report(&a.session.editor.snapshot, root)
            .unwrap()
            .accepted_bindings_complete
    );
    let accepted = ty::load(&a.session.editor.snapshot).unwrap();
    click(&mut a, &ctx, "parametric:source-unit:Second");
    click(&mut a, &ctx, "parametric:Apply");
    assert!(a.parametrics.note.contains("QUANTITY_KIND_MISMATCH"));
    assert_eq!(ty::load(&a.session.editor.snapshot).unwrap(), accepted);
}
#[test]
fn rendered_parametric_utf8_admission_failure_and_foreign_owner_keep_exact_new_text_until_explicit_discard()
 {
    let (_d, mut a, ctx) = app();
    let native = a.session.editor.snapshot.clone();
    fill(&mut a, &ctx, "name", &"é".repeat(2500));
    assert!(a.pm_pending());
    click(&mut a, &ctx, "parametric:Close");
    assert!(a.parametrics.open);
    assert_eq!(a.pm_audit()["current"]["name"], "é".repeat(2500));
    a.parametrics.pending.as_mut().unwrap().owner.instance = "foreign".into();
    settle(&mut a, &ctx);
    assert_eq!(a.pm_audit()["owner_current"], false);
    click(&mut a, &ctx, "parametric:Copy new text");
    click(&mut a, &ctx, "parametric:Discard new text");
    assert!(!a.pm_pending());
    click(&mut a, &ctx, "parametric:Close");
    assert!(!a.parametrics.open);
    assert_eq!(a.session.editor.snapshot, native);
}
#[test]
fn rendered_parametric_unsupported_optional_source_preserves_pending_copy_discard_and_close_route()
{
    let (_d, mut a, ctx) = app();
    fill(&mut a, &ctx, "name", &"é".repeat(2500));
    let source = a
        .session
        .editor
        .snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .get_mut(ty::NAMESPACE)
        .unwrap();
    source["future"] = serde_json::json!(true);
    let native = a.session.editor.snapshot.clone();
    settle(&mut a, &ctx);
    assert!(a.pm_pending());
    click(&mut a, &ctx, "parametric:Copy new text");
    click(&mut a, &ctx, "parametric:Discard new text");
    click(&mut a, &ctx, "parametric:Close");
    assert!(!a.parametrics.open);
    assert_eq!(a.session.editor.snapshot, native);
}
#[test]
fn parametric_stale_source_and_other_actor_guards_preserve_selected_scope_raw_model_and_views() {
    let (_d, mut a, ctx) = app();
    let (root, _, _, b, _) = seed(&mut a);
    pick(&mut a, b);
    let stamp = a.views.views.stamp(&a.session.editor);
    let form = a.parametrics.current.clone().unwrap();
    a.session
        .editor
        .snapshot
        .records
        .get_mut(&RecordId(1))
        .unwrap()
        .name = "Different source same counters".into();
    let native = a.session.editor.snapshot.clone();
    let layout = a.views.views.document().encoded().unwrap();
    assert!(a.pm_pick(form, Some(&stamp)).is_err());
    for id in [
        crate::command_catalog::Id::New,
        crate::command_catalog::Id::Scenarios,
        crate::command_catalog::Id::Activities,
        crate::command_catalog::Id::LayoutUndo,
    ] {
        a.dispatch_workspace(
            id,
            crate::command_catalog::Surface::Menu,
            ShortcutTarget::Model,
        );
        assert_eq!(a.session.editor.snapshot, native);
    }
    assert!(a.dock_blocked());
    assert_eq!(a.parametrics.specification, Some(root));
    assert_eq!(a.views.views.document().encoded().unwrap(), layout);
    click(&mut a, &ctx, "parametric:Close");
    assert!(!a.parametrics.open);
}
#[test]
fn rendered_parametric_report_copy_keeps_accepted_type_scope_explicit_when_source_value_has_invalid_raw()
 {
    use mbse_model::stage1_calc::UnitId;
    let (_d, mut a, ctx) = app();
    let (root, x, _, b, _) = seed(&mut a);
    let form = Form::Binding {
        specification: root,
        id: Some(b),
        parameter: x.0.to_string(),
        source: SourceForm::TypedValue {
            property: "1020".into(),
            usage: "1009".into(),
            unit: UnitId::Metre,
        },
    };
    let stamp = a.views.views.stamp(&a.session.editor);
    a.pm_model(&stamp, pm::Action::Set(form.clone())).unwrap();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.pm_model(&stamp, pm::Action::Apply(form.key())).unwrap();
    pick(&mut a, b);
    let old = pm::report(&a.session.editor.snapshot, root).unwrap();
    a.session
        .editor
        .command(
            &a.session.editor.freshness(),
            Command::Typed(ty::Action::Value(ty::values::Action::Set(
                ty::values::Form {
                    target: ty::values::Target::Override {
                        usage: Id(1009),
                        property: Id(1020),
                    },
                    text: "1e".into(),
                    expression: String::new(),
                },
            ))),
        )
        .unwrap();
    settle(&mut a, &ctx);
    let fresh = pm::report(&a.session.editor.snapshot, root).unwrap();
    assert!(fresh.accepted_bindings_complete && !fresh.execution_performed);
    assert_ne!(fresh.snapshot_hash, old.snapshot_hash);
    let native = a.session.editor.snapshot.clone();
    let layout = a.views.views.document().encoded().unwrap();
    let r = rect(&a, "parametric:Copy review");
    assert!(r[3] - r[1] >= 12.);
    let pos = egui::pos2((r[0] + r[2]) / 2., (r[1] + r[3]) / 2.);
    let mut copies = Vec::new();
    for down in [true, false] {
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(2200., 1600.),
                )),
                focused: true,
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: down,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                a.canvas_body(ui);
                a.pm_dialog(&ctx);
            },
        );
        copies.extend(out.platform_output.commands.iter().filter_map(|c| {
            if let egui::OutputCommand::CopyText(text) = c {
                Some(text.clone())
            } else {
                None
            }
        }));
        out.drop_without_applying_deltas();
    }
    let copied: serde_json::Value =
        serde_json::from_str(copies.last().expect("actual copy output")).unwrap();
    assert_eq!(copied["execution_performed"], false);
    assert_eq!(copied["snapshot_hash"], fresh.snapshot_hash);
    assert_eq!(a.session.editor.snapshot, native);
    assert_eq!(a.views.views.document().encoded().unwrap(), layout);
}

#[test]
fn rendered_parametric_mismatched_quantity_draft_is_never_repaired_by_observation_or_disabled_foreign_owner()
 {
    use mbse_model::stage1_calc::{QuantityKind, QuantityType, UnitId};
    let (_d, mut a, ctx) = app();
    let (root, x, _, _, _) = seed(&mut a);
    let form = Form::Parameter {
        specification: root,
        id: Some(x),
        name: "x".into(),
        quantity: QuantityType {
            kind: QuantityKind::Length,
            unit: UnitId::Second,
        },
    };
    let stamp = a.views.views.stamp(&a.session.editor);
    a.pm_model(&stamp, pm::Action::Set(form.clone())).unwrap();
    pick(&mut a, x);
    let native = a.session.editor.snapshot.clone();
    settle(&mut a, &ctx);
    assert_eq!(a.session.editor.snapshot, native);
    assert_eq!(
        pm::forms(&a.session.editor.snapshot).unwrap()[&form.key()],
        form
    );
    a.parametrics.owner.as_mut().unwrap().instance = "foreign".into();
    settle(&mut a, &ctx);
    assert_eq!(a.session.editor.snapshot, native);
    assert_eq!(
        a.pm_audit()["current"],
        serde_json::to_value(&form).unwrap()
    );
    a.parametrics.owner = Some(a.pm_owner());
    click(&mut a, &ctx, "parametric:quantity:Metre");
    let edited = pm::forms(&a.session.editor.snapshot).unwrap();
    assert!(matches!(
        edited[&form.key()],
        Form::Parameter {
            quantity: QuantityType {
                kind: QuantityKind::Length,
                unit: UnitId::Metre
            },
            ..
        }
    ));
    assert_eq!(
        ty::load(&a.session.editor.snapshot).unwrap(),
        ty::load(&native).unwrap()
    );
}

#[path="ui_parametric_analysis_tests.rs"]
mod analysis_tests;
