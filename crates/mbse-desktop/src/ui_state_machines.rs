//! Native flat state-machine proposals over the existing typed authority.
use super::*;
use mbse_model::{
    native_views::{self as nv, OccurrenceId, Stamp, ViewId},
    types::{
        self as ty, Id,
        state_machines::{self as sm, Form, StateKind},
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
    pub transitions: Vec<Id>,
    pub endpoints: Vec<(Id, [f32; 2], [f32; 2])>,
    pub labels: Vec<(Id, [f32; 4], String)>,
    pub overlapping: usize,
    pub preview: bool,
}
#[derive(Default)]
pub(super) struct Panel {
    pub open: bool,
    pub machine: Option<Id>,
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
        Form::Machine { id, .. } => *id,
        _ => f.machine(),
    }
}
fn family(f: &Form) -> &'static str {
    match f {
        Form::Machine { .. } => "Machine",
        Form::State { .. } => "State",
        Form::Event { .. } => "Event",
        Form::Transition { .. } => "Transition",
    }
}
impl Workspace {
    fn sm_owner(&self) -> Owner {
        Owner {
            instance: self.session.editor.freshness().instance_id,
            document: self.session.editor.snapshot.document_id.clone(),
        }
    }
    fn sm_owner_matches(&self, o: &Owner) -> bool {
        o.instance == self.session.editor.freshness().instance_id
            && o.document == self.session.editor.snapshot.document_id
    }
    pub(super) fn sm_frame_start(&mut self) {
        self.state_machines.rendered = None;
    }
    pub(super) fn sm_gesture_active(&self) -> bool {
        self.state_machines.drag.is_some()
    }
    pub(super) fn sm_pending(&self) -> bool {
        self.state_machines.pending.is_some()
    }
    pub(super) fn sm_close_blocked(&mut self) -> bool {
        if self.sm_pending() {
            self.state_machines.open = true;
            self.state_machines.note="New text remains in this window. Admit it, copy it, or explicitly discard it before closing.".into();
            true
        } else {
            self.state_machines.drag = None;
            false
        }
    }
    pub(super) fn sm_ready(&self) -> bool {
        !self.dock_blocked_without_state_machine()
            && ty::load(&self.session.editor.snapshot).is_ok()
            && sm::forms(&self.session.editor.snapshot).is_ok()
    }
    pub(super) fn sm_open(&mut self) {
        self.state_machines.open = true;
        if self.state_machines.current.is_none() {
            let f = self.sm_new_form("Machine");
            let _ = self.sm_pick(f, None);
        }
    }
    fn sm_new_form(&self, family: &str) -> Form {
        let machine = self.state_machines.machine.unwrap_or(Id(0));
        match family {
            "State" => Form::State {
                machine,
                id: None,
                name: String::new(),
                state_kind: StateKind::State,
            },
            "Event" => Form::Event {
                machine,
                id: None,
                name: String::new(),
            },
            "Transition" => Form::Transition {
                machine,
                id: None,
                name: String::new(),
                source: String::new(),
                target: String::new(),
                event: String::new(),
                guard: String::new(),
            },
            _ => Form::Machine {
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
    pub(super) fn sm_pick(&mut self, f: Form, stamp: Option<&Stamp>) -> Result<(), String> {
        if self.sm_pending() {
            return Err(
                "Admit, copy or explicitly discard new text before changing selection.".into(),
            );
        }
        if self.dock_blocked_without_state_machine() {
            return Err("Finish the gesture/write or close the other editor before selecting state-machine data.".into());
        }
        if stamp.is_some_and(|s| *s != self.views.views.stamp(&self.session.editor)) {
            return Err(
                "State-machine selection source changed; original selection retained.".into(),
            );
        }
        let f = sm::forms(&self.session.editor.snapshot)?
            .get(&f.key())
            .cloned()
            .unwrap_or(f);
        self.state_machines.machine = scope(&f);
        self.state_machines.current = Some(f);
        self.state_machines.owner = Some(self.sm_owner());
        self.state_machines.open = true;
        Ok(())
    }
    pub(super) fn sm_model(&mut self, stamp: &Stamp, action: sm::Action) -> Result<(), String> {
        if !self.sm_ready() {
            return Err("Finish pending work or open an admitted native model copy before state-machine editing.".into());
        }
        let result = self
            .views
            .views
            .dispatch(
                &mut self.session.editor,
                stamp,
                nv::NativeAction::Command(Command::Typed(ty::Action::StateMachine(action))),
            )
            .map(|_| ());
        self.views.reconcile();
        result
    }
    pub(super) fn sm_set_form(&mut self, stamp: &Stamp, f: Form) -> Result<(), String> {
        let result = self.sm_model(stamp, sm::Action::Set(f.clone()));
        match &result {
            Ok(()) => {
                self.state_machines.pending = None;
                self.state_machines.note =
                    "Proposal retained. Apply to change accepted state-machine data.".into();
            }
            Err(e) => {
                self.state_machines.pending = Some(Pending {
                    owner: self
                        .state_machines
                        .owner
                        .clone()
                        .unwrap_or_else(|| self.sm_owner()),
                    form: f.clone(),
                });
                self.state_machines.note =
                    format!("{e} New text remains in this window; accepted data is unchanged.");
            }
        }
        self.state_machines.current = Some(f);
        result
    }
    pub(super) fn sm_layout(
        &mut self,
        stamp: &Stamp,
        a: nv::state_machines::Layout,
    ) -> Result<(), String> {
        if self.dock_blocked_without_state_machine() || self.sm_pending() {
            return Err(
                "Finish pending work and preserve new proposal text before diagram changes.".into(),
            );
        }
        self.views
            .views
            .apply(&self.session.editor, stamp, nv::Layout::StateMachine(a))?;
        self.views.reconcile();
        Ok(())
    }
    fn sm_apply(&mut self, stamp: &Stamp, f: &Form) -> Result<(), String> {
        let id = f.id().unwrap_or(Id(self.session.editor.snapshot.next_id));
        self.sm_model(stamp, sm::Action::Apply(f.key()))?;
        let mut f = f.clone();
        match &mut f {
            Form::Machine { id: slot, .. }
            | Form::State { id: slot, .. }
            | Form::Event { id: slot, .. }
            | Form::Transition { id: slot, .. } => *slot = Some(id),
        }
        self.state_machines.machine = scope(&f);
        self.state_machines.current = Some(f);
        self.state_machines.note =
            "Accepted data updated. Model Undo/Redo preserves newer proposals.".into();
        Ok(())
    }
    pub(super) fn sm_create_diagram(&mut self, stamp: &Stamp, machine: Id) -> Result<(), String> {
        let store = ty::load(&self.session.editor.snapshot)?;
        let a = store
            .state_machines
            .get(&machine)
            .ok_or("State machine missing; proposal retained.")?;
        let view: ViewId =
            serde_json::from_str(&format!("\"{}\"", self.views.views.document().next_id()))
                .map_err(|e| format!("Diagram identity unavailable: {e}"))?;
        self.sm_layout(
            stamp,
            nv::state_machines::Layout::Create {
                machine,
                title: format!("State machine: {}", a.name),
            },
        )?;
        self.views.active = view;
        self.state_machines.selected = None;
        self.views.reconcile();
        self.state_machines.note =
            "Independent diagram created from accepted state data. No execution is implied.".into();
        Ok(())
    }
    fn sm_text(
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
                    .id(egui::Id::new((
                        "state-machine-proposal",
                        instance,
                        slot,
                        key,
                    ))),
            );
            self.bound(format!("state-machine:field:{key}"), &r);
        });
    }
    pub(super) fn sm_dialog(&mut self, ctx: &egui::Context) {
        if !self.state_machines.open {
            return;
        }
        if self.state_machines.current.is_none() {
            self.state_machines.current = Some(self.sm_new_form("Machine"));
            self.state_machines.owner = Some(self.sm_owner());
        }
        let store = match ty::load(&self.session.editor.snapshot) {
            Ok(s) => s,
            Err(e) => {
                let pending = self.state_machines.pending.clone();
                egui::Window::new("State machines: authoring and diagrams")
                    .id(egui::Id::new("mbse-state-machines"))
                    .show(ctx, |ui| {
                        ui.colored_label(ERROR, e);
                        if let Some(p) = pending {
                            ui.label(
                                "New text remains in this window; accepted data is unchanged.",
                            );
                            let r = ui.button("Copy retained new proposal");
                            self.bound("state-machine:Copy new text", &r);
                            if r.clicked() {
                                ctx.copy_text(serde_json::to_string_pretty(&p.form).unwrap());
                            }
                            let r = ui.button("Discard unadmitted new text");
                            self.bound("state-machine:Discard new text", &r);
                            if r.clicked() {
                                self.state_machines.pending = None;
                                self.state_machines.current = None;
                                self.state_machines.owner = None;
                                self.state_machines.machine = None;
                            }
                        }
                        let r = ui.button("Close state-machine editor");
                        self.bound("state-machine:Close", &r);
                        if r.clicked() && !self.sm_close_blocked() {
                            self.state_machines.open = false;
                        }
                    });
                return;
            }
        };
        let forms = sm::forms(&self.session.editor.snapshot).unwrap_or_default();
        let blueprint = self.state_machines.current.clone().unwrap();
        let owner_ok = self
            .state_machines
            .pending
            .as_ref()
            .map(|p| &p.owner)
            .or(self.state_machines.owner.as_ref())
            .is_none_or(|o| self.sm_owner_matches(o));
        let mut f = self
            .state_machines
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
        let ready = self.sm_ready() && owner_ok;
        let admitted = !self.sm_pending();
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
        egui::Window::new("State machines: authoring and diagrams").id(egui::Id::new("mbse-state-machines")).open(&mut open).default_width(1080.).default_height(1000.).max_height((ctx.content_rect().height()-80.).max(300.)).vscroll(true).show(ctx,|ui|{
   ui.heading("State-machine authoring");if let Err(error)=sm::forms(&self.session.editor.snapshot){ui.colored_label(ERROR,error);}ui.label("Flat states, exact event identities and literal guarded transitions. Accepted values and unfinished proposals stay separate. No execution or transition selection policy is inferred.");if !owner_ok{ui.colored_label(ERROR,"This proposal belongs to another native instance. Copy it or explicitly discard new text; no IDs are rebound.");}
   ui.add_enabled_ui(ready&&admitted,|ui|{ui.horizontal_wrapped(|ui|{for name in["Machine","State","Event","Transition"]{let r=ui.add_enabled(name=="Machine"||self.state_machines.machine.is_some(),egui::Button::new(format!("New {name}")));self.bound(format!("state-machine:new:{name}"),&r);if r.clicked(){pick=Some(self.sm_new_form(name));}}});
    let browse=ui.collapsing("Machines and owned elements",|ui|{for a in store.state_machines.values(){let r=ui.button(format!("Machine #{} · {} · System #{}",a.id.0,a.name,a.system.0));self.bound(format!("state-machine:select:{}",a.id.0),&r);if r.clicked(){pick=Some(Form::Machine{id:Some(a.id),system:a.system.0.to_string(),name:a.name.clone()});}if self.state_machines.machine==Some(a.id){ui.indent(("state-machine-elements",a.id.0),|ui|{
     for s in a.states.values(){let r=ui.button(format!("{:?} #{} · {}",s.kind,s.id.0,s.name));self.bound(format!("state-machine:edit:state:{}",s.id.0),&r);if r.clicked(){pick=Some(Form::State{machine:a.id,id:Some(s.id),name:s.name.clone(),state_kind:s.kind});}}
     for e in a.events.values(){let r=ui.button(format!("Event #{} · {}",e.id.0,e.name));self.bound(format!("state-machine:edit:event:{}",e.id.0),&r);if r.clicked(){pick=Some(Form::Event{machine:a.id,id:Some(e.id),name:e.name.clone()});}}
     for t in a.transitions.values(){let r=ui.button(format!("Transition #{} · {} → {} · Event {} · {}",t.id.0,t.source.0,t.target.0,t.event.map_or("completion".into(),|id|id.0.to_string()),super::activity_editor::guard_text(t.guard)));self.bound(format!("state-machine:edit:transition:{}",t.id.0),&r);if r.clicked(){pick=Some(Form::Transition{machine:a.id,id:Some(t.id),name:t.name.clone(),source:t.source.0.to_string(),target:t.target.0.to_string(),event:t.event.map_or(String::new(),|id|id.0.to_string()),guard:super::activity_editor::guard_text(t.guard)});}}
    });}}});self.bound("state-machine:Browse elements",&browse.header_response);
    let proposals=ui.collapsing(format!("Retained proposals ({})",forms.len()),|ui|{for(key,p)in &forms{let r=ui.button(format!("{key} · {} proposal",family(p)));self.bound(format!("state-machine:proposal:{key}"),&r);if r.clicked(){pick=Some(p.clone());}}});self.bound("state-machine:Browse proposals",&proposals.header_response);
   });
   ui.separator();ui.heading(format!("{} proposal · {}",family(&f),f.id().map_or("new".into(),|id|format!("#{}",id.0))));if let Some(m)=f.machine(){ui.label(format!("Owned by Machine #{}",m.0));}
   ui.add_enabled_ui(ready,|ui|{match &mut f{
    Form::Machine{system,name,..}=>{self.sm_text(ui,"name","Machine name",name,&slot,&instance);self.sm_text(ui,"system","Native System ID",system,&slot,&instance);for r in self.session.editor.snapshot.records.values().filter(|r|r.kind==RecordKind::System){ui.label(format!("System #{} · {}",r.id.0,r.name));}},
    Form::State{name,state_kind,..}=>{self.sm_text(ui,"name","State name",name,&slot,&instance);ui.horizontal(|ui|{for k in[StateKind::Initial,StateKind::State,StateKind::Final]{let r=ui.selectable_value(state_kind,k,format!("{k:?}"));self.bound(format!("state-machine:state-kind:{k:?}"),&r);}});},
    Form::Event{name,..}=>self.sm_text(ui,"name","Event name",name,&slot,&instance),
    Form::Transition{name,source,target,event,guard,..}=>{self.sm_text(ui,"name","Transition name",name,&slot,&instance);self.sm_text(ui,"source","Source state ID",source,&slot,&instance);self.sm_text(ui,"target","Target state ID",target,&slot,&instance);self.sm_text(ui,"event","Event ID (empty = completion)",event,&slot,&instance);self.sm_text(ui,"guard","Guard (empty / true / false / else)",guard,&slot,&instance);if let Some(a)=self.state_machines.machine.and_then(|id|store.state_machines.get(&id)){for s in a.states.values(){ui.label(format!("State #{} · {:?} · {}",s.id.0,s.kind,s.name));}for e in a.events.values(){ui.label(format!("Event #{} · {}",e.id.0,e.name));}}}
   }});let edited=f!=before;
   ui.horizontal_wrapped(|ui|{let r=ui.add_enabled(ready&&admitted&&!edited&&forms.contains_key(&slot),egui::Button::new("Apply proposal"));self.bound("state-machine:Apply",&r);apply=r.clicked();let r=ui.add_enabled(ready&&admitted&&!edited&&forms.contains_key(&slot),egui::Button::new("Discard selected proposal"));self.bound("state-machine:Discard proposal",&r);discard=r.clicked();let r=ui.add_enabled(ready&&admitted&&!edited&&accepted_present(&store,&f),egui::Button::new(if matches!(f,Form::Machine{..}){"Remove accepted machine and owned contents"}else{"Remove accepted element"}));self.bound("state-machine:Remove accepted",&r);remove=r.clicked();});
   ui.horizontal_wrapped(|ui|{ui.label("Global model history");for(redo,label)in[(false,"Model Undo"),(true,"Model Redo")]{let enabled=if redo{self.session.editor.can_redo()}else{self.session.editor.can_undo()};let r=ui.add_enabled(ready&&admitted&&!edited&&enabled,egui::Button::new(label));self.bound(format!("state-machine:{label}"),&r);if r.clicked(){history=Some(redo);}}if let Some(m)=self.state_machines.machine{let r=ui.add_enabled(ready&&admitted&&!edited&&store.state_machines.contains_key(&m),egui::Button::new("Create accepted machine diagram"));self.bound("state-machine:Create diagram",&r);if r.clicked(){create=Some(m);}}});
   if self.sm_pending(){ui.colored_label(ERROR,"New text is retained only in this window.");ui.horizontal(|ui|{let r=ui.button("Copy retained new proposal");self.bound("state-machine:Copy new text",&r);if r.clicked(){ctx.copy_text(serde_json::to_string_pretty(&f).unwrap());}let r=ui.button("Discard unadmitted new text");self.bound("state-machine:Discard new text",&r);discard_pending=r.clicked();});}
   ui.label(&self.state_machines.note);ui.separator();ui.heading("Accepted data and diagnostics");ui.label(accepted_text(&store,&f));if let Some(m)=self.state_machines.machine.and_then(|id|store.state_machines.get(&id)){ui.label(format!("Machine #{} · {} · System #{} · {} states · {} events · {} transitions",m.id.0,m.name,m.system.0,m.states.len(),m.events.len(),m.transitions.len()));for d in sm::diagnostics(&store,m.id).unwrap_or_default(){ui.label(format!("{} · elements {:?}: {}",d.code,d.elements,d.message));}}
   let r=ui.button("Close state-machine editor");self.bound("state-machine:Close",&r);close=r.clicked();
  });
        let edited = f != before;
        let outcome = if discard_pending {
            self.state_machines.pending = None;
            self.state_machines.current = if owner_ok {
                forms
                    .get(&slot)
                    .cloned()
                    .or_else(|| Some(self.sm_new_form(family(&f))))
            } else {
                Some(self.sm_new_form("Machine"))
            };
            self.state_machines.machine = self.state_machines.current.as_ref().and_then(scope);
            self.state_machines.owner = Some(self.sm_owner());
            self.state_machines.note="Unadmitted new text explicitly discarded; stored proposals and accepted data retained.".into();
            Ok(())
        } else if edited {
            self.sm_set_form(&stamp, f.clone())
        } else if let Some(next) = pick {
            self.sm_pick(next, Some(&stamp))
        } else if apply {
            self.sm_apply(&stamp, &f)
        } else if discard {
            self.sm_model(&stamp, sm::Action::Discard(slot.clone()))
                .map(|_| {
                    self.state_machines.current = Some(self.sm_new_form(family(&f)));
                    self.state_machines.machine =
                        self.state_machines.current.as_ref().and_then(scope);
                    self.state_machines.owner = Some(self.sm_owner());
                    self.state_machines.note =
                        "Selected proposal discarded; accepted data unchanged.".into();
                })
        } else if remove {
            self.sm_model(&stamp,sm::Action::Remove{machine:scope(&f).unwrap(),element:f.id().unwrap()}).map(|_|self.state_machines.note="Accepted data removed; proposals retained. Model Undo restores exact identities.".into())
        } else if let Some(redo) = history {
            if !self.sm_ready() {
                Err("Finish pending work before model history.".into())
            } else {
                self.views.views.dispatch(&mut self.session.editor,&stamp,if redo{nv::NativeAction::ModelRedo}else{nv::NativeAction::ModelUndo}).map(|_|{self.views.reconcile();self.state_machines.note="Global model history restored; newer state-machine proposals retained.".into();})
            }
        } else if let Some(m) = create {
            self.sm_create_diagram(&stamp, m)
        } else {
            Ok(())
        };
        if let Err(e) = outcome {
            if !edited {
                self.state_machines.note = e;
            }
        }
        if close || !open {
            if self.sm_pending() {
                self.sm_close_blocked();
            } else {
                self.state_machines.open = false;
            }
        }
    }
    pub(super) fn sm_audit(&self) -> serde_json::Value {
        let forms = sm::forms(&self.session.editor.snapshot).ok();
        let owner_current = self
            .state_machines
            .pending
            .as_ref()
            .map(|p| &p.owner)
            .or(self.state_machines.owner.as_ref())
            .is_none_or(|o| self.sm_owner_matches(o));
        let current = self
            .state_machines
            .pending
            .as_ref()
            .map(|p| p.form.clone())
            .or_else(|| {
                self.state_machines.current.as_ref().map(|f| {
                    forms
                        .as_ref()
                        .filter(|_| owner_current)
                        .and_then(|m| m.get(&f.key()).cloned())
                        .unwrap_or_else(|| f.clone())
                })
            });
        let store = ty::load(&self.session.editor.snapshot).ok();
        serde_json::json!({"open":self.state_machines.open,"owner_current":owner_current,"machine":self.state_machines.machine,"current":current,"form_admitted":!self.sm_pending(),"note":self.state_machines.note,"accepted":store.as_ref().map(|s|&s.state_machines),"proposals":forms,"diagnostics":store.as_ref().and_then(|s|self.state_machines.machine.and_then(|m|sm::diagnostics(s,m).ok())),"gesture_active":self.sm_gesture_active(),"selected":self.state_machines.selected,"rendered":self.state_machines.rendered.as_ref().map(|r|serde_json::json!({"view":r.view,"current":r.view==self.views.active&&r.stamp==self.views.views.stamp(&self.session.editor),"preview_only":r.preview,"occurrences":r.occurrences.iter().map(|(id,element,g)|serde_json::json!({"occurrence":id,"element":element,"geometry":g})).collect::<Vec<_>>(),"transitions":r.transitions,"endpoints":r.endpoints,"labels":r.labels,"overlapping_transitions":r.overlapping}))})
    }
}
fn accepted_present(s: &ty::Store, f: &Form) -> bool {
    let Some(id) = f.id() else {
        return false;
    };
    match f {
        Form::Machine { .. } => s.state_machines.contains_key(&id),
        Form::State { machine, .. } => s
            .state_machines
            .get(machine)
            .is_some_and(|a| a.states.contains_key(&id)),
        Form::Event { machine, .. } => s
            .state_machines
            .get(machine)
            .is_some_and(|a| a.events.contains_key(&id)),
        Form::Transition { machine, .. } => s
            .state_machines
            .get(machine)
            .is_some_and(|a| a.transitions.contains_key(&id)),
    }
}
fn accepted_text(s: &ty::Store, f: &Form) -> String {
    if !accepted_present(s, f) {
        return "No accepted object in this slot. Missing identities are not rebound; proposals stay separate.".into();
    }
    let id = f.id().unwrap();
    match f {
        Form::Machine { .. } => {
            let a = &s.state_machines[&id];
            format!(
                "Accepted Machine #{} · {} · System #{}",
                id.0, a.name, a.system.0
            )
        }
        _ => {
            let a = &s.state_machines[&f.machine().unwrap()];
            match f {
                Form::State { .. } => {
                    let n = &a.states[&id];
                    format!("Accepted {:?} #{} · {}", n.kind, id.0, n.name)
                }
                Form::Event { .. } => format!("Accepted Event #{} · {}", id.0, a.events[&id].name),
                Form::Transition { .. } => {
                    let t = &a.transitions[&id];
                    format!(
                        "Accepted Transition #{} · {} · {} → {} · Event {} · guard {}",
                        id.0,
                        t.name,
                        t.source.0,
                        t.target.0,
                        t.event.map_or("completion".into(), |id| id.0.to_string()),
                        super::activity_editor::guard_text(t.guard)
                    )
                }
                _ => unreachable!(),
            }
        }
    }
}
#[cfg(test)]
#[path = "ui_state_machine_tests.rs"]
mod tests;
