//! One workspace dispatch boundary shared by menus, toolbars and fixed shortcuts.
use super::*;
use crate::command_catalog::{self as catalog, Id, Surface};

impl Workspace {
    pub(super) fn history_scope(&self) -> ShortcutTarget {
        if self.state_machines.open || self.sm_gesture_active() || self.activities.open || self.activity_gesture_active() || self.viewpoints.open || self.matrix.open || self.tag_panel.open || self.deletion_panel.open || self.refactor_panel.open || self.value_panel.open || self.connector_panel.open || self.port_panel.open || self.type_panel.open || self.files.open || self.files.pending() || self.views.inspection_visible()
            || self.assurance.is_visible()
            || self.exchange.is_visible()
            || self.shortcuts.open
            || self.docking.window
            || self.docking_chrome_focused()
        {
            return ShortcutTarget::Widget;
        }
        shortcut_policy(
            self.focused_raw(),
            self.context.text_edit_focused()
                || self.creation.is_some()
                || self.context_creation.is_some()
                || self.settings_window
                || self.recovery_window
                || self.shortcuts.open
                || self.pending.is_some(),
            false,
        )
    }
    /// Cheap hints only: no filesystem reads or output authorization in a frame.
    /// Handlers still validate paths, files, revisions and pending tickets on use.
    pub(super) fn command_reason(&self, id: Id, scope: &ShortcutTarget) -> Option<String> {
        let editing = id.is_history() || id.is_layout_history();
        if self.sm_gesture_active(){return Some("Finish or cancel the state-machine diagram gesture before other commands.".into());}
        if self.state_machines.open && id!=Id::StateMachines{return Some("Close the state-machine editor before other workspace commands; proposals retained.".into());}
        if id==Id::StateMachines && self.dock_blocked_without_state_machine(){return Some("Finish pending work or close the other editor before opening state machines.".into());}
        if self.activity_gesture_active(){return Some("Finish or cancel the activity diagram gesture before other commands.".into());}
        if self.activities.open && id!=Id::Activities{return Some("Close the activity editor before other workspace commands; proposals retained.".into());}
        if id==Id::Activities && self.dock_blocked_without_activity(){return Some("Finish pending work or close the other editor before opening activities.".into());}
        if self.viewpoints.open && id!=Id::Viewpoints{return Some("Close the viewpoint editor before other workspace commands; proposals retained.".into());}
        if id==Id::Viewpoints && self.dock_blocked_without_viewpoint(){return Some("Finish pending work or close the auxiliary editor before opening viewpoints.".into());}
        if self.matrix.open && id!=Id::Matrix{return Some("Relation matrix: close it before other workspace commands; model work retained.".into());}
        if id==Id::Matrix && self.dock_blocked_without_matrix(){return Some("Finish pending writes/gestures or close the auxiliary editor before opening the matrix.".into());}
        if self.tag_panel.open && id!=Id::ProfileTags{return Some("Profiles: close the editor before other commands; unfinished form retained.".into());}
        if id==Id::ProfileTags && self.views.occurrence_navigation_blocked(&self.session){return Some("Finish pending writes and gestures before profile editing.".into());}
        if self.deletion_panel.open && id!=Id::DeleteModel{return Some("Deletion: close the preview before other commands; unfinished form retained.".into());}
        if id==Id::DeleteModel && self.views.occurrence_navigation_blocked(&self.session){return Some("Finish pending writes and gestures before deletion review.".into());}
        if self.refactor_panel.open && id!=Id::Refactor{return Some("Clone/move: close the editor before other commands; unfinished form retained.".into());}
        if id==Id::Refactor && self.views.occurrence_navigation_blocked(&self.session){return Some("Finish pending writes and gestures before cloning or moving.".into());}
        if self.value_panel.open && id!=Id::Values{return Some("Values: close the editor before other commands; unfinished value text retained.".into());}
        if id==Id::Values && self.views.occurrence_navigation_blocked(&self.session){return Some("Finish pending writes and gestures before opening values.".into());}
        if self.connector_panel.open && id!=Id::Connectors{return Some("Connectors: close the editor before other commands; unfinished forms retained.".into());}
        if id==Id::Connectors && self.views.occurrence_navigation_blocked(&self.session){return Some("Finish pending writes and gestures before opening connectors.".into());}
        if self.port_panel.open && id!=Id::Ports{return Some("Ports and interfaces: close the editor to use other commands; unfinished forms retained.".into());}
        if id==Id::Ports && self.views.occurrence_navigation_blocked(&self.session){return Some("Finish pending native/views writes and gestures before opening ports.".into());}
        if self.type_panel.open && id!=Id::Types{return Some("Types and properties: close the editor to use native file/history commands; unfinished property text is retained.".into());}
        if self.files.open || self.files.pending(){return Some("Finish or cancel the file operation first; native work retained.".into());}
        if id==Id::Files && self.dock_blocked(){return Some("Finish pending writes/gestures or close the auxiliary editor before choosing a file.".into());}
        if self.shortcuts.open
            && (editing
                || matches!(
                    id,
                    Id::New
                        | Id::Open
                        | Id::Recover
                        | Id::Import
                        | Id::Save
                        | Id::Recovery
                        | Id::Export
                        | Id::Quit
                ))
        {
            return Some("Keyboard shortcuts: close the editor to use native file/history commands. Unapplied shortcut text is retained.".into());
        }
        if self.views.inspection_visible()
            && matches!(
                id,
                Id::Save
                    | Id::Recovery
                    | Id::Export
                    | Id::New
                    | Id::Open
                    | Id::Recover
                    | Id::Import
                    | Id::Quit
            )
        {
            return Some("Read-only source inspection: close it to use model file commands; mapping Apply remains explicit.".into());
        }
        if (self.assurance.is_visible() || self.exchange.is_visible())
            && (editing
                || matches!(
                    id,
                    Id::New
                        | Id::Open
                        | Id::Recover
                        | Id::Import
                        | Id::Save
                        | Id::Recovery
                        | Id::Export
                        | Id::Quit
                ))
        {
            return Some("Finish or close the auxiliary editor; its Save and text Undo use that editor's scope.".into());
        }
        if self.pending.is_some() && id != Id::Quit {
            return Some(
                "Resolve the preservation dialog with Save, recovery or Cancel first.".into(),
            );
        }
        if self.views.occurrence_navigation_blocked(&self.session)
            && (editing || matches!(id, Id::Save | Id::Recovery | Id::Export))
        {
            return Some("Finish the gesture or pending native/views write first.".into());
        }
        match id {
            Id::Open | Id::Recover | Id::Import | Id::Export if self.path.trim().is_empty() => {
                Some(
                    "Enter an explicit source or export path in Open / import / recover / export."
                        .into(),
                )
            }
            Id::Save if self.save_path.trim().is_empty() => {
                Some("Enter a native save destination before saving.".into())
            }
            Id::Usages | Id::Impact
                if !self
                    .session
                    .editor
                    .selected
                    .is_some_and(|id| self.session.editor.snapshot.records.contains_key(&id)) =>
            {
                Some("Select an available native record first.".into())
            }
            Id::Undo | Id::Redo => {
                let redo = id == Id::Redo;
                match scope {
                    ShortcutTarget::Widget => Some(
                        "Text widget owns Undo/Redo; leave that field to address model history."
                            .into(),
                    ),
                    ShortcutTarget::Field(key)
                        if self.session.editor.selected != Some(key.record)
                            || !self.session.editor.field_available(key) =>
                    {
                        Some(
                            "The focused native field changed; focus it again before history."
                                .into(),
                        )
                    }
                    ShortcutTarget::Field(key)
                        if !self.session.editor.can_text_history(key, redo) =>
                    {
                        Some("No field text history available.".into())
                    }
                    ShortcutTarget::Model
                        if !(if redo {
                            self.session.editor.can_redo()
                        } else {
                            self.session.editor.can_undo()
                        }) =>
                    {
                        Some("No model history available.".into())
                    }
                    _ => None,
                }
            }
            Id::LayoutUndo | Id::LayoutRedo if self.views.inspection_visible() => {
                Some("Close presentation inspection before layout history.".into())
            }
            Id::LayoutUndo | Id::LayoutRedo
                if !self.views.views.can_layout_history(id == Id::LayoutRedo) =>
            {
                Some("No layout history available in this direction.".into())
            }
            _ => None,
        }
    }
    pub(super) fn dispatch_workspace(&mut self, id: Id, surface: Surface, scope: ShortcutTarget) {
        let reason = self.command_reason(id, &scope);
        self.last_workspace_command = Some(
            serde_json::json!({"id":id.spec().key,"surface":surface.name(),"scope":match &scope{ShortcutTarget::Field(_)=>"field",ShortcutTarget::Model=>"model",ShortcutTarget::Widget=>"widget"},"refused":reason.is_some(),"reason":reason}),
        );
        if let Some(reason) = reason {
            self.result(Err(reason));
            return;
        }
        match id {
            Id::Activities=>{self.activity_open();},
            Id::StateMachines=>{self.sm_open();},
            Id::Viewpoints=>{self.viewpoints.open=true;},
            Id::Matrix=>{self.matrix.open=true;self.matrix_refresh();},
            Id::ProfileTags=>{self.tag_panel.open=true;},
            Id::DeleteModel=>{self.deletion_panel.open=true;},
            Id::Refactor=>{self.refactor_panel.open=true;},
            Id::Values=>{self.value_panel.open=true;},
            Id::Connectors=>{self.connector_panel.open=true;},
            Id::Ports=>{self.port_panel.open=true;},
            Id::Types=>{self.type_panel.open=true;},
            Id::Files => self.open_file_picker(),
            Id::New => self.request_load(Load::New),
            Id::Open => self.request_load(Load::Open(PathBuf::from(self.path.trim()))),
            Id::Recover => self.request_load(Load::Recover(PathBuf::from(self.path.trim()))),
            Id::Import => self.request_load(Load::Import(PathBuf::from(self.path.trim()))),
            Id::Save => {
                self.save();
            }
            Id::Recovery => {
                self.recovery();
            }
            Id::Export => {
                let result = self
                    .session
                    .begin_export_sysml(&PathBuf::from(self.path.trim()))
                    .and_then(|job| self.session.complete_export(stage1_io::execute_export(job)))
                    .and_then(|ok| {
                        if ok {
                            Ok("Exported restricted SysML copy.".into())
                        } else {
                            Err("Export receipt was not accepted.".into())
                        }
                    });
                self.result(result);
            }
            Id::Recoveries => {
                self.refresh_recoveries();
                self.recovery_window = true;
            }
            Id::Quit => {
                self.close_intent = true;
            }
            Id::Undo | Id::Redo => self.history(
                id == Id::Undo,
                match scope {
                    ShortcutTarget::Field(key) => Some(key),
                    _ => None,
                },
            ),
            Id::LayoutUndo | Id::LayoutRedo => self
                .views
                .layout_history(&self.session.editor, id == Id::LayoutRedo),
            Id::Search => self.open_search(),
            Id::Usages => {
                self.open_usages(self.session.editor.selected.expect("checked selection"))
            }
            Id::Impact => {
                self.open_impact(self.session.editor.selected.expect("checked selection"))
            }
            Id::Presentation => self.views.open_inspection(),
            Id::External => self.exchange.open(&self.path),
            Id::Assurance => self.assurance.open_window(),
            Id::Library => self.catalog.open(&self.path),
            Id::Panels => {self.docking.window=true;},
            Id::Shortcuts => {
                self.shortcuts.open = true;
            }
            Id::Settings => {
                self.settings_window = true;
            }
        }
    }
    pub(super) fn command_alias(id: Id) -> String {
        match id {
            Id::Recoveries => "recovery:Review copies".into(),
            Id::Settings => "settings:Open".into(),
            Id::LayoutUndo | Id::LayoutRedo => format!("view:{}", id.spec().label),
            _ => format!("action:{}", id.spec().label),
        }
    }
    pub(super) fn workspace_button(&mut self, ui: &mut egui::Ui, id: Id, scope: &ShortcutTarget) {
        let reason = self.command_reason(id, scope);
        let response = if id.is_history() {
            let visual = ButtonVisuals {
                fill: Color32::from_rgb(231, 239, 238),
                stroke: Stroke::new(1., LINE),
                icon: ACCENT,
            };
            let style = IconButtonStyle {
                normal: visual,
                hovered: visual,
                pressed: Some(visual),
                selected: visual,
                corner_radius: egui::CornerRadius::same(4),
                focus_stroke: Stroke::new(2., ACCENT),
            };
            ui.add_enabled_ui(reason.is_none(), |ui| {
                IconButton::new(id.spec().label, Vec2::new(44., 28.), &style).show(
                    ui,
                    |ui, rect, tint| {
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            id.spec().label,
                            FontId::proportional(12.),
                            tint,
                        );
                    },
                )
            })
            .inner
        } else {
            ui.add_enabled(reason.is_none(), egui::Button::new(id.spec().label))
        };
        let response = response
            .on_hover_text(format!(
                "{} · {}",
                id.spec().label,
                self.shortcut_label(id, scope, ui.ctx())
            ))
            .on_disabled_hover_text(reason.unwrap_or_default());
        self.bound(Self::command_alias(id), &response);
        self.bound(format!("command:toolbar:{}", id.spec().key), &response);
        if response.clicked() {
            self.dispatch_workspace(id, Surface::Toolbar, scope.clone());
        }
    }
    pub(super) fn command_menus(&mut self, ui: &mut egui::Ui) {
        if !egui::Popup::is_any_open(ui.ctx()) {
            self.menu_scope = None;
        }
        let scope = self
            .menu_scope
            .clone()
            .unwrap_or_else(|| self.history_scope());
        let mut activated = None;
        egui::MenuBar::new().ui(ui, |ui| {
            for menu in catalog::MENUS {
                let shown = ui.menu_button(*menu, |ui| {
                    ui.set_min_width(275.);
                    for spec in catalog::ALL.iter().filter(|s| s.menu == *menu) {
                        let reason = self.command_reason(spec.id, &scope);
                        let r = ui.add_enabled(
                            reason.is_none(),
                            egui::Button::new(spec.label).shortcut_text(self.shortcut_label(
                                spec.id,
                                &scope,
                                ui.ctx(),
                            )),
                        );
                        let r = r.on_disabled_hover_text(reason.clone().unwrap_or_default());
                        self.bound(format!("command:menu:{}", spec.key), &r);
                        if let Some(reason) = reason {
                            ui.label(egui::RichText::new(reason).small());
                        }
                        if r.clicked() {
                            activated = Some(spec.id);
                            ui.close();
                        }
                    }
                });
                self.bound(format!("menu:{menu}"), &shown.response);
                if shown.inner.is_some() && self.menu_scope.is_none() {
                    self.menu_scope = Some(scope.clone());
                }
            }
        });
        if let Some(id) = activated {
            self.dispatch_workspace(id, Surface::Menu, scope);
            self.menu_scope = None;
        }
    }
    pub(super) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // The assurance editor owns its Save shortcut. This is not native Save.
        if !self.shortcuts.open && self.assurance.is_visible() && ctx.input(|i|i.events.iter().any(|e|matches!(e,egui::Event::Key{key:egui::Key::S,pressed:true,repeat:false,modifiers,..} if modifiers.matches_exact(egui::Modifiers::COMMAND)))) {
            ctx.input_mut(|i|i.consume_key(egui::Modifiers::COMMAND,egui::Key::S));self.assurance.request_save();return;
        }
        if egui::Popup::is_any_open(ctx) {
            return;
        }
        let transition = ctx.input(|i| {
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
        });
        let scope = if transition {
            ShortcutTarget::Widget
        } else {
            self.history_scope()
        };
        // Exact modifiers, no auto-repeat and one dispatch per frame. Eguis
        // consume_shortcut otherwise ignores extra Shift/Alt, aliasing histories.
        let pressed = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } = e
                {
                    catalog::ALL.iter().find_map(|spec| {
                        let chord = self.keyboard_binding(spec.id, &scope)?;
                        (chord.logical_key == *key && modifiers.matches_exact(chord.modifiers))
                            .then_some((spec.id, chord))
                    })
                } else {
                    None
                }
            })
        });
        if let Some((id, chord)) = pressed {
            ctx.input_mut(|i| i.consume_key(chord.modifiers, chord.logical_key));
            self.dispatch_workspace(id, Surface::Keyboard, scope);
        }
    }
    pub(super) fn keyboard_binding(
        &self,
        id: Id,
        scope: &ShortcutTarget,
    ) -> Option<egui::KeyboardShortcut> {
        if id.is_history() {
            return match scope {
                ShortcutTarget::Field(_) => Some(id.spec().shortcut),
                ShortcutTarget::Widget => None,
                ShortcutTarget::Model => self.shortcuts.active.get(id),
            };
        }
        if id.is_layout_history() && !matches!(scope, ShortcutTarget::Model) {
            return None;
        }
        self.shortcuts.active.get(id)
    }
    pub(super) fn shortcut_label(
        &self,
        id: Id,
        scope: &ShortcutTarget,
        ctx: &egui::Context,
    ) -> String {
        let chord = if id.is_history() && matches!(scope, ShortcutTarget::Field(_)) {
            Some(id.spec().shortcut)
        } else {
            self.shortcuts.active.get(id)
        };
        chord
            .map(|c| ctx.format_shortcut(&c))
            .unwrap_or_else(|| "Unbound".into())
    }
    pub(super) fn command_audit(&self) -> serde_json::Value {
        let scope = self
            .menu_scope
            .clone()
            .unwrap_or_else(|| self.history_scope());
        serde_json::json!({"schema":"mbse-craft.command-catalog/1","last":self.last_workspace_command,"items":catalog::ALL.iter().map(|s|{
            let reason=self.command_reason(s.id,&scope);
            serde_json::json!({"id":s.key,"label":s.label,"menu":s.menu,"shortcut":self.shortcut_label(s.id,&scope,&self.context),"default_shortcut":self.context.format_shortcut(&s.shortcut),"keyboard_scope":crate::shortcuts::scope(s.id),"enabled":reason.is_none(),"reason":reason})
        }).collect::<Vec<_>>()})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mbse_model::{native_views::Layout, stage1::Editor};
    fn app() -> (tempfile::TempDir, Workspace, egui::Context) {
        let area = tempfile::Builder::new()
            .prefix("p09-")
            .tempdir_in(std::env::var_os("TMPDIR").expect("workspace TMPDIR"))
            .unwrap();
        let session = Session::new(area.path()).unwrap();
        let ctx = egui::Context::default();
        let app = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        (area, app, ctx)
    }
    fn state(app: &Workspace) -> (String, String, (usize, usize), u64) {
        (
            app.session.editor.snapshot.digest(),
            app.views.views.document().digest().unwrap(),
            app.views.views.history_usage(),
            app.views.views.document().next_id(),
        )
    }
    fn press(app: &mut Workspace, ctx: &egui::Context, id: Id) {
        let s = id.spec().shortcut;
        ctx.run_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::ModifiersChanged(s.modifiers),
                    egui::Event::Key {
                        key: s.logical_key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: s.modifiers,
                    },
                    egui::Event::Key {
                        key: s.logical_key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: s.modifiers,
                    },
                ],
                ..Default::default()
            },
            |ui| app.handle_shortcuts(ui.ctx()),
        )
        .drop_without_applying_deltas();
    }
    #[test]
    fn command_ids_defaults_and_menu_groups_are_unique_and_complete() {
        use std::collections::BTreeSet;
        assert_eq!(catalog::ALL.len(), 35);
        assert_eq!(
            catalog::ALL
                .iter()
                .map(|s| s.key)
                .collect::<BTreeSet<_>>()
                .len(),
            35
        );
        assert_eq!(
            catalog::ALL
                .iter()
                .map(|s| s.id)
                .collect::<BTreeSet<_>>()
                .len(),
            35
        );
        for (i, s) in catalog::ALL.iter().enumerate() {
            assert!(!matches!(s.shortcut.logical_key,egui::Key::C|egui::Key::X|egui::Key::V),"egui-winit consumes Command+C/X/V, including extra modifiers, before workspace dispatch");
            assert!(catalog::MENUS.contains(&s.menu));
            assert!(s.key.contains('.'));
            assert_eq!(s.id.spec().key, s.key);
            for other in &catalog::ALL[i + 1..] {
                assert_ne!(s.shortcut, other.shortcut);
            }
        }
    }
    #[test]
    fn all_surfaces_refuse_same_missing_path_busy_and_stale_selection_without_mutation() {
        for surface in [Surface::Menu, Surface::Toolbar, Surface::Keyboard] {
            let (_area, mut app, _) = app();
            let before = state(&app);
            for id in [Id::Save, Id::Open, Id::Recover, Id::Import, Id::Export] {
                let reason = app.command_reason(id, &ShortcutTarget::Model).unwrap();
                app.dispatch_workspace(id, surface, ShortcutTarget::Model);
                assert_eq!(app.feedback, reason);
                assert_eq!(state(&app), before);
                assert!(!app.session.pending());
            }
            let job = app.session.begin_recovery().unwrap();
            let stamp = state(&app);
            for id in [Id::Recovery, Id::Undo, Id::LayoutUndo] {
                app.dispatch_workspace(id, surface, ShortcutTarget::Model);
                assert!(app.feedback.contains("pending"));
                assert_eq!(state(&app), stamp);
                assert!(app.session.pending());
            }
            assert!(app
                .session
                .failed_start(job.start_token(), "Unstarted P09 fixture cleanup".into()));
            app.session.editor.selected = None;
            app.dispatch_workspace(Id::Impact, surface, ShortcutTarget::Model);
            assert!(app.feedback.contains("Select an available"));
            assert_eq!(state(&app), before);
        }
    }
    #[test]
    fn all_surfaces_share_model_history_and_keep_raw_and_layout_separate() {
        for surface in [Surface::Menu, Surface::Toolbar, Surface::Keyboard] {
            let (_area, mut app, ctx) = app();
            let old = app.session.editor.snapshot.records.clone();
            app.command(Command::Add {
                kind: RecordKind::Mission,
                owner: RecordId(1),
                name: "Mission".into(),
            });
            app.views.apply(
                &app.session.editor,
                Layout::AddView("Retained layout".into()),
            );
            let layout = app.views.views.document().views.clone();
            if surface == Surface::Keyboard {
                press(&mut app, &ctx, Id::Undo);
            } else {
                app.dispatch_workspace(Id::Undo, surface, ShortcutTarget::Model);
            }
            assert_eq!(app.session.editor.snapshot.records, old);
            assert_eq!(app.views.views.document().views, layout);
            app.dispatch_workspace(Id::Redo, surface, ShortcutTarget::Model);
            assert_eq!(app.session.editor.snapshot.records.len(), 2);
            assert_eq!(
                app.last_workspace_command.as_ref().unwrap()["surface"],
                surface.name()
            );
        }
    }
    #[test]
    fn raw_field_menu_scope_survives_popup_and_never_falls_through_to_model_undo() {
        for surface in [Surface::Menu, Surface::Toolbar, Surface::Keyboard] {
            let (_area, mut app, ctx) = app();
            let key = RawKey {
                record: RecordId(1),
                field: "name".into(),
                item: None,
            };
            let old = app.session.editor.display_text(&key);
            app.command(Command::EditText {
                key: key.clone(),
                text: "Exact field edit".into(),
            });
            let focus = egui::Id::new("p09-native-field");
            app.model_fields.insert(focus, key.clone());
            ctx.memory_mut(|m| m.request_focus(focus));
            let scope = app.history_scope();
            assert_eq!(scope, ShortcutTarget::Field(key.clone()));
            app.menu_scope = Some(scope.clone());
            if surface == Surface::Keyboard {
                press(&mut app, &ctx, Id::Undo);
            } else {
                app.dispatch_workspace(Id::Undo, surface, scope);
            }
            assert_eq!(app.session.editor.display_text(&key), old);
            assert_eq!(
                app.last_workspace_command.as_ref().unwrap()["scope"],
                "field"
            );
            app.session.editor.selected = None;
            let before = state(&app);
            app.dispatch_workspace(Id::Redo, surface, ShortcutTarget::Field(key));
            assert_eq!(state(&app), before);
            assert!(app.feedback.contains("focused native field changed"));
        }
    }
    #[test]
    fn exact_keyboard_modifiers_select_layout_history_and_recovery_instead_of_shorter_bindings() {
        let (_area, mut app, ctx) = app();
        app.command(Command::Add {
            kind: RecordKind::Mission,
            owner: RecordId(1),
            name: "Mission".into(),
        });
        let native = app.session.editor.snapshot.digest();
        let old_views = app.views.views.document().views.clone();
        app.views
            .apply(&app.session.editor, Layout::AddView("Camera review".into()));
        press(&mut app, &ctx, Id::LayoutUndo);
        assert_eq!(app.session.editor.snapshot.digest(), native);
        assert_eq!(app.views.views.document().views, old_views);
        assert_eq!(
            app.last_workspace_command.as_ref().unwrap()["id"],
            "view.layout_undo"
        );
        press(&mut app, &ctx, Id::LayoutRedo);
        assert_eq!(app.views.views.document().views.len(), old_views.len() + 1);
        app.views.open_inspection();
        let before = state(&app);
        press(&mut app, &ctx, Id::Recovery);
        assert_eq!(
            app.last_workspace_command.as_ref().unwrap()["id"],
            "file.recovery_copy"
        );
        assert_eq!(state(&app), before);
        assert!(!app.session.pending());
    }
    #[test]
    fn histories_disable_independently_and_widget_undo_and_repeat_keys_remain_unconsumed() {
        let (_area, mut app, ctx) = app();
        let before = state(&app);
        for id in [Id::Undo, Id::Redo, Id::LayoutUndo, Id::LayoutRedo] {
            assert!(app
                .command_reason(id, &ShortcutTarget::Model)
                .unwrap()
                .contains("history"));
        }
        assert!(app
            .command_reason(Id::Undo, &ShortcutTarget::Widget)
            .unwrap()
            .contains("Text widget owns"));
        // Non-model text focus must retain the key for TextEdit's own undo stack.
        app.settings_window = true;
        let mut retained = false;
        let s = Id::Undo.spec().shortcut;
        ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: s.logical_key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: s.modifiers,
                }],
                ..Default::default()
            },
            |ui| {
                app.handle_shortcuts(ui.ctx());
                retained = ui.ctx().input(|i| {
                    i.events.iter().any(|e| {
                        matches!(
                            e,
                            egui::Event::Key {
                                key: egui::Key::Z,
                                ..
                            }
                        )
                    })
                });
            },
        )
        .drop_without_applying_deltas();
        assert!(retained);
        assert!(app.last_workspace_command.is_none());
        let s = Id::Search.spec().shortcut;
        // A real repeat has a preceding key-down still held in InputState.
        ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: s.logical_key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: s.modifiers,
                }],
                ..Default::default()
            },
            |ui| app.handle_shortcuts(ui.ctx()),
        )
        .drop_without_applying_deltas();
        assert!(app.search_open);
        app.search_open = false;
        app.last_workspace_command = None;
        ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: s.logical_key,
                    physical_key: None,
                    pressed: true,
                    repeat: true,
                    modifiers: s.modifiers,
                }],
                ..Default::default()
            },
            |ui| app.handle_shortcuts(ui.ctx()),
        )
        .drop_without_applying_deltas();
        assert!(!app.search_open);
        assert_eq!(state(&app), before);
    }
    #[test]
    fn navigation_commands_preserve_invalid_drafts_until_explicit_dialog_resolution() {
        for surface in [Surface::Menu, Surface::Toolbar, Surface::Keyboard] {
            let (_area, mut app, ctx) = app();
            let snapshot = mbse_model::stage1::Snapshot::from_fixture(
                app.session.editor.snapshot.document_id.clone(),
                &serde_json::from_str(include_str!(
                    "../../mbse-model/fixtures/stage1/inspection.json"
                ))
                .unwrap(),
            )
            .unwrap();
            app.session.editor =
                Editor::new(snapshot, app.session.editor.freshness().instance_id).unwrap();
            app.views = crate::views_ui::ViewWorkspace::new(&app.session);
            app.command(Command::EditText {
                key: RawKey {
                    record: RecordId(301),
                    field: "criterion".into(),
                    item: None,
                },
                text: "9e".into(),
            });
            let before = state(&app);
            if surface == Surface::Keyboard {
                press(&mut app, &ctx, Id::New);
            } else {
                app.dispatch_workspace(Id::New, surface, ShortcutTarget::Model);
            }
            assert!(matches!(app.pending, Some(Load::New)));
            assert_eq!(state(&app), before);
            app.dispatch_workspace(Id::Undo, surface, ShortcutTarget::Model);
            assert_eq!(state(&app), before);
            assert!(app.feedback.contains("preservation dialog"));
            app.pending = None;
            assert_eq!(
                app.session.editor.display_text(&RawKey {
                    record: RecordId(301),
                    field: "criterion".into(),
                    item: None
                }),
                "9e"
            );
        }
    }
    #[test]
    fn menu_toolbar_keyboard_save_actual_pairs_with_the_same_snapshot_and_empty_reopened_history() {
        for surface in [Surface::Menu, Surface::Toolbar, Surface::Keyboard] {
            let (area, mut app, ctx) = app();
            app.command(Command::EditText {
                key: RawKey {
                    record: RecordId(1),
                    field: "name".into(),
                    item: None,
                },
                text: "Catalog save".into(),
            });
            let snapshot = app.session.editor.snapshot.digest();
            let path = area.path().join("catalog-save.kernspan");
            app.save_path = path.display().to_string();
            if surface == Surface::Keyboard {
                press(&mut app, &ctx, Id::Save);
            } else {
                app.dispatch_workspace(Id::Save, surface, ShortcutTarget::Model);
            }
            let deadline = Instant::now() + Duration::from_secs(5);
            while app.session.pending() || app.views.pair_pending() || app.views.io.pending() {
                assert!(Instant::now() < deadline);
                app.finish_writes(&ctx);
                app.views.poll(&app.session);
                std::thread::yield_now();
            }
            assert!(!app.error, "{}", app.feedback);
            assert!(app.views.io.is_saved(&app.session, &app.views.views));
            let reopened = Session::open(&area.path().join("reopen"), &path).unwrap();
            assert_eq!(reopened.editor.snapshot.digest(), snapshot);
            assert!(!reopened.editor.can_undo());
            assert_eq!(
                app.last_workspace_command.as_ref().unwrap()["surface"],
                surface.name()
            );
        }
    }
    #[test]
    fn inspection_and_auxiliary_scope_refuse_native_actions_without_writes_or_history_loss() {
        for surface in [Surface::Menu, Surface::Toolbar, Surface::Keyboard] {
            let (area, mut app, _) = app();
            app.command(Command::Add {
                kind: RecordKind::Mission,
                owner: RecordId(1),
                name: "Retain".into(),
            });
            let before = state(&app);
            let dest = area.path().join("must-not-write.kernspan");
            app.save_path = dest.display().to_string();
            app.views.open_inspection();
            for id in [Id::Save, Id::Undo, Id::LayoutUndo] {
                app.dispatch_workspace(id, surface, ShortcutTarget::Widget);
                assert_eq!(state(&app), before);
                assert!(!app.session.pending());
            }
            assert!(!dest.exists());
            assert_eq!(app.command_audit()["items"].as_array().unwrap().len(), catalog::ALL.len());
        }
    }
}
