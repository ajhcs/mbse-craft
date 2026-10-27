use super::*;
fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p30c/fixtures/native.kernspan"
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
        crate::command_catalog::Id::Activities,
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
            a.activity_dialog(ctx);
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
                k.starts_with("activity:") && v[3] - v[1] >= 12. && v[1] >= 0. && v[3] <= 1600.
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
    click(a, ctx, &format!("activity:field:{key}"));
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
fn new_activity(a: &mut Workspace, ctx: &egui::Context) -> Id {
    let id = Id(a.session.editor.snapshot.next_id);
    click(a, ctx, "activity:new:Activity");
    fill(a, ctx, "name", "Inspect");
    click(a, ctx, "activity:Apply");
    assert!(
        ty::load(&a.session.editor.snapshot)
            .unwrap()
            .activities
            .contains_key(&id)
    );
    id
}
fn new_node(a: &mut Workspace, ctx: &egui::Context, kind: ac::NodeKind) -> Id {
    click(a, ctx, "activity:new:Node");
    fill(a, ctx, "name", &format!("{kind:?}"));
    click(a, ctx, &format!("activity:node-kind:{kind:?}"));
    let id = Id(a.session.editor.snapshot.next_id);
    click(a, ctx, "activity:Apply");
    id
}
fn new_flow(a: &mut Workspace, ctx: &egui::Context, source: Id, target: Id, guard: &str) -> Id {
    click(a, ctx, "activity:new:Flow");
    fill(a, ctx, "name", "Flow");
    fill(a, ctx, "source", &source.0.to_string());
    fill(a, ctx, "target", &target.0.to_string());
    fill(a, ctx, "guard", guard);
    let id = Id(a.session.editor.snapshot.next_id);
    click(a, ctx, "activity:Apply");
    id
}
fn seeded(a: &mut Workspace) -> (Id, Id, Id, Id) {
    let id = Id(a.session.editor.snapshot.next_id);
    let stamp = a.views.views.stamp(&a.session.editor);
    a.activity_model(
        &stamp,
        ac::Action::Activity {
            id: None,
            system: RecordId(1),
            name: "Seeded activity".into(),
        },
    )
    .unwrap();
    let mut nodes = Vec::new();
    for kind in [
        ac::NodeKind::Initial,
        ac::NodeKind::Action,
        ac::NodeKind::Final,
    ] {
        let n = Id(a.session.editor.snapshot.next_id);
        let stamp = a.views.views.stamp(&a.session.editor);
        a.activity_model(
            &stamp,
            ac::Action::Node {
                activity: id,
                id: None,
                name: format!("{kind:?}"),
                kind,
                partition: None,
            },
        )
        .unwrap();
        nodes.push(n);
    }
    for (source, target) in [(nodes[0], nodes[1]), (nodes[1], nodes[2])] {
        let stamp = a.views.views.stamp(&a.session.editor);
        a.activity_model(
            &stamp,
            ac::Action::SetFlowForm(ac::FlowForm {
                activity: id,
                id: None,
                name: "Seeded flow".into(),
                kind: ac::FlowKind::Control,
                source: source.0.to_string(),
                target: target.0.to_string(),
                guard: "".into(),
            }),
        )
        .unwrap();
        let stamp = a.views.views.stamp(&a.session.editor);
        a.activity_model(&stamp, ac::Action::ApplyFlow).unwrap();
    }
    a.activities.activity = Some(id);
    (id, nodes[0], nodes[1], nodes[2])
}
#[test]
fn rendered_activity_creation_partition_nodes_pin_and_guard_refusal_preserve_original_native_type_meaning()
 {
    let (_area, mut a, ctx) = app();
    let before = a.session.editor.snapshot.clone();
    let prior = ty::load(&before).unwrap();
    let id = new_activity(&mut a, &ctx);
    click(&mut a, &ctx, "activity:new:Partition");
    fill(&mut a, &ctx, "name", "Reviewer");
    let partition = Id(a.session.editor.snapshot.next_id);
    click(&mut a, &ctx, "activity:Apply");
    let decision = new_node(&mut a, &ctx, ac::NodeKind::Decision);
    let action = new_node(&mut a, &ctx, ac::NodeKind::Action);
    fill(&mut a, &ctx, "partition", &partition.0.to_string());
    click(&mut a, &ctx, "activity:Apply");
    click(&mut a, &ctx, "activity:new:Pin");
    fill(&mut a, &ctx, "name", "Input");
    fill(&mut a, &ctx, "owner", &action.0.to_string());
    fill(&mut a, &ctx, "value-type", "1005");
    let pin = Id(a.session.editor.snapshot.next_id);
    click(&mut a, &ctx, "activity:Apply");
    let flow = new_flow(&mut a, &ctx, decision, action, "true");
    fill(&mut a, &ctx, "guard", "x >");
    let accepted = ty::load(&a.session.editor.snapshot).unwrap();
    let snapshot = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "activity:Apply");
    assert_eq!(a.session.editor.snapshot, snapshot);
    let store = ty::load(&a.session.editor.snapshot).unwrap();
    assert_eq!(store, accepted);
    assert_eq!(
        store.activities[&id].flows[&flow].guard,
        Some(ac::Guard::Boolean(true))
    );
    assert_eq!(store.activities[&id].pins[&pin].owner, action);
    assert_eq!(
        store.activities[&id].nodes[&action].partition,
        Some(partition)
    );
    assert_eq!(a.session.editor.snapshot.records, before.records);
    assert_eq!(a.session.editor.snapshot.drafts, before.drafts);
    assert_eq!(store.definitions, prior.definitions);
    assert_eq!(store.properties, prior.properties);
}
#[test]
fn rendered_distinct_flow_proposals_survive_selection_and_model_history_without_native_field_changes()
 {
    let (_area, mut a, ctx) = app();
    let id = new_activity(&mut a, &ctx);
    let d = new_node(&mut a, &ctx, ac::NodeKind::Decision);
    let n = new_node(&mut a, &ctx, ac::NodeKind::Action);
    let end = new_node(&mut a, &ctx, ac::NodeKind::Final);
    let one = new_flow(&mut a, &ctx, d, n, "true");
    fill(&mut a, &ctx, "guard", "first ?");
    let two = new_flow(&mut a, &ctx, d, end, "else");
    fill(&mut a, &ctx, "guard", "second ?");
    click(&mut a, &ctx, "activity:Browse elements");
    click(&mut a, &ctx, &format!("activity:edit:flow:{}", one.0));
    assert_eq!(a.activity_audit()["current"]["form"]["guard"], "first ?");
    click(&mut a, &ctx, &format!("activity:edit:flow:{}", two.0));
    assert_eq!(a.activity_audit()["current"]["form"]["guard"], "second ?");
    let raw = af::load(&a.session.editor.snapshot).unwrap();
    let records = a.session.editor.snapshot.records.clone();
    click(&mut a, &ctx, "activity:Model Undo");
    assert!(
        !ty::load(&a.session.editor.snapshot).unwrap().activities[&id]
            .flows
            .contains_key(&two)
    );
    assert_eq!(af::load(&a.session.editor.snapshot).unwrap(), raw);
    click(&mut a, &ctx, "activity:Model Redo");
    assert_eq!(
        ty::load(&a.session.editor.snapshot).unwrap().activities[&id]
            .flows
            .len(),
        2
    );
    assert_eq!(af::load(&a.session.editor.snapshot).unwrap(), raw);
    assert_eq!(a.session.editor.snapshot.records, records);
}
#[test]
fn rendered_independent_activity_diagrams_nudge_pin_hide_fit_and_layout_history_preserve_whole_native()
 {
    let (_area, mut a, ctx) = app();
    let (id, _, node, _) = seeded(&mut a);
    let stamp = a.views.views.stamp(&a.session.editor);
    a.activity_create_diagram(&stamp, id).unwrap();
    let first = a.views.active;
    let stamp = a.views.views.stamp(&a.session.editor);
    a.activity_create_diagram(&stamp, id).unwrap();
    let second = a.views.active;
    let before = a.session.editor.snapshot.clone();
    let original = a.views.views.document().clone();
    a.activities.open = false;
    click(&mut a, &ctx, &format!("view:tab:{}", first.get()));
    settle(&mut a, &ctx);
    let occurrence = a.views.views.document().activity_diagrams[&first]
        .occurrences
        .values()
        .find(|o| o.element == node)
        .unwrap()
        .id;
    click(
        &mut a,
        &ctx,
        &format!("activity-occurrence:{}", occurrence.get()),
    );
    click(&mut a, &ctx, "activity:Close");
    click(&mut a, &ctx, "activity-canvas:Move right");
    assert_eq!(
        a.views.views.document().activity_diagrams[&first].occurrences[&occurrence]
            .geometry
            .x,
        original.activity_diagrams[&first].occurrences[&occurrence]
            .geometry
            .x
            + 40.
    );
    click(&mut a, &ctx, "activity-canvas:Diagram Undo");
    assert_eq!(
        a.views.views.document().activity_diagrams,
        original.activity_diagrams
    );
    click(&mut a, &ctx, "activity-canvas:Diagram Redo");
    click(&mut a, &ctx, "activity-canvas:Toggle pin");
    let geometry = a.views.views.document().activity_diagrams[&first].occurrences[&occurrence]
        .geometry
        .clone();
    click(&mut a, &ctx, "activity-canvas:Move right");
    assert_eq!(
        a.views.views.document().activity_diagrams[&first].occurrences[&occurrence].geometry,
        geometry
    );
    click(&mut a, &ctx, "activity-canvas:Toggle visibility");
    assert!(
        !a.activity_audit()["rendered"]["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["element"] == node.0)
    );
    click(&mut a, &ctx, &format!("view:tab:{}", second.get()));
    assert_eq!(
        a.views.views.document().activity_diagrams[&second],
        original.activity_diagrams[&second]
    );
    click(&mut a, &ctx, "camera:Fit");
    assert_eq!(a.session.editor.snapshot, before);
}
#[test]
fn stale_activity_selection_layout_and_busy_admission_refuse_without_native_or_diagram_changes() {
    let (_area, mut a, _ctx) = app();
    let (id, _, node, _) = seeded(&mut a);
    let stamp = a.views.views.stamp(&a.session.editor);
    a.activity_create_diagram(&stamp, id).unwrap();
    let view = a.views.active;
    let old = a.views.views.stamp(&a.session.editor);
    let current = a.activities.current.clone();
    a.session
        .editor
        .snapshot
        .records
        .get_mut(&RecordId(1))
        .unwrap()
        .name = "Changed source with same counters".into();
    let before = a.session.editor.snapshot.clone();
    let diagram = a.views.views.document().clone();
    assert!(a.activity_select_element(&old, view, node).is_err());
    assert_eq!(a.activities.current, current);
    let occurrence = a.views.views.document().activity_diagrams[&view]
        .occurrences
        .values()
        .find(|o| o.element == node)
        .unwrap()
        .id;
    assert!(
        a.activity_layout(
            &old,
            nv::activities::Layout::Move {
                view,
                occurrence,
                geometry: nv::Geometry::at(100., 100.)
            }
        )
        .is_err()
    );
    a.settings_window = true;
    let stamp = a.views.views.stamp(&a.session.editor);
    assert!(
        a.activity_model(
            &stamp,
            ac::Action::Activity {
                id: None,
                system: RecordId(1),
                name: "Busy".into()
            }
        )
        .is_err()
    );
    assert_eq!(a.session.editor.snapshot, before);
    assert_eq!(a.views.views.document(), &diagram);
}
#[test]
fn rendered_unadmitted_multibyte_activity_text_is_retained_and_close_requires_explicit_discard() {
    let (_area, mut a, ctx) = app();
    let before = a.session.editor.snapshot.clone();
    let text = "é".repeat(2500);
    fill(&mut a, &ctx, "name", &text);
    let Form::Activity { name, .. } = &a.activities.pending.as_ref().unwrap().form else {
        panic!("wrong retained family")
    };
    assert_eq!(name, &text);
    assert!(a.activity_close_blocked());
    assert_eq!(a.session.editor.snapshot, before);
    click(&mut a, &ctx, "activity:Close");
    assert!(a.activities.open);
    click(&mut a, &ctx, "activity:Discard new text");
    assert!(a.activities.pending.is_none());
    assert!(!a.activity_close_blocked());
    assert_eq!(a.session.editor.snapshot, before);
}
#[test]
fn activity_tool_shortcut_is_delivered_to_shared_dispatch_and_scopes_file_history_and_docking() {
    let (_area, mut a, ctx) = app();
    a.activities.open = false;
    let chord = crate::command_catalog::Id::Activities.spec().shortcut;
    ctx.run_ui(
        egui::RawInput {
            focused: true,
            events: vec![egui::Event::Key {
                key: chord.logical_key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: chord.modifiers,
            }],
            ..Default::default()
        },
        |ui| a.handle_shortcuts(ui.ctx()),
    )
    .drop_without_applying_deltas();
    assert!(a.activities.open);
    assert_eq!(a.history_scope(), ShortcutTarget::Widget);
    let before = a.session.editor.snapshot.clone();
    for id in [
        crate::command_catalog::Id::New,
        crate::command_catalog::Id::Undo,
        crate::command_catalog::Id::Save,
    ] {
        a.dispatch_workspace(
            id,
            crate::command_catalog::Surface::Menu,
            ShortcutTarget::Widget,
        );
    }
    assert_eq!(a.session.editor.snapshot, before);
    assert!(a.dock_blocked());
}

#[test]
fn rendered_unsupported_type_context_keeps_pending_text_copy_discard_and_close_available() {
    let (_area, mut a, ctx) = app();
    let form = Form::Activity {
        id: None,
        system: "1".into(),
        name: "retained".repeat(600),
    };
    let stamp = a.views.views.stamp(&a.session.editor);
    assert!(a.activity_set_form(&stamp, form.clone()).is_err());
    a.session
        .editor
        .snapshot
        .extensions
        .as_mut()
        .unwrap()
        .required_namespaces
        .retain(|n| n != ty::NAMESPACE);
    a.session.editor.snapshot.validate().unwrap();
    let before = a.session.editor.snapshot.clone();
    settle(&mut a, &ctx);
    assert_eq!(a.activities.pending.as_ref().unwrap().form, form);
    click(&mut a, &ctx, "activity:Close");
    assert!(a.activities.open && a.activities.pending.is_some());
    click(&mut a, &ctx, "activity:Discard new text");
    assert!(a.activities.pending.is_none());
    click(&mut a, &ctx, "activity:Close");
    assert!(!a.activities.open);
    assert_eq!(a.session.editor.snapshot, before);
}
#[test]
fn rendered_activity_drag_publishes_one_layout_entry_and_single_undo_keeps_entire_native() {
    let (_area, mut a, ctx) = app();
    let (id, _, node, _) = seeded(&mut a);
    let stamp = a.views.views.stamp(&a.session.editor);
    a.activity_create_diagram(&stamp, id).unwrap();
    let view = a.views.active;
    a.activities.open = false;
    settle(&mut a, &ctx);
    let occurrence = a.views.views.document().activity_diagrams[&view]
        .occurrences
        .values()
        .find(|o| o.element == node)
        .unwrap()
        .id;
    let geometry = a.views.views.document().activity_diagrams[&view].occurrences[&occurrence]
        .geometry
        .clone();
    let native = a.session.editor.snapshot.clone();
    let entries = a.views.views.history_usage().0;
    let r = rect(&a, &format!("activity-occurrence:{}", occurrence.get()));
    let start = egui::pos2((r[0] + r[2]) / 2., (r[1] + r[3]) / 2.);
    render(
        &mut a,
        &ctx,
        vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for offset in [egui::vec2(20., 10.), egui::vec2(60., 30.)] {
        render(
            &mut a,
            &ctx,
            vec![egui::Event::PointerMoved(start + offset)],
        );
    }
    assert!(a.activity_gesture_active());
    assert!(
        a.command_reason(crate::command_catalog::Id::Save, &ShortcutTarget::Model)
            .is_some()
    );
    assert!(a.dock_blocked());
    let end = start + egui::vec2(60., 30.);
    render(
        &mut a,
        &ctx,
        vec![
            egui::Event::PointerMoved(end),
            egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    settle(&mut a, &ctx);
    assert!(!a.activity_gesture_active());
    let moved =
        &a.views.views.document().activity_diagrams[&view].occurrences[&occurrence].geometry;
    assert!(
        (moved.x - geometry.x - 60.).abs() < 0.01 && (moved.y - geometry.y - 30.).abs() < 0.01,
        "{moved:?} vs {geometry:?}"
    );
    assert_eq!(a.views.views.history_usage().0, entries + 1);
    click(&mut a, &ctx, "activity-canvas:Diagram Undo");
    assert_eq!(
        a.views.views.document().activity_diagrams[&view].occurrences[&occurrence].geometry,
        geometry
    );
    assert_eq!(a.session.editor.snapshot, native);
}

#[test]
fn rendered_activity_execution_prepare_step_trace_copy_and_clear_preserve_whole_native_and_layout()
{
    use mbse_model::types::activities::execution::Status;
    let (_area, mut a, ctx) = app();
    let (activity, start, action, end) = seeded(&mut a);
    let before = a.session.editor.snapshot.clone();
    let layout = a.views.views.document().clone();
    click(&mut a, &ctx, "activity:Execution controls");
    click(&mut a, &ctx, "activity:execution:Prepare");
    assert_eq!(
        a.activities
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .activity,
        activity
    );
    assert_eq!(
        a.activities
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .status,
        Status::Ready
    );
    for _ in 0..3 {
        click(&mut a, &ctx, "activity:execution:Step");
    }
    let live = a.activities.execution.live.as_ref().unwrap();
    assert_eq!(live.report().status, Status::Completed);
    assert_eq!(
        live.report()
            .visits
            .iter()
            .map(|v| v.node)
            .collect::<Vec<_>>(),
        vec![start, action, end]
    );
    assert!(live.is_current(&a.session.editor));
    click(&mut a, &ctx, "activity:execution:Copy");
    assert_eq!(a.session.editor.snapshot, before);
    assert_eq!(a.views.views.document(), &layout);
    click(&mut a, &ctx, "activity:execution:Clear");
    assert!(a.activities.execution.live.is_none());
}
fn seed_cycle(a: &mut Workspace) -> Id {
    let (id, _, action, end) = seeded(a);
    let stamp = a.views.views.stamp(&a.session.editor);
    a.activity_model(
        &stamp,
        ac::Action::Node {
            activity: id,
            id: Some(end),
            name: "Merge cycle".into(),
            kind: ac::NodeKind::Merge,
            partition: None,
        },
    )
    .unwrap();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.activity_model(
        &stamp,
        ac::Action::SetFlowForm(ac::FlowForm {
            activity: id,
            id: None,
            name: "Cycle back".into(),
            kind: ac::FlowKind::Control,
            source: end.0.to_string(),
            target: action.0.to_string(),
            guard: "".into(),
        }),
    )
    .unwrap();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.activity_model(&stamp, ac::Action::ApplyFlow).unwrap();
    id
}
#[test]
fn rendered_activity_execution_auto_budget_and_explicit_cancel_close_guards_preserve_source() {
    use mbse_model::types::activities::execution::Status;
    let (_area, mut a, ctx) = app();
    seed_cycle(&mut a);
    let before = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "activity:Execution controls");
    fill(&mut a, &ctx, "execution-budget", "3");
    click(&mut a, &ctx, "activity:execution:Prepare");
    click(&mut a, &ctx, "activity:execution:Run");
    assert!(a.activity_execution_automatic());
    assert!(a.dock_blocked());
    assert!(!a.activity_ready());
    for _ in 0..4 {
        a.activities.execution.next_tick = None;
        a.activity_execution_tick();
    }
    assert_eq!(
        a.activities
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .status,
        Status::BudgetExhausted
    );
    assert_eq!(
        a.activities
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .visits
            .len(),
        3
    );
    assert!(!a.activity_execution_automatic());
    fill(&mut a, &ctx, "execution-budget", "4096");
    click(&mut a, &ctx, "activity:execution:Prepare");
    click(&mut a, &ctx, "activity:execution:Run");
    click(&mut a, &ctx, "activity:Close");
    assert!(a.activities.open);
    assert!(a.activity_execution_active());
    assert!(a.activities.note.contains("Cancel"));
    click(&mut a, &ctx, "activity:execution:Cancel");
    assert_eq!(
        a.activities
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .status,
        Status::Cancelled
    );
    assert!(!a.activity_execution_automatic());
    click(&mut a, &ctx, "activity:Close");
    assert!(!a.activities.open);
    assert_eq!(a.session.editor.snapshot, before);
}
#[test]
fn rendered_activity_execution_paused_new_raw_source_change_stops_and_marks_historical_trace() {
    use mbse_model::types::activities::execution::Status;
    let (_area, mut a, ctx) = app();
    seeded(&mut a);
    let accepted = ty::load(&a.session.editor.snapshot).unwrap();
    click(&mut a, &ctx, "activity:Execution controls");
    click(&mut a, &ctx, "activity:execution:Prepare");
    click(&mut a, &ctx, "activity:execution:Step");
    let visits = a
        .activities
        .execution
        .live
        .as_ref()
        .unwrap()
        .report()
        .visits
        .clone();
    fill(&mut a, &ctx, "name", "New raw source proposal");
    settle(&mut a, &ctx);
    let live = a.activities.execution.live.as_ref().unwrap();
    assert_eq!(live.report().status, Status::StaleSource);
    assert!(!live.is_current(&a.session.editor));
    assert_eq!(live.report().visits, visits);
    assert_eq!(ty::load(&a.session.editor.snapshot).unwrap(), accepted);
    assert_eq!(
        a.activity_audit()["current"]["name"],
        "New raw source proposal"
    );
    click(&mut a, &ctx, "activity:execution:Copy");
    click(&mut a, &ctx, "activity:Close");
    assert!(!a.activities.open);
}
#[test]
fn rendered_activity_execution_deadlock_bad_budget_and_payload_refusal_remain_visible_without_overwrite()
 {
    use mbse_model::types::activities::execution::Status;
    let (_area, mut a, ctx) = app();
    let (id, _, action, end) = seeded(&mut a);
    let flow = ty::load(&a.session.editor.snapshot).unwrap().activities[&id]
        .flows
        .values()
        .find(|f| f.target == end)
        .unwrap()
        .id;
    let stamp = a.views.views.stamp(&a.session.editor);
    a.activity_model(
        &stamp,
        ac::Action::Remove {
            activity: id,
            element: flow,
        },
    )
    .unwrap();
    click(&mut a, &ctx, "activity:Execution controls");
    fill(&mut a, &ctx, "execution-budget", "0");
    let before = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "activity:execution:Prepare");
    assert!(a.activities.execution.live.is_none());
    assert!(a.activities.execution.note.contains("BUDGET"));
    assert_eq!(a.session.editor.snapshot, before);
    fill(&mut a, &ctx, "execution-budget", "8");
    click(&mut a, &ctx, "activity:execution:Prepare");
    click(&mut a, &ctx, "activity:execution:Step");
    click(&mut a, &ctx, "activity:execution:Step");
    assert_eq!(
        a.activities
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .status,
        Status::Deadlock { node: action }
    );
    let prior = a
        .activities
        .execution
        .live
        .as_ref()
        .unwrap()
        .report()
        .clone();
    let root: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p30c/verification/native-04/run/activity-cold-copy.kernspan"
    ))
    .unwrap();
    a.session.editor = mbse_model::stage1::Editor::new(
        serde_json::from_value(root["snapshot"].clone()).unwrap(),
        a.session.editor.freshness().instance_id,
    )
    .unwrap();
    a.activities.activity = Some(Id(1033));
    let before = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "activity:execution:Prepare");
    assert!(
        a.activities
            .execution
            .note
            .contains("OBJECT_PINS_OR_FLOWS_UNSUPPORTED")
    );
    assert_eq!(
        a.activities.execution.live.as_ref().unwrap().report(),
        &prior
    );
    assert!(
        !a.activities
            .execution
            .live
            .as_ref()
            .unwrap()
            .is_current(&a.session.editor)
    );
    assert_eq!(a.session.editor.snapshot, before);
}
