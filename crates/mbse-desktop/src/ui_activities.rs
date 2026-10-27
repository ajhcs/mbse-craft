//! Native activity authoring; one guarded typed authority and durable keyed proposals.
use super::*;
use mbse_model::{
    native_views::{self as nv, OccurrenceId, Stamp, ViewId},
    types::{
        self as ty, Id,
        activities::{
            self as ac,
            forms::{self as af, Form},
        },
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
pub(super) struct Drag {
    pub view: ViewId,
    pub occurrence: OccurrenceId,
    pub stamp: Stamp,
    pub geometry: nv::Geometry,
    pub start: egui::Pos2,
    pub viewport: Rect,
    pub canvas_origin: egui::Pos2,
}
pub(super) struct Rendered {
    pub view: ViewId,
    pub stamp: Stamp,
    pub occurrences: Vec<(OccurrenceId, Id, nv::Geometry)>,
    pub flows: Vec<Id>,
    pub endpoints: Vec<(Id, [f32; 2], [f32; 2])>,
    pub overlapping_flows: usize,
    pub preview: bool,
}
#[derive(Default)]
pub(super) struct ActivityPanel {
    pub execution: super::activity_execution::Panel,
    pub open: bool,
    pub activity: Option<Id>,
    pub current: Option<Form>,
    owner: Option<Owner>,
    pending: Option<Pending>,
    pub note: String,
    pub selected: Option<(ViewId, OccurrenceId)>,
    pub drag: Option<Drag>,
    pub rendered: Option<Rendered>,
}
fn scope(form: &Form) -> Option<Id> {
    match form {
        Form::Activity { id, .. } => *id,
        Form::Partition { activity, .. }
        | Form::Node { activity, .. }
        | Form::Pin { activity, .. } => Some(*activity),
        Form::Flow { form } => Some(form.activity),
    }
}
fn family(form: &Form) -> &'static str {
    match form {
        Form::Activity { .. } => "Activity",
        Form::Partition { .. } => "Partition",
        Form::Node { .. } => "Node",
        Form::Pin { .. } => "Pin",
        Form::Flow { .. } => "Flow",
    }
}
impl Workspace {
    fn activity_owner(&self) -> Owner {
        Owner {
            instance: self.session.editor.freshness().instance_id,
            document: self.session.editor.snapshot.document_id.clone(),
        }
    }
    fn activity_owner_matches(&self, owner: &Owner) -> bool {
        owner.instance == self.session.editor.freshness().instance_id
            && owner.document == self.session.editor.snapshot.document_id
    }
    pub(super) fn activity_frame_start(&mut self) {
        self.activities.rendered = None;
        self.activity_execution_tick();
    }
    pub(super) fn activity_gesture_active(&self) -> bool {
        self.activities.drag.is_some()
    }
    pub(super) fn activity_pending(&self) -> bool {
        self.activities.pending.is_some()
    }
    pub(super) fn activity_close_blocked(&mut self) -> bool {
        if self.activity_execution_active() {
            self.activities.open = true;
            self.activities.note = "Cancel the prepared or running execution before closing.".into();
            self.activities.execution.note = self.activities.note.clone();
            return true;
        }
        if self.activities.pending.is_some() {
            self.activities.open = true;
            self.activities.note="New text is retained in this window. Admit it, copy it, or explicitly discard it before closing.".into();
            true
        } else {
            self.activities.drag = None;
            false
        }
    }
    pub(super) fn activity_open(&mut self) {
        self.activities.open = true;
        if self.activities.current.is_none() {
            let form = self.activity_new_form("Activity");
            let _ = self.activity_pick(form, None);
        }
    }
    pub(super) fn activity_ready(&self) -> bool {
        !self.activity_execution_automatic()
            && !self.dock_blocked_without_activity()
            && self.session.editor.snapshot.extensions.is_some()
            && ty::load(&self.session.editor.snapshot).is_ok()
            && af::load(&self.session.editor.snapshot).is_ok()
    }
    fn activity_new_form(&self, family: &str) -> Form {
        let a = self.activities.activity.unwrap_or(Id(0));
        match family {
            "Partition" => Form::Partition {
                activity: a,
                id: None,
                name: String::new(),
            },
            "Node" => Form::Node {
                activity: a,
                id: None,
                name: String::new(),
                node_kind: ac::NodeKind::Action,
                partition: String::new(),
            },
            "Pin" => Form::Pin {
                activity: a,
                id: None,
                owner: String::new(),
                name: String::new(),
                direction: ac::Direction::In,
                value_type: String::new(),
            },
            "Flow" => Form::Flow {
                form: ac::FlowForm {
                    activity: a,
                    id: None,
                    name: String::new(),
                    kind: ac::FlowKind::Control,
                    source: String::new(),
                    target: String::new(),
                    guard: String::new(),
                },
            },
            _ => Form::Activity {
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
    pub(super) fn activity_pick(
        &mut self,
        form: Form,
        stamp: Option<&Stamp>,
    ) -> Result<(), String> {
        if self.activities.pending.is_some() {
            return Err(
                "Preserve or explicitly discard the new text before changing selection.".into(),
            );
        }
        if self.dock_blocked_without_activity() {
            return Err("Finish the gesture/write or close the other editor before selecting activity data.".into());
        }
        if stamp.is_some_and(|stamp| *stamp != self.views.views.stamp(&self.session.editor)) {
            return Err("Activity selection source changed; original selection retained.".into());
        }
        let key = form.key();
        let form = af::load(&self.session.editor.snapshot)?
            .get(&key)
            .cloned()
            .unwrap_or(form);
        self.activities.activity = scope(&form);
        self.activities.current = Some(form);
        self.activities.owner = Some(self.activity_owner());
        self.activities.open = true;
        Ok(())
    }
    pub(super) fn activity_set_form(&mut self, stamp: &Stamp, form: Form) -> Result<(), String> {
        let result = self.activity_model(stamp, ac::Action::Form(af::Action::Set(form.clone())));
        match &result {
            Ok(()) => {
                self.activities.pending = None;
                self.activities.note =
                    "Proposal retained. Apply to update accepted activity data.".into();
            }
            Err(error) => {
                let owner = self
                    .activities
                    .owner
                    .clone()
                    .unwrap_or_else(|| self.activity_owner());
                self.activities.pending = Some(Pending {
                    owner,
                    form: form.clone(),
                });
                self.activities.note = format!(
                    "{error} New text is retained in this window; accepted data is unchanged."
                );
            }
        }
        self.activities.current = Some(form);
        result
    }
    pub(super) fn activity_model(
        &mut self,
        stamp: &Stamp,
        action: ac::Action,
    ) -> Result<(), String> {
        if !self.activity_ready() {
            return Err("Finish pending work or open an admitted MBSE-Craft model copy before activity editing.".into());
        }
        let result = self
            .views
            .views
            .dispatch(
                &mut self.session.editor,
                stamp,
                nv::NativeAction::Command(Command::Typed(ty::Action::Activity(action))),
            )
            .map(|_| ());
        self.views.reconcile();
        result
    }
    pub(super) fn activity_layout(
        &mut self,
        stamp: &Stamp,
        action: nv::activities::Layout,
    ) -> Result<(), String> {
        if self.dock_blocked_without_activity() || self.activities.pending.is_some() {
            return Err(
                "Finish pending work and preserve the new form text before changing this diagram."
                    .into(),
            );
        }
        self.views
            .views
            .apply(&self.session.editor, stamp, nv::Layout::Activity(action))?;
        self.views.reconcile();
        Ok(())
    }
    fn activity_apply_form(&mut self, stamp: &Stamp, form: &Form) -> Result<(), String> {
        let allocated = form
            .id()
            .unwrap_or(Id(self.session.editor.snapshot.next_id));
        self.activity_model(stamp, ac::Action::Form(af::Action::Apply(form.key())))?;
        let mut current = form.clone();
        match &mut current {
            Form::Activity { id, .. }
            | Form::Partition { id, .. }
            | Form::Node { id, .. }
            | Form::Pin { id, .. } => *id = Some(allocated),
            Form::Flow { form } => form.id = Some(allocated),
        }
        self.activities.activity = scope(&current);
        self.activities.current = Some(current);
        self.activities.note =
            "Accepted activity data updated. Model Undo/Redo retains newer proposals.".into();
        Ok(())
    }
    pub(super) fn activity_create_diagram(
        &mut self,
        stamp: &Stamp,
        activity: Id,
    ) -> Result<(), String> {
        let store = ty::load(&self.session.editor.snapshot)?;
        let name = &store
            .activities
            .get(&activity)
            .ok_or("Activity is missing; proposal retained.")?
            .name;
        let view: ViewId =
            serde_json::from_str(&format!("\"{}\"", self.views.views.document().next_id()))
                .map_err(|e| format!("Diagram identity unavailable: {e}"))?;
        self.activity_layout(
            stamp,
            nv::activities::Layout::Create {
                activity,
                title: format!("Activity: {name}"),
            },
        )?;
        self.views.active = view;
        self.views.reconcile();
        self.activities.selected = None;
        self.activities.note="Independent diagram created from accepted activity data. It does not execute the activity.".into();
        Ok(())
    }
    fn activity_text(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        label: &str,
        value: &mut String,
        slot: &str,
        instance: &str,
    ) {
        ui.horizontal(|ui| {
            ui.label(label);
            let r = ui.add(
                egui::TextEdit::singleline(value)
                    .char_limit(4096)
                    .desired_width(490.)
                    .id(egui::Id::new(("activity-proposal", instance, slot, key))),
            );
            self.bound(format!("activity:field:{key}"), &r);
        });
    }
    pub(super) fn activity_dialog(&mut self, ctx: &egui::Context) {
        if !self.activities.open {
            return;
        }
        if self.activities.current.is_none() {
            self.activities.current = Some(self.activity_new_form("Activity"));
            self.activities.owner = Some(self.activity_owner());
        }
        let store = match ty::load(&self.session.editor.snapshot) {
            Ok(s) => s,
            Err(error) => {
                let pending = self.activities.pending.clone();
                egui::Window::new("Activities: authoring and diagrams")
                    .id(egui::Id::new("mbse-activities"))
                    .show(ctx, |ui| {
                        ui.colored_label(ERROR, &error);
                        if let Some(p) = &pending {
                            ui.label("New text remains in this window; the model is unchanged.");
                            let r = ui.button("Copy retained new proposal");
                            self.bound("activity:Copy new text", &r);
                            if r.clicked() {
                                ctx.copy_text(serde_json::to_string_pretty(&p.form).unwrap());
                            }
                            let r = ui.button("Discard unadmitted new text");
                            self.bound("activity:Discard new text", &r);
                            if r.clicked() {
                                self.activities.pending = None;
                                self.activities.current = None;
                                self.activities.owner = None;
                                self.activities.activity = None;
                            }
                        }
                        self.activity_execution_ui(ui, false);
                        let r = ui.button("Close activity editor");
                        self.bound("activity:Close", &r);
                        if r.clicked() && !self.activity_close_blocked() {
                            self.activities.open = false;
                        }
                    });
                return;
            }
        };
        let forms = af::load(&self.session.editor.snapshot).unwrap_or_default();
        let blueprint = self.activities.current.clone().unwrap();
        let owner_ok = self
            .activities
            .pending
            .as_ref()
            .map(|p| &p.owner)
            .or(self.activities.owner.as_ref())
            .is_none_or(|o| self.activity_owner_matches(o));
        let mut form = self
            .activities
            .pending
            .as_ref()
            .map(|p| p.form.clone())
            .or_else(|| {
                owner_ok
                    .then(|| forms.get(&blueprint.key()).cloned())
                    .flatten()
            })
            .unwrap_or(blueprint);
        let before = form.clone();
        let slot = form.key();
        let instance = self.session.editor.freshness().instance_id;
        let stamp = self.views.views.stamp(&self.session.editor);
        let ready = self.activity_ready() && owner_ok;
        let admitted = self.activities.pending.is_none();
        let mut pick = None;
        let mut apply = false;
        let mut discard = false;
        let mut discard_pending = false;
        let mut remove = false;
        let mut history = None;
        let mut create = None;
        let mut close = false;
        let mut open = true;
        egui::Window::new("Activities: authoring and diagrams").id(egui::Id::new("mbse-activities")).open(&mut open).default_width(1080.).default_height(1000.).max_height((ctx.content_rect().height()-80.).max(300.)).vscroll(true).show(ctx,|ui|{
  ui.heading("Activity authoring");ui.label("Accepted data and unfinished proposals stay separate. Diagrams describe the model; they do not execute it.");
  if !owner_ok{ui.colored_label(ERROR,"This proposal belongs to another model instance. It will not be rebound; copy it or explicitly discard new text.");}
  if self.session.editor.snapshot.extensions.is_none(){ui.colored_label(ERROR,"Open an admitted MBSE-Craft model copy before authoring activities. The current model remains unchanged.");}
  ui.add_enabled_ui(ready&&admitted,|ui|{
   ui.horizontal_wrapped(|ui|{for name in ["Activity","Partition","Node","Pin","Flow"]{let r=ui.add_enabled(name=="Activity"||self.activities.activity.is_some(),egui::Button::new(format!("New {name}")));self.bound(format!("activity:new:{name}"),&r);if r.clicked(){pick=Some(self.activity_new_form(name));}}});
   let browse=ui.collapsing("Activities and owned elements",|ui|{for a in store.activities.values(){let r=ui.button(format!("Activity #{} · {} · System #{}",a.id.0,a.name,a.system.0));self.bound(format!("activity:select:{}",a.id.0),&r);if r.clicked(){pick=Some(Form::Activity{id:Some(a.id),system:a.system.0.to_string(),name:a.name.clone()});}if self.activities.activity==Some(a.id){ui.indent(("activity-elements",a.id.0),|ui|{
    for p in a.partitions.values(){let r=ui.button(format!("Partition #{} · {}",p.id.0,p.name));self.bound(format!("activity:edit:partition:{}",p.id.0),&r);if r.clicked(){pick=Some(Form::Partition{activity:a.id,id:Some(p.id),name:p.name.clone()});}}
    for n in a.nodes.values(){let r=ui.button(format!("{:?} #{} · {}",n.kind,n.id.0,n.name));self.bound(format!("activity:edit:node:{}",n.id.0),&r);if r.clicked(){pick=Some(Form::Node{activity:a.id,id:Some(n.id),name:n.name.clone(),node_kind:n.kind,partition:n.partition.map_or(String::new(),|id|id.0.to_string())});}}
    for p in a.pins.values(){let r=ui.button(format!("{:?} pin #{} · {} · Action #{} · Type #{}",p.direction,p.id.0,p.name,p.owner.0,p.value_type.0));self.bound(format!("activity:edit:pin:{}",p.id.0),&r);if r.clicked(){pick=Some(Form::Pin{activity:a.id,id:Some(p.id),owner:p.owner.0.to_string(),name:p.name.clone(),direction:p.direction,value_type:p.value_type.0.to_string()});}}
    for f in a.flows.values(){let r=ui.button(format!("{:?} flow #{} · {} → {} · {:?}",f.kind,f.id.0,f.source.0,f.target.0,f.guard));self.bound(format!("activity:edit:flow:{}",f.id.0),&r);if r.clicked(){pick=Some(Form::Flow{form:ac::FlowForm{activity:a.id,id:Some(f.id),name:f.name.clone(),kind:f.kind,source:f.source.0.to_string(),target:f.target.0.to_string(),guard:guard_text(f.guard)}});}}
   });}}});self.bound("activity:Browse elements",&browse.header_response);
   let proposals=ui.collapsing(format!("Retained proposals ({})",forms.len()),|ui|{for(key,proposal)in &forms{let r=ui.button(format!("{key} · {} proposal",family(proposal)));self.bound(format!("activity:proposal:{key}"),&r);if r.clicked(){pick=Some(proposal.clone());}}});self.bound("activity:Browse proposals",&proposals.header_response);
  });
  ui.separator();ui.heading(format!("{} proposal · {}",family(&form),form.id().map_or("new".into(),|id|format!("#{}",id.0))));if !matches!(form,Form::Activity{..}){if let Some(a)=scope(&form){ui.label(format!("Owned by activity #{}",a.0));}}
  ui.add_enabled_ui(ready,|ui|{match &mut form{
   Form::Activity{system,name,..}=>{self.activity_text(ui,"name","Activity name",name,&slot,&instance);self.activity_text(ui,"system","Native System ID",system,&slot,&instance);for record in self.session.editor.snapshot.records.values().filter(|r|r.kind==RecordKind::System){ui.label(format!("System #{} · {}",record.id.0,record.name));}},
   Form::Partition{name,..}=>self.activity_text(ui,"name","Partition name",name,&slot,&instance),
   Form::Node{name,node_kind,partition,..}=>{self.activity_text(ui,"name","Node name",name,&slot,&instance);ui.horizontal_wrapped(|ui|{for kind in [ac::NodeKind::Initial,ac::NodeKind::Action,ac::NodeKind::Decision,ac::NodeKind::Merge,ac::NodeKind::Final]{let r=ui.selectable_value(node_kind,kind,format!("{kind:?}"));self.bound(format!("activity:node-kind:{kind:?}"),&r);}});self.activity_text(ui,"partition","Partition ID (empty = none)",partition,&slot,&instance);},
   Form::Pin{owner,name,direction,value_type,..}=>{self.activity_text(ui,"name","Pin name",name,&slot,&instance);self.activity_text(ui,"owner","Owning Action node ID",owner,&slot,&instance);ui.horizontal(|ui|{for d in [ac::Direction::In,ac::Direction::Out]{let r=ui.selectable_value(direction,d,format!("{d:?}"));self.bound(format!("activity:pin-direction:{d:?}"),&r);}});self.activity_text(ui,"value-type","Value type ID",value_type,&slot,&instance);ui.label("Pins require an existing Value type in the same System; object flows require exact type identity.");for d in store.definitions.values().filter(|d|matches!(d.kind,ty::Kind::Value(_))){ui.label(format!("Value type #{} · {} · System #{}",d.id.0,d.name,d.system.0));}},
   Form::Flow{form:f}=>{self.activity_text(ui,"name","Flow name",&mut f.name,&slot,&instance);ui.horizontal(|ui|{for k in [ac::FlowKind::Control,ac::FlowKind::Object]{let r=ui.selectable_value(&mut f.kind,k,format!("{k:?}"));self.bound(format!("activity:flow-kind:{k:?}"),&r);}});self.activity_text(ui,"source","Source node/pin ID",&mut f.source,&slot,&instance);self.activity_text(ui,"target","Target node/pin ID",&mut f.target,&slot,&instance);self.activity_text(ui,"guard","Decision guard (true, false, else)",&mut f.guard,&slot,&instance);ui.label("Control flows use node IDs. Object flows use Out→In pin IDs. Only Decision control flows have a guard; leave other guards empty.");},
  }});
  let edited=form!=before;ui.horizontal_wrapped(|ui|{
   let r=ui.add_enabled(ready&&admitted&&!edited&&forms.contains_key(&slot),egui::Button::new("Apply proposal"));self.bound("activity:Apply",&r);apply=r.clicked();
   let r=ui.add_enabled(ready&&admitted&&!edited&&forms.contains_key(&slot),egui::Button::new("Discard selected proposal"));self.bound("activity:Discard proposal",&r);discard=r.clicked();
   let r=ui.add_enabled(ready&&admitted&&!edited&&accepted_present(&store,&form),egui::Button::new(if matches!(form,Form::Activity{..}){"Remove accepted activity and its owned contents"}else{"Remove accepted element (references must be removed first)"}));self.bound("activity:Remove accepted",&r);remove=r.clicked();
  });
  ui.horizontal_wrapped(|ui|{ui.label("Global model history");for(redo,label)in[(false,"Model Undo"),(true,"Model Redo")]{let enabled=if redo{self.session.editor.can_redo()}else{self.session.editor.can_undo()};let r=ui.add_enabled(ready&&admitted&&!edited&&enabled,egui::Button::new(label));self.bound(format!("activity:{label}"),&r);if r.clicked(){history=Some(redo);}}if let Some(a)=self.activities.activity{let r=ui.add_enabled(ready&&admitted&&!edited&&store.activities.contains_key(&a),egui::Button::new("Create independent diagram"));self.bound("activity:Create diagram",&r);if r.clicked(){create=Some(a);}}});
  if self.activities.pending.is_some(){ui.colored_label(ERROR,"New text is retained only in this window.");ui.horizontal(|ui|{let r=ui.button("Copy retained new proposal");self.bound("activity:Copy new text",&r);if r.clicked(){ctx.copy_text(serde_json::to_string_pretty(&form).expect("proposal serializes"));}let r=ui.button("Discard unadmitted new text");self.bound("activity:Discard new text",&r);discard_pending=r.clicked();});}
  self.activity_execution_ui(ui, ready && admitted && !edited);
  ui.label(&self.activities.note);ui.separator();ui.heading("Accepted data");if accepted_present(&store,&form){ui.label(accepted_text(&store,&form));}else{ui.label("No accepted object in this slot. Unfinished proposals remain separate; missing IDs are not rebound.");}if let Some(a)=self.activities.activity.and_then(|id|store.activities.get(&id)){ui.label(format!("Activity #{} · {} · System #{} · {} partitions · {} nodes · {} pins · {} flows",a.id.0,a.name,a.system.0,a.partitions.len(),a.nodes.len(),a.pins.len(),a.flows.len()));for d in ac::diagnostics(&store,a.id).unwrap_or_default(){ui.label(format!("{}: {}",d.code,d.message));}}
  let r=ui.button("Close activity editor");self.bound("activity:Close",&r);close=r.clicked();
 });
        let edited = form != before;
        let outcome = if discard_pending {
            self.activities.pending = None;
            self.activities.current = if owner_ok {
                forms
                    .get(&slot)
                    .cloned()
                    .or_else(|| Some(self.activity_new_form(family(&form))))
            } else {
                Some(self.activity_new_form("Activity"))
            };
            self.activities.activity = self.activities.current.as_ref().and_then(scope);
            self.activities.owner = Some(self.activity_owner());
            self.activities.note="Unadmitted new text explicitly discarded; stored proposals and accepted data retained.".into();
            Ok(())
        } else if edited {
            self.activity_set_form(&stamp, form.clone())
        } else if let Some(next) = pick {
            self.activity_pick(next, Some(&stamp))
        } else if apply {
            self.activity_apply_form(&stamp, &form)
        } else if discard {
            self.activity_model(&stamp, ac::Action::Form(af::Action::Discard(slot.clone())))
                .map(|_| {
                    self.activities.current = Some(self.activity_new_form(family(&form)));
                    self.activities.activity = self.activities.current.as_ref().and_then(scope);
                    self.activities.owner = Some(self.activity_owner());
                    self.activities.note =
                        "Selected raw proposal explicitly discarded; accepted data unchanged."
                            .into();
                })
        } else if remove {
            let id = form.id().unwrap();
            let a = scope(&form).unwrap();
            self.activity_model(&stamp,ac::Action::Remove{activity:a,element:id}).map(|_|self.activities.note="Accepted element removed; proposals retained. Model Undo can restore the exact data.".into())
        } else if let Some(redo) = history {
            if !self.activity_ready() {
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
                        self.activities.note =
                            "Global model history restored; newer activity proposals retained."
                                .into();
                    })
            }
        } else if let Some(a) = create {
            self.activity_create_diagram(&stamp, a)
        } else {
            Ok(())
        };
        if let Err(error) = outcome {
            if !edited {
                self.activities.note = error;
            }
        }
        if close || !open {
            if self.activities.pending.is_some() || self.activity_execution_active() {
                self.activity_close_blocked();
            } else {
                self.activities.open = false;
            }
        }
    }
    pub(super) fn activity_audit(&self) -> serde_json::Value {
        let forms = af::load(&self.session.editor.snapshot).ok();
        let owner_current = self
            .activities
            .pending
            .as_ref()
            .map(|p| &p.owner)
            .or(self.activities.owner.as_ref())
            .is_none_or(|o| self.activity_owner_matches(o));
        let current = self
            .activities
            .pending
            .as_ref()
            .map(|p| p.form.clone())
            .or_else(|| {
                self.activities.current.as_ref().map(|f| {
                    forms
                        .as_ref()
                        .filter(|_| owner_current)
                        .and_then(|m| m.get(&f.key()).cloned())
                        .unwrap_or_else(|| f.clone())
                })
            });
        serde_json::json!({"execution":self.activity_execution_audit(),"open":self.activities.open,"owner_current":owner_current,"activity":self.activities.activity,"current":current,"form_admitted":self.activities.pending.is_none(),"note":self.activities.note,"accepted":ty::load(&self.session.editor.snapshot).ok().map(|s|s.activities),"proposals":forms,"gesture_active":self.activity_gesture_active(),"selected":self.activities.selected,"rendered":self.activities.rendered.as_ref().map(|r|serde_json::json!({"view":r.view,"current":r.view==self.views.active&&r.stamp==self.views.views.stamp(&self.session.editor),"preview_only":r.preview,"occurrences":r.occurrences.iter().map(|(id,element,geometry)|serde_json::json!({"occurrence":id,"element":element,"geometry":geometry})).collect::<Vec<_>>(),"flows":r.flows,"endpoints":r.endpoints,"overlapping_flows":r.overlapping_flows}))})
    }
}
pub(super) fn guard_text(guard: Option<ac::Guard>) -> String {
    match guard {
        None => String::new(),
        Some(ac::Guard::Boolean(true)) => "true".into(),
        Some(ac::Guard::Boolean(false)) => "false".into(),
        Some(ac::Guard::Else) => "else".into(),
    }
}
fn accepted_present(s: &ty::Store, f: &Form) -> bool {
    let Some(id) = f.id() else { return false };
    match f {
        Form::Activity { .. } => s.activities.contains_key(&id),
        Form::Partition { activity, .. } => s
            .activities
            .get(activity)
            .is_some_and(|a| a.partitions.contains_key(&id)),
        Form::Node { activity, .. } => s
            .activities
            .get(activity)
            .is_some_and(|a| a.nodes.contains_key(&id)),
        Form::Pin { activity, .. } => s
            .activities
            .get(activity)
            .is_some_and(|a| a.pins.contains_key(&id)),
        Form::Flow { form } => s
            .activities
            .get(&form.activity)
            .is_some_and(|a| a.flows.contains_key(&id)),
    }
}
fn accepted_text(s: &ty::Store, f: &Form) -> String {
    let id = f.id().unwrap();
    match f {
        Form::Activity { .. } => {
            let a = &s.activities[&id];
            format!(
                "Accepted activity #{} · {} · System #{}",
                id.0, a.name, a.system.0
            )
        }
        _ => {
            let a = &s.activities[&scope(f).unwrap()];
            match f {
                Form::Partition { .. } => {
                    format!("Accepted partition #{} · {}", id.0, a.partitions[&id].name)
                }
                Form::Node { .. } => {
                    let n = &a.nodes[&id];
                    format!(
                        "Accepted {:?} #{} · {} · Partition {}",
                        n.kind,
                        id.0,
                        n.name,
                        n.partition.map_or("none".into(), |id| id.0.to_string())
                    )
                }
                Form::Pin { .. } => {
                    let p = &a.pins[&id];
                    format!(
                        "Accepted {:?} pin #{} · {} · Action #{} · Type #{}",
                        p.direction, id.0, p.name, p.owner.0, p.value_type.0
                    )
                }
                Form::Flow { .. } => {
                    let flow = &a.flows[&id];
                    format!(
                        "Accepted {:?} flow #{} · {} · {} → {} · guard {}",
                        flow.kind,
                        id.0,
                        flow.name,
                        flow.source.0,
                        flow.target.0,
                        guard_text(flow.guard)
                    )
                }
                _ => String::new(),
            }
        }
    }
}
#[cfg(test)]
#[path = "ui_activity_tests.rs"]
mod tests;
