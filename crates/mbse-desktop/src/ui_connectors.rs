//! Explicit part-qualified semantic exchange authoring; presentation is separate.
use super::*;
use mbse_model::types::{
    self, Id,
    connectors::{self, Action, Form},
};
#[derive(Default)]
pub(super) struct ConnectorPanel {
    pub open: bool,
    owner: Option<Id>,
    selected: Option<Id>,
    note: String,
}
impl Workspace {
    fn connector_action(&mut self, action: Action) -> bool {
        let stamp = self.views.views.stamp(&self.session.editor);
        match self.views.views.dispatch(
            &mut self.session.editor,
            &stamp,
            mbse_model::native_views::NativeAction::Command(Command::Typed(
                types::Action::Connector(action),
            )),
        ) {
            Ok(_) => {
                self.connector_panel.note="Connector data and unfinished form retained. Save preserves both; model history leaves newer text intact.".into();
                true
            }
            Err(error) => {
                self.connector_panel.note = error;
                false
            }
        }
    }
    pub(super) fn connector_dialog(&mut self, ctx: &egui::Context) {
        if !self.connector_panel.open {
            return;
        }
        let instance = self.session.editor.freshness().instance_id;
        let mut open = true;
        let mut close = false;
        let mut owner = None;
        let mut select = None;
        let mut action = None;
        let mut raw = None;
        let mut seed = None;
        let mut history = None;
        egui::Window::new("Connectors and conveyed items").id(egui::Id::new("mbse-connectors")).open(&mut open).default_width(900.).default_height(950.).max_height((ctx.content_rect().height()-80.).max(240.)).vscroll(true).show(ctx,|ui|{
   ui.label("Each endpoint names a Part usage and a Port on its target type. Both Parts belong to the selected owner; source is Out/InOut, target In/InOut. Conveyed Value identity must match both interface payloads. Visual routing is unavailable.");
   let store=match types::load(&self.session.editor.snapshot){Ok(s)=>s,Err(error)=>{ui.colored_label(ERROR,error);let r=ui.button("Close connectors");self.bound("connectors:close",&r);close=r.clicked();return;}};
   ui.horizontal(|ui|{let ready=!self.views.occurrence_navigation_blocked(&self.session);let r=ui.add_enabled(ready&&self.session.editor.can_undo(),egui::Button::new("Undo model"));self.bound("connectors:undo-model",&r);if r.clicked(){history=Some(false);}let r=ui.add_enabled(ready&&self.session.editor.can_redo(),egui::Button::new("Redo model"));self.bound("connectors:redo-model",&r);if r.clicked(){history=Some(true);}});
   ui.label("Select owning Component definition");egui::ScrollArea::vertical().id_salt("connector-owners").max_height(100.).show(ui,|ui|{for d in store.definitions.values().filter(|d|d.kind==types::Kind::Component){let r=ui.selectable_label(self.connector_panel.owner==Some(d.id),format!("{} #{} · System#{}",d.name,d.id.0,d.system.0));self.bound(format!("connectors:owner:{}",d.id.0),&r);if r.clicked(){owner=Some(d.id);}}});
   if let Some(selected)=self.connector_panel.owner{ui.label(format!("Parts owned by Component #{}",selected.0));for usage in store.usages.values().filter(|u|u.owner==selected&&u.kind==types::UsageKind::Part){ui.label(format!("Part #{} · {} · target type#{}",usage.id.0,usage.name,usage.target.0));}}
   let r=ui.add_enabled(self.connector_panel.owner.is_some(),egui::Button::new("New connector draft"));self.bound("connectors:new",&r);if r.clicked(){action=Some(Action::Begin{owner:self.connector_panel.owner.unwrap()});}
   egui::CollapsingHeader::new("Port and value IDs").id_salt("connector-reference-help").show(ui,|ui|{for p in store.ports.values(){let i=&store.interfaces[&p.assignment.unwrap_or(p.contract)];ui.label(format!("Port#{} · {} · owner type#{} · {:?} · payload#{}",p.id.0,p.name,p.owner.0,i.direction,i.payload.0));}for d in store.definitions.values().filter(|d|matches!(d.kind,types::Kind::Value(_))){ui.label(format!("Value#{} · {}",d.id.0,d.name));}});
   let forms=match connectors::forms(&self.session.editor.snapshot){Ok(f)=>f,Err(error)=>{ui.colored_label(ERROR,error);let r=ui.button("Close connectors");self.bound("connectors:close",&r);close=r.clicked();return;}};
   for c in store.connectors.values(){ui.label(format!("Accepted connector#{} · {} · Part#{} Port#{} to Part#{} Port#{} · conveyed#{}",c.id.0,c.name,c.source.usage.0,c.source.port.0,c.target.usage.0,c.target.port.0,c.conveyed.0));let r=ui.button(format!("Edit connector#{}",c.id.0));self.bound(format!("connectors:edit:{}",c.id.0),&r);if r.clicked(){select=Some(c.id);if !forms.contains_key(&c.id){seed=Some(Action::Set{id:c.id,form:Form{owner:c.owner,name:c.name.clone(),source_usage:c.source.usage.0.to_string(),source_port:c.source.port.0.to_string(),target_usage:c.target.usage.0.to_string(),target_port:c.target.port.0.to_string(),conveyed:c.conveyed.0.to_string()}});}}}
   for(id,f)in &forms{let r=ui.selectable_label(self.connector_panel.selected==Some(*id),format!("Connector form #{} · {} · owner#{}",id.0,f.name,f.owner.0));self.bound(format!("connectors:form:{}",id.0),&r);if r.clicked(){select=Some(*id);}}
   if let Some((id,mut form))=self.connector_panel.selected.and_then(|id|forms.get(&id).cloned().map(|f|(id,f))){ui.separator();ui.label(format!("Connector #{} · owner#{} · text remains independent of accepted exchange",id.0,form.owner.0));let before=form.clone();for(label,key,text)in [("Connector name","name",&mut form.name),("Source Part usage ID","source-usage",&mut form.source_usage),("Source Port ID","source-port",&mut form.source_port),("Target Part usage ID","target-usage",&mut form.target_usage),("Target Port ID","target-port",&mut form.target_port),("Conveyed Value definition ID","conveyed",&mut form.conveyed)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(text).id(egui::Id::new(("mbse-connector",&instance,id.0,key))));self.bound(format!("connectors:field:{key}"),&r);});}let edited=form!=before;if edited{raw=Some(Action::Set{id,form});}let checked=connectors::preview(&self.session.editor.snapshot,id);if let Err(error)=&checked{ui.colored_label(ERROR,format!("Unapplied connector retained: {error}"));}let r=ui.add_enabled(checked.is_ok()&&!edited,egui::Button::new("Apply connector"));self.bound("connectors:apply",&r);if r.clicked(){action=Some(Action::Apply{id});}}
   ui.label(&self.connector_panel.note);let r=ui.button("Close connectors");self.bound("connectors:close",&r);close=r.clicked();
  });
        self.connector_panel.open = open && !close;
        if let Some(id) = owner {
            self.connector_panel.owner = Some(id);
        }
        if let Some(id) = select {
            self.connector_panel.selected = Some(id);
        }
        for change in [raw, seed].into_iter().flatten() {
            self.connector_action(change);
        }
        if let Some(action) = action {
            let next = Id(self.session.editor.snapshot.next_id);
            let begin = matches!(action, Action::Begin { .. });
            if self.connector_action(action) && begin {
                self.connector_panel.selected = Some(next);
            }
        }
        if let Some(redo) = history {
            self.connector_panel.note = match self.typed_model_history(redo) {
                Ok(true) => "Model history applied; current connector text retained.".into(),
                Ok(false) => "No model history in this direction.".into(),
                Err(error) => error,
            };
        }
    }
    pub(super) fn connectors_audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.connectors-ui/1","open":self.connector_panel.open,"owner":self.connector_panel.owner,"selected":self.connector_panel.selected,"store":types::load(&self.session.editor.snapshot).ok(),"forms":connectors::forms(&self.session.editor.snapshot).ok(),"note":self.connector_panel.note})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connector_text_focus_survives_row_insertion_and_blocks_workspace_history() {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/parity/p18a/fixtures/ports.kernspan"
        ))
        .unwrap();
        let instance = session.editor.freshness().instance_id;
        session.editor = mbse_model::stage1::Editor::new(
            serde_json::from_value(v["snapshot"].clone()).unwrap(),
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
        let id = Id(app.session.editor.snapshot.next_id);
        assert!(app.connector_action(Action::Begin { owner: Id(1004) }));
        let f = Form {
            owner: Id(1004),
            name: "pending connector".into(),
            source_usage: "1009".into(),
            source_port: "1015".into(),
            target_usage: "1010".into(),
            target_port: "1016".into(),
            conveyed: "1005?".into(),
        };
        assert!(app.connector_action(Action::Set {
            id,
            form: f.clone()
        }));
        app.connector_panel.open = true;
        app.connector_panel.owner = Some(Id(1004));
        app.connector_panel.selected = Some(id);
        let render = |app: &mut Workspace| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(2200., 1600.),
                    )),
                    focused: true,
                    ..Default::default()
                },
                |_| app.connector_dialog(&ctx),
            )
            .drop_without_applying_deltas();
        };
        for _ in 0..3 {
            render(&mut app);
        }
        let field = egui::Id::new((
            "mbse-connector",
            &app.session.editor.freshness().instance_id,
            id.0,
            "conveyed",
        ));
        assert!(ctx.read_response(field).unwrap().enabled());
        ctx.memory_mut(|m| m.request_focus(field));
        let second = Id(app.session.editor.snapshot.next_id);
        assert!(app.connector_action(Action::Begin { owner: Id(1004) }));
        app.connector_panel.selected = Some(id);
        render(&mut app);
        assert!(ctx.read_response(field).unwrap().enabled());
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
        assert_eq!(
            connectors::forms(&app.session.editor.snapshot).unwrap()[&id],
            f
        );
        assert!(
            connectors::forms(&app.session.editor.snapshot)
                .unwrap()
                .contains_key(&second)
        );
        assert_eq!(app.history_scope(), ShortcutTarget::Widget);
        for command in [
            crate::command_catalog::Id::Save,
            crate::command_catalog::Id::Undo,
            crate::command_catalog::Id::Ports,
        ] {
            assert!(app.command_reason(command, &app.history_scope()).is_some());
        }
    }
}
