//! Dock bodies borrow the one live workspace; moves never reconstruct content.
use super::*;
use crate::docking::Panel;
use craft_ui::docking::{Action, DockArea, DockStyle, Placement, Zone};
impl Workspace {
    fn dock_id() -> egui::Id {
        egui::Id::new("mbse-owned-workspace-docking")
    }
    pub(super) fn docking_chrome_focused(&self) -> bool {
        DockArea::new(Self::dock_id()).chrome_has_focus(&self.context, &self.docking.layout)
    }
    pub(super) fn dock_blocked(&self) -> bool {self.matrix.open||self.viewpoints.open||self.dock_blocked_base()}
    pub(super) fn dock_blocked_without_matrix(&self) -> bool {self.viewpoints.open||self.dock_blocked_base()}
    pub(super) fn dock_blocked_without_viewpoint(&self) -> bool {self.matrix.open||self.dock_blocked_base()}
    pub(super) fn dock_blocked_without_activity(&self)->bool{self.matrix.open||self.viewpoints.open||self.dock_blocked_legacy_base()}
    fn dock_blocked_base(&self)->bool{self.activities.open||self.dock_blocked_legacy_base()}
    fn dock_blocked_legacy_base(&self) -> bool {
        if self.activity_gesture_active(){return true;}
        self.tag_panel.open || self.deletion_panel.open || self.refactor_panel.open || self.value_panel.open || self.connector_panel.open || self.port_panel.open || self.type_panel.open || self.files.open || self.files.pending() || self.views.occurrence_navigation_blocked(&self.session)
            || self.pending.is_some()
            || self.creation.is_some()
            || self.context_creation.is_some()
            || self.shortcuts.open
            || self.settings_window
            || self.views.inspection_visible()
            || self.assurance.is_visible()
            || self.exchange.is_visible()
            || self.recovery_window
            || self.search_open
            || self.impact_open
            || self.usages.is_some()
            || self.relation.is_some()
            || self.popout.is_some()
    }
    pub(super) fn apply_dock(
        &mut self,
        expected: u64,
        actions: Vec<Action<Panel>>,
    ) -> Result<(), String> {
        if self.dock_blocked() {
            return Err("DOCK_BUSY: finish the gesture/write or close the auxiliary editor first; all panels retained.".into());
        }
        let focus = self.focused_raw().and_then(|key| {
            self.context
                .memory(|m| m.focused())
                .map(|id| (id, key, self.session.editor.freshness(), 3))
        });
        let revision = self.docking.revision;
        self.docking.apply(expected, actions)?;
        if self.docking.revision != revision {
            self.persist_dock();
        }
        self.docking_focus = focus;
        Ok(())
    }
    fn persist_dock(&mut self) {
        match crate::perspectives::encode(&self.docking.layout) {
            Ok(text) => {
                if self.preferences.value.workspace_layout != text {
                    self.preferences.value.workspace_layout = text;
                    self.preferences.changed();
                }
            }
            Err(error) => self.docking.note = error,
        }
    }
    fn replace_dock(&mut self, next: craft_ui::docking::Layout<Panel>) -> Result<(), String> {
        if self.dock_blocked() {
            return Err("DOCK_BUSY: finish the gesture/write or close the auxiliary editor first; work retained".into());
        }
        crate::docking::WorkspaceDock::check(&next)?;
        crate::perspectives::encode(&next)?;
        if next == self.docking.layout {
            return Ok(());
        }
        let revision = self
            .docking
            .revision
            .checked_add(1)
            .ok_or("DOCK_REVISION_EXHAUSTED")?;
        self.docking_focus = self.focused_raw().and_then(|key| {
            self.context
                .memory(|m| m.focused())
                .map(|id| (id, key, self.session.editor.freshness(), 3))
        });
        self.docking.layout = next;
        self.docking.revision = revision;
        self.docking.last_actions = vec!["Restore workspace arrangement".into()];
        self.docking.note =
            "Workspace restored; native model, selection and incomplete text retained.".into();
        self.persist_dock();
        Ok(())
    }
    fn save_perspective(&mut self) -> Result<(), String> {
        if self.dock_blocked() {
            return Err("DOCK_BUSY: close the auxiliary editor first; work retained".into());
        }
        let name = crate::perspectives::name(&self.perspective_name)?.to_string();
        let rows = &self.preferences.value.workspace_perspectives;
        if rows.len() > crate::perspectives::MAX_PERSPECTIVES
            || (rows.len() == crate::perspectives::MAX_PERSPECTIVES && !rows.contains_key(&name))
        {
            return Err("At most eight named perspectives; existing entries retained".into());
        }
        let text = crate::perspectives::encode(&self.docking.layout)?;
        self.preferences
            .value
            .workspace_perspectives
            .insert(name, text);
        self.preferences.changed();
        self.docking.note = "Perspective saved to preferences; native model untouched.".into();
        Ok(())
    }
    fn restore_perspective(&mut self, name: &str) -> Result<(), String> {
        let text = self
            .preferences
            .value
            .workspace_perspectives
            .get(name)
            .ok_or("Perspective not found")?;
        let next = crate::perspectives::decode(text)?;
        self.replace_dock(next)
    }
    fn reset_dock(&mut self) -> Result<(), String> {
        self.replace_dock(crate::docking::WorkspaceDock::default().layout)?;
        // Empty is an explicit current/default marker in the shared settings store.
        self.preferences.value.workspace_layout = String::new();
        self.preferences.changed();
        self.docking.note =
            "Default panel arrangement restored; named perspectives retained.".into();
        Ok(())
    }
    pub(super) fn dock_surface(&mut self, root: &mut egui::Ui) {
        if root.ctx().input(|i| {
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
        }) {
            self.docking_focus = None;
        }
        let screen = root.available_rect_before_wrap();
        if let Ok(next) = crate::perspectives::reachable(&self.docking.layout, screen) {
            if next != self.docking.layout && !self.dock_blocked() {
                if let Err(error) = self.replace_dock(next) {
                    self.docking.note = error;
                }
            }
        }
        self.views.clear_toolbar_bounds();
        self.docking.body_ids.clear();
        let layout = self.docking.layout.clone();
        let revision = self.docking.revision;
        egui::CentralPanel::default().show(root, |ui| {
            let style = DockStyle::from_ui(ui);
            let area=DockArea::new(Self::dock_id());
            let area=if self.validation_raise_inspector(){area.raise_panel(Panel::Inspector)}else{area};
            let out = area.show(
                ui,
                &layout,
                &style,
                |p| p.label().into(),
                |p| p.permissions(),
                |ui, p| {
                    self.docking
                        .body_ids
                        .insert(p.key().into(), format!("{:?}", ui.id()));
                    let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
                    self.audit.insert(
                        format!("dock:body:{}", p.key()),
                        [rect.min.x, rect.min.y, rect.max.x, rect.max.y],
                    );
                    match p {
                        Panel::Browser => self.browser_body(ui),
                        Panel::Inspector => self.inspector_body(ui),
                        Panel::Canvas => self.canvas_body(ui),
                    }
                },
            );
            if let Some(error) = out.error {
                self.docking.note = format!("Dock renderer refused: {error:?}; work retained.");
            }
            if !out.actions.is_empty() {
                if let Err(error) = self.apply_dock(revision, out.actions) {
                    self.docking.note = error;
                }
            }
            for panel in Panel::ALL {
                if let Some(response) = ui.ctx().read_response(Self::dock_id().with(("tab", panel)))
                {
                    self.bound(format!("dock:tab:{}", panel.key()), &response);
                }
            }
        });
        self.restore_dock_focus(root.ctx());
    }
    pub(super) fn restore_dock_focus(&mut self, ctx: &egui::Context) {
        let Some((id, key, stamp, remaining)) = self.docking_focus.take() else {
            return;
        };
        if self.session.editor.freshness() != stamp
            || self.session.editor.selected != Some(key.record)
            || !self.session.editor.field_available(&key)
        {
            return;
        }
        if !self.docking.body_ids.contains_key("inspector") {
            return;
        }
        if remaining == 3 {
            self.docking_focus = Some((id, key, stamp, 2));
            return;
        }
        if ctx
            .read_response(id)
            .is_some_and(|r| r.enabled() && r.rect.intersects(r.interact_rect))
        {
            ctx.memory_mut(|m| m.request_focus(id));
        } else if remaining > 1 {
            self.docking_focus = Some((id, key, stamp, remaining - 1));
        }
    }
    pub(super) fn docking_dialog(&mut self, ctx: &egui::Context) {
        if !self.docking.window {
            return;
        }
        let mut open = true;
        let mut close = false;
        let mut action = None;
        let mut reset = false;
        let mut save = false;
        let mut restore = None;
        let revision = self.docking.revision;
        egui::Window::new("Panel layout").id(egui::Id::new("workspace-panel-controls")).open(&mut open).default_width(610.).show(ctx,|ui|{
            ui.label("Drag panel tabs to move, split or float. Floating panels stay inside this application viewport. Panel contents and unfinished native text remain owned by this workspace.");
            for panel in Panel::ALL{let anchor=if panel==Panel::Canvas{Panel::Browser}else{Panel::Canvas};let tab_anchor=if panel==Panel::Inspector{Panel::Browser}else{Panel::Inspector};
                ui.group(|ui|{ui.label(panel.label());ui.horizontal_wrapped(|ui|{
                    let r=ui.button("Float");self.bound(format!("dock:float:{}",panel.key()),&r);if r.clicked(){let screen=ctx.content_rect();action=Some(Action::Float{panel,rect:[screen.left()+60.,screen.top()+160.,screen.width().min(540.).max(200.),(screen.height()-240.).clamp(200.,850.)]});}
                    let r=ui.button(format!("Tab with {}",tab_anchor.label()));self.bound(format!("dock:tab-with:{}",panel.key()),&r);if r.clicked(){action=Some(Action::Move{panel,anchor:tab_anchor,placement:Placement::Tab{before:None}});}
                    let r=ui.button(format!("Left of {}",anchor.label()));self.bound(format!("dock:left:{}",panel.key()),&r);if r.clicked(){action=Some(Action::Move{panel,anchor,placement:Placement::Split(Zone::Left)});}
                });});
            }
            ui.separator();
            let r=ui.button("Reset default panels");self.bound("dock:reset",&r);reset=r.clicked();
            ui.horizontal(|ui|{ui.label("Perspective name");let r=ui.text_edit_singleline(&mut self.perspective_name);self.bound("dock:perspective-name",&r);let r=ui.button("Save perspective");self.bound("dock:save-perspective",&r);save=r.clicked();});
            let names=self.preferences.value.workspace_perspectives.keys().take(crate::perspectives::MAX_PERSPECTIVES).cloned().collect::<Vec<_>>();
            for name in names {let r=ui.button(format!("Restore {name}"));self.bound(format!("dock:restore:{name}"),&r);if r.clicked(){restore=Some(name);}}
            ui.label(&self.preferences.status);ui.label(&self.docking.note);let r=ui.button("Close controls");self.bound("dock:close-controls",&r);close=r.clicked();
        });
        self.docking.window = open && !close;
        let result = if reset {
            self.reset_dock()
        } else if save {
            self.save_perspective()
        } else if let Some(name) = restore {
            self.restore_perspective(&name)
        } else {
            Ok(())
        };
        if let Err(error) = result {
            self.docking.note = error;
        }
        if let Some(action) = action {
            if let Err(error) = self.apply_dock(revision, vec![action]) {
                self.docking.note = error;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let snap = mbse_model::stage1::Snapshot::from_fixture(
            session.editor.snapshot.document_id.clone(),
            &serde_json::from_str(include_str!(
                "../../mbse-model/fixtures/stage1/inspection.json"
            ))
            .unwrap(),
        )
        .unwrap();
        session.editor =
            mbse_model::stage1::Editor::new(snap, session.editor.freshness().instance_id).unwrap();
        session.editor.select(RecordId(301)).unwrap();
        let ctx = egui::Context::default();
        let mut app = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        app.command(Command::EditText {
            key: RawKey {
                record: RecordId(301),
                field: "criterion".into(),
                item: None,
            },
            text: "9e".into(),
        });
        (area, app, ctx)
    }
    fn retained(a: &Workspace) -> (String, String, Option<RecordId>, (usize, usize), String) {
        (
            a.session.editor.snapshot.digest(),
            a.views.views.document().digest().unwrap(),
            a.session.editor.selected,
            a.views.views.history_usage(),
            a.save_path.clone(),
        )
    }
    fn render(a: &mut Workspace, ctx: &egui::Context) {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(2200., 1600.),
                )),
                focused: true,
                ..Default::default()
            },
            |ui| a.dock_surface(ui),
        )
        .drop_without_applying_deltas();
    }
    #[test]
    fn panel_moves_never_reconstruct_or_mutate_native_views_raw_selection_or_save_path() {
        let (_area, mut a, _) = app();
        let before = retained(&a);
        for action in [
            Action::Float {
                panel: Panel::Inspector,
                rect: [20., 80., 400., 800.],
            },
            Action::Move {
                panel: Panel::Inspector,
                anchor: Panel::Browser,
                placement: Placement::Tab { before: None },
            },
            Action::Move {
                panel: Panel::Inspector,
                anchor: Panel::Canvas,
                placement: Placement::Split(Zone::Right),
            },
        ] {
            a.apply_dock(a.docking.revision, vec![action]).unwrap();
            assert_eq!(retained(&a), before);
            assert_eq!(
                a.session.editor.display_text(&RawKey {
                    record: RecordId(301),
                    field: "criterion".into(),
                    item: None
                }),
                "9e"
            );
        }
    }
    #[test]
    fn real_body_and_native_field_widget_ids_and_focus_survive_floating() {
        let (_area, mut a, ctx) = app();
        render(&mut a, &ctx);
        let body = a.docking.body_ids["inspector"].clone();
        let key = RawKey {
            record: RecordId(301),
            field: "criterion".into(),
            item: None,
        };
        let field = *a.model_fields.iter().find(|(_, k)| **k == key).unwrap().0;
        ctx.memory_mut(|m| m.request_focus(field));
        let before = retained(&a);
        a.apply_dock(
            a.docking.revision,
            vec![Action::Float {
                panel: Panel::Inspector,
                rect: [60., 80., 420., 1000.],
            }],
        )
        .unwrap();
        render(&mut a, &ctx);
        render(&mut a, &ctx); // Newly floated egui areas have one disabled measuring pass.
        assert_eq!(a.docking.body_ids["inspector"], body);
        assert_eq!(a.model_fields[&field], key);
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
        assert_eq!(retained(&a), before);
    }
    #[test]
    fn busy_or_modal_dock_refusal_retains_panel_layout_and_the_owning_native_ticket() {
        let (_area, mut a, _) = app();
        let before = retained(&a);
        let layout = a.docking.layout.clone();
        let job = a.session.begin_recovery().unwrap();
        let action = || Action::Float {
            panel: Panel::Inspector,
            rect: [20., 80., 400., 800.],
        };
        assert!(a
            .apply_dock(0, vec![action()])
            .unwrap_err()
            .contains("BUSY"));
        assert!(a.session.pending());
        assert_eq!(a.docking.layout, layout);
        assert_eq!(retained(&a), before);
        assert!(a
            .session
            .failed_start(job.start_token(), "Unstarted P11 fixture cleanup".into()));
        a.shortcuts.open = true;
        assert!(a.apply_dock(0, vec![action()]).is_err());
        assert_eq!(a.docking.layout, layout);
    }
    #[test]
    fn invalid_pointer_targets_or_canceled_proposals_have_no_mutation_authority() {
        use craft_ui::docking::{resolve_drop, DropTarget};
        let (_area, a, _) = app();
        let before = retained(&a);
        let layout = a.docking.layout.clone();
        let target = DropTarget {
            anchor: Panel::Browser,
            rect: egui::Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(200., 200.)),
            tab_strip: None,
            tabs: vec![],
            allow_splits: false,
            accepts_tabs: false,
        };
        assert!(resolve_drop(&Panel::Inspector, egui::pos2(90., 90.), &[target]).is_none());
        assert!(resolve_drop::<Panel>(&Panel::Inspector, egui::pos2(f32::NAN, 0.), &[]).is_none());
        assert_eq!(a.docking.layout, layout);
        assert_eq!(retained(&a), before);
    }
    #[test]
    fn tabbing_away_canvas_clears_hidden_control_audit_and_retains_native_content() {
        let (_area, mut a, ctx) = app();
        render(&mut a, &ctx);
        assert!(a.views.toolbar_bounds().count() > 0);
        let before = retained(&a);
        a.apply_dock(
            a.docking.revision,
            vec![
                Action::Move {
                    panel: Panel::Canvas,
                    anchor: Panel::Inspector,
                    placement: Placement::Tab { before: None },
                },
                Action::Activate {
                    panel: Panel::Inspector,
                },
            ],
        )
        .unwrap();
        render(&mut a, &ctx);
        assert_eq!(a.views.toolbar_bounds().count(), 0);
        assert_eq!(retained(&a), before);
        assert_eq!(a.docking.layout.panels().len(), 3);
    }
    #[test]
    fn named_restore_reset_and_failed_restore_preserve_live_raw_and_native_state() {
        let (_area, mut a, _) = app();
        let before = retained(&a);
        a.apply_dock(
            0,
            vec![Action::Float {
                panel: Panel::Inspector,
                rect: [80., 180., 400., 800.],
            }],
        )
        .unwrap();
        let saved = a.docking.layout.clone();
        a.perspective_name = " Review ".into();
        a.save_perspective().unwrap();
        assert!(a
            .preferences
            .value
            .workspace_perspectives
            .contains_key("Review"));
        a.reset_dock().unwrap();
        assert_eq!(
            a.docking.layout,
            crate::docking::WorkspaceDock::default().layout
        );
        assert!(a.preferences.value.workspace_layout.is_empty());
        a.restore_perspective("Review").unwrap();
        assert_eq!(a.docking.layout, saved);
        assert_eq!(retained(&a), before);
        let revision = a.docking.revision;
        a.preferences
            .value
            .workspace_perspectives
            .insert("Broken".into(), "{broken".into());
        assert!(a.restore_perspective("Broken").is_err());
        assert_eq!(a.docking.layout, saved);
        assert_eq!(a.docking.revision, revision);
        assert_eq!(
            a.preferences.value.workspace_perspectives["Broken"],
            "{broken"
        );
        assert_eq!(
            a.session.editor.display_text(&RawKey {
                record: RecordId(301),
                field: "criterion".into(),
                item: None
            }),
            "9e"
        );
    }
    #[test]
    fn perspective_actions_refuse_busy_and_budget_without_partial_settings_or_native_mutation() {
        let (_area, mut a, _) = app();
        let before = retained(&a);
        let layout = a.docking.layout.clone();
        for i in 0..crate::perspectives::MAX_PERSPECTIVES {
            a.preferences.value.workspace_perspectives.insert(
                format!("Perspective{i}"),
                crate::perspectives::encode(&layout).unwrap(),
            );
        }
        let prefs = a.preferences.value.clone();
        a.perspective_name = "Ninth".into();
        assert!(a.save_perspective().is_err());
        assert_eq!(a.preferences.value, prefs);
        a.shortcuts.open = true;
        assert!(a.reset_dock().is_err());
        assert!(a.restore_perspective("Perspective0").is_err());
        assert_eq!(a.preferences.value, prefs);
        assert_eq!(a.docking.layout, layout);
        assert_eq!(retained(&a), before);
    }
    #[test]
    fn corrupted_settings_open_with_safe_default_and_exact_native_state_without_rewrite() {
        let (area, a, ctx) = app();
        let before = retained(&a);
        drop(a);
        let config = area.path().join("config");
        std::fs::create_dir(&config).unwrap();
        let file = config.join("config.toml");
        let bytes="schema_version = 1\nworkspace_layout = '{broken'\nperiodic_recovery = false\nrecovery_interval_seconds = 42\n";
        std::fs::write(&file, bytes).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let snapshot = mbse_model::stage1::Snapshot::from_fixture(
            session.editor.snapshot.document_id.clone(),
            &serde_json::from_str(include_str!(
                "../../mbse-model/fixtures/stage1/inspection.json"
            ))
            .unwrap(),
        )
        .unwrap();
        session.editor =
            mbse_model::stage1::Editor::new(snapshot, session.editor.freshness().instance_id)
                .unwrap();
        session.editor.select(RecordId(301)).unwrap();
        let app = Workspace::new(
            session,
            None,
            Some(config),
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        assert_eq!(
            app.docking.layout,
            crate::docking::WorkspaceDock::default().layout
        );
        assert!(app.docking.note.contains("refused"));
        assert!(app.preferences.value.workspace_layout.contains("broken"));
        assert_eq!(std::fs::read_to_string(file).unwrap(), bytes);
        assert_eq!(app.session.editor.selected, before.2);
    }
}
