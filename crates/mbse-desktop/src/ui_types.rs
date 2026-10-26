//! Types share the native editor, persistence and model history; forms are durable journals.
use super::*;
use mbse_model::types::{self, Action, Id, Kind, Scalar};
pub(super) struct TypePanel {
    pub open: bool,
    selected: Option<Id>,
    form: Option<Id>,
    note: String,
}
impl Default for TypePanel {
    fn default() -> Self {
        Self {
            open: false,
            selected: None,
            form: None,
            note: String::new(),
        }
    }
}
impl Workspace {
    pub(super) fn reset_type_context(&mut self) {
        self.type_panel.selected = None;
        self.type_panel.form = None;
        self.type_panel.open = false;
    }
    fn typed_action(&mut self, action: Action) -> bool {
        match self
            .session
            .editor
            .command(&self.session.editor.freshness(), Command::Typed(action))
        {
            Ok(_) => {
                self.type_panel.note="Typed change retained in native workspace; Save keeps accepted data and unfinished forms.".into();
                true
            }
            Err(error) => {
                self.type_panel.note = error;
                false
            }
        }
    }
    pub(super) fn type_dialog(&mut self, ctx: &egui::Context) {
        if !self.type_panel.open {
            return;
        }
        let native_instance = self.session.editor.freshness().instance_id;
        let mut open = true;
        let mut action = None;
        let mut definition_change = None;
        let mut property_change = None;
        let mut usage_change = None;
        let mut usage_choice = None;
        let mut select = None;
        let mut choose_form = None;
        let mut close = false;
        let mut history = None;
        egui::Window::new("Types and properties").id(egui::Id::new("mbse-types-properties")).open(&mut open).default_width(850.).default_height(900.).vscroll(true).max_height((ctx.content_rect().height()-80.).max(240.)).show(ctx,|ui|{
   ui.label("Component definitions own typed properties. Value definitions declare Boolean, Integer, Real or Text types. System scope is membership, not containment.");
   if self.session.editor.snapshot.extensions.is_none(){ui.label("Open a migrated native copy to enable extensions. Use --migrate-extensions SOURCE --destination NEW with an owned --state-dir; original native files stay protected.");let r=ui.button("Close types");self.bound("types:close",&r);close=r.clicked();return;}
   let store=match types::load(&self.session.editor.snapshot){Ok(store)=>store,Err(error)=>{ui.colored_label(ERROR,error);let r=ui.button("Close types");self.bound("types:close",&r);close=r.clicked();return;}};
   ui.horizontal_wrapped(|ui|{let ready=!self.views.occurrence_navigation_blocked(&self.session);let r=ui.add_enabled(ready&&self.session.editor.can_undo(),egui::Button::new("Undo model"));self.bound("types:undo-model",&r);if r.clicked(){history=Some(false);}let r=ui.add_enabled(ready&&self.session.editor.can_redo(),egui::Button::new("Redo model"));self.bound("types:redo-model",&r);if r.clicked(){history=Some(true);}});
   let retained=match types::definition_form(&self.session.editor.snapshot){Ok(form)=>form,Err(error)=>{ui.colored_label(ERROR,error);let r=ui.button("Close types");self.bound("types:close",&r);close=r.clicked();return;}};
   let default_system=self.session.editor.selected.and_then(|id|self.session.editor.snapshot.records.get(&id)).map(|r|r.scope).or_else(||self.session.editor.snapshot.records.values().find(|r|r.kind==RecordKind::System).map(|r|r.id)).unwrap_or(RecordId(1));
   let mut definition=retained.unwrap_or(types::DefinitionForm{system:default_system,name:String::new(),value:false,scalar:Scalar::Real});let before_definition=definition.clone();
   let systems:Vec<_>=self.session.editor.snapshot.records.values().filter(|r|r.kind==RecordKind::System).map(|r|(r.id,r.name.clone())).collect();
   ui.separator();ui.horizontal_wrapped(|ui|{ui.label(format!("System scope #{}",definition.system.0));for(id,name)in systems{let r=ui.selectable_value(&mut definition.system,id,format!("{} #{}",name,id.0));self.bound(format!("types:system:{}",id.0),&r);}});
   ui.horizontal(|ui|{ui.label("Definition name");let r=ui.add(egui::TextEdit::singleline(&mut definition.name).id(egui::Id::new(("mbse-typed-definition",&native_instance))));self.bound("types:definition-name",&r);let r=ui.checkbox(&mut definition.value,"Value definition");self.bound("types:value-definition",&r);});
   if definition.value{ui.horizontal(|ui|{for scalar in [Scalar::Boolean,Scalar::Integer,Scalar::Real,Scalar::Text]{let r=ui.selectable_value(&mut definition.scalar,scalar,format!("{scalar:?}"));self.bound(format!("types:scalar:{scalar:?}"),&r);}});}
   let definition_edited=definition!=before_definition;if definition_edited{definition_change=Some(Action::SetDefinitionForm{form:definition.clone()});}
   let r=ui.add_enabled(!definition_edited,egui::Button::new("Add definition"));self.bound("types:add-definition",&r);if r.clicked(){action=Some(Action::AddDefinition{system:definition.system,name:definition.name.clone(),kind:definition.kind()});}
   egui::ScrollArea::vertical().id_salt("typed-definition-rows").max_height(160.).show(ui,|ui|{for d in store.definitions.values(){let r=ui.selectable_label(self.type_panel.selected==Some(d.id),format!("{} · {:?} #{} · System #{}",d.name,d.kind,d.id.0,d.system.0));self.bound(format!("types:definition:{}",d.id.0),&r);if r.clicked(){select=Some(d.id);}}});
   if let Some(owner)=self.type_panel.selected.filter(|id|store.definitions.get(id).is_some_and(|d|d.kind==Kind::Component)){
    let r=ui.button("New property draft");self.bound("types:new-property",&r);if r.clicked(){action=Some(Action::BeginProperty{owner});}
    for p in store.properties.values().filter(|p|p.owner==owner){ui.label(format!("Accepted property #{}: {} · value type #{} · {}..{}",p.id.0,p.name,p.value_type.0,p.multiplicity.lower,p.multiplicity.upper.map_or("*".into(),|v|v.to_string())));let r=ui.button(format!("Edit property #{}",p.id.0));self.bound(format!("types:edit-property:{}",p.id.0),&r);if r.clicked(){choose_form=Some(p.id);if !types::forms(&self.session.editor.snapshot).unwrap_or_default().contains_key(&p.id){action=Some(Action::SetForm{id:p.id,form:types::Form{owner:p.owner,name:p.name.clone(),value_type:p.value_type.0.to_string(),lower:p.multiplicity.lower.to_string(),upper:p.multiplicity.upper.map_or("*".into(),|n|n.to_string())}});}}}
   }
   let forms=types::forms(&self.session.editor.snapshot).unwrap_or_default();for(id,form)in &forms{let r=ui.selectable_label(self.type_panel.form==Some(*id),format!("Retained property form #{} · owner #{} · {}",id.0,form.owner.0,form.name));self.bound(format!("types:form:{}",id.0),&r);if r.clicked(){choose_form=Some(*id);}}
   if let Some((id,mut form))=self.type_panel.form.and_then(|id|forms.get(&id).cloned().map(|form|(id,form))){
    ui.separator();ui.label(format!("Property form #{}; text is retained independently of accepted property data.",id.0));let before=form.clone();
    for(label,key,text)in [("Property name","name",&mut form.name),("Value type ID","value-type",&mut form.value_type),("Lower multiplicity","lower",&mut form.lower),("Upper multiplicity or *","upper",&mut form.upper)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(text).id(egui::Id::new(("mbse-typed-property",&native_instance,id.0,key))));self.bound(format!("types:property:{key}"),&r);});}
    let edited=form!=before;if edited{property_change=Some(Action::SetForm{id,form});}
    let checked=types::preview(&self.session.editor.snapshot,id);if let Err(error)=&checked{ui.colored_label(ERROR,format!("Unapplied form retained: {error}"));}
    let r=ui.add_enabled(checked.is_ok()&&!edited,egui::Button::new("Apply property"));self.bound("types:apply-property",&r);if r.clicked(){action=Some(Action::ApplyProperty{id});}
   }
   ui.separator();ui.label("Parts and references · owner and target use Component definition IDs");
   let r=ui.button("New usage form (replace current form)");self.bound("types:usage:new",&r);if r.clicked(){usage_choice=Some(Action::SetUsageForm{form:types::UsageForm{id:None,name:String::new(),owner:self.type_panel.selected.map_or(String::new(),|id|id.0.to_string()),target:String::new(),kind:types::UsageKind::Part}});}
   for usage in store.usages.values(){let r=ui.button(format!("Edit {:?} #{} · {} · owner#{} uses type#{}",usage.kind,usage.id.0,usage.name,usage.owner.0,usage.target.0));self.bound(format!("types:usage:edit:{}",usage.id.0),&r);if r.clicked(){usage_choice=Some(Action::SetUsageForm{form:types::UsageForm{id:Some(usage.id),name:usage.name.clone(),owner:usage.owner.0.to_string(),target:usage.target.0.to_string(),kind:usage.kind}});}}
   if let Ok(Some(mut form))=types::usage_form(&self.session.editor.snapshot){
    let before=form.clone();ui.label(form.id.map_or("New usage (unallocated)".into(),|id|format!("Usage #{}; editing owner moves this same identity",id.0)));
    for(label,key,text)in [("Usage name","name",&mut form.name),("Owner definition ID","owner",&mut form.owner),("Target definition ID","target",&mut form.target)]{ui.horizontal(|ui|{ui.label(label);let r=ui.add(egui::TextEdit::singleline(text).id(egui::Id::new(("mbse-usage",&native_instance,key))));self.bound(format!("types:usage:{key}"),&r);});}
    ui.horizontal(|ui|{for kind in [types::UsageKind::Part,types::UsageKind::Reference]{let r=ui.selectable_value(&mut form.kind,kind,format!("{kind:?}"));self.bound(format!("types:usage:kind:{kind:?}"),&r);}});
    let edited=form!=before;if edited{usage_change=Some(Action::SetUsageForm{form});}
    let checked=types::preview_usage(&self.session.editor.snapshot);if let Err(error)=&checked{ui.colored_label(ERROR,format!("Unapplied usage retained: {error}"));}
    let r=ui.add_enabled(checked.is_ok()&&!edited,egui::Button::new("Apply usage / move"));self.bound("types:usage:apply",&r);if r.clicked(){action=Some(Action::ApplyUsage);}
   }
   ui.label(&self.type_panel.note);let r=ui.button("Close types");self.bound("types:close",&r);close=r.clicked();
  });
        self.type_panel.open = open && !close;
        if let Some(id) = select {
            self.type_panel.selected = Some(id);
        }
        if let Some(id) = choose_form {
            self.type_panel.form = Some(id);
        }
        if let Some(change) = definition_change {
            self.typed_action(change);
        }
        for change in [property_change, usage_change, usage_choice]
            .into_iter()
            .flatten()
        {
            self.typed_action(change);
        }
        if let Some(action) = action {
            let next = Id(self.session.editor.snapshot.next_id);
            let add = matches!(action, Action::AddDefinition { .. });
            let draft = matches!(action, Action::BeginProperty { .. });
            if self.typed_action(action) {
                if add {
                    self.type_panel.selected = Some(next);
                }
                if draft {
                    self.type_panel.form = Some(next);
                }
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
            self.type_panel.note = match result {
                Ok(true) => "Model history restored; current property text retained.".into(),
                Ok(false) => "No model history in this direction.".into(),
                Err(error) => error,
            };
        }
    }
    pub(super) fn typed_structure_browser(&mut self, ui: &mut egui::Ui) {
        if self.session.editor.snapshot.extensions.is_none() {
            return;
        }
        let store = match types::load(&self.session.editor.snapshot) {
            Ok(store) => store,
            Err(error) => {
                ui.colored_label(ERROR, error);
                return;
            }
        };
        if store.definitions.is_empty() {
            return;
        }
        ui.separator();
        ui.label("Typed definitions · System membership (not containment)");
        for definition in store.definitions.values() {
            let r = ui.selectable_label(
                self.type_panel.selected == Some(definition.id),
                format!(
                    "{} #{} · {:?} · member of System #{}",
                    definition.name, definition.id.0, definition.kind, definition.system.0
                ),
            );
            self.bound(format!("tree:definition:{}", definition.id.0), &r);
            if r.clicked() {
                self.type_panel.selected = Some(definition.id);
                self.type_panel.open = true;
            }
            ui.indent(("owned-usages", definition.id.0), |ui| {
                for usage in store.usages.values().filter(|u| u.owner == definition.id) {
                    let relation = if usage.kind == types::UsageKind::Part {
                        "Part containment"
                    } else {
                        "Reference (not containment)"
                    };
                    let target = &store.definitions[&usage.target];
                    let r = ui.label(format!(
                        "{relation} #{} · {} uses type {} #{}",
                        usage.id.0, usage.name, target.name, target.id.0
                    ));
                    self.bound(format!("tree:usage:{}", usage.id.0), &r);
                }
            });
        }
    }
    pub(super) fn typed_audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.types-ui/1","open":self.type_panel.open,"selected":self.type_panel.selected,"form":self.type_panel.form,"store":types::load(&self.session.editor.snapshot).ok(),"forms":types::forms(&self.session.editor.snapshot).ok(),"definition_form":types::definition_form(&self.session.editor.snapshot).ok().flatten(),"usage_form":types::usage_form(&self.session.editor.snapshot).ok().flatten(),"note":self.type_panel.note})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_property_field_focus_and_raw_form_survive_definition_row_insertion() {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        session.editor.snapshot.extensions = Some(mbse_model::extensions::Extensions::default());
        let ctx = egui::Context::default();
        let mut app = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        let owner = Id(app.session.editor.snapshot.next_id);
        assert!(app.typed_action(Action::AddDefinition {
            system: RecordId(1),
            name: "Subsystem".into(),
            kind: Kind::Component
        }));
        let property = Id(app.session.editor.snapshot.next_id);
        assert!(app.typed_action(Action::BeginProperty { owner }));
        let form = types::Form {
            owner,
            name: "unfinished property".into(),
            value_type: String::new(),
            lower: "1".into(),
            upper: "1..".into(),
        };
        assert!(app.typed_action(Action::SetForm {
            id: property,
            form: form.clone()
        }));
        app.type_panel.open = true;
        app.type_panel.selected = Some(owner);
        app.type_panel.form = Some(property);
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
                |_| app.type_dialog(&ctx),
            )
            .drop_without_applying_deltas();
        };
        // The real window/scroll renderer publishes a live TextEdit response on
        // the third startup frame; retain the exact enabled/focus assertions.
        for _ in 0..3 {
            render(&mut app);
        }

        let field = egui::Id::new((
            "mbse-typed-property",
            &app.session.editor.freshness().instance_id,
            property.0,
            "upper",
        ));
        assert!(
            ctx.read_response(field).unwrap().enabled(),
            "Property response must be live: {:?}",
            ctx.read_response(field)
        );
        ctx.memory_mut(|m| m.request_focus(field));
        assert!(app.typed_action(Action::AddDefinition {
            system: RecordId(1),
            name: "Another row".into(),
            kind: Kind::Value(Scalar::Real)
        }));
        render(&mut app);
        assert!(
            ctx.read_response(field).unwrap().enabled(),
            "Property response must be live: {:?}",
            ctx.read_response(field)
        );
        assert_eq!(ctx.memory(|m| m.focused()), Some(field));
        assert_eq!(
            types::forms(&app.session.editor.snapshot).unwrap()[&property],
            form
        );
    }
}
