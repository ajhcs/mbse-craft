//! Native flat scenario proposals over the existing typed authority.
use super::*;
use mbse_model::{
    native_views::{self as nv, OccurrenceId, Stamp, ViewId},
    types::{
        self as ty, Id,
        scenarios::{self as sc, Form, MessageKind, ParticipantKind},
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
pub(super) struct Rendered {
    pub view: ViewId,
    pub stamp: Stamp,
    pub occurrences: Vec<(OccurrenceId, Id, nv::Geometry)>,
    pub messages: Vec<Id>,
    pub rows: serde_json::Value,
    pub endpoints: Vec<(Id, [f32; 2], [f32; 2])>,
    pub labels: Vec<(Id, [f32; 4], String)>,
    pub preview: bool,
}
#[derive(Default)]
pub(super) struct Panel {
    pub open: bool,
    pub scenario: Option<Id>,
    pub current: Option<Form>,
    owner: Option<Owner>,
    pending: Option<Pending>,
    pub note: String,
    pub selected: Option<(ViewId, OccurrenceId)>,
    pub drag: Option<super::activity_editor::Drag>,
    pub rendered: Option<Rendered>,
}
fn scope(f: &Form) -> Option<Id> {
    match f {
        Form::Scenario { id, .. } => *id,
        _ => f.scenario(),
    }
}
fn family(f: &Form) -> &'static str {
    match f {
        Form::Scenario { .. } => "Scenario",
        Form::Lifeline { .. } => "Lifeline",
        Form::Message { .. } => "Message",
        Form::Alternative { .. } => "Alternative",
        Form::Operand { .. } => "Operand",
    }
}
impl Workspace {
    fn sc_owner(&self) -> Owner {
        Owner {
            instance: self.session.editor.freshness().instance_id,
            document: self.session.editor.snapshot.document_id.clone(),
        }
    }
    fn sc_owner_matches(&self, o: &Owner) -> bool {
        o.instance == self.session.editor.freshness().instance_id
            && o.document == self.session.editor.snapshot.document_id
    }
    pub(super) fn sc_frame_start(&mut self) {
        self.scenarios.rendered = None;
    }
    pub(super) fn sc_gesture_active(&self) -> bool {
        self.scenarios.drag.is_some()
    }
    pub(super) fn sc_pending(&self) -> bool {
        self.scenarios.pending.is_some()
    }
    pub(super) fn sc_close_blocked(&mut self) -> bool {
        if self.sc_pending() {
            self.scenarios.open = true;
            self.scenarios.note="New text remains in this window. Admit it, copy it, or explicitly discard it before closing.".into();
            true
        } else {
            self.scenarios.drag = None;
            false
        }
    }
    pub(super) fn sc_ready(&self) -> bool {
        !self.dock_blocked_without_scenario()
            && ty::load(&self.session.editor.snapshot).is_ok()
            && sc::forms(&self.session.editor.snapshot).is_ok()
    }
    pub(super) fn sc_open(&mut self) {
        self.scenarios.open = true;
        if self.scenarios.current.is_none() {
            let f = self.sc_new_form("Scenario");
            let _ = self.sc_pick(f, None);
        }
    }
    fn sc_new_form(&self, family: &str) -> Form {
        let scenario = self.scenarios.scenario.unwrap_or(Id(0));
        match family {
            "Lifeline" => Form::Lifeline {
                scenario,
                id: None,
                name: String::new(),
                participant: String::new(),
                expected: ParticipantKind::Component,
            },
            "Message" => Form::Message {
                scenario,
                id: None,
                name: String::new(),
                source: String::new(),
                target: String::new(),
                order: String::new(),
                operand: String::new(),
                message_kind: MessageKind::Synchronous,
                reply_to: String::new(),
            },
            "Alternative" => Form::Alternative {
                scenario,
                id: None,
                name: String::new(),
                order: String::new(),
            },
            "Operand" => Form::Operand {
                scenario,
                id: None,
                name: String::new(),
                alternative: String::new(),
                order: String::new(),
                guard: String::new(),
            },
            _ => Form::Scenario {
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
            },
        }
    }
    pub(super) fn sc_pick(&mut self, f: Form, stamp: Option<&Stamp>) -> Result<(), String> {
        if self.sc_pending() {
            return Err(
                "Admit, copy or explicitly discard new text before changing selection.".into(),
            );
        }
        if self.dock_blocked_without_scenario() {
            return Err("Finish the gesture/write or close the other editor before selecting scenario data.".into());
        }
        if stamp.is_some_and(|s| *s != self.views.views.stamp(&self.session.editor)) {
            return Err("Scenario selection source changed; original selection retained.".into());
        }
        let f = sc::forms(&self.session.editor.snapshot)?
            .get(&f.key())
            .cloned()
            .unwrap_or(f);
        self.scenarios.scenario = scope(&f);
        self.scenarios.current = Some(f);
        self.scenarios.owner = Some(self.sc_owner());
        self.scenarios.open = true;
        Ok(())
    }
    pub(super) fn sc_model(&mut self, stamp: &Stamp, action: sc::Action) -> Result<(), String> {
        if !self.sc_ready() {
            return Err("Finish pending work or open an admitted native model copy before scenario editing.".into());
        }
        let result = self
            .views
            .views
            .dispatch(
                &mut self.session.editor,
                stamp,
                nv::NativeAction::Command(Command::Typed(ty::Action::Scenario(action))),
            )
            .map(|_| ());
        self.views.reconcile();
        result
    }
    pub(super) fn sc_set_form(&mut self, stamp: &Stamp, f: Form) -> Result<(), String> {
        let result = self.sc_model(stamp, sc::Action::Set(f.clone()));
        match &result {
            Ok(()) => {
                self.scenarios.pending = None;
                self.scenarios.note =
                    "Proposal retained. Apply to change accepted scenario data.".into();
            }
            Err(e) => {
                self.scenarios.pending = Some(Pending {
                    owner: self
                        .scenarios
                        .owner
                        .clone()
                        .unwrap_or_else(|| self.sc_owner()),
                    form: f.clone(),
                });
                self.scenarios.note =
                    format!("{e} New text remains in this window; accepted data is unchanged.");
            }
        }
        self.scenarios.current = Some(f);
        result
    }
    pub(super) fn sc_layout(
        &mut self,
        stamp: &Stamp,
        a: nv::scenarios::Layout,
    ) -> Result<(), String> {
        if self.dock_blocked_without_scenario() || self.sc_pending() {
            return Err(
                "Finish pending work and preserve new proposal text before diagram changes.".into(),
            );
        }
        self.views
            .views
            .apply(&self.session.editor, stamp, nv::Layout::Scenario(a))?;
        self.views.reconcile();
        Ok(())
    }
    fn sc_apply(&mut self, stamp: &Stamp, f: &Form) -> Result<(), String> {
        let id_value = f.id().unwrap_or(Id(self.session.editor.snapshot.next_id));
        self.sc_model(stamp, sc::Action::Apply(f.key()))?;
        let mut f = f.clone();
        match &mut f {
            Form::Scenario { id, .. }
            | Form::Lifeline { id, .. }
            | Form::Message { id, .. }
            | Form::Alternative { id, .. }
            | Form::Operand { id, .. } => *id = Some(id_value),
        }
        self.scenarios.scenario = scope(&f);
        self.scenarios.current = Some(f);
        self.scenarios.note =
            "Accepted data updated. Model Undo/Redo preserves newer proposals.".into();
        Ok(())
    }
    pub(super) fn sc_create_diagram(&mut self, stamp: &Stamp, scenario: Id) -> Result<(), String> {
        let store = ty::load(&self.session.editor.snapshot)?;
        let a = store
            .scenarios
            .get(&scenario)
            .ok_or("Scenario missing; proposal retained.")?;
        let view: ViewId =
            serde_json::from_str(&format!("\"{}\"", self.views.views.document().next_id()))
                .map_err(|e| format!("Diagram identity unavailable: {e}"))?;
        self.sc_layout(
            stamp,
            nv::scenarios::Layout::Create {
                scenario,
                title: format!("Scenario: {}", a.name),
            },
        )?;
        self.views.active = view;
        self.scenarios.selected = None;
        self.views.reconcile();
        self.scenarios.note =
            "Independent sequence diagram created from accepted scenario data. No execution is implied.".into();
        Ok(())
    }
    fn sc_text(
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
                    .id(egui::Id::new(("scenario-proposal", instance, slot, key))),
            );
            self.bound(format!("scenario:field:{key}"), &r);
        });
    }
    pub(super) fn sc_dialog(&mut self, ctx: &egui::Context) {
        if !self.scenarios.open {
            return;
        }
        if self.scenarios.current.is_none() {
            self.scenarios.current = Some(self.sc_new_form("Scenario"));
            self.scenarios.owner = Some(self.sc_owner());
        }
        let store = match ty::load(&self.session.editor.snapshot) {
            Ok(s) => s,
            Err(e) => {
                let pending = self.scenarios.pending.clone();
                egui::Window::new("Scenarios: authoring and diagrams")
                    .id(egui::Id::new("mbse-scenarios"))
                    .show(ctx, |ui| {
                        ui.colored_label(ERROR, e);
                        if let Some(p) = pending {
                            ui.label(
                                "New text remains in this window; accepted data is unchanged.",
                            );
                            let r = ui.button("Copy retained new proposal");
                            self.bound("scenario:Copy new text", &r);
                            if r.clicked() {
                                ctx.copy_text(serde_json::to_string_pretty(&p.form).unwrap());
                            }
                            let r = ui.button("Discard unadmitted new text");
                            self.bound("scenario:Discard new text", &r);
                            if r.clicked() {
                                self.scenarios.pending = None;
                                self.scenarios.current = None;
                                self.scenarios.owner = None;
                                self.scenarios.scenario = None;
                            }
                        }
                        let r = ui.button("Close scenario editor");
                        self.bound("scenario:Close", &r);
                        if r.clicked() && !self.sc_close_blocked() {
                            self.scenarios.open = false;
                        }
                    });
                return;
            }
        };
        let forms = sc::forms(&self.session.editor.snapshot).unwrap_or_default();
        let blueprint = self.scenarios.current.clone().unwrap();
        let owner_ok = self
            .scenarios
            .pending
            .as_ref()
            .map(|p| &p.owner)
            .or(self.scenarios.owner.as_ref())
            .is_none_or(|o| self.sc_owner_matches(o));
        let mut f = self
            .scenarios
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
        let ready = self.sc_ready() && owner_ok;
        let admitted = !self.sc_pending();
        let (
            mut pick,
            mut apply,
            mut discard,
            mut discard_pending,
            mut remove,
            mut history,
            mut create,
            mut close,
            mut open,
        ) = (None, false, false, false, false, None, None, false, true);
        egui::Window::new("Scenarios: authoring and diagrams").id(egui::Id::new("mbse-scenarios")).open(&mut open).default_width(1080.).default_height(1000.).max_height((ctx.content_rect().height()-80.).max(300.)).vscroll(true).show(ctx,|ui|{
   ui.heading("Scenario authoring");if let Err(error)=sc::forms(&self.session.editor.snapshot){ui.colored_label(ERROR,error);}ui.label("Exact participant identities, authored message order, calls and replies, and flat alternative operands. Guards are documentary text. No execution, payload or timing is inferred.");if !owner_ok{ui.colored_label(ERROR,"This proposal belongs to another native instance. Copy it or explicitly discard new text; no IDs are rebound.");}
   ui.add_enabled_ui(ready&&admitted,|ui|{ui.horizontal_wrapped(|ui|{for name in["Scenario","Lifeline","Message","Alternative","Operand"]{let r=ui.add_enabled(name=="Scenario"||self.scenarios.scenario.is_some(),egui::Button::new(format!("New {name}")));self.bound(format!("scenario:new:{name}"),&r);if r.clicked(){pick=Some(self.sc_new_form(name));}}});
    let browse=ui.collapsing("Scenarios and owned elements",|ui|{for a in store.scenarios.values(){
     let r=ui.button(format!("Scenario #{} · {} · System #{}",a.id.0,a.name,a.system.0));self.bound(format!("scenario:select:{}",a.id.0),&r);if r.clicked(){pick=accepted_form(a,a.id);}
     if self.scenarios.scenario==Some(a.id){ui.indent(("scenario-elements",a.id.0),|ui|{for id in a.lifelines.keys().chain(a.messages.keys()).chain(a.alternatives.keys()).chain(a.alternatives.values().flat_map(|alt|alt.operands.keys())){if let Some(f)=accepted_form(a,*id){let r=ui.button(format!("{} #{}",family(&f),id.0));self.bound(format!("scenario:edit:{}",id.0),&r);if r.clicked(){pick=Some(f);}}}});}
    }});self.bound("scenario:Browse elements",&browse.header_response);
    let proposals=ui.collapsing(format!("Retained proposals ({})",forms.len()),|ui|{for(key,p)in &forms{let r=ui.button(format!("{key} · {} proposal",family(p)));self.bound(format!("scenario:proposal:{key}"),&r);if r.clicked(){pick=Some(p.clone());}}});self.bound("scenario:Browse proposals",&proposals.header_response);
   });
   ui.separator();ui.heading(format!("{} proposal · {}",family(&f),f.id().map_or("new".into(),|id|format!("#{}",id.0))));if let Some(m)=f.scenario(){ui.label(format!("Owned by Scenario #{}",m.0));}
   ui.add_enabled_ui(ready,|ui|{match &mut f{
    Form::Scenario{system,name,..}=>{self.sc_text(ui,"name","Scenario name",name,&slot,&instance);self.sc_text(ui,"system","Native System ID",system,&slot,&instance);},
    Form::Lifeline{name,participant,expected,..}=>{self.sc_text(ui,"name","Lifeline name",name,&slot,&instance);self.sc_text(ui,"participant","Exact participant ID",participant,&slot,&instance);ui.horizontal_wrapped(|ui|{for kind in participant_kinds(){let r=ui.selectable_value(expected,kind,format!("{kind:?}"));self.bound(format!("scenario:participant-kind:{kind:?}"),&r);}});let source=ui.collapsing("Available participant identities",|ui|{for r in self.session.editor.snapshot.records.values(){ui.label(format!("Native {:?} #{} · {}",r.kind,r.id.0,r.name));}for d in store.definitions.values(){ui.label(format!("Definition {:?} #{} · {}",d.kind,d.id.0,d.name));}for u in store.usages.values(){ui.label(format!("Usage {:?} #{} · {}",u.kind,u.id.0,u.name));}});self.bound("scenario:Browse participants",&source.header_response);},
    Form::Message{name,source,target,order,operand,message_kind,reply_to,..}=>{for(key,label,text)in[("name","Message name",name),("source","Source lifeline ID",source),("target","Target lifeline ID",target),("order","Authored order (1..4096)",order),("operand","Operand ID (empty = main sequence)",operand),("reply_to","Prior synchronous message ID (reply only)",reply_to)]{self.sc_text(ui,key,label,text,&slot,&instance);}ui.horizontal(|ui|{for kind in[MessageKind::Synchronous,MessageKind::Asynchronous,MessageKind::Reply]{let r=ui.selectable_value(message_kind,kind,format!("{kind:?}"));self.bound(format!("scenario:message-kind:{kind:?}"),&r);}});},
    Form::Alternative{name,order,..}=>{self.sc_text(ui,"name","Alternative name",name,&slot,&instance);self.sc_text(ui,"order","Main sequence order",order,&slot,&instance);},
    Form::Operand{name,alternative,order,guard,..}=>{self.sc_text(ui,"name","Operand name",name,&slot,&instance);self.sc_text(ui,"alternative","Alternative ID",alternative,&slot,&instance);self.sc_text(ui,"order","Operand order",order,&slot,&instance);ui.label("Documentary guard (no evaluation)");let r=ui.add(egui::TextEdit::multiline(guard).char_limit(4096).desired_rows(3).desired_width(700.).id(egui::Id::new(("scenario-guard",&instance,&slot))));self.bound("scenario:field:guard",&r);},
   }});let edited=f!=before;
   ui.horizontal_wrapped(|ui|{let r=ui.add_enabled(ready&&admitted&&!edited&&forms.contains_key(&slot),egui::Button::new("Apply proposal"));self.bound("scenario:Apply",&r);apply=r.clicked();let r=ui.add_enabled(ready&&admitted&&!edited&&forms.contains_key(&slot),egui::Button::new("Discard selected proposal"));self.bound("scenario:Discard proposal",&r);discard=r.clicked();let r=ui.add_enabled(ready&&admitted&&!edited&&accepted_present(&store,&f),egui::Button::new(if matches!(f,Form::Scenario{..}){"Remove accepted scenario and owned contents"}else{"Remove accepted element"}));self.bound("scenario:Remove accepted",&r);remove=r.clicked();});
   ui.horizontal_wrapped(|ui|{ui.label("Global model history");for(redo,label)in[(false,"Model Undo"),(true,"Model Redo")]{let enabled=if redo{self.session.editor.can_redo()}else{self.session.editor.can_undo()};let r=ui.add_enabled(ready&&admitted&&!edited&&enabled,egui::Button::new(label));self.bound(format!("scenario:{label}"),&r);if r.clicked(){history=Some(redo);}}if let Some(m)=self.scenarios.scenario{let r=ui.add_enabled(ready&&admitted&&!edited&&store.scenarios.contains_key(&m),egui::Button::new("Create accepted scenario diagram"));self.bound("scenario:Create diagram",&r);if r.clicked(){create=Some(m);}}});
   if self.sc_pending(){ui.colored_label(ERROR,"New text is retained only in this window.");ui.horizontal(|ui|{let r=ui.button("Copy retained new proposal");self.bound("scenario:Copy new text",&r);if r.clicked(){ctx.copy_text(serde_json::to_string_pretty(&f).unwrap());}let r=ui.button("Discard unadmitted new text");self.bound("scenario:Discard new text",&r);discard_pending=r.clicked();});}
   ui.label(&self.scenarios.note);ui.separator();ui.heading("Accepted data and diagnostics");ui.label(accepted_text(&store,&f));if let Some(m)=self.scenarios.scenario.and_then(|id|store.scenarios.get(&id)){ui.label(format!("Scenario #{} · {} · System #{} · {} lifelines · {} messages · {} alternatives",m.id.0,m.name,m.system.0,m.states.len(),m.events.len(),m.transitions.len()));for d in sc::diagnostics(&self.session.editor.snapshot,&store,m.id).unwrap_or_default(){ui.label(format!("{} · elements {:?}: {}",d.code,d.elements,d.message));}}
   let r=ui.button("Close scenario editor");self.bound("scenario:Close",&r);close=r.clicked();
  });
        let edited = f != before;
        let outcome = if discard_pending {
            self.scenarios.pending = None;
            self.scenarios.current = if owner_ok {
                forms
                    .get(&slot)
                    .cloned()
                    .or_else(|| Some(self.sc_new_form(family(&f))))
            } else {
                Some(self.sc_new_form("Scenario"))
            };
            self.scenarios.scenario = self.scenarios.current.as_ref().and_then(scope);
            self.scenarios.owner = Some(self.sc_owner());
            self.scenarios.note="Unadmitted new text explicitly discarded; stored proposals and accepted data retained.".into();
            Ok(())
        } else if edited {
            self.sc_set_form(&stamp, f.clone())
        } else if let Some(next) = pick {
            self.sc_pick(next, Some(&stamp))
        } else if apply {
            self.sc_apply(&stamp, &f)
        } else if discard {
            self.sc_model(&stamp, sc::Action::Discard(slot.clone()))
                .map(|_| {
                    self.scenarios.current = Some(self.sc_new_form(family(&f)));
                    self.scenarios.scenario = self.scenarios.current.as_ref().and_then(scope);
                    self.scenarios.owner = Some(self.sc_owner());
                    self.scenarios.note =
                        "Selected proposal discarded; accepted data unchanged.".into();
                })
        } else if remove {
            self.sc_model(&stamp,sc::Action::Remove{scenario:scope(&f).unwrap(),element:f.id().unwrap()}).map(|_|self.scenarios.note="Accepted data removed; proposals retained. Model Undo restores exact identities.".into())
        } else if let Some(redo) = history {
            if !self.sc_ready() {
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
                        self.scenarios.note =
                            "Global model history restored; newer scenario proposals retained."
                                .into();
                    })
            }
        } else if let Some(m) = create {
            self.sc_create_diagram(&stamp, m)
        } else {
            Ok(())
        };
        if let Err(e) = outcome {
            if !edited {
                self.scenarios.note = e;
            }
        }
        if close || !open {
            if self.sc_pending() {
                self.sc_close_blocked();
            } else {
                self.scenarios.open = false;
            }
        }
    }
    pub(super) fn sc_audit(&self) -> serde_json::Value {
        let forms = sc::forms(&self.session.editor.snapshot).ok();
        let owner_current = self
            .scenarios
            .pending
            .as_ref()
            .map(|p| &p.owner)
            .or(self.scenarios.owner.as_ref())
            .is_none_or(|o| self.sc_owner_matches(o));
        let current = self
            .scenarios
            .pending
            .as_ref()
            .map(|p| p.form.clone())
            .or_else(|| {
                self.scenarios.current.as_ref().map(|f| {
                    forms
                        .as_ref()
                        .filter(|_| owner_current)
                        .and_then(|m| m.get(&f.key()).cloned())
                        .unwrap_or_else(|| f.clone())
                })
            });
        let store = ty::load(&self.session.editor.snapshot).ok();
        serde_json::json!({"open":self.scenarios.open,"owner_current":owner_current,"scenario":self.scenarios.scenario,"current":current,"form_admitted":!self.sc_pending(),"note":self.scenarios.note,"accepted":store.as_ref().map(|s|&s.scenarios),"proposals":forms,"diagnostics":store.as_ref().and_then(|s|self.scenarios.scenario.and_then(|m|sc::diagnostics(&self.session.editor.snapshot,s,m).ok())),"gesture_active":self.sc_gesture_active(),"selected":self.scenarios.selected,"rendered":self.scenarios.rendered.as_ref().map(|r|serde_json::json!({"view":r.view,"current":r.view==self.views.active&&r.stamp==self.views.views.stamp(&self.session.editor),"preview_only":r.preview,"occurrences":r.occurrences.iter().map(|(id,element,g)|serde_json::json!({"occurrence":id,"element":element,"geometry":g})).collect::<Vec<_>>(),"messages":r.messages,"rows":r.rows,"endpoints":r.endpoints,"labels":r.labels}))})
    }
}
pub(super) fn accepted_form(a: &sc::Scenario, id: Id) -> Option<Form> {
    if id == a.id {
        return Some(Form::Scenario {
            id: Some(id),
            name: a.name.clone(),
            system: a.system.0.to_string(),
        });
    }
    if let Some(l) = a.lifelines.get(&id) {
        return Some(Form::Lifeline {
            scenario: a.id,
            id: Some(id),
            name: l.name.clone(),
            participant: l.participant.0.to_string(),
            expected: l.expected,
        });
    }
    if let Some(m) = a.messages.get(&id) {
        return Some(Form::Message {
            scenario: a.id,
            id: Some(id),
            name: m.name.clone(),
            source: m.source.0.to_string(),
            target: m.target.0.to_string(),
            order: m.order.to_string(),
            operand: m.operand.map_or(String::new(), |id| id.0.to_string()),
            message_kind: m.kind,
            reply_to: m.reply_to.map_or(String::new(), |id| id.0.to_string()),
        });
    }
    if let Some(alt) = a.alternatives.get(&id) {
        return Some(Form::Alternative {
            scenario: a.id,
            id: Some(id),
            name: alt.name.clone(),
            order: alt.order.to_string(),
        });
    }
    for alt in a.alternatives.values() {
        if let Some(o) = alt.operands.get(&id) {
            return Some(Form::Operand {
                scenario: a.id,
                id: Some(id),
                name: o.name.clone(),
                alternative: alt.id.0.to_string(),
                order: o.order.to_string(),
                guard: o.guard.clone(),
            });
        }
    }
    None
}
fn participant_kinds() -> Vec<ParticipantKind> {
    let mut kinds = [
        RecordKind::System,
        RecordKind::Mission,
        RecordKind::Capability,
        RecordKind::Objective,
        RecordKind::Measure,
        RecordKind::Stakeholder,
        RecordKind::Expectation,
        RecordKind::Environment,
        RecordKind::Organization,
        RecordKind::Actor,
    ]
    .into_iter()
    .map(ParticipantKind::Native)
    .collect::<Vec<_>>();
    kinds.extend([
        ParticipantKind::Component,
        ParticipantKind::Usage(ty::UsageKind::Part),
        ParticipantKind::Usage(ty::UsageKind::Reference),
    ]);
    kinds
}
fn accepted_present(s: &ty::Store, f: &Form) -> bool {
    scope(f)
        .and_then(|root| s.scenarios.get(&root))
        .zip(f.id())
        .and_then(|(a, id)| accepted_form(a, id))
        .is_some()
}
fn accepted_text(s: &ty::Store, f: &Form) -> String {
    scope(f)
        .and_then(|id| s.scenarios.get(&id))
        .zip(f.id())
        .and_then(|(a, id)| accepted_form(a, id))
        .map_or(
            "No accepted object in this slot; identities are not rebound.".into(),
            |f| format!("Accepted {}", serde_json::to_string_pretty(&f).unwrap()),
        )
}
#[cfg(test)]
#[path = "ui_scenario_tests.rs"]
mod tests;
