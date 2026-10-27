use super::*;
fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p32c/fixtures/native.kernspan"
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
        crate::command_catalog::Id::StateMachines,
        crate::command_catalog::Surface::Menu,
        ShortcutTarget::Model,
    );
    (area, a, ctx)
}
fn render(a: &mut Workspace, ctx: &egui::Context, events: Vec<egui::Event>) {
    a.audit.clear();
    a.activity_frame_start();
    a.sm_frame_start();
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
            a.sm_dialog(ctx);
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
                k.starts_with("state-machine:") && v[3] - v[1] >= 12. && v[1] >= 0. && v[3] <= 1600.
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
    click(a, ctx, &format!("state-machine:field:{key}"));
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
fn machine(a: &mut Workspace, ctx: &egui::Context) -> Id {
    let id = Id(a.session.editor.snapshot.next_id);
    click(a, ctx, "state-machine:new:Machine");
    fill(a, ctx, "name", "Controller");
    click(a, ctx, "state-machine:Apply");
    assert!(
        ty::load(&a.session.editor.snapshot)
            .unwrap()
            .state_machines
            .contains_key(&id)
    );
    id
}
fn state(a: &mut Workspace, ctx: &egui::Context, kind: StateKind) -> Id {
    click(a, ctx, "state-machine:new:State");
    fill(a, ctx, "name", &format!("{kind:?}"));
    click(a, ctx, &format!("state-machine:state-kind:{kind:?}"));
    let id = Id(a.session.editor.snapshot.next_id);
    click(a, ctx, "state-machine:Apply");
    id
}
fn event(a: &mut Workspace, ctx: &egui::Context) -> Id {
    click(a, ctx, "state-machine:new:Event");
    fill(a, ctx, "name", "Go");
    let id = Id(a.session.editor.snapshot.next_id);
    click(a, ctx, "state-machine:Apply");
    id
}
fn transition(
    a: &mut Workspace,
    ctx: &egui::Context,
    source: Id,
    target: Id,
    event: Option<Id>,
    guard: &str,
) -> Id {
    click(a, ctx, "state-machine:new:Transition");
    fill(a, ctx, "name", "Transition");
    fill(a, ctx, "source", &source.0.to_string());
    fill(a, ctx, "target", &target.0.to_string());
    fill(
        a,
        ctx,
        "event",
        &event.map_or(String::new(), |id| id.0.to_string()),
    );
    fill(a, ctx, "guard", guard);
    let id = Id(a.session.editor.snapshot.next_id);
    click(a, ctx, "state-machine:Apply");
    id
}
fn seeded(a: &mut Workspace) -> (Id, Id, Id, Id, Id, Id) {
    fn accept(a: &mut Workspace, f: Form) -> Id {
        let id = f.id().unwrap_or(Id(a.session.editor.snapshot.next_id));
        let key = f.key();
        let stamp = a.views.views.stamp(&a.session.editor);
        a.sm_model(&stamp, sm::Action::Set(f)).unwrap();
        let stamp = a.views.views.stamp(&a.session.editor);
        a.sm_model(&stamp, sm::Action::Apply(key)).unwrap();
        id
    }
    let m = accept(
        a,
        Form::Machine {
            id: None,
            system: "1".into(),
            name: "Seeded controller".into(),
        },
    );
    let mut nodes = Vec::new();
    for kind in [StateKind::Initial, StateKind::State, StateKind::Final] {
        nodes.push(accept(
            a,
            Form::State {
                machine: m,
                id: None,
                name: format!("{kind:?}"),
                state_kind: kind,
            },
        ));
    }
    let go = accept(
        a,
        Form::Event {
            machine: m,
            id: None,
            name: "Go".into(),
        },
    );
    let mut edges = Vec::new();
    for (source, target, event, guard) in [
        (nodes[0], nodes[1], None, ""),
        (nodes[1], nodes[2], Some(go), "true"),
    ] {
        edges.push(accept(
            a,
            Form::Transition {
                machine: m,
                id: None,
                name: "Transition".into(),
                source: source.0.to_string(),
                target: target.0.to_string(),
                event: event.map_or(String::new(), |id| id.0.to_string()),
                guard: guard.into(),
            },
        ));
    }
    a.state_machines.machine = Some(m);
    (m, nodes[0], nodes[1], nodes[2], go, edges[1])
}
#[test]
fn rendered_state_machine_widgets_author_states_events_transitions_invalid_guard_and_target_without_native_reclassification()
 {
    let (_area, mut a, ctx) = app();
    let before = a.session.editor.snapshot.clone();
    let prior = ty::load(&before).unwrap();
    let m = machine(&mut a, &ctx);
    let start = state(&mut a, &ctx, StateKind::Initial);
    let waiting = state(&mut a, &ctx, StateKind::State);
    let end = state(&mut a, &ctx, StateKind::Final);
    let go = event(&mut a, &ctx);
    transition(&mut a, &ctx, start, waiting, None, "");
    let edge = transition(&mut a, &ctx, waiting, end, Some(go), "true");
    let accepted = ty::load(&a.session.editor.snapshot).unwrap();
    assert_eq!(accepted.state_machines[&m].states.len(), 3);
    assert_eq!(accepted.state_machines[&m].events.len(), 1);
    assert_eq!(
        accepted.state_machines[&m].transitions[&edge].event,
        Some(go)
    );
    fill(&mut a, &ctx, "guard", "x >");
    let raw = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "state-machine:Apply");
    assert_eq!(a.session.editor.snapshot, raw);
    assert_eq!(a.sm_audit()["current"]["guard"], "x >");
    assert_eq!(ty::load(&a.session.editor.snapshot).unwrap(), accepted);
    fill(&mut a, &ctx, "guard", "true");
    fill(&mut a, &ctx, "target", "99999");
    let raw = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "state-machine:Apply");
    assert_eq!(a.session.editor.snapshot, raw);
    assert!(a.state_machines.note.contains("TARGET"));
    assert_eq!(a.session.editor.snapshot.records, before.records);
    assert_eq!(a.session.editor.snapshot.drafts, before.drafts);
    let mut after = ty::load(&a.session.editor.snapshot).unwrap();
    after.state_machines.clear();
    assert_eq!(after, prior);
}
#[test]
fn rendered_state_machine_ambiguous_definitions_distinct_raw_selection_and_model_history_keep_exact_data()
 {
    let (_area, mut a, ctx) = app();
    let (m, _, waiting, end, go, first) = seeded(&mut a);
    let second = transition(&mut a, &ctx, waiting, end, Some(go), "");
    assert!(
        sm::diagnostics(&ty::load(&a.session.editor.snapshot).unwrap(), m)
            .unwrap()
            .iter()
            .any(|d| d.code == "SM_AMBIGUOUS_TRANSITIONS" && d.elements == vec![first, second])
    );
    click(&mut a, &ctx, "state-machine:Browse elements");
    click(
        &mut a,
        &ctx,
        &format!("state-machine:edit:transition:{}", first.0),
    );
    fill(&mut a, &ctx, "guard", "x >");
    click(
        &mut a,
        &ctx,
        &format!("state-machine:edit:transition:{}", second.0),
    );
    fill(&mut a, &ctx, "guard", "else ?");
    let proposals = sm::forms(&a.session.editor.snapshot).unwrap();
    click(&mut a, &ctx, "state-machine:Model Undo");
    assert_eq!(sm::forms(&a.session.editor.snapshot).unwrap(), proposals);
    assert!(
        !ty::load(&a.session.editor.snapshot).unwrap().state_machines[&m]
            .transitions
            .contains_key(&second)
    );
    click(&mut a, &ctx, "state-machine:Model Redo");
    assert_eq!(sm::forms(&a.session.editor.snapshot).unwrap(), proposals);
    click(
        &mut a,
        &ctx,
        &format!("state-machine:edit:transition:{}", first.0),
    );
    assert_eq!(a.sm_audit()["current"]["guard"], "x >");
    click(
        &mut a,
        &ctx,
        &format!("state-machine:edit:transition:{}", second.0),
    );
    assert_eq!(a.sm_audit()["current"]["guard"], "else ?");
    click(&mut a, &ctx, &format!("state-machine:edit:event:{}", go.0));
    let before = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "state-machine:Remove accepted");
    assert_eq!(a.session.editor.snapshot, before);
    assert!(a.state_machines.note.contains("EVENT"));
}
#[test]
fn rendered_state_machine_two_diagrams_shapes_transition_ledger_layout_history_pin_hide_fit_preserve_semantics()
 {
    let (_area, mut a, ctx) = app();
    let (m, _, node, _, _, edge) = seeded(&mut a);
    let native = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "state-machine:Create diagram");
    let first = a.views.active;
    click(&mut a, &ctx, "state-machine:Create diagram");
    let second = a.views.active;
    assert_ne!(first, second);
    click(&mut a, &ctx, "state-machine:Close");
    settle(&mut a, &ctx);
    assert_eq!(
        a.sm_audit()["rendered"]["transitions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let id = a.views.views.document().state_machine_diagrams[&second]
        .occurrences
        .values()
        .find(|o| o.element == node)
        .unwrap()
        .id;
    let original = a.views.views.document().state_machine_diagrams[&second].occurrences[&id]
        .geometry
        .clone();
    click(
        &mut a,
        &ctx,
        &format!("state-machine-occurrence:{}", id.get()),
    );
    assert_eq!(a.sm_audit()["current"]["id"], node.0);
    click(&mut a, &ctx, "state-machine:Close");
    click(&mut a, &ctx, "state-machine-canvas:Move right");
    let moved = a.views.views.document().state_machine_diagrams[&second].occurrences[&id]
        .geometry
        .clone();
    assert_eq!(moved.x, original.x + 40.);
    click(&mut a, &ctx, "state-machine-canvas:Toggle pin");
    click(&mut a, &ctx, "state-machine-canvas:Move right");
    assert_eq!(
        a.views.views.document().state_machine_diagrams[&second].occurrences[&id].geometry,
        moved
    );
    click(&mut a, &ctx, "state-machine-canvas:Toggle visibility");
    assert_eq!(
        a.sm_audit()["rendered"]["transitions"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    click(&mut a, &ctx, "state-machine-canvas:Diagram Undo");
    assert_eq!(
        a.sm_audit()["rendered"]["transitions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    click(
        &mut a,
        &ctx,
        &format!("state-machine-transition:{}", edge.0),
    );
    assert_eq!(a.sm_audit()["current"]["id"], edge.0);
    click(&mut a, &ctx, "state-machine:Close");
    click(&mut a, &ctx, &format!("view:tab:{}", first.get()));
    settle(&mut a, &ctx);
    assert_eq!(
        a.views.views.document().state_machine_diagrams[&first]
            .occurrences
            .values()
            .find(|o| o.element == node)
            .unwrap()
            .geometry,
        original
    );
    let second_camera = a.views.views.document().views[&second].camera.clone();
    click(&mut a, &ctx, "camera:Fit");
    assert_ne!(a.views.views.document().views[&first].camera.zoom, 1.);
    assert_eq!(a.views.note, "Camera fitted to visible occurrences.");
    click(&mut a, &ctx, "camera:Pan right");
    assert!(a.views.views.document().views[&first].camera.x > 0.);
    assert_eq!(
        a.views.views.document().views[&second].camera,
        second_camera
    );
    assert_eq!(a.session.editor.snapshot, native);
    assert_eq!(a.state_machines.machine, Some(m));
}
#[test]
fn rendered_state_machine_drag_publishes_one_layout_entry_and_single_undo_preserves_entire_native()
{
    let (_area, mut a, ctx) = app();
    let (m, _, node, _, _, _) = seeded(&mut a);
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sm_create_diagram(&stamp, m).unwrap();
    let view = a.views.active;
    a.state_machines.open = false;
    settle(&mut a, &ctx);
    let id = a.views.views.document().state_machine_diagrams[&view]
        .occurrences
        .values()
        .find(|o| o.element == node)
        .unwrap()
        .id;
    let geometry = a.views.views.document().state_machine_diagrams[&view].occurrences[&id]
        .geometry
        .clone();
    let native = a.session.editor.snapshot.clone();
    let entries = a.views.views.history_usage().0;
    let r = rect(&a, &format!("state-machine-occurrence:{}", id.get()));
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
    assert!(a.sm_gesture_active());
    assert!(a.dock_blocked());
    assert!(
        a.command_reason(crate::command_catalog::Id::Save, &ShortcutTarget::Model)
            .is_some()
    );
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
    assert!(!a.sm_gesture_active());
    let moved = &a.views.views.document().state_machine_diagrams[&view].occurrences[&id].geometry;
    assert!((moved.x - geometry.x - 60.).abs() < 0.01 && (moved.y - geometry.y - 30.).abs() < 0.01);
    assert_eq!(a.views.views.history_usage().0, entries + 1);
    click(&mut a, &ctx, "state-machine-canvas:Diagram Undo");
    assert_eq!(
        a.views.views.document().state_machine_diagrams[&view].occurrences[&id].geometry,
        geometry
    );
    assert_eq!(a.session.editor.snapshot, native);
}
#[test]
fn rendered_state_machine_utf8_admission_refusal_keeps_new_text_and_blocks_close_until_explicit_discard()
 {
    let (_area, mut a, ctx) = app();
    let before = a.session.editor.snapshot.clone();
    fill(&mut a, &ctx, "name", &"é".repeat(2500));
    assert!(a.sm_pending());
    assert_eq!(a.session.editor.snapshot, before);
    click(&mut a, &ctx, "state-machine:Close");
    assert!(a.state_machines.open);
    assert_eq!(a.sm_audit()["current"]["name"], "é".repeat(2500));
    click(&mut a, &ctx, "state-machine:Copy new text");
    click(&mut a, &ctx, "state-machine:Discard new text");
    assert!(!a.sm_pending());
    click(&mut a, &ctx, "state-machine:Close");
    assert!(!a.state_machines.open);
    assert_eq!(a.session.editor.snapshot, before);
}
#[test]
fn rendered_state_machine_unsupported_optional_source_keeps_pending_copy_discard_close_route() {
    for namespace in [ty::NAMESPACE, ty::DRAFTS] {
        let (_area, mut a, ctx) = app();
        fill(&mut a, &ctx, "name", &"é".repeat(2500));
        assert!(a.sm_pending());
        a.session
            .editor
            .snapshot
            .extensions
            .as_mut()
            .unwrap()
            .required_namespaces
            .retain(|n| n != namespace);
        let before = a.session.editor.snapshot.clone();
        settle(&mut a, &ctx);
        click(&mut a, &ctx, "state-machine:Close");
        assert!(a.state_machines.open);
        assert!(a.sm_pending());
        click(&mut a, &ctx, "state-machine:Copy new text");
        click(&mut a, &ctx, "state-machine:Discard new text");
        click(&mut a, &ctx, "state-machine:Close");
        assert!(!a.state_machines.open);
        assert_eq!(a.session.editor.snapshot, before);
    }
}
#[test]
fn state_machine_same_counter_source_and_view_stamps_refuse_without_rebinding_or_losing_pending() {
    let (_area, mut a, ctx) = app();
    let (m, _, node, _, _, _) = seeded(&mut a);
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sm_create_diagram(&stamp, m).unwrap();
    let view = a.views.active;
    let stale = a.views.views.stamp(&a.session.editor);
    let before = a.session.editor.snapshot.clone();
    a.session.editor =
        mbse_model::stage1::Editor::new(before.clone(), "different:runtime".into()).unwrap();
    assert!(a.sm_select_element(&stale, view, node).is_err());
    let before_views = a.views.views.document().clone();
    assert!(
        a.sm_layout(&stale, nv::state_machines::Layout::Clear(view))
            .is_err()
    );
    assert_eq!(a.views.views.document(), &before_views);
    assert_eq!(a.session.editor.snapshot, before);
    a.state_machines.current = None;
    a.sm_open();
    settle(&mut a, &ctx);
    fill(&mut a, &ctx, "name", &"é".repeat(2500));
    a.session.editor =
        mbse_model::stage1::Editor::new(before.clone(), "third:runtime".into()).unwrap();
    settle(&mut a, &ctx);
    assert_eq!(a.sm_audit()["owner_current"], false);
    assert_eq!(a.sm_audit()["current"]["name"], "é".repeat(2500));
    click(&mut a, &ctx, "state-machine:Discard new text");
    assert!(!a.sm_pending());
    assert_eq!(a.session.editor.snapshot, before);
}
#[test]
fn state_machine_shared_shortcut_file_history_and_other_actor_docking_guards_use_one_dispatch() {
    let (_area, mut a, ctx) = app();
    a.state_machines.open = false;
    let spec = crate::command_catalog::Id::StateMachines.spec();
    assert_eq!(spec.key, "tools.state_machines");
    assert_eq!(spec.shortcut.logical_key, egui::Key::S);
    ctx.run_ui(
        egui::RawInput {
            focused: true,
            events: vec![egui::Event::Key {
                key: spec.shortcut.logical_key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: spec.shortcut.modifiers,
            }],
            ..Default::default()
        },
        |ui| a.handle_shortcuts(ui.ctx()),
    )
    .drop_without_applying_deltas();
    settle(&mut a, &ctx);
    assert!(a.state_machines.open);
    assert!(
        a.command_reason(crate::command_catalog::Id::Save, &ShortcutTarget::Model)
            .is_some()
    );
    assert!(
        a.command_reason(crate::command_catalog::Id::Undo, &ShortcutTarget::Model)
            .is_some()
    );
    assert!(
        a.command_reason(
            crate::command_catalog::Id::Activities,
            &ShortcutTarget::Model
        )
        .is_some()
    );
    assert!(a.dock_blocked());
    click(&mut a, &ctx, "state-machine:Close");
    a.activity_open();
    assert!(
        a.command_reason(
            crate::command_catalog::Id::StateMachines,
            &ShortcutTarget::Model
        )
        .is_some()
    );
}

#[test]
fn rendered_parallel_state_transition_labels_keep_exact_id_event_guards_and_disjoint_text_bounds() {
    let (_area, mut a, ctx) = app();
    let (m, _, node, last, go, edge) = seeded(&mut a);
    for guard in ["else", ""] {
        let f = Form::Transition {
            machine: m,
            id: None,
            name: "Parallel route".into(),
            source: node.0.to_string(),
            target: last.0.to_string(),
            event: go.0.to_string(),
            guard: guard.into(),
        };
        let key = f.key();
        let stamp = a.views.views.stamp(&a.session.editor);
        a.sm_model(&stamp, sm::Action::Set(f)).unwrap();
        let stamp = a.views.views.stamp(&a.session.editor);
        a.sm_model(&stamp, sm::Action::Apply(key)).unwrap();
    }
    let native = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "state-machine:Create diagram");
    click(&mut a, &ctx, "state-machine:Close");
    settle(&mut a, &ctx);
    let r = a.state_machines.rendered.as_ref().unwrap();
    assert_eq!(r.labels.len(), 4);
    for (i, (id, bounds, text)) in r.labels.iter().enumerate() {
        assert!(text.contains(&format!("Transition #{}", id.0)));
        let rect = Rect::from_min_max(
            egui::pos2(bounds[0], bounds[1]),
            egui::pos2(bounds[2], bounds[3]),
        );
        assert!(rect.width() > 0. && rect.height() > 0.);
        for (occ, _, _) in &r.occurrences {
            let b = a.audit[&format!("state-machine-occurrence:{}", occ.get())];
            let node = Rect::from_min_max(egui::pos2(b[0], b[1]), egui::pos2(b[2], b[3]));
            assert!(
                !rect.intersects(node),
                "Transition {id:?} label hides a state caption"
            );
        }
        for (_, other, _) in r.labels.iter().skip(i + 1) {
            assert!(!rect.intersects(Rect::from_min_max(
                egui::pos2(other[0], other[1]),
                egui::pos2(other[2], other[3])
            )));
        }
    }
    assert!(
        r.labels
            .iter()
            .any(|(id, _, s)| *id == edge && s.contains(&format!("Go #{} [true]", go.0)))
    );
    for guard in ["else", "unconditional"] {
        assert!(
            r.labels
                .iter()
                .any(|(_, _, s)| s.contains(&format!("Go #{} [{guard}]", go.0)))
        );
    }
    assert_eq!(a.session.editor.snapshot, native);
}

#[test]
fn rendered_state_execution_prepare_exact_events_step_trace_copy_clear_preserve_full_native_and_layout()
 {
    use mbse_model::types::state_machines::execution::Status;
    let (_area, mut a, ctx) = app();
    let (m, first, node, last, go, edge) = seeded(&mut a);
    let before = a.session.editor.snapshot.clone();
    let layout = a.views.views.document().clone();
    click(&mut a, &ctx, "state-machine:Execution controls");
    fill(&mut a, &ctx, "execution-events", &go.0.to_string());
    click(&mut a, &ctx, "state-machine:execution:Prepare");
    assert_eq!(
        a.state_machines
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .machine,
        m
    );
    assert_eq!(
        a.state_machines
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .status,
        Status::Ready
    );
    for _ in 0..2 {
        click(&mut a, &ctx, "state-machine:execution:Step");
    }
    let live = a.state_machines.execution.live.as_ref().unwrap();
    assert_eq!(live.report().status, Status::Completed);
    assert_eq!(
        live.report()
            .steps
            .iter()
            .map(|s| (s.source, s.target))
            .collect::<Vec<_>>(),
        vec![(first, node), (node, last)]
    );
    assert_eq!(live.report().steps[1].transition, Some(edge));
    assert_eq!(live.report().consumed_events, 1);
    assert!(live.is_current(&a.session.editor));
    click(&mut a, &ctx, "state-machine:execution:Copy");
    assert_eq!(a.session.editor.snapshot, before);
    assert_eq!(a.views.views.document(), &layout);
    click(&mut a, &ctx, "state-machine:execution:Clear");
    assert!(a.state_machines.execution.live.is_none());
}
fn execution_cycle(a: &mut Workspace) {
    let (m, _, node, _, _, edge) = seeded(a);
    let f = Form::Transition {
        machine: m,
        id: Some(edge),
        name: "Completion loop".into(),
        source: node.0.to_string(),
        target: node.0.to_string(),
        event: String::new(),
        guard: "true".into(),
    };
    let key = f.key();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sm_model(&stamp, sm::Action::Set(f)).unwrap();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sm_model(&stamp, sm::Action::Apply(key)).unwrap();
}
#[test]
fn rendered_state_execution_auto_budget_cancel_prepared_close_and_docking_guards_preserve_source() {
    use mbse_model::types::state_machines::execution::Status;
    let (_area, mut a, ctx) = app();
    execution_cycle(&mut a);
    let before = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "state-machine:Execution controls");
    fill(&mut a, &ctx, "execution-budget", "3");
    click(&mut a, &ctx, "state-machine:execution:Prepare");
    click(&mut a, &ctx, "state-machine:Close");
    assert!(a.state_machines.open && a.sm_close_blocked());
    click(&mut a, &ctx, "state-machine:execution:Run");
    assert!(a.sm_execution_automatic() && a.dock_blocked() && !a.sm_ready());
    for _ in 0..4 {
        a.state_machines.execution.next_tick = None;
        a.sm_execution_tick();
    }
    assert_eq!(
        a.state_machines
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .status,
        Status::BudgetExhausted
    );
    assert_eq!(
        a.state_machines
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .steps
            .len(),
        3
    );
    assert!(!a.sm_execution_automatic());
    click(&mut a, &ctx, "state-machine:execution:Prepare");
    click(&mut a, &ctx, "state-machine:execution:Run");
    click(&mut a, &ctx, "state-machine:execution:Cancel");
    let stopped = a
        .state_machines
        .execution
        .live
        .as_ref()
        .unwrap()
        .report()
        .clone();
    assert_eq!(stopped.status, Status::Cancelled);
    a.sm_execution_tick();
    assert_eq!(
        a.state_machines.execution.live.as_ref().unwrap().report(),
        &stopped
    );
    click(&mut a, &ctx, "state-machine:Close");
    assert!(!a.state_machines.open);
    assert_eq!(a.session.editor.snapshot, before);
}
#[test]
fn rendered_state_execution_paused_raw_change_stales_prior_trace_and_keeps_accepted_source() {
    use mbse_model::types::state_machines::execution::Status;
    let (_area, mut a, ctx) = app();
    let (_, _, _, _, go, _) = seeded(&mut a);
    let accepted = ty::load(&a.session.editor.snapshot).unwrap();
    click(&mut a, &ctx, "state-machine:Execution controls");
    fill(&mut a, &ctx, "execution-events", &go.0.to_string());
    click(&mut a, &ctx, "state-machine:execution:Prepare");
    click(&mut a, &ctx, "state-machine:execution:Step");
    let steps = a
        .state_machines
        .execution
        .live
        .as_ref()
        .unwrap()
        .report()
        .steps
        .clone();
    fill(
        &mut a,
        &ctx,
        "name",
        "Unapplied state execution source edit",
    );
    settle(&mut a, &ctx);
    let live = a.state_machines.execution.live.as_ref().unwrap();
    assert_eq!(live.report().status, Status::StaleSource);
    assert!(!live.is_current(&a.session.editor));
    assert_eq!(live.report().steps, steps);
    assert_eq!(ty::load(&a.session.editor.snapshot).unwrap(), accepted);
    assert_eq!(
        a.sm_audit()["current"]["name"],
        "Unapplied state execution source edit"
    );
    click(&mut a, &ctx, "state-machine:Close");
    assert!(!a.state_machines.open);
}
#[test]
fn rendered_state_execution_conflict_exact_ids_quiescence_bad_budget_and_foreign_event_preserve_prior_result()
 {
    use mbse_model::types::state_machines::execution::Status;
    let (_area, mut a, ctx) = app();
    let (m, _, node, last, go, edge) = seeded(&mut a);
    let conflict = Id(a.session.editor.snapshot.next_id);
    let f = Form::Transition {
        machine: m,
        id: None,
        name: "Competing route".into(),
        source: node.0.to_string(),
        target: last.0.to_string(),
        event: go.0.to_string(),
        guard: String::new(),
    };
    let key = f.key();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sm_model(&stamp, sm::Action::Set(f)).unwrap();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sm_model(&stamp, sm::Action::Apply(key)).unwrap();
    let before = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, "state-machine:Execution controls");
    fill(&mut a, &ctx, "execution-events", &go.0.to_string());
    click(&mut a, &ctx, "state-machine:execution:Prepare");
    for _ in 0..2 {
        click(&mut a, &ctx, "state-machine:execution:Step");
    }
    let report = a
        .state_machines
        .execution
        .live
        .as_ref()
        .unwrap()
        .report()
        .clone();
    assert_eq!(
        report.status,
        Status::Conflict {
            state: node,
            event: Some(go),
            transitions: vec![edge, conflict]
        }
    );
    assert_eq!(report.consumed_events, 0);
    fill(&mut a, &ctx, "execution-budget", "0");
    click(&mut a, &ctx, "state-machine:execution:Prepare");
    assert!(a.state_machines.execution.note.contains("BUDGET"));
    assert_eq!(
        a.state_machines.execution.live.as_ref().unwrap().report(),
        &report
    );
    fill(&mut a, &ctx, "execution-budget", "8");
    fill(&mut a, &ctx, "execution-events", &node.0.to_string());
    click(&mut a, &ctx, "state-machine:execution:Prepare");
    assert!(a.state_machines.execution.note.contains("EVENT_NOT_OWNED"));
    assert_eq!(
        a.state_machines.execution.live.as_ref().unwrap().report(),
        &report
    );
    fill(&mut a, &ctx, "execution-events", "");
    click(&mut a, &ctx, "state-machine:execution:Prepare");
    for _ in 0..2 {
        click(&mut a, &ctx, "state-machine:execution:Step");
    }
    assert_eq!(
        a.state_machines
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .status,
        Status::WaitingForEvent { state: node }
    );
    assert_eq!(a.session.editor.snapshot, before);
}

#[test]
fn rendered_state_execution_pins_other_machine_selection_until_cancel_without_rebinding_trace_or_raw()
 {
    let (_area, mut a, ctx) = app();
    let (m, _, _, _, go, _) = seeded(&mut a);
    let other = Id(a.session.editor.snapshot.next_id);
    let f = Form::Machine {
        id: None,
        system: "1".into(),
        name: "Other controller".into(),
    };
    let key = f.key();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sm_model(&stamp, sm::Action::Set(f)).unwrap();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sm_model(&stamp, sm::Action::Apply(key)).unwrap();
    click(&mut a, &ctx, "state-machine:Browse elements");
    click(&mut a, &ctx, &format!("state-machine:select:{}", m.0));
    click(&mut a, &ctx, "state-machine:Execution controls");
    fill(&mut a, &ctx, "execution-events", &go.0.to_string());
    click(&mut a, &ctx, "state-machine:execution:Prepare");
    let report = a
        .state_machines
        .execution
        .live
        .as_ref()
        .unwrap()
        .report()
        .clone();
    let native = a.session.editor.snapshot.clone();
    click(&mut a, &ctx, &format!("state-machine:select:{}", other.0));
    assert_eq!(a.state_machines.machine, Some(m));
    assert_eq!(a.sm_audit()["current"]["id"], m.0);
    assert!(
        a.state_machines
            .note
            .contains("before selecting another machine")
    );
    assert_eq!(
        a.state_machines.execution.live.as_ref().unwrap().report(),
        &report
    );
    assert_eq!(a.session.editor.snapshot, native);
    click(&mut a, &ctx, "state-machine:execution:Cancel");
    click(&mut a, &ctx, &format!("state-machine:select:{}", other.0));
    assert_eq!(a.state_machines.machine, Some(other));
    assert_eq!(
        a.state_machines
            .execution
            .live
            .as_ref()
            .unwrap()
            .report()
            .machine,
        m
    );
    assert_eq!(a.session.editor.snapshot, native);
}
