//! Draft-preserving shortcut editor. Active settings change only on valid Apply.
use super::*;
use crate::{command_catalog as catalog, shortcuts::Bindings};

impl Workspace {
    pub(super) fn apply_shortcuts(&mut self) -> Result<(), String> {
        let checked =
            Bindings::resolve(&self.shortcuts.draft).map_err(|errors| errors.join("\n"))?;
        self.shortcuts.active = checked;
        self.shortcuts.load_errors.clear();
        if self.preferences.value.shortcut_overrides != self.shortcuts.draft {
            self.preferences.value.shortcut_overrides = self.shortcuts.draft.clone();
            self.preferences.changed();
        }
        self.shortcuts.note=if !self.preferences.persistent || self.preferences.read_only {
            "Shortcuts active for this session. File persistence is unavailable; original settings retained."
        }else{"Shortcuts active. Persistence is confirmed only by the preferences status below."}.into();
        Ok(())
    }
    pub(super) fn shortcut_dialog(&mut self, ctx: &egui::Context) {
        if !self.shortcuts.open {
            return;
        }
        let mut open = true;
        let mut close = false;
        let mut apply = false;
        egui::Window::new("Keyboard shortcuts").id(egui::Id::new("shortcut-configuration")).open(&mut open).default_width(850.).show(ctx,|ui|{
            ui.label("Configure workspace commands by stable ID. Changes apply together only after validation.");
            ui.label("Use Primary+Alt+U (Primary = Ctrl on Linux/Windows, Cmd on macOS). Blank = default; Unbound = remove the model/workspace binding.");
            ui.label("Native fields keep Ctrl/Cmd+Z and Ctrl/Cmd+Shift+Z for field Undo/Redo. Model overrides apply outside text fields; layout shortcuts apply on the canvas. Text selection/copy/cut/paste keys are reserved.");
            ui.horizontal(|ui|{ui.label("Find command");let r=ui.text_edit_singleline(&mut self.shortcuts.filter);self.bound("shortcuts:filter",&r);});
            let query=self.shortcuts.filter.to_lowercase();
            egui::ScrollArea::vertical().id_salt("shortcut-rows").max_height(400.).show(ui,|ui|{
                for spec in catalog::ALL.iter().filter(|s|query.is_empty()||s.key.contains(&query)||s.label.to_lowercase().contains(&query)){
                    ui.group(|ui|{
                        ui.label(format!("{} · {}",spec.label,spec.key));
                        ui.label(format!("Scope: {}",crate::shortcuts::scope(spec.id)));
                        ui.horizontal(|ui|{
                            ui.label(format!("Default: {}",ctx.format_shortcut(&spec.shortcut)));
                            let mut text=self.shortcuts.draft.get(spec.key).filter(|v|!v.trim().eq_ignore_ascii_case("Default")).cloned().unwrap_or_default();
                            let r=ui.add(egui::TextEdit::singleline(&mut text).char_limit(96).desired_width(290.).hint_text("Default"));
                            self.bound(format!("shortcuts:binding:{}",spec.key),&r);
                            if r.changed(){self.shortcuts.draft.insert(spec.key.into(),if text.is_empty(){"Default".into()}else{text});}
                            let r=ui.button("Default");self.bound(format!("shortcuts:default:{}",spec.key),&r);if r.clicked(){self.shortcuts.draft.insert(spec.key.into(),"Default".into());}
                        });
                    });
                }
            });
            let candidate=Bindings::resolve(&self.shortcuts.draft);
            if let Err(errors)=&candidate {egui::ScrollArea::vertical().id_salt("shortcut-errors").max_height(130.).show(ui,|ui|{for error in errors{ui.colored_label(ERROR,error);}});}
            if !self.shortcuts.load_errors.is_empty(){ui.colored_label(ERROR,"Stored shortcut configuration was refused; defaults remain active and original file is retained.");egui::ScrollArea::vertical().id_salt("shortcut-load-errors").max_height(100.).show(ui,|ui|{for e in &self.shortcuts.load_errors{ui.label(e);}});}
            let unknown=self.shortcuts.unknown();if !unknown.is_empty(){ui.label(format!("{} unknown command entries retained as inert data.",unknown.len()));}
            ui.horizontal(|ui|{
                let r=ui.add_enabled(candidate.is_ok(),egui::Button::new("Apply shortcuts"));self.bound("shortcuts:apply",&r);apply=r.clicked();
                let r=ui.button("Restore defaults in draft");self.bound("shortcuts:reset",&r);if r.clicked(){self.shortcuts.reset_known();}
                let r=ui.button("Close (retain draft)");self.bound("shortcuts:close",&r);close=r.clicked();
            });
            ui.label(&self.shortcuts.note);ui.label(&self.preferences.report);ui.label(&self.preferences.status);
            if self.preferences.pending(){ui.label("Preference write pending; active shortcuts are session state until saving succeeds.");}
        });
        self.shortcuts.open = open && !close;
        if apply {
            if let Err(error) = self.apply_shortcuts() {
                self.shortcuts.note = error;
            }
        }
    }
    pub(super) fn shortcut_audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.shortcut-configuration/1","open":self.shortcuts.open,"draft":self.shortcuts.draft,"stored_overrides":self.preferences.value.shortcut_overrides,"draft_errors":Bindings::resolve(&self.shortcuts.draft).err().unwrap_or_default(),"load_errors":self.shortcuts.load_errors,"note":self.shortcuts.note,"persistent":self.preferences.persistent,"read_only":self.preferences.read_only,"pending":self.preferences.pending(),"status":self.preferences.status})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_catalog::{Id, Surface};
    fn app(config: Option<PathBuf>) -> (tempfile::TempDir, Workspace, egui::Context) {
        let area = tempfile::Builder::new()
            .prefix("p10-ui-")
            .tempdir_in(std::env::var_os("TMPDIR").expect("workspace TMPDIR"))
            .unwrap();
        let session = Session::new(area.path()).unwrap();
        let ctx = egui::Context::default();
        let app = Workspace::new(
            session,
            None,
            config,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        (area, app, ctx)
    }
    fn press(app: &mut Workspace, ctx: &egui::Context, text: &str) {
        let c = crate::shortcuts::parse(text).unwrap().unwrap();
        ctx.run_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::Key {
                        key: c.logical_key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: c.modifiers,
                    },
                    egui::Event::Key {
                        key: c.logical_key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: c.modifiers,
                    },
                ],
                ..Default::default()
            },
            |ui| app.handle_shortcuts(ui.ctx()),
        )
        .drop_without_applying_deltas();
    }
    fn activate(app: &mut Workspace, pairs: &[(&str, &str)]) {
        for (key, value) in pairs {
            app.shortcuts.draft.insert((*key).into(), (*value).into());
        }
        app.apply_shortcuts().unwrap();
    }
    #[test]
    fn invalid_apply_retains_active_settings_model_and_unfinished_draft() {
        let (_a, mut app, _) = app(None);
        let model = app.session.editor.snapshot.digest();
        let status = app.preferences.status.clone();
        app.shortcuts
            .draft
            .insert("edit.search".into(), "Primary+S".into());
        assert!(app.apply_shortcuts().unwrap_err().contains("Conflict"));
        assert!(app.preferences.value.shortcut_overrides.is_empty());
        assert!(!app.preferences.pending());
        assert_eq!(app.preferences.status, status);
        assert_eq!(app.session.editor.snapshot.digest(), model);
        app.shortcuts.open = false;
        app.dispatch_workspace(Id::Shortcuts, Surface::Menu, ShortcutTarget::Model);
        assert_eq!(app.shortcuts.draft["edit.search"], "Primary+S");
        assert_eq!(
            app.shortcuts.active.get(Id::Search),
            Some(Id::Search.spec().shortcut)
        );
    }
    #[test]
    fn overridden_or_unbound_model_undo_leaves_fixed_native_field_history_intact() {
        for binding in ["Primary+Alt+U", "Unbound"] {
            let (_a, mut app, ctx) = app(None);
            activate(
                &mut app,
                &[("edit.undo", binding), ("edit.redo", "Primary+Alt+Shift+U")],
            );
            let key = RawKey {
                record: RecordId(1),
                field: "name".into(),
                item: None,
            };
            let old = app.session.editor.display_text(&key);
            app.command(Command::EditText {
                key: key.clone(),
                text: "Field retained".into(),
            });
            let id = egui::Id::new("p10-raw-field");
            app.model_fields.insert(id, key.clone());
            ctx.memory_mut(|m| m.request_focus(id));
            assert_eq!(
                app.keyboard_binding(Id::Undo, &ShortcutTarget::Field(key.clone())),
                Some(Id::Undo.spec().shortcut)
            );
            press(&mut app, &ctx, "Primary+Z");
            assert_eq!(app.session.editor.display_text(&key), old);
            assert_eq!(
                app.last_workspace_command.as_ref().unwrap()["scope"],
                "field"
            );
            assert_eq!(
                app.keyboard_binding(Id::Undo, &ShortcutTarget::Widget),
                None
            );
        }
    }
    #[test]
    fn configured_model_and_canvas_histories_use_separate_exact_keys_without_text_theft() {
        let (_a, mut app, ctx) = app(None);
        activate(
            &mut app,
            &[
                ("edit.undo", "Primary+Alt+U"),
                ("view.layout_undo", "Primary+Alt+L"),
            ],
        );
        app.command(Command::Add {
            kind: RecordKind::Mission,
            owner: RecordId(1),
            name: "Keep native distinct".into(),
        });
        let native = app.session.editor.snapshot.digest();
        app.views.apply(
            &app.session.editor,
            mbse_model::native_views::Layout::AddView("Canvas".into()),
        );
        let old_count = app.views.views.document().views.len();
        press(&mut app, &ctx, "Primary+Alt+L");
        assert_eq!(app.views.views.document().views.len(), old_count - 1);
        assert_eq!(app.session.editor.snapshot.digest(), native);
        assert_eq!(
            app.last_workspace_command.as_ref().unwrap()["id"],
            "view.layout_undo"
        );
        press(&mut app, &ctx, "Primary+Alt+U");
        assert_eq!(app.session.editor.snapshot.records.len(), 1);
        assert_eq!(
            app.last_workspace_command.as_ref().unwrap()["id"],
            "edit.undo"
        );
        assert_eq!(
            app.keyboard_binding(Id::LayoutUndo, &ShortcutTarget::Widget),
            None
        );
        assert_eq!(
            app.keyboard_binding(
                Id::LayoutUndo,
                &ShortcutTarget::Field(RawKey {
                    record: RecordId(1),
                    field: "name".into(),
                    item: None
                })
            ),
            None
        );
    }
    #[test]
    fn editor_scope_blocks_native_file_and_history_and_menu_labels_follow_context() {
        let (_a, mut app, ctx) = app(None);
        activate(&mut app, &[("edit.undo", "Primary+Alt+U")]);
        let key = RawKey {
            record: RecordId(1),
            field: "name".into(),
            item: None,
        };
        assert_ne!(
            app.shortcut_label(Id::Undo, &ShortcutTarget::Model, &ctx),
            app.shortcut_label(Id::Undo, &ShortcutTarget::Field(key), &ctx)
        );
        app.shortcuts.open = true;
        let before = (
            app.session.editor.snapshot.digest(),
            app.views.views.document().digest().unwrap(),
        );
        for id in [Id::Save, Id::Undo, Id::LayoutUndo, Id::New, Id::Quit] {
            app.dispatch_workspace(id, Surface::Keyboard, ShortcutTarget::Widget);
            assert!(app.feedback.contains("Keyboard shortcuts"));
            assert_eq!(
                (
                    app.session.editor.snapshot.digest(),
                    app.views.views.document().digest().unwrap()
                ),
                before
            );
            assert!(!app.session.pending());
        }
    }
    #[test]
    fn readonly_future_settings_allow_explicit_session_override_and_preserve_original_bytes() {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let config = area.path().join("config");
        std::fs::create_dir(&config).unwrap();
        let path = config.join("config.toml");
        let bytes = b"schema_version = 99\nfuture_key = 'original'\n";
        std::fs::write(&path, bytes).unwrap();
        {
            let (_a, mut app, _) = app(Some(config));
            assert!(app.preferences.read_only);
            activate(&mut app, &[("edit.undo", "Primary+Alt+U")]);
            assert!(app.shortcuts.note.contains("session"));
            assert!(!app.preferences.pending());
            assert_eq!(
                app.shortcuts.active.get(Id::Undo),
                crate::shortcuts::parse("Primary+Alt+U").unwrap()
            );
        }
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn stored_conflicts_are_visible_with_defaults_and_no_automatic_file_rewrite() {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let config = area.path().join("config");
        std::fs::create_dir(&config).unwrap();
        let path = config.join("config.toml");
        let bytes=b"schema_version = 1\n# preserve bad config\n[shortcut_overrides]\n'edit.search' = 'Primary+S'\n'future.command' = 'unknown chord retained'\n";
        std::fs::write(&path, bytes).unwrap();
        {
            let (_a, app, _) = app(Some(config));
            assert!(!app.shortcuts.load_errors.is_empty());
            assert_eq!(
                app.shortcuts.active.get(Id::Search),
                Some(Id::Search.spec().shortcut)
            );
            assert!(!app.preferences.pending());
            assert_eq!(
                app.shortcut_audit()["stored_overrides"]["future.command"],
                "unknown chord retained"
            );
        }
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
