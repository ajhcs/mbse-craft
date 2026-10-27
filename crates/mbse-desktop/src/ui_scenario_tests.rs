use super::*;
fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p33b/verification/native-02/run/state-execution-cold-copy.kernspan"
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
        crate::command_catalog::Id::Scenarios,
        crate::command_catalog::Surface::Menu,
        ShortcutTarget::Model,
    );
    (area, a, ctx)
}
fn render(a: &mut Workspace, ctx: &egui::Context, events: Vec<egui::Event>) {
    a.audit.clear();
    a.activity_frame_start();
    a.sc_frame_start();
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
            a.sc_dialog(ctx);
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
                k.starts_with("scenario:") && v[3] - v[1] >= 12. && v[1] >= 0. && v[3] <= 1600.
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
    click(a, ctx, &format!("scenario:field:{key}"));
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

fn submit(a: &mut Workspace, ctx: &egui::Context, kind: &str, fields: &[(&str, &str)]) -> Id {
    click(a, ctx, &format!("scenario:new:{kind}"));
    for (k, v) in fields {
        fill(a, ctx, k, v);
    }
    let id = Id(a.session.editor.snapshot.next_id);
    click(a, ctx, "scenario:Apply");
    id
}
fn seed(a: &mut Workspace) -> (Id, Id, Id, Id, Id) {
    let root = Id(a.session.editor.snapshot.next_id);
    let l = Id(root.0 + 1);
    let r = Id(root.0 + 2);
    let call = Id(root.0 + 3);
    let reply = Id(root.0 + 4);
    for f in [
        Form::Scenario {
            id: None,
            system: "1".into(),
            name: "Review request".into(),
        },
        Form::Lifeline {
            scenario: root,
            id: None,
            name: "Sender".into(),
            participant: "1004".into(),
            expected: ParticipantKind::Component,
        },
        Form::Lifeline {
            scenario: root,
            id: None,
            name: "Receiver".into(),
            participant: "1009".into(),
            expected: ParticipantKind::Usage(ty::UsageKind::Part),
        },
        Form::Message {
            scenario: root,
            id: None,
            name: "request".into(),
            source: l.0.to_string(),
            target: r.0.to_string(),
            order: "1".into(),
            operand: "".into(),
            message_kind: MessageKind::Synchronous,
            reply_to: "".into(),
        },
        Form::Message {
            scenario: root,
            id: None,
            name: "response".into(),
            source: r.0.to_string(),
            target: l.0.to_string(),
            order: "2".into(),
            operand: "".into(),
            message_kind: MessageKind::Reply,
            reply_to: call.0.to_string(),
        },
    ] {
        let stamp = a.views.views.stamp(&a.session.editor);
        a.sc_model(&stamp, sc::Action::Set(f.clone())).unwrap();
        let stamp = a.views.views.stamp(&a.session.editor);
        a.sc_model(&stamp, sc::Action::Apply(f.key())).unwrap();
    }
    a.scenarios.scenario = Some(root);
    (root, l, r, call, reply)
}
#[test]
fn rendered_scenario_forms_author_exact_participants_calls_replies_alternative_operands_and_documentary_guard()
 {
    let (_d, mut a, ctx) = app();
    let original = a.session.editor.snapshot.records.clone();
    let root = submit(&mut a, &ctx, "Scenario", &[("name", "Review request")]);
    let l = submit(
        &mut a,
        &ctx,
        "Lifeline",
        &[("name", "Sender"), ("participant", "1004")],
    );
    click(&mut a, &ctx, "scenario:new:Lifeline");
    fill(&mut a, &ctx, "name", "Receiver");
    fill(&mut a, &ctx, "participant", "1009");
    click(&mut a, &ctx, "scenario:participant-kind:Usage(Part)");
    let r = Id(a.session.editor.snapshot.next_id);
    click(&mut a, &ctx, "scenario:Apply");
    let call = submit(
        &mut a,
        &ctx,
        "Message",
        &[
            ("name", "request"),
            ("source", &l.0.to_string()),
            ("target", &r.0.to_string()),
            ("order", "1"),
        ],
    );
    click(&mut a, &ctx, "scenario:new:Message");
    for (k, v) in [
        ("name", "response"),
        ("source", &r.0.to_string()),
        ("target", &l.0.to_string()),
        ("order", "2"),
        ("reply_to", &call.0.to_string()),
    ] {
        fill(&mut a, &ctx, k, v);
    }
    click(&mut a, &ctx, "scenario:message-kind:Reply");
    click(&mut a, &ctx, "scenario:Apply");
    let alt = submit(
        &mut a,
        &ctx,
        "Alternative",
        &[("name", "Review decision"), ("order", "3")],
    );
    let op = submit(
        &mut a,
        &ctx,
        "Operand",
        &[
            ("name", "Accept"),
            ("alternative", &alt.0.to_string()),
            ("order", "1"),
            ("guard", "temperature > 50 C\nreviewer agrees"),
        ],
    );
    let store = ty::load(&a.session.editor.snapshot).unwrap();
    let s = &store.scenarios[&root];
    assert_eq!(s.lifelines[&r].participant, Id(1009));
    assert_eq!(s.messages.len(), 2);
    assert_eq!(
        s.messages
            .values()
            .find(|m| m.kind == MessageKind::Reply)
            .unwrap()
            .reply_to,
        Some(call)
    );
    assert_eq!(
        s.alternatives[&alt].operands[&op].guard,
        "temperature > 50 C\nreviewer agrees"
    );
    assert_eq!(a.session.editor.snapshot.records, original);
}
#[test]
fn rendered_scenario_invalid_proposals_and_newer_raw_survive_model_history_without_rebinding() {
    let (_d, mut a, ctx) = app();
    let (root, l, r, call, _) = seed(&mut a);
    let stamp = a.views.views.stamp(&a.session.editor);
    let f = super::accepted_form(
        &ty::load(&a.session.editor.snapshot).unwrap().scenarios[&root],
        call,
    )
    .unwrap();
    a.sc_pick(f, Some(&stamp)).unwrap();
    fill(&mut a, &ctx, "target", "999999");
    let accepted = ty::load(&a.session.editor.snapshot)
        .unwrap()
        .scenarios
        .clone();
    click(&mut a, &ctx, "scenario:Apply");
    assert!(a.scenarios.note.contains("SCENARIO"));
    assert_eq!(
        ty::load(&a.session.editor.snapshot).unwrap().scenarios,
        accepted
    );
    let raw = sc::forms(&a.session.editor.snapshot).unwrap();
    click(&mut a, &ctx, "scenario:Model Undo");
    assert_eq!(sc::forms(&a.session.editor.snapshot).unwrap(), raw);
    click(&mut a, &ctx, "scenario:Model Redo");
    assert_eq!(
        ty::load(&a.session.editor.snapshot).unwrap().scenarios,
        accepted
    );
    assert_eq!(sc::forms(&a.session.editor.snapshot).unwrap(), raw);
    let mut f = super::accepted_form(&accepted[&root], l).unwrap();
    if let Form::Lifeline { participant, .. } = &mut f {
        *participant = r.0.to_string();
    }
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sc_pick(f.clone(), Some(&stamp)).unwrap();
    a.sc_set_form(&stamp, f.clone()).unwrap();
    let stamp = a.views.views.stamp(&a.session.editor);
    assert!(a.sc_apply(&stamp, &f).is_err());
    assert_eq!(
        ty::load(&a.session.editor.snapshot).unwrap().scenarios,
        accepted
    );
}
#[test]
fn rendered_sequence_rows_arrows_two_views_layout_history_pin_hide_fit_preserve_entire_snapshot() {
    let (_d, mut a, ctx) = app();
    let (root, _, _, call, reply) = seed(&mut a);
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sc_create_diagram(&stamp, root).unwrap();
    let one = a.views.active;
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sc_create_diagram(&stamp, root).unwrap();
    let two = a.views.active;
    let snapshot = a.session.editor.snapshot.clone();
    a.scenarios.open = false;
    settle(&mut a, &ctx);
    let rendered = a.scenarios.rendered.as_ref().unwrap();
    assert_eq!(rendered.messages, vec![call, reply]);
    assert_eq!(rendered.endpoints.len(), 2);
    assert!(rendered.endpoints[0].1[1] < rendered.endpoints[1].1[1]);
    let occ = rendered.occurrences[0].0;
    a.scenarios.selected = Some((two, occ));
    click(&mut a, &ctx, "scenario-canvas:Move right");
    let moved = a.views.views.document().scenario_diagrams[&two].occurrences[&occ]
        .geometry
        .clone();
    assert_eq!(moved.x, 65.);
    assert!(
        a.views.views.document().scenario_diagrams[&one]
            .occurrences
            .values()
            .all(|o| o.geometry.x != 65.)
    );
    click(&mut a, &ctx, "scenario-canvas:Diagram Undo");
    assert_eq!(
        a.views.views.document().scenario_diagrams[&two].occurrences[&occ]
            .geometry
            .x,
        25.
    );
    click(&mut a, &ctx, "scenario-canvas:Diagram Redo");
    click(&mut a, &ctx, "scenario-canvas:Toggle pin");
    let stamp = a.views.views.stamp(&a.session.editor);
    let mut g = moved;
    g.x += 5.;
    assert!(
        a.sc_layout(
            &stamp,
            nv::scenarios::Layout::Move {
                view: two,
                occurrence: occ,
                geometry: g
            }
        )
        .is_err()
    );
    click(&mut a, &ctx, "scenario-canvas:Toggle visibility");
    assert!(a.scenarios.rendered.as_ref().unwrap().messages.is_empty());
    click(&mut a, &ctx, "scenario-canvas:Toggle visibility");
    assert_eq!(a.scenarios.rendered.as_ref().unwrap().messages.len(), 2);
    click(&mut a, &ctx, "scenario-canvas:Toggle visibility");
    a.scenarios.selected = None;
    click(&mut a, &ctx, "scenario-canvas:Browse occurrences");
    click(&mut a, &ctx, &format!("scenario-canvas:pick:{}", occ.get()));
    click(&mut a, &ctx, "scenario-canvas:Toggle visibility");
    assert!(a.views.views.document().scenario_diagrams[&two].occurrences[&occ].visible);
    click(&mut a, &ctx, "camera:Fit");
    assert!(a.views.camera_audit()["zoom"].as_f64().unwrap() < 1.);
    assert_eq!(a.views.views.document().views[&one].camera.zoom, 1.);
    assert_eq!(a.session.editor.snapshot, snapshot);
}
#[test]
fn scenario_failed_utf8_admission_and_foreign_owner_keep_new_text_until_explicit_discard() {
    let (_d, mut a, ctx) = app();
    fill(&mut a, &ctx, "name", &"é".repeat(2500));
    assert!(a.sc_pending());
    assert!(a.sc_close_blocked());
    let f = a.scenarios.pending.as_ref().unwrap().form.clone();
    assert!(matches!(f,Form::Scenario{name,..}if name=="é".repeat(2500)));
    a.scenarios.pending.as_mut().unwrap().owner.instance = "foreign".into();
    settle(&mut a, &ctx);
    assert!(!a.sc_audit()["owner_current"].as_bool().unwrap());
    click(&mut a, &ctx, "scenario:Discard new text");
    assert!(!a.sc_pending());
    assert!(sc::forms(&a.session.editor.snapshot).unwrap().is_empty());
    assert!(
        ty::load(&a.session.editor.snapshot)
            .unwrap()
            .scenarios
            .is_empty()
    );
}
#[test]
fn scenario_source_and_view_stamps_refuse_same_counter_changes_and_guard_other_actors() {
    let (_d, mut a, _ctx) = app();
    let (root, _, _, _, _) = seed(&mut a);
    let stamp = a.views.views.stamp(&a.session.editor);
    let f = super::accepted_form(
        &ty::load(&a.session.editor.snapshot).unwrap().scenarios[&root],
        root,
    )
    .unwrap();
    a.session
        .editor
        .snapshot
        .records
        .get_mut(&RecordId(1))
        .unwrap()
        .name = "Different source same counters".into();
    assert!(a.sc_pick(f, Some(&stamp)).is_err());
    let snapshot = a.session.editor.snapshot.clone();
    a.dispatch_workspace(
        crate::command_catalog::Id::New,
        crate::command_catalog::Surface::Menu,
        ShortcutTarget::Model,
    );
    assert_eq!(a.session.editor.snapshot, snapshot);
    assert!(a.dock_blocked());
    a.scenarios.open = false;
    assert!(!a.dock_blocked_without_scenario());
}
#[test]
fn rendered_sequence_missing_participant_keeps_headers_rows_geometry_and_suppresses_affected_arrows()
 {
    let (_d, mut a, ctx) = app();
    let (root, _, _, _, _) = seed(&mut a);
    let unused = Id(a.session.editor.snapshot.next_id);
    let mut store = ty::load(&a.session.editor.snapshot).unwrap();
    let mut definition = store.definitions[&Id(1004)].clone();
    definition.id = unused;
    definition.name = "Independent scenario participant".into();
    store.definitions.insert(unused, definition);
    let first = *store.scenarios[&root].lifelines.keys().next().unwrap();
    store
        .scenarios
        .get_mut(&root)
        .unwrap()
        .lifelines
        .get_mut(&first)
        .unwrap()
        .participant = unused;
    a.session.editor.snapshot.next_id += 1;
    a.session
        .editor
        .snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert(ty::NAMESPACE.into(), serde_json::to_value(store).unwrap());
    a.session.editor.snapshot.validate().unwrap();
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sc_create_diagram(&stamp, root).unwrap();
    a.scenarios.open = false;
    settle(&mut a, &ctx);
    let before = a.scenarios.rendered.as_ref().unwrap().occurrences.clone();
    let rows = a.scenarios.rendered.as_ref().unwrap().rows.clone();
    let key = ty::NAMESPACE;
    let mut s = ty::load(&a.session.editor.snapshot).unwrap();
    s.definitions.get_mut(&unused).unwrap().kind = ty::Kind::Value(ty::Scalar::Boolean);
    a.session
        .editor
        .snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert(key.into(), serde_json::to_value(s).unwrap());
    settle(&mut a, &ctx);
    let r = a.scenarios.rendered.as_ref().unwrap();
    assert_eq!(r.occurrences, before);
    assert_eq!(r.rows, rows);
    assert!(r.messages.is_empty());
    assert!(
        a.sc_audit()["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "SCENARIO_PARTICIPANT_UNRESOLVED")
    );
}
#[test]
fn rendered_sequence_drag_publishes_one_layout_entry_and_single_undo_preserves_entire_native() {
    let (_area, mut a, ctx) = app();
    let (m, node, _, _, _) = seed(&mut a);
    let stamp = a.views.views.stamp(&a.session.editor);
    a.sc_create_diagram(&stamp, m).unwrap();
    let view = a.views.active;
    a.scenarios.open = false;
    settle(&mut a, &ctx);
    let id = a.views.views.document().scenario_diagrams[&view]
        .occurrences
        .values()
        .find(|o| o.element == node)
        .unwrap()
        .id;
    let geometry = a.views.views.document().scenario_diagrams[&view].occurrences[&id]
        .geometry
        .clone();
    let native = a.session.editor.snapshot.clone();
    let entries = a.views.views.history_usage().0;
    let r = rect(&a, &format!("scenario-occurrence:{}", id.get()));
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
    assert!(a.sc_gesture_active());
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
    assert!(!a.sc_gesture_active());
    let moved = &a.views.views.document().scenario_diagrams[&view].occurrences[&id].geometry;
    assert!((moved.x - geometry.x - 60.).abs() < 0.01 && (moved.y - geometry.y - 30.).abs() < 0.01);
    assert_eq!(a.views.views.history_usage().0, entries + 1);
    click(&mut a, &ctx, "scenario-canvas:Diagram Undo");
    assert_eq!(
        a.views.views.document().scenario_diagrams[&view].occurrences[&id].geometry,
        geometry
    );
    assert_eq!(a.session.editor.snapshot, native);
}
