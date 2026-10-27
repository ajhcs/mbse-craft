//! Exact field focus from a sealed validation location; no model command is inferred.
use super::*;
use crate::assurance_ui::ValidationNavigation;
use mbse_assurance::validation::{PropertyField, Target, UsageField};

struct Pending {
    request: ValidationNavigation,
    target: Target,
    frames: u8,
}
#[derive(Default)]
pub(super) struct State {
    pending: Option<Pending>,
    last: Option<(ValidationNavigation, Target, egui::Id)>,
    note: String,
}
impl Workspace {
    pub(super) fn validation_frame_start(&mut self) {
        if self.validation_focus.pending.is_some()
            && self.context.input(|i| {
                i.events.iter().any(|e| {
                    matches!(
                        e,
                        egui::Event::PointerButton { pressed: true, .. }
                            | egui::Event::Key {
                                key: egui::Key::Tab,
                                pressed: true,
                                ..
                            }
                    )
                })
            })
        {
            self.validation_focus.pending = None;
            self.validation_focus.note =
                "Field focus canceled by new pointer/Tab input; source retained.".into();
        }
        if let Some(p) = &mut self.validation_focus.pending {
            p.frames += 1;
            if p.frames > 12 {
                self.validation_focus.pending = None;
                self.validation_focus.note="Target field could not be revealed; source retained. Reopen the suite to retry.".into();
            }
        }
    }
    pub(super) fn validation_raise_inspector(&self) -> bool {
        self.validation_focus
            .pending
            .as_ref()
            .is_some_and(|p| matches!(p.target, Target::NativeField(_)))
    }
    pub(super) fn handle_validation_navigation(&mut self) {
        let Some(request) = self.assurance.take_validation_navigation() else {
            return;
        };
        let target = match self
            .assurance
            .resolve_validation_navigation(&request, &self.session.editor)
        {
            Ok(t) => t,
            Err(error) => {
                self.validation_focus.note = error;
                return;
            }
        };
        if self.assurance.validation_navigation_blocked() {
            self.validation_focus.note="Finish independent register IO or preservation prompts before issue navigation; work retained.".into();
            return;
        }
        self.assurance.hide_for_validation_navigation();
        if self.dock_blocked() {
            self.assurance.open_window();
            self.validation_focus.note="Finish pending writes/gestures or close another auxiliary editor before issue navigation; all work retained.".into();
            return;
        }
        let result = (|| -> Result<(), String> {
            match &target {
                Target::Property { property, .. } => {
                    let form = mbse_model::types::forms(&self.session.editor.snapshot)?
                        .get(property)
                        .cloned()
                        .ok_or("VALIDATION_PROPERTY_FORM_MISSING")?;
                    self.type_panel.navigate_property(*property, form.owner);
                }
                Target::Usage { usage, .. } => {
                    let form = mbse_model::types::usage_form(&self.session.editor.snapshot)?
                        .ok_or("VALIDATION_USAGE_FORM_MISSING")?;
                    if form.id != *usage {
                        return Err("VALIDATION_USAGE_FORM_CHANGED".into());
                    }
                    self.type_panel
                        .navigate_usage(form.owner.parse::<u64>().ok().map(mbse_model::types::Id));
                }
                Target::NativeField(key) => {
                    if !self.session.editor.field_available(key) {
                        return Err("VALIDATION_NATIVE_FIELD_UNAVAILABLE".into());
                    }
                    self.apply_dock(
                        self.docking.revision,
                        vec![craft_ui::docking::Action::Activate {
                            panel: crate::docking::Panel::Inspector,
                        }],
                    )?;
                    self.select(key.record);
                    self.docking_focus = None;
                }
                Target::Requirement { .. } => {
                    return Err("VALIDATION_REQUIREMENT_ROUTE_BELONGS_TO_REGISTER".into());
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.validation_focus.pending = Some(Pending {
                    request,
                    target,
                    frames: 0,
                });
                self.validation_focus.note="Issue location selected; exact field focus is being revealed. Accepted data and raw drafts retained.".into();
            }
            Err(error) => {
                self.assurance.open_window();
                self.validation_focus.note = error;
            }
        }
    }
    pub(super) fn validation_focus_response(&mut self, target: &Target, response: &egui::Response) {
        let Some(mut pending) = self.validation_focus.pending.take() else {
            return;
        };
        let resolved = self
            .assurance
            .resolve_validation_navigation(&pending.request, &self.session.editor);
        if resolved.as_ref() != Ok(&pending.target) {
            self.validation_focus.note =
                resolved.err().unwrap_or("VALIDATION_TARGET_CHANGED".into());
            return;
        }
        if &pending.target == target {
            response.request_focus();
            response.scroll_to_me(Some(egui::Align::Center));
            self.validation_focus.last =
                Some((pending.request.clone(), target.clone(), response.id));
            self.validation_focus.note = "Exact issue field focused; source unchanged.".into();
            if pending.frames < 3 {
                self.validation_focus.pending = Some(pending);
            }
        } else {
            self.validation_focus.pending = Some(pending);
        }
    }
    pub(super) fn validation_navigation_audit(&self) -> serde_json::Value {
        serde_json::json!({"pending":self.validation_focus.pending.as_ref().map(|p|&p.target),"note":self.validation_focus.note,"last_focus":self.validation_focus.last.as_ref().map(|(request,target,id)|serde_json::json!({"target":target,"source_current":self.assurance.resolve_validation_navigation(request,&self.session.editor).is_ok(),"focused":self.context.memory(|m|m.focused())==Some(*id)}))})
    }
    pub(super) fn validation_property_focus(
        &mut self,
        id: mbse_model::types::Id,
        key: &str,
        response: &egui::Response,
    ) {
        let field = match key {
            "name" => PropertyField::Name,
            "value-type" => PropertyField::ValueType,
            "lower" => PropertyField::Lower,
            "upper" => PropertyField::Upper,
            _ => return,
        };
        self.validation_focus_response(
            &Target::Property {
                property: id,
                field,
            },
            response,
        );
    }
    pub(super) fn validation_usage_focus(
        &mut self,
        id: Option<mbse_model::types::Id>,
        key: &str,
        response: &egui::Response,
    ) {
        let field = match key {
            "owner" => UsageField::Owner,
            "target" => UsageField::Target,
            _ => return,
        };
        self.validation_focus_response(&Target::Usage { usage: id, field }, response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/parity/p26a/fixtures/native.kernspan"
        ))
        .unwrap();
        session.editor = mbse_model::stage1::Editor::new(
            serde_json::from_value(v["snapshot"].clone()).unwrap(),
            session.editor.freshness().instance_id,
        )
        .unwrap();
        session.editor.select(RecordId(1003)).unwrap();
        let e = &mut session.editor;
        e.command(
            &e.freshness(),
            Command::Typed(mbse_model::types::Action::SetForm {
                id: mbse_model::types::Id(1020),
                form: mbse_model::types::Form {
                    owner: mbse_model::types::Id(1007),
                    name: "instance gain".into(),
                    value_type: "999999".into(),
                    lower: "1".into(),
                    upper: "1".into(),
                },
            }),
        )
        .unwrap();
        e.command(
            &e.freshness(),
            Command::Typed(mbse_model::types::Action::SetUsageForm {
                form: mbse_model::types::UsageForm {
                    id: Some(mbse_model::types::Id(1011)),
                    name: "reference to assembly".into(),
                    owner: "1021".into(),
                    target: "1021".into(),
                    kind: mbse_model::types::UsageKind::Part,
                },
            }),
        )
        .unwrap();
        e.command(
            &e.freshness(),
            Command::EditText {
                key: RawKey {
                    record: RecordId(1003),
                    field: "calc.expression".into(),
                    item: None,
                },
                text: "x + x1".into(),
            },
        )
        .unwrap();
        let ctx = egui::Context::default();
        let mut app = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        app.assurance.open_window();
        (area, app, ctx)
    }
    fn render(a: &mut Workspace, ctx: &egui::Context, events: Vec<egui::Event>, handle: bool) {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(2200., 1600.),
                )),
                focused: true,
                events,
                ..Default::default()
            },
            |ui| {
                a.validation_frame_start();
                a.dock_surface(ui);
                a.type_dialog(ctx);
                a.assurance.show(ctx, &a.session.editor);
                a.audit.extend(a.assurance.audit());
                if handle {
                    a.handle_validation_navigation();
                }
            },
        )
        .drop_without_applying_deltas();
    }
    fn click(a: &mut Workspace, ctx: &egui::Context, key: &str, handle: bool, settle: bool) {
        for _ in 0..3 {
            render(a, ctx, vec![], handle);
        }
        for _ in 0..20 {
            let r = a.audit[key];
            if r[2] > r[0] && r[3] > r[1] && r[1] >= 0. && r[3] <= 1600. {
                break;
            }
            let delta = if r[3] < 200. { 240. } else { -240. };
            render(
                a,
                ctx,
                vec![
                    egui::Event::PointerMoved(egui::pos2((r[0] + r[2]) / 2., 650.)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        phase: egui::TouchPhase::Move,
                        delta: egui::vec2(0., delta),
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                handle,
            );
        }
        let r = a.audit[key];
        assert!(
            r[2] > r[0] && r[3] > r[1],
            "clipped test widget {key}: {r:?}"
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
                handle,
            );
        }
        if settle {
            for _ in 0..6 {
                render(a, ctx, vec![], handle);
            }
        }
    }
    fn ready(a: &mut Workspace, ctx: &egui::Context) {
        for _ in 0..3 {
            render(a, ctx, vec![], true);
        }
        click(a, ctx, "validation:section", true, true);
        click(a, ctx, "validation:Run", true, true);
        for _ in 0..100 {
            render(a, ctx, vec![], true);
            if a.assurance.authoring_audit(&a.session.editor, ctx)["validation"]["current"] == true
            {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("validation did not complete");
    }
    #[test]
    fn rendered_issue_navigation_focuses_property_usage_and_native_expression_without_mutation() {
        let (_area, mut a, ctx) = app();
        ready(&mut a, &ctx);
        let native = a.session.editor.snapshot.clone();
        let views = a.views.views.document().digest().unwrap();
        for (id, want) in [
            (
                "MBSE.TYPE.REFERENCE:property1020:ValueType",
                serde_json::json!({"Property":{"property":1020,"field":"ValueType"}}),
            ),
            (
                "MBSE.STRUCT.PART_CONTAINMENT:usage1011:Target",
                serde_json::json!({"Usage":{"usage":1011,"field":"Target"}}),
            ),
            (
                "MBSE.CALC.UNIT_COMPATIBILITY:record1003:calc.expression",
                serde_json::json!({"NativeField":{"record":1003,"field":"calc.expression","item":null}}),
            ),
        ] {
            a.assurance.open_window();
            for _ in 0..3 {
                render(&mut a, &ctx, vec![], true);
            }
            click(&mut a, &ctx, &format!("validation:issue:{id}"), true, true);
            let audit = a.validation_navigation_audit();
            assert_eq!(audit["last_focus"]["target"], want, "{audit}");
            assert_eq!(audit["last_focus"]["focused"], true, "{audit}");
            assert_eq!(audit["last_focus"]["source_current"], true);
            assert_eq!(a.session.editor.snapshot, native);
            assert_eq!(a.views.views.document().digest().unwrap(), views);
            if a.type_panel.open {
                click(&mut a, &ctx, "types:close", true, true);
            }
        }
    }
    #[test]
    fn queued_stale_location_preserves_selection_model_and_auxiliary_work() {
        let (_area, mut a, ctx) = app();
        ready(&mut a, &ctx);
        let key = "validation:issue:MBSE.TYPE.REFERENCE:property1020:ValueType";
        click(&mut a, &ctx, key, false, false);
        let selected = a.session.editor.selected;
        a.session
            .editor
            .snapshot
            .records
            .get_mut(&RecordId(1003))
            .unwrap()
            .name = "New accepted payload same counters".into();
        let expected = a.session.editor.snapshot.clone();
        a.handle_validation_navigation();
        assert_eq!(a.session.editor.snapshot, expected);
        assert_eq!(a.session.editor.selected, selected);
        assert!(a.assurance.is_visible());
        assert!(!a.type_panel.open);
        assert!(a.validation_focus.note.contains("STALE"));
    }
    #[test]
    fn new_tab_input_cancels_pending_automatic_field_focus_without_source_changes() {
        let (_area, mut a, ctx) = app();
        ready(&mut a, &ctx);
        click(
            &mut a,
            &ctx,
            "validation:issue:MBSE.TYPE.REFERENCE:property1020:ValueType",
            true,
            false,
        );
        assert!(a.validation_focus.pending.is_some());
        let before = a.session.editor.snapshot.clone();
        render(
            &mut a,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            true,
        );
        assert!(a.validation_focus.pending.is_none());
        assert_eq!(a.session.editor.snapshot, before);
    }
    #[test]
    fn another_auxiliary_editor_blocks_navigation_without_hiding_register_or_changing_source() {
        let (_area, mut a, ctx) = app();
        ready(&mut a, &ctx);
        let before = a.session.editor.snapshot.clone();
        let selected = a.session.editor.selected;
        a.settings_window = true;
        click(
            &mut a,
            &ctx,
            "validation:issue:MBSE.TYPE.REFERENCE:property1020:ValueType",
            true,
            false,
        );
        assert!(a.assurance.is_visible());
        assert!(!a.type_panel.open);
        assert_eq!(a.session.editor.snapshot, before);
        assert_eq!(a.session.editor.selected, selected);
        assert!(a.validation_focus.note.contains("all work retained"));
    }
}
