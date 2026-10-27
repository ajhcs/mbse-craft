//! Native flat specification proposals over the existing typed authority.
use super::*;
use mbse_model::{
    native_views::{self as nv, Stamp},
    types::{
        self as ty, Id,
        parametrics::{self as pm, Form, SourceForm},
    },
};
#[derive(Clone)]
struct Owner {
    instance: String,
    document: mbse_model::stage1::DocumentId,
}
#[derive(Clone)]
struct Pending {
    owner: Owner,
    form: Form,
}
#[derive(Default)]
pub(super) struct Panel {
    pub open: bool,
    pub specification: Option<Id>,
    pub current: Option<Form>,
    owner: Option<Owner>,
    pending: Option<Pending>,
    pub note: String,
}
fn scope(f: &Form) -> Option<Id> {
    match f {
        Form::Specification { id, .. } => *id,
        _ => f.specification(),
    }
}
fn family(f: &Form) -> &'static str {
    match f {
        Form::Specification { .. } => "Specification",
        Form::Parameter { .. } => "Parameter",
        Form::Binding { .. } => "Binding",
    }
}
impl Workspace {
    fn pm_owner(&self) -> Owner {
        Owner {
            instance: self.session.editor.freshness().instance_id,
            document: self.session.editor.snapshot.document_id.clone(),
        }
    }
    fn pm_owner_matches(&self, o: &Owner) -> bool {
        o.instance == self.session.editor.freshness().instance_id
            && o.document == self.session.editor.snapshot.document_id
    }
    pub(super) fn pm_pending(&self) -> bool {
        self.parametrics.pending.is_some()
    }
    pub(super) fn pm_close_blocked(&mut self) -> bool {
        if self.pm_pending() {
            self.parametrics.open = true;
            self.parametrics.note="New text remains in this window. Admit it, copy it, or explicitly discard it before closing.".into();
            true
        } else {
            false
        }
    }
    pub(super) fn pm_ready(&self) -> bool {
        !self.dock_blocked_without_parametric()
            && ty::load(&self.session.editor.snapshot).is_ok()
            && pm::forms(&self.session.editor.snapshot).is_ok()
    }
    pub(super) fn pm_open(&mut self) {
        self.parametrics.open = true;
        if self.parametrics.current.is_none() {
            let f = self.pm_new_form("Specification");
            let _ = self.pm_pick(f, None);
        }
    }
    fn pm_new_form(&self, family: &str) -> Form {
        let specification = self.parametrics.specification.unwrap_or(Id(0));
        match family {
            "Parameter" => Form::Parameter {
                specification,
                id: None,
                name: String::new(),
                quantity: quantity(mbse_model::stage1_calc::UnitId::ScalarOne),
            },
            "Binding" => Form::Binding {
                specification,
                id: None,
                parameter: String::new(),
                source: SourceForm::NativeInput {
                    calculation: String::new(),
                    name: String::new(),
                },
            },
            _ => Form::Specification {
                id: None,
                system: self
                    .session
                    .editor
                    .snapshot
                    .records
                    .values()
                    .find(|r| r.kind == RecordKind::System)
                    .map_or(String::new(), |r| r.id.0.to_string()),
                name: String::new(),
                calculation: String::new(),
            },
        }
    }
    pub(super) fn pm_pick(&mut self, f: Form, stamp: Option<&Stamp>) -> Result<(), String> {
        if self.pm_pending() {
            return Err(
                "Admit, copy or explicitly discard new text before changing selection.".into(),
            );
        }
        if self.dock_blocked_without_parametric() {
            return Err("Finish the gesture/write or close the other editor before selecting specification data.".into());
        }
        if stamp.is_some_and(|s| *s != self.views.views.stamp(&self.session.editor)) {
            return Err(
                "Specification selection source changed; original selection retained.".into(),
            );
        }
        let f = pm::forms(&self.session.editor.snapshot)?
            .get(&f.key())
            .cloned()
            .unwrap_or(f);
        self.parametrics.specification = scope(&f);
        self.parametrics.current = Some(f);
        self.parametrics.owner = Some(self.pm_owner());
        self.parametrics.open = true;
        Ok(())
    }
    pub(super) fn pm_model(&mut self, stamp: &Stamp, action: pm::Action) -> Result<(), String> {
        if !self.pm_ready() {
            return Err("Finish pending work or open an admitted native model copy before specification editing.".into());
        }
        let result = self
            .views
            .views
            .dispatch(
                &mut self.session.editor,
                stamp,
                nv::NativeAction::Command(Command::Typed(ty::Action::Parametric(action))),
            )
            .map(|_| ());
        self.views.reconcile();
        result
    }
    pub(super) fn pm_set_form(&mut self, stamp: &Stamp, f: Form) -> Result<(), String> {
        let result = self.pm_model(stamp, pm::Action::Set(f.clone()));
        match &result {
            Ok(()) => {
                self.parametrics.pending = None;
                self.parametrics.note =
                    "Proposal retained. Apply to change accepted specification data.".into();
            }
            Err(e) => {
                self.parametrics.pending = Some(Pending {
                    owner: self
                        .parametrics
                        .owner
                        .clone()
                        .unwrap_or_else(|| self.pm_owner()),
                    form: f.clone(),
                });
                self.parametrics.note =
                    format!("{e} New text remains in this window; accepted data is unchanged.");
            }
        }
        self.parametrics.current = Some(f);
        result
    }
    fn pm_apply(&mut self, stamp: &Stamp, f: &Form) -> Result<(), String> {
        let id_value = f.id().unwrap_or(Id(self.session.editor.snapshot.next_id));
        self.pm_model(stamp, pm::Action::Apply(f.key()))?;
        let mut f = f.clone();
        match &mut f {
            Form::Specification { id, .. }
            | Form::Parameter { id, .. }
            | Form::Binding { id, .. } => *id = Some(id_value),
        }
        self.parametrics.specification = scope(&f);
        self.parametrics.current = Some(f);
        self.parametrics.note =
            "Accepted data updated. Model Undo/Redo preserves newer proposals.".into();
        Ok(())
    }
    fn pm_text(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        label: &str,
        text: &mut String,
        slot: &str,
        instance: &str,
    ) {
        ui.horizontal(|ui| {
            ui.label(label);
            let r = ui.add(
                egui::TextEdit::singleline(text)
                    .char_limit(4096)
                    .desired_width(490.)
                    .id(egui::Id::new(("parametric-proposal", instance, slot, key))),
            );
            self.bound(format!("parametric:field:{key}"), &r);
        });
    }
    pub(super) fn pm_dialog(&mut self, ctx: &egui::Context) {
        if !self.parametrics.open {
            return;
        }
        if self.parametrics.current.is_none() {
            self.parametrics.current = Some(self.pm_new_form("Specification"));
            self.parametrics.owner = Some(self.pm_owner());
        }
        let store = match ty::load(&self.session.editor.snapshot) {
            Ok(s) => s,
            Err(e) => {
                let pending = self.parametrics.pending.clone();
                egui::Window::new("Parametric specifications and bindings")
                    .id(egui::Id::new("mbse-parametrics"))
                    .show(ctx, |ui| {
                        ui.colored_label(ERROR, e);
                        if let Some(p) = pending {
                            ui.label(
                                "New text remains in this window; accepted data is unchanged.",
                            );
                            let r = ui.button("Copy retained new proposal");
                            self.bound("parametric:Copy new text", &r);
                            if r.clicked() {
                                ctx.copy_text(serde_json::to_string_pretty(&p.form).unwrap());
                            }
                            let r = ui.button("Discard unadmitted new text");
                            self.bound("parametric:Discard new text", &r);
                            if r.clicked() {
                                self.parametrics.pending = None;
                                self.parametrics.current = None;
                                self.parametrics.owner = None;
                                self.parametrics.specification = None;
                            }
                        }
                        let r = ui.button("Close parametric editor");
                        self.bound("parametric:Close", &r);
                        if r.clicked() && !self.pm_close_blocked() {
                            self.parametrics.open = false;
                        }
                    });
                return;
            }
        };
        let forms = pm::forms(&self.session.editor.snapshot).unwrap_or_default();
        let blueprint = self.parametrics.current.clone().unwrap();
        let owner_ok = self
            .parametrics
            .pending
            .as_ref()
            .map(|p| &p.owner)
            .or(self.parametrics.owner.as_ref())
            .is_none_or(|o| self.pm_owner_matches(o));
        let mut f = self
            .parametrics
            .pending
            .as_ref()
            .map(|p| p.form.clone())
            .or_else(|| {
                owner_ok
                    .then(|| forms.get(&blueprint.key()).cloned())
                    .flatten()
            })
            .unwrap_or(blueprint);
        let before = f.clone();
        let slot = f.key();
        let instance = self.session.editor.freshness().instance_id;
        let stamp = self.views.views.stamp(&self.session.editor);
        let ready = self.pm_ready() && owner_ok;
        let admitted = !self.pm_pending();
        let (
            mut pick,
            mut apply,
            mut discard,
            mut discard_pending,
            mut remove,
            mut history,
            mut close,
            mut open,
        ) = (None, false, false, false, false, None, false, true);
        egui::Window::new("Parametric specifications and bindings").id(egui::Id::new("mbse-parametrics")).open(&mut open).default_width(1080.).default_height(1000.).max_height((ctx.content_rect().height()-80.).max(300.)).vscroll(true).show(ctx,|ui|{
   ui.heading("Parametric specifications and bindings");if let Err(error)=pm::forms(&self.session.editor.snapshot){ui.colored_label(ERROR,error);}ui.label("Declare existing calculation inputs and bind exact native or typed-value sources. Accepted values and unfinished proposals stay separate. This review checks accepted types only; it does not run calculations, inspect current raw validity or check cycles.");if !owner_ok{ui.colored_label(ERROR,"This proposal belongs to another native instance. Copy it or explicitly discard new text; no IDs are rebound.");}
   ui.add_enabled_ui(ready&&admitted,|ui|{ui.horizontal_wrapped(|ui|{for name in["Specification","Parameter","Binding"]{let r=ui.add_enabled(name=="Specification"||self.parametrics.specification.is_some(),egui::Button::new(format!("New {name}")));self.bound(format!("parametric:new:{name}"),&r);if r.clicked(){pick=Some(self.pm_new_form(name));}}});
    let browse=ui.collapsing("Specifications and owned parameters/bindings",|ui|{for a in store.parametrics.values(){let r=ui.button(format!("Specification #{} · {} · System #{}",a.id.0,a.name,a.system.0));self.bound(format!("parametric:select:{}",a.id.0),&r);if r.clicked(){pick=accepted_form(a,a.id);}if self.parametrics.specification==Some(a.id){ui.indent(("parametric-elements",a.id.0),|ui|{for id in a.parameters.keys().chain(a.bindings.keys()){let f=accepted_form(a,*id).unwrap();let r=ui.button(accepted_label(a,*id));self.bound(format!("parametric:edit:{}",id.0),&r);if r.clicked(){pick=Some(f);}}});}}});self.bound("parametric:Browse elements",&browse.header_response);
    let proposals=ui.collapsing(format!("Retained proposals ({})",forms.len()),|ui|{for(key,p)in &forms{let r=ui.button(format!("{key} · {} proposal",family(p)));self.bound(format!("parametric:proposal:{key}"),&r);if r.clicked(){pick=Some(p.clone());}}});self.bound("parametric:Browse proposals",&proposals.header_response);
   });
   ui.separator();ui.heading(format!("{} proposal · {}",family(&f),f.id().map_or("new".into(),|id|format!("#{}",id.0))));if let Some(m)=f.specification(){ui.label(format!("Owned by Specification #{}",m.0));}
   ui.add_enabled_ui(ready,|ui|{match &mut f{
    Form::Specification{system,name,calculation,..}=>{self.pm_text(ui,"name","Specification name",name,&slot,&instance);self.pm_text(ui,"system","Native System ID",system,&slot,&instance);self.pm_text(ui,"calculation","Native calculation ID (empty = unassigned)",calculation,&slot,&instance);if let Ok(defs)=self.session.editor.snapshot.calculations(){let source=ui.collapsing("Existing native calculations",|ui|{for d in defs.values(){let r=ui.button(format!("Use calculation #{} · {} · {}",d.id.0,self.session.editor.snapshot.records[&d.owner].name,quantity_text(d.output)));self.bound(format!("parametric:calculation:{}",d.id.0),&r);if r.clicked(){*calculation=d.id.0.to_string();}}});self.bound("parametric:Browse calculations",&source.header_response);}},
    Form::Parameter{name,quantity,..}=>{self.pm_text(ui,"name","Exact native input name",name,&slot,&instance);self.pm_unit(ui,"quantity",&mut quantity.unit);quantity.kind=mbse_model::stage1_calc::unit_kind(quantity.unit);if let Some(a)=self.parametrics.specification.and_then(|id|store.parametrics.get(&id)){if let Some(d)=a.calculation.and_then(|id|self.session.editor.snapshot.calculations().ok().and_then(|defs|defs.get(&id).cloned())){for(n,input)in &d.bindings{let q=match input{mbse_model::stage1_calc::Input::Literal(l)=>Some(l.quantity),mbse_model::stage1_calc::Input::Output(id)=>self.session.editor.snapshot.calculations().ok().and_then(|defs|defs.get(id).map(|d|d.output))};if let Some(native)=q{let r=ui.button(format!("Use input {n} · {}",quantity_text(native)));self.bound(format!("parametric:declare:{n}"),&r);if r.clicked(){*name=n.clone();*quantity=native;}}}}}},
    Form::Binding{parameter,source,..}=>{self.pm_text(ui,"parameter","Exact parameter ID",parameter,&slot,&instance);ui.horizontal_wrapped(|ui|{for kind in["Native input","Native output","Typed value"]{let selected=matches!((&source,kind),(SourceForm::NativeInput{..},"Native input")|(SourceForm::NativeOutput{..},"Native output")|(SourceForm::TypedValue{..},"Typed value"));let r=ui.selectable_label(selected,kind);self.bound(format!("parametric:source-kind:{kind}"),&r);if r.clicked()&&!selected{let calc=match source{SourceForm::NativeInput{calculation,..}|SourceForm::NativeOutput{calculation}=>calculation.clone(),_=>String::new()};*source=match kind{"Native output"=>SourceForm::NativeOutput{calculation:calc},"Typed value"=>SourceForm::TypedValue{property:String::new(),usage:String::new(),unit:mbse_model::stage1_calc::UnitId::ScalarOne},_=>SourceForm::NativeInput{calculation:calc,name:String::new()}};}}});match source{
     SourceForm::NativeInput{calculation,name}=>{self.pm_text(ui,"source_calculation","Source calculation ID",calculation,&slot,&instance);self.pm_text(ui,"source_name","Source native input name",name,&slot,&instance);},
     SourceForm::NativeOutput{calculation}=>self.pm_text(ui,"source_calculation","Source calculation ID",calculation,&slot,&instance),
     SourceForm::TypedValue{property,usage,unit}=>{self.pm_text(ui,"property","Exact property ID",property,&slot,&instance);self.pm_text(ui,"usage","Exact Part ID (empty = default)",usage,&slot,&instance);ui.label("Authored source unit (not inherited from its scalar definition)");self.pm_unit(ui,"source-unit",unit);},
    }let sources=ui.collapsing("Existing source identities",|ui|{if let Ok(defs)=self.session.editor.snapshot.calculations(){for d in defs.values(){let r=ui.button(format!("Use native output #{} · {}",d.id.0,quantity_text(d.output)));self.bound(format!("parametric:output:{}",d.id.0),&r);if r.clicked(){*source=SourceForm::NativeOutput{calculation:d.id.0.to_string()};}for name in d.bindings.keys(){let r=ui.button(format!("Use native input #{} / {name}",d.id.0));self.bound(format!("parametric:input:{}:{name}",d.id.0),&r);if r.clicked(){*source=SourceForm::NativeInput{calculation:d.id.0.to_string(),name:name.clone()};}}}}for p in store.properties.values(){ui.label(format!("Property #{} · {} · component #{} · value type #{}",p.id.0,p.name,p.owner.0,p.value_type.0));}for u in store.usages.values().filter(|u|u.kind==ty::UsageKind::Part){ui.label(format!("Part #{} · {} · component #{}",u.id.0,u.name,u.target.0));}});self.bound("parametric:Browse sources",&sources.header_response);},
   }});let edited=f!=before;
   ui.horizontal_wrapped(|ui|{let r=ui.add_enabled(ready&&admitted&&!edited&&forms.contains_key(&slot),egui::Button::new("Apply proposal"));self.bound("parametric:Apply",&r);apply=r.clicked();let r=ui.add_enabled(ready&&admitted&&!edited&&forms.contains_key(&slot),egui::Button::new("Discard selected proposal"));self.bound("parametric:Discard proposal",&r);discard=r.clicked();let r=ui.add_enabled(ready&&admitted&&!edited&&accepted_present(&store,&f),egui::Button::new(if matches!(f,Form::Specification{..}){"Remove accepted specification and owned bindings"}else{"Remove accepted element"}));self.bound("parametric:Remove accepted",&r);remove=r.clicked();});
   ui.horizontal_wrapped(|ui|{ui.label("Global model history");for(redo,label)in[(false,"Model Undo"),(true,"Model Redo")]{let enabled=if redo{self.session.editor.can_redo()}else{self.session.editor.can_undo()};let r=ui.add_enabled(ready&&admitted&&!edited&&enabled,egui::Button::new(label));self.bound(format!("parametric:{label}"),&r);if r.clicked(){history=Some(redo);}}});
   if self.pm_pending(){ui.colored_label(ERROR,"New text is retained only in this window.");ui.horizontal(|ui|{let r=ui.button("Copy retained new proposal");self.bound("parametric:Copy new text",&r);if r.clicked(){ctx.copy_text(serde_json::to_string_pretty(&f).unwrap());}let r=ui.button("Discard unadmitted new text");self.bound("parametric:Discard new text",&r);discard_pending=r.clicked();});}
   ui.label(&self.parametrics.note);ui.separator();ui.heading("Accepted data and type review");ui.label(accepted_text(&store,&f));self.pm_report_ui(ui);
   let r=ui.button("Close parametric editor");self.bound("parametric:Close",&r);close=r.clicked();
  });
        let edited = f != before;
        let outcome = if discard_pending {
            self.parametrics.pending = None;
            self.parametrics.current = if owner_ok {
                forms
                    .get(&slot)
                    .cloned()
                    .or_else(|| Some(self.pm_new_form(family(&f))))
            } else {
                Some(self.pm_new_form("Specification"))
            };
            self.parametrics.specification = self.parametrics.current.as_ref().and_then(scope);
            self.parametrics.owner = Some(self.pm_owner());
            self.parametrics.note="Unadmitted new text explicitly discarded; stored proposals and accepted data retained.".into();
            Ok(())
        } else if edited {
            self.pm_set_form(&stamp, f.clone())
        } else if let Some(next) = pick {
            self.pm_pick(next, Some(&stamp))
        } else if apply {
            self.pm_apply(&stamp, &f)
        } else if discard {
            self.pm_model(&stamp, pm::Action::Discard(slot.clone()))
                .map(|_| {
                    self.parametrics.current = Some(self.pm_new_form(family(&f)));
                    self.parametrics.specification =
                        self.parametrics.current.as_ref().and_then(scope);
                    self.parametrics.owner = Some(self.pm_owner());
                    self.parametrics.note =
                        "Selected proposal discarded; accepted data unchanged.".into();
                })
        } else if remove {
            self.pm_model(&stamp,pm::Action::Remove{specification:scope(&f).unwrap(),element:f.id().unwrap()}).map(|_|self.parametrics.note="Accepted data removed; proposals retained. Model Undo restores exact identities.".into())
        } else if let Some(redo) = history {
            if !self.pm_ready() {
                Err("Finish pending work before model history.".into())
            } else {
                self.views
                    .views
                    .dispatch(
                        &mut self.session.editor,
                        &stamp,
                        if redo {
                            nv::NativeAction::ModelRedo
                        } else {
                            nv::NativeAction::ModelUndo
                        },
                    )
                    .map(|_| {
                        self.views.reconcile();
                        self.parametrics.note =
                            "Global model history restored; newer specification proposals retained."
                                .into();
                    })
            }
        } else {
            Ok(())
        };
        if let Err(e) = outcome {
            if !edited {
                self.parametrics.note = e;
            }
        }
        if close || !open {
            if self.pm_pending() {
                self.pm_close_blocked();
            } else {
                self.parametrics.open = false;
            }
        }
    }
    pub(super) fn pm_audit(&self) -> serde_json::Value {
        let forms = pm::forms(&self.session.editor.snapshot).ok();
        let owner_current = self
            .parametrics
            .pending
            .as_ref()
            .map(|p| &p.owner)
            .or(self.parametrics.owner.as_ref())
            .is_none_or(|o| self.pm_owner_matches(o));
        let current = self
            .parametrics
            .pending
            .as_ref()
            .map(|p| p.form.clone())
            .or_else(|| {
                self.parametrics.current.as_ref().map(|f| {
                    forms
                        .as_ref()
                        .filter(|_| owner_current)
                        .and_then(|m| m.get(&f.key()).cloned())
                        .unwrap_or_else(|| f.clone())
                })
            });
        let store = ty::load(&self.session.editor.snapshot).ok();
        serde_json::json!({"open":self.parametrics.open,"owner_current":owner_current,"specification":self.parametrics.specification,"current":current,"form_admitted":!self.pm_pending(),"note":self.parametrics.note,"accepted":store.as_ref().map(|s|&s.parametrics),"proposals":forms,"report":self.parametrics.specification.and_then(|id|pm::report(&self.session.editor.snapshot,id).ok())})
    }
}
pub(super) fn accepted_form(a: &pm::Specification, id: Id) -> Option<Form> {
    if id == a.id {
        return Some(Form::Specification {
            id: Some(id),
            system: a.system.0.to_string(),
            name: a.name.clone(),
            calculation: a.calculation.map_or(String::new(), |id| id.0.to_string()),
        });
    }
    if let Some(p) = a.parameters.get(&id) {
        return Some(Form::Parameter {
            specification: a.id,
            id: Some(id),
            name: p.name.clone(),
            quantity: p.quantity,
        });
    }
    a.bindings.get(&id).map(|b| Form::Binding {
        specification: a.id,
        id: Some(id),
        parameter: b.parameter.0.to_string(),
        source: match &b.source {
            pm::Source::NativeInput { calculation, name } => SourceForm::NativeInput {
                calculation: calculation.0.to_string(),
                name: name.clone(),
            },
            pm::Source::NativeOutput { calculation } => SourceForm::NativeOutput {
                calculation: calculation.0.to_string(),
            },
            pm::Source::TypedValue { target, unit } => SourceForm::TypedValue {
                property: target.property().0.to_string(),
                usage: match target {
                    ty::values::Target::Override { usage, .. } => usage.0.to_string(),
                    _ => String::new(),
                },
                unit: *unit,
            },
        },
    })
}
fn accepted_label(a: &pm::Specification, id: Id) -> String {
    if id == a.id {
        return format!(
            "Specification #{} · {} · System #{} · Calculation {}",
            id.0,
            a.name,
            a.system.0,
            a.calculation
                .map_or("unassigned".into(), |id| format!("#{}", id.0))
        );
    }
    if let Some(p) = a.parameters.get(&id) {
        return format!(
            "Parameter #{} · {} · {}",
            id.0,
            p.name,
            quantity_text(p.quantity)
        );
    }
    let b = &a.bindings[&id];
    format!(
        "Binding #{} · parameter #{} · {}",
        id.0,
        b.parameter.0,
        source_text(&b.source)
    )
}
fn source_text(s: &pm::Source) -> String {
    match s {
        pm::Source::NativeInput { calculation, name } => {
            format!("native input #{} / {name}", calculation.0)
        }
        pm::Source::NativeOutput { calculation } => format!("native output #{}", calculation.0),
        pm::Source::TypedValue { target, unit } => format!(
            "property #{} · {} · {}",
            target.property().0,
            match target {
                ty::values::Target::Default { .. } => "default".into(),
                ty::values::Target::Override { usage, .. } => format!("Part #{}", usage.0),
            },
            mbse_model::stage1_calc::unit_label(*unit)
        ),
    }
}
fn quantity(unit: mbse_model::stage1_calc::UnitId) -> mbse_model::stage1_calc::QuantityType {
    mbse_model::stage1_calc::QuantityType {
        kind: mbse_model::stage1_calc::unit_kind(unit),
        unit,
    }
}
fn quantity_text(q: mbse_model::stage1_calc::QuantityType) -> String {
    format!(
        "{} · {}",
        mbse_model::stage1_calc::quantity_label(q.kind),
        mbse_model::stage1_calc::unit_label(q.unit)
    )
}
fn accepted_present(s: &ty::Store, f: &Form) -> bool {
    f.specification()
        .and_then(|id| s.parametrics.get(&id))
        .zip(f.id())
        .and_then(|(a, id)| accepted_form(a, id))
        .is_some()
}
fn accepted_text(s: &ty::Store, f: &Form) -> String {
    f.specification()
        .and_then(|id| s.parametrics.get(&id))
        .zip(f.id())
        .filter(|(a, id)| accepted_form(a, *id).is_some())
        .map_or(
            "No accepted object in this slot; IDs are not rebound.".into(),
            |(a, id)| format!("Accepted {}", accepted_label(a, id)),
        )
}
impl Workspace {
    fn pm_unit(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        unit: &mut mbse_model::stage1_calc::UnitId,
    ) {
        use mbse_model::stage1_calc::{self as c, UnitId::*};
        ui.horizontal_wrapped(|ui| {
            for value in [
                ScalarOne,
                RatioOne,
                Percent,
                Count,
                Metre,
                Kilometre,
                Second,
                Minute,
                MetrePerSecond,
                KilometrePerHour,
            ] {
                let r = ui.selectable_value(unit, value, c::unit_label(value));
                self.bound(format!("parametric:{key}:{value:?}"), &r);
            }
        });
    }
    fn pm_report_ui(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.parametrics.specification else {
            return;
        };
        match pm::report(&self.session.editor.snapshot, id) {
            Ok(r) => {
                ui.heading(if r.accepted_bindings_complete {
                    "Accepted type links are complete"
                } else {
                    "Unbound or unresolved inputs remain"
                });
                ui.label("Calculations have not been run. Current raw source edits, cycles and engineering correctness are not checked by this type review.");
                if let Ok(s) = ty::load(&self.session.editor.snapshot) {
                    if let Some(a) = s.parametrics.get(&id) {
                        for p in a.parameters.values() {
                            let link = a
                                .bindings
                                .values()
                                .find(|b| b.parameter == p.id)
                                .map_or("UNBOUND".into(), |b| source_text(&b.source));
                            ui.label(format!(
                                "Parameter #{} · {} · {} · {link}",
                                p.id.0,
                                p.name,
                                quantity_text(p.quantity)
                            ));
                        }
                    }
                }
                let copy = ui.button("Copy accepted-type review");
                self.bound("parametric:Copy review", &copy);
                if copy.clicked() {
                    ui.ctx()
                        .copy_text(serde_json::to_string_pretty(&r).unwrap());
                }
                for d in r.diagnostics {
                    ui.colored_label(ERROR, format!("{} · {}", d.code, d.message));
                }
                ui.label(format!(
                    "Accepted source: model r{} · raw r{}",
                    r.model_revision, r.raw_version
                ));
            }
            Err(e) => {
                ui.colored_label(ERROR, e);
            }
        }
    }
}
#[cfg(test)]
#[path = "ui_parametric_tests.rs"]
mod tests;
