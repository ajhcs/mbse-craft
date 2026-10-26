//! Bound typed-deletion previews and explicit coupled semantic/presentation Apply.
use super::*;
use mbse_model::types::{
    self,
    deletion::{self, Action},
};
#[derive(Default)]
pub(super) struct DeletionPanel {
    pub open: bool,
    prepared: Option<deletion::Prepared>,
    note: String,
}
impl Workspace {
    fn deletion_action(&mut self, a: Action) -> bool {
        let stamp = self.views.views.stamp(&self.session.editor);
        match self.views.views.dispatch(
            &mut self.session.editor,
            &stamp,
            mbse_model::native_views::NativeAction::Command(Command::Typed(
                types::Action::Deletion(a),
            )),
        ) {
            Ok(_) => {
                self.deletion_panel.note="Deletion data and unfinished form retained. Model Undo restores accepted records and this runtime's exact presentation.".into();
                true
            }
            Err(error) => {
                self.deletion_panel.note = error;
                false
            }
        }
    }
    pub(super) fn deletion_dialog(&mut self, ctx: &egui::Context) {
        if !self.deletion_panel.open {
            return;
        }
        let instance = self.session.editor.freshness().instance_id;
        let mut open = true;
        let mut close = false;
        let mut raw = None;
        let mut prepare = false;
        let mut apply = None;
        let mut cancel = false;
        let mut history = None;
        egui::Window::new("Delete typed model records with impact preview").id(egui::Id::new("mbse-deletion")).open(&mut open).default_width(1000.).default_height(1100.).vscroll(true).max_height((ctx.content_rect().height()-80.).max(240.)).show(ctx,|ui|{
   ui.label("Delete accepted typed definitions or members with the listed dependencies. Unknown metadata prevents a complete impact verdict and blocks Delete. Raw forms remain available for recovery.");
   ui.horizontal(|ui|{let ready=!self.views.occurrence_navigation_blocked(&self.session);for(redo,key,label,enabled)in [(false,"undo-model","Undo model",self.session.editor.can_undo()),(true,"redo-model","Redo model",self.session.editor.can_redo())]{let r=ui.add_enabled(ready&&enabled,egui::Button::new(label));self.bound(format!("delete:{key}"),&r);if r.clicked(){history=Some(redo);}}});
   let mut form=deletion::form(&self.session.editor.snapshot).ok().flatten().unwrap_or_default();let before=form.clone();ui.horizontal(|ui|{ui.label("Typed record IDs, comma separated");let r=ui.add(egui::TextEdit::singleline(&mut form.ids).id(egui::Id::new(("mbse-deletion",&instance,"ids"))));self.bound("delete:field:ids",&r);});let r=ui.checkbox(&mut form.cascade,"Delete the listed dependent accepted records and value entries; keep unfinished forms");self.bound("delete:cascade",&r);let edited=form!=before;if edited{raw=Some(Action::SetForm(form));}
   let r=ui.add_enabled(!edited,egui::Button::new("Prepare deletion impact"));self.bound("delete:prepare",&r);prepare=r.clicked();
   if let Some(prepared)=&self.deletion_panel.prepared{let summary=prepared.summary().clone();let plan=prepared.plan().cloned();let current=plan.as_ref().is_some_and(|p|p.check(&self.session.editor,&self.views.views).is_ok());ui.separator();ui.label(format!("{} accepted records · {} authored value entries · {} incoming references · {} port occurrences · {} connector routes",summary.removed.len(),summary.removed_values.len(),summary.incoming.len(),summary.port_occurrences.len(),summary.connector_routes.len()));if let Some(refusal)=&summary.refusal{ui.colored_label(ERROR,refusal);}else if !current{ui.colored_label(ERROR,"Impact preview is outdated; prepare again before Delete.");}
    egui::ScrollArea::vertical().id_salt("deletion-impact-rows").max_height(260.).show(ui,|ui|{for record in summary.removed.iter().take(128){ui.label(format!("Delete {}#{} · {}",record.kind,record.id.0,record.name));}for entry in summary.removed_values.iter().take(128){ui.label(entry.usage.map_or_else(||format!("Remove authored default for Property#{}",entry.property.0),|id|format!("Remove explicit value Part#{} Property#{}; the Part remains unless listed above",id.0,entry.property.0)));}for reference in summary.incoming.iter().take(512){ui.label(format!("Incoming #{} · {} · target#{} is handled by the listed record/value treatment",reference.owner.0,reference.role,reference.target.0));}for location in &summary.unknown.locations{ui.colored_label(ERROR,location);}});
    ui.label(format!("{} typed raw forms and all native journals are retained. Affected occurrences/routes detach; exact same-runtime model Undo restores their identities. Cold recovery retains detached state without inventing history authority.",summary.retained_raw_forms));let r=ui.add_enabled(current&&!edited,egui::Button::new("Delete reviewed records"));self.bound("delete:apply",&r);if r.clicked(){apply=plan;}let r=ui.button("Cancel deletion preview (keep form)");self.bound("delete:cancel",&r);cancel=r.clicked();
   }
   ui.label(&self.deletion_panel.note);let r=ui.button("Close deletion");self.bound("delete:close",&r);close=r.clicked();
  });
        self.deletion_panel.open = open && !close;
        if let Some(action) = raw {
            self.deletion_action(action);
        }
        if prepare {
            match deletion::prepare(&self.session.editor, &self.views.views) {
                Ok(p) => {
                    self.deletion_panel.prepared = Some(p);
                    self.deletion_panel.note =
                        "Impact prepared without changing accepted data or presentation.".into();
                }
                Err(error) => {
                    self.deletion_panel.prepared = None;
                    self.deletion_panel.note = error;
                }
            }
        }
        if let Some(plan) = apply {
            self.deletion_action(Action::Apply(Box::new(plan)));
        }
        if cancel {
            self.deletion_panel.prepared = None;
            self.deletion_panel.note =
                "Deletion preview cancelled; accepted model, views and unfinished form retained."
                    .into();
        }
        if let Some(redo) = history {
            self.deletion_panel.note = match self.typed_model_history(redo) {
                Ok(true) => "Model history applied; current unfinished form retained.".into(),
                Ok(false) => "No model history in this direction.".into(),
                Err(error) => error,
            };
        }
    }
    pub(super) fn deletion_audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.deletion-ui/1","open":self.deletion_panel.open,"form":deletion::form(&self.session.editor.snapshot).ok().flatten(),"summary":self.deletion_panel.prepared.as_ref().map(|p|p.summary()),"applicable":self.deletion_panel.prepared.as_ref().and_then(|p|p.plan()).is_some_and(|p|p.check(&self.session.editor,&self.views.views).is_ok()),"note":self.deletion_panel.note})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rendered_unknown_hold_cancel_stale_guard_and_delete_undo_preserve_newer_form() {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/parity/p21a/fixtures/deletioned.kernspan"
        ))
        .unwrap();
        let instance = session.editor.freshness().instance_id;
        session.editor = mbse_model::stage1::Editor::new(
            serde_json::from_value(raw["snapshot"].clone()).unwrap(),
            instance,
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
        let mut form = deletion::Form {
            ids: "1021".into(),
            cascade: true,
        };
        assert!(app.deletion_action(Action::SetForm(form.clone())));
        app.deletion_panel.open = true;
        let render = |app: &mut Workspace, events| {
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
                |_| app.deletion_dialog(&ctx),
            )
            .drop_without_applying_deltas();
        };
        for _ in 0..3 {
            render(&mut app, vec![]);
        }
        let click = |app: &mut Workspace, key: &str| {
            let rect = app.audit[key];
            let pos = egui::pos2((rect[0] + rect[2]) / 2., (rect[1] + rect[3]) / 2.);
            for pressed in [true, false] {
                render(
                    app,
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
            render(app, vec![]);
        };
        let before = app.session.editor.snapshot.clone();
        click(&mut app, "delete:prepare");
        assert!(
            app.deletion_panel
                .prepared
                .as_ref()
                .unwrap()
                .plan()
                .is_none()
        );
        assert!(!app.deletion_audit()["applicable"].as_bool().unwrap());
        click(&mut app, "delete:apply");
        assert_eq!(app.session.editor.snapshot, before);
        click(&mut app, "delete:cancel");
        assert!(app.deletion_panel.prepared.is_none());
        assert_eq!(app.session.editor.snapshot, before);
        // Controlled synthetic test only: original opaque source remains the blocking fixture.
        app.session
            .editor
            .snapshot
            .records
            .get_mut(&mbse_model::stage1::RecordId(1))
            .unwrap()
            .fields
            .remove("opaque.p14.future");
        click(&mut app, "delete:prepare");
        assert!(app.deletion_audit()["applicable"].as_bool().unwrap());
        form.ids = "1021?".into();
        assert!(app.deletion_action(Action::SetForm(form.clone())));
        render(&mut app, vec![]);
        assert!(!app.deletion_audit()["applicable"].as_bool().unwrap());
        let accepted = types::load(&app.session.editor.snapshot).unwrap();
        click(&mut app, "delete:apply");
        assert_eq!(types::load(&app.session.editor.snapshot).unwrap(), accepted);
        assert_eq!(app.history_scope(), ShortcutTarget::Widget);
        assert!(
            app.command_reason(crate::command_catalog::Id::Save, &app.history_scope())
                .is_some()
        );
        form.ids = "1021".into();
        assert!(app.deletion_action(Action::SetForm(form.clone())));
        click(&mut app, "delete:prepare");
        click(&mut app, "delete:apply");
        assert!(
            !types::load(&app.session.editor.snapshot)
                .unwrap()
                .definitions
                .contains_key(&types::Id(1021))
        );
        form.ids = "1021?".into();
        assert!(app.deletion_action(Action::SetForm(form.clone())));
        click(&mut app, "delete:undo-model");
        assert_eq!(types::load(&app.session.editor.snapshot).unwrap(), accepted);
        assert_eq!(
            deletion::form(&app.session.editor.snapshot).unwrap(),
            Some(form)
        );
        assert_eq!(app.session.editor.snapshot.next_id, before.next_id);
    }
}
