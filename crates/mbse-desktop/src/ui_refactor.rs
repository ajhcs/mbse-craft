//! Explicit accepted-model clone/move previews; raw form retained independently.
use super::*;
use mbse_model::types::{
    self,
    refactor::{self, Action, Mode},
};
#[derive(Default)]
pub(super) struct RefactorPanel {
    pub open: bool,
    prepared: Option<refactor::Prepared>,
    note: String,
}
impl Workspace {
    fn refactor_action(&mut self, action: Action) -> bool {
        let stamp = self.views.views.stamp(&self.session.editor);
        match self.views.views.dispatch(
            &mut self.session.editor,
            &stamp,
            mbse_model::native_views::NativeAction::Command(Command::Typed(
                types::Action::Refactor(action),
            )),
        ) {
            Ok(_) => {
                self.refactor_panel.note="Change retained; model Undo keeps newer unfinished forms. Original presentation is retained separately.".into();
                true
            }
            Err(e) => {
                self.refactor_panel.note = e;
                false
            }
        }
    }
    pub(super) fn refactor_dialog(&mut self, ctx: &egui::Context) {
        if !self.refactor_panel.open {
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
        egui::Window::new("Clone and move with reference preview").id(egui::Id::new("mbse-refactor")).open(&mut open).default_width(1000.).default_height(1050.).vscroll(true).max_height((ctx.content_rect().height()-80.).max(240.)).show(ctx,|ui|{
   ui.label("Clone a definition and its owned accepted members, or move selected usages with enclosed connectors. Part type definitions remain shared targets. Source unfinished forms and presentation stay retained.");
   let store=match types::load(&self.session.editor.snapshot){Ok(s)=>s,Err(e)=>{ui.colored_label(ERROR,e);let r=ui.button("Close clone/move");self.bound("refactor:close",&r);close=r.clicked();return;}};
   ui.horizontal(|ui|{let ready=!self.views.occurrence_navigation_blocked(&self.session);for(redo,key,label,enabled)in [(false,"undo-model","Undo model",self.session.editor.can_undo()),(true,"redo-model","Redo model",self.session.editor.can_redo())]{let r=ui.add_enabled(ready&&enabled,egui::Button::new(label));self.bound(format!("refactor:{key}"),&r);if r.clicked(){history=Some(redo);}}});
   let mut form=refactor::form(&self.session.editor.snapshot).ok().flatten().unwrap_or_default();let before=form.clone();ui.horizontal(|ui|{for(mode,label,key)in [(Mode::CloneDefinition,"Clone definition","clone"),(Mode::MoveUsages,"Move usages","move")]{let r=ui.selectable_value(&mut form.mode,mode,label);self.bound(format!("refactor:mode:{key}"),&r);}});
   egui::CollapsingHeader::new("Definition and usage IDs").id_salt("refactor-ids").show(ui,|ui|{egui::ScrollArea::vertical().id_salt("refactor-id-rows").max_height(120.).show(ui,|ui|{for d in store.definitions.values(){ui.label(format!("Definition#{} · {} · {:?} · System#{}",d.id.0,d.name,d.kind,d.system.0));}for u in store.usages.values(){ui.label(format!("Usage#{} · {} · {:?} · owner#{} · target#{}",u.id.0,u.name,u.kind,u.owner.0,u.target.0));}});});
   match form.mode{Mode::CloneDefinition=>{for(label,key,text)in [("Source definition ID","definition",&mut form.definition),("New definition name","name",&mut form.name)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(text).id(egui::Id::new(("mbse-refactor",&instance,key))));self.bound(format!("refactor:field:{key}"),&r);});}},Mode::MoveUsages=>{for(label,key,text)in [("Usage IDs, comma separated","usages",&mut form.usages),("New Component owner ID","owner",&mut form.owner)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(text).id(egui::Id::new(("mbse-refactor",&instance,key))));self.bound(format!("refactor:field:{key}"),&r);});}}}
   let r=ui.checkbox(&mut form.keep_external,"Keep existing external target identities; incoming links keep their current targets");self.bound("refactor:keep-external",&r);let edited=form!=before;if edited{raw=Some(Action::SetForm(form));}
   let r=ui.add_enabled(!edited,egui::Button::new("Prepare reference preview"));self.bound("refactor:prepare",&r);prepare=r.clicked();
   if let Some(prepared)=&self.refactor_panel.prepared{let summary=prepared.summary().clone();let plan=prepared.plan().cloned();let current=plan.as_ref().is_some_and(|p|p.check(&self.session.editor).is_ok());ui.separator();ui.label(&summary.operation);ui.label(format!("{} cloned IDs · {} moved usages · {} enclosed connectors · {} connector boundaries",summary.id_mapping.len(),summary.moved_usages.len(),summary.moved_connectors.len(),summary.boundary_connectors.len()));if let Some(refusal)=&summary.refusal{ui.colored_label(ERROR,refusal);}else if !current{ui.colored_label(ERROR,"Preview is outdated; prepare again before applying.");}
    egui::ScrollArea::vertical().id_salt("refactor-preview-rows").max_height(200.).show(ui,|ui|{for key in &summary.copied_profile_applications{ui.label(format!("Copy profile/tag application {key}"));}for(old,new)in &summary.id_mapping{ui.label(format!("Clone identity #{0} to fresh #{1}",old.0,new.0));}for r in summary.internal_remapped.iter().take(128){ui.label(format!("Remap #{} · {} · to #{}",r.owner.0,r.role,r.target.0));}for r in summary.internal_retained.iter().take(128){ui.label(format!("Keep internal identity #{} · {} · from #{}",r.target.0,r.role,r.owner.0));}for r in summary.external_targets_retained.iter().take(128){ui.label(format!("Keep external target #{} · {} · from #{}",r.target.0,r.role,r.owner.0));}for r in summary.incoming_targets_retained.iter().take(128){ui.label(format!("Incoming #{} · {} · retains target #{}",r.owner.0,r.role,r.target.0));}});
    ui.label(format!("Known typed references only. {} native fields and other opaque data are retained without reinterpretation; this is not an exhaustive global reference verdict.",summary.retained_native_field_count));
    let r=ui.add_enabled(current&&!edited,egui::Button::new("Apply reviewed clone / move"));self.bound("refactor:apply",&r);if r.clicked(){apply=plan;}let r=ui.button("Cancel preview (keep unfinished form)");self.bound("refactor:cancel",&r);cancel=r.clicked();
   }
   ui.label(&self.refactor_panel.note);let r=ui.button("Close clone/move");self.bound("refactor:close",&r);close=r.clicked();
  });
        self.refactor_panel.open = open && !close;
        if let Some(action) = raw {
            self.refactor_action(action);
        }
        if prepare {
            match refactor::prepare(&self.session.editor) {
                Ok(p) => {
                    self.refactor_panel.prepared = Some(p);
                    self.refactor_panel.note =
                        "Preview prepared without allocating live IDs or changing accepted data."
                            .into();
                }
                Err(e) => {
                    self.refactor_panel.note = e;
                    self.refactor_panel.prepared = None;
                }
            }
        }
        if let Some(plan) = apply {
            self.refactor_action(Action::Apply(Box::new(plan)));
        }
        if cancel {
            self.refactor_panel.prepared = None;
            self.refactor_panel.note =
                "Preview cancelled; accepted model, allocator and unfinished form retained.".into();
        }
        if let Some(redo) = history {
            self.refactor_panel.note = match self.typed_model_history(redo) {
                Ok(true) => "Model history applied; newer unfinished form retained.".into(),
                Ok(false) => "No model history in this direction.".into(),
                Err(e) => e,
            };
        }
    }
    pub(super) fn refactor_audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.refactor-ui/1","open":self.refactor_panel.open,"form":refactor::form(&self.session.editor.snapshot).ok().flatten(),"summary":self.refactor_panel.prepared.as_ref().map(|p|p.summary()),"applicable":self.refactor_panel.prepared.as_ref().and_then(|p|p.plan()).is_some_and(|p|p.check(&self.session.editor).is_ok()),"note":self.refactor_panel.note})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_preview_cancel_stale_guard_and_apply_keep_source_work_and_allocate_only_on_apply() {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/parity/p20/fixtures/values.kernspan"
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
        let mut form = refactor::Form {
            mode: Mode::CloneDefinition,
            definition: "1004".into(),
            name: "preview copy".into(),
            usages: String::new(),
            owner: String::new(),
            keep_external: true,
        };
        assert!(app.refactor_action(Action::SetForm(form.clone())));
        app.refactor_panel.open = true;
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
                |_| app.refactor_dialog(&ctx),
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
        click(&mut app, "refactor:prepare");
        assert!(
            app.refactor_panel
                .prepared
                .as_ref()
                .unwrap()
                .plan()
                .is_some()
        );
        assert_eq!(app.session.editor.snapshot, before);
        click(&mut app, "refactor:cancel");
        assert!(app.refactor_panel.prepared.is_none());
        assert_eq!(app.session.editor.snapshot, before);
        click(&mut app, "refactor:prepare");
        let field = egui::Id::new((
            "mbse-refactor",
            &app.session.editor.freshness().instance_id,
            "name",
        ));
        ctx.memory_mut(|m| m.request_focus(field));
        form.name = "newer retained name".into();
        assert!(app.refactor_action(Action::SetForm(form.clone())));
        render(&mut app, vec![]);
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
        assert!(!app.refactor_audit()["applicable"].as_bool().unwrap());
        let accepted = types::load(&app.session.editor.snapshot).unwrap();
        click(&mut app, "refactor:apply");
        assert_eq!(types::load(&app.session.editor.snapshot).unwrap(), accepted);
        assert_eq!(app.session.editor.snapshot.next_id, before.next_id);
        assert_eq!(app.history_scope(), ShortcutTarget::Widget);
        assert!(
            app.command_reason(crate::command_catalog::Id::Save, &app.history_scope())
                .is_some()
        );
        click(&mut app, "refactor:prepare");
        click(&mut app, "refactor:apply");
        let store = types::load(&app.session.editor.snapshot).unwrap();
        assert_eq!(
            store.definitions[&types::Id(1021)].name,
            "newer retained name"
        );
        assert_eq!(
            store.definitions[&types::Id(1004)],
            accepted.definitions[&types::Id(1004)]
        );
        assert_eq!(app.session.editor.snapshot.next_id, before.next_id + 6);
        assert_eq!(
            refactor::form(&app.session.editor.snapshot).unwrap(),
            Some(form)
        );
    }
}
