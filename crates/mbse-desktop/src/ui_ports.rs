//! P17a semantic interface/port authoring; typed view reuse belongs to P17b.
use super::*;
use mbse_model::types::{
    self, Id,
    ports::{self, Action, Direction},
};
#[derive(Default)]
pub(super) struct PortPanel {
    pub open: bool,
    owner: Option<Id>,
    port: Option<Id>,
    note: String,
}
impl Workspace {
    fn port_action(&mut self, action: Action) -> bool {
        match self.session.editor.command(
            &self.session.editor.freshness(),
            Command::Typed(types::Action::Port(action)),
        ) {
            Ok(_) => {
                self.port_panel.note="Port/interface change retained; Save preserves accepted data and unfinished forms.".into();
                true
            }
            Err(error) => {
                self.port_panel.note = error;
                false
            }
        }
    }
    pub(super) fn port_dialog(&mut self, ctx: &egui::Context) {
        if !self.port_panel.open {
            return;
        }
        let instance = self.session.editor.freshness().instance_id;
        let mut open = true;
        let mut close = false;
        let mut action = None;
        let mut interface_change = None;
        let mut port_change = None;
        let mut port_choice = None;
        let mut owner_choice = None;
        let mut history = None;
        egui::Window::new("Ports and interfaces").id(egui::Id::new("mbse-ports-interfaces")).open(&mut open).default_width(820.).default_height(900.).max_height((ctx.content_rect().height()-80.).max(240.)).vscroll(true).show(ctx,|ui|{
   ui.label("Interfaces declare one exact Value definition payload and flow direction. Compatible assignment requires equal payload identity, direction and System. Port presentation/reuse follows in P17b.");
   let store=match types::load(&self.session.editor.snapshot){Ok(s)=>s,Err(error)=>{ui.colored_label(ERROR,error);let r=ui.button("Close ports");self.bound("ports:close",&r);close=r.clicked();return;}};
   ui.horizontal(|ui|{let ready=!self.views.occurrence_navigation_blocked(&self.session);let r=ui.add_enabled(ready&&self.session.editor.can_undo(),egui::Button::new("Undo model"));self.bound("ports:undo-model",&r);if r.clicked(){history=Some(false);}let r=ui.add_enabled(ready&&self.session.editor.can_redo(),egui::Button::new("Redo model"));self.bound("ports:redo-model",&r);if r.clicked(){history=Some(true);}});
   let system=self.session.editor.selected.and_then(|id|self.session.editor.snapshot.records.get(&id)).map(|r|r.scope).unwrap_or(RecordId(1));
   let mut form=match ports::interface_form(&self.session.editor.snapshot){Ok(f)=>f.unwrap_or(ports::InterfaceForm{system,name:String::new(),payload:String::new(),direction:Direction::In}),Err(error)=>{ui.colored_label(ERROR,error);let r=ui.button("Close ports");self.bound("ports:close",&r);close=r.clicked();return;}};let before=form.clone();
   let systems:Vec<_>=self.session.editor.snapshot.records.values().filter(|r|r.kind==RecordKind::System).map(|r|(r.id,r.name.clone())).collect();
   ui.separator();ui.horizontal_wrapped(|ui|{ui.label(format!("Interface System #{}",form.system.0));for(id,name)in systems{let r=ui.selectable_value(&mut form.system,id,format!("{name} #{}",id.0));self.bound(format!("ports:system:{}",id.0),&r);}});
   for(label,key,text)in [("Interface name","name",&mut form.name),("Payload Value definition ID","payload",&mut form.payload)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(text).id(egui::Id::new(("mbse-interface",&instance,key))));self.bound(format!("ports:interface:{key}"),&r);});}
   ui.horizontal(|ui|{for direction in [Direction::In,Direction::Out,Direction::InOut]{let r=ui.selectable_value(&mut form.direction,direction,format!("{direction:?}"));self.bound(format!("ports:direction:{direction:?}"),&r);}});
   let edited=form!=before;if edited{interface_change=Some(Action::SetInterfaceForm(form));}let checked=ports::preview_interface(&self.session.editor.snapshot);if let Err(error)=&checked{ui.colored_label(ERROR,format!("Unapplied interface retained: {error}"));}let r=ui.add_enabled(checked.is_ok()&&!edited,egui::Button::new("Add interface"));self.bound("ports:add-interface",&r);if r.clicked(){action=Some(Action::AddInterface);}
   egui::ScrollArea::vertical().id_salt("interface-rows").max_height(130.).show(ui,|ui|{for i in store.interfaces.values(){ui.label(format!("Interface #{} · {} · payload#{} · {:?} · System#{}",i.id.0,i.name,i.payload.0,i.direction,i.system.0));}});
   ui.separator();ui.label("Select Component owner");egui::ScrollArea::vertical().id_salt("port-owner-rows").max_height(110.).show(ui,|ui|{for d in store.definitions.values().filter(|d|d.kind==types::Kind::Component){let r=ui.selectable_label(self.port_panel.owner==Some(d.id),format!("{} #{} · System#{}",d.name,d.id.0,d.system.0));self.bound(format!("ports:owner:{}",d.id.0),&r);if r.clicked(){owner_choice=Some(d.id);}}});
   let r=ui.add_enabled(self.port_panel.owner.is_some(),egui::Button::new("New port draft"));self.bound("ports:new-port",&r);if r.clicked(){action=Some(Action::BeginPort{owner:self.port_panel.owner.unwrap()});}
   for p in store.ports.values(){ui.label(format!("Accepted port #{} · {} · owner#{} · contract#{} · assignment{:?}",p.id.0,p.name,p.owner.0,p.contract.0,p.assignment.map(|id|id.0)));}
   let forms=ports::forms(&self.session.editor.snapshot).unwrap_or_default();for(id,f)in &forms{let r=ui.selectable_label(self.port_panel.port==Some(*id),format!("Port form #{} · {} · owner#{}",id.0,f.name,f.owner.0));self.bound(format!("ports:form:{}",id.0),&r);if r.clicked(){port_choice=Some(*id);}}
   if let Some((id,mut form))=self.port_panel.port.and_then(|id|forms.get(&id).cloned().map(|f|(id,f))){let before=form.clone();ui.separator();ui.label(format!("Port #{} · owner#{} · unfinished text independent of accepted data",id.0,form.owner.0));for(label,key,text)in [("Port name","name",&mut form.name),("Declared interface ID","contract",&mut form.contract),("Assigned interface ID (empty uses declaration)","assignment",&mut form.assignment)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(text).id(egui::Id::new(("mbse-port",&instance,id.0,key))));self.bound(format!("ports:port:{key}"),&r);});}
    let edited=form!=before;if edited{port_change=Some(Action::SetPortForm{id,form});}let checked=ports::preview(&self.session.editor.snapshot,id);if let Err(error)=&checked{ui.colored_label(ERROR,format!("Unapplied port retained: {error}"));}let r=ui.add_enabled(checked.is_ok()&&!edited,egui::Button::new("Apply port"));self.bound("ports:apply-port",&r);if r.clicked(){action=Some(Action::ApplyPort{id});}
   }
   ui.label(&self.port_panel.note);let r=ui.button("Close ports");self.bound("ports:close",&r);close=r.clicked();
  });
        self.port_panel.open = open && !close;
        if let Some(owner) = owner_choice {
            self.port_panel.owner = Some(owner);
        }
        if let Some(port) = port_choice {
            self.port_panel.port = Some(port);
        }
        for change in [interface_change, port_change].into_iter().flatten() {
            self.port_action(change);
        }
        if let Some(action) = action {
            let next = Id(self.session.editor.snapshot.next_id);
            let begin = matches!(action, Action::BeginPort { .. });
            if self.port_action(action) && begin {
                self.port_panel.port = Some(next);
            }
        }
        if let Some(redo) = history {
            let result = if redo {
                self.session
                    .editor
                    .redo_model(&self.session.editor.freshness())
            } else {
                self.session
                    .editor
                    .undo_model(&self.session.editor.freshness())
            };
            self.port_panel.note = match result {
                Ok(true) => "Model history applied; current unfinished forms retained.".into(),
                Ok(false) => "No model history in this direction.".into(),
                Err(error) => error,
            };
        }
    }
    pub(super) fn ports_audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.ports-ui/1","open":self.port_panel.open,"owner":self.port_panel.owner,"port":self.port_panel.port,"store":types::load(&self.session.editor.snapshot).ok(),"interface_form":ports::interface_form(&self.session.editor.snapshot).ok().flatten(),"forms":ports::forms(&self.session.editor.snapshot).ok(),"note":self.port_panel.note})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_port_field_focus_and_unfinished_text_survive_interface_row_insertion() {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        session.editor.snapshot.extensions = Some(mbse_model::extensions::Extensions::default());
        let mut define = |name: &str, kind: types::Kind| {
            let id = Id(session.editor.snapshot.next_id);
            session
                .editor
                .command(
                    &session.editor.freshness(),
                    Command::Typed(types::Action::AddDefinition {
                        system: RecordId(1),
                        name: name.into(),
                        kind,
                    }),
                )
                .unwrap();
            id
        };
        let owner = define("Device", types::Kind::Component);
        let value = define("Real", types::Kind::Value(types::Scalar::Real));
        let ctx = egui::Context::default();
        let mut app = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        let port = Id(app.session.editor.snapshot.next_id);
        assert!(app.port_action(Action::BeginPort { owner }));
        let form = ports::PortForm {
            owner,
            name: "port draft".into(),
            contract: String::new(),
            assignment: "incomplete#".into(),
        };
        assert!(app.port_action(Action::SetPortForm {
            id: port,
            form: form.clone()
        }));
        app.port_panel.open = true;
        app.port_panel.owner = Some(owner);
        app.port_panel.port = Some(port);
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
                |_| app.port_dialog(&ctx),
            )
            .drop_without_applying_deltas();
        };
        for _ in 0..3 {
            render(&mut app);
        }
        let field = egui::Id::new((
            "mbse-port",
            &app.session.editor.freshness().instance_id,
            port.0,
            "assignment",
        ));
        assert!(ctx.read_response(field).unwrap().enabled());
        ctx.memory_mut(|m| m.request_focus(field));
        assert!(
            app.port_action(Action::SetInterfaceForm(ports::InterfaceForm {
                system: RecordId(1),
                name: "Inserted interface".into(),
                payload: value.0.to_string(),
                direction: Direction::In
            }))
        );
        assert!(app.port_action(Action::AddInterface));
        render(&mut app);
        assert!(ctx.read_response(field).unwrap().enabled());
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
        assert_eq!(
            ports::forms(&app.session.editor.snapshot).unwrap()[&port],
            form
        );
        assert_eq!(app.history_scope(), ShortcutTarget::Widget);
        assert!(
            app.command_reason(crate::command_catalog::Id::Save, &app.history_scope())
                .is_some()
        );
    }
}
