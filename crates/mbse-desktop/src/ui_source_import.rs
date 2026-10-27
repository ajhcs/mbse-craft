//! Explicit source review and one native Apply. Workers never receive a live target.
use super::*;
use mbse_exchange::{project_sets::ProjectSet,source_admission as source};
use std::{collections::BTreeSet,sync::{Arc,mpsc::{self,Receiver}}};
enum Completion {Prepared(source::Prepared),Checked(source::Checked)}
struct Pending {receiver:Receiver<Result<Completion,String>>,cancelled:bool}
#[derive(Default)]
pub(super) struct Panel {
 pub open:bool,
 source:Option<Arc<ProjectSet>>,
 member:String,
 system:String,
 prepared:Option<source::Prepared>,
 acknowledgements:BTreeSet<String>,
 pending:Option<Pending>,
 applied:Option<source::Applied>,
 note:String,
 error:bool,
}
impl Panel {
 pub fn pending(&self)->bool{self.pending.is_some()}
 fn system(&self)->Result<RecordId,String>{let s=&self.system;let id=s.parse::<u64>().map_err(|_|"Choose a canonical native System ID")?;if id==0||id.to_string()!=*s{return Err("Choose a canonical native System ID".into());}Ok(RecordId(id))}
 fn scope_matches(&self,p:&source::Prepared)->bool{self.system().ok()==Some(p.summary().system)&&self.member==p.summary().selected_member&&self.source.as_ref().is_some_and(|s|s.summary().source_set_digest==p.summary().source_set_digest)}
}
impl Workspace {
 pub(super) fn source_import_request_close(&mut self){
  self.source_import.open=false;
  if let Some(pending)=&mut self.source_import.pending{pending.cancelled=true;}
 }
 pub(super) fn source_import_busy(&self)->bool{
  self.views.occurrence_navigation_blocked(&self.session)||self.pending.is_some()||self.creation.is_some()||self.context_creation.is_some()||self.files.open||self.files.pending()||self.activity_execution_automatic()||self.sm_execution_automatic()
 }
 pub(super) fn open_source_import(&mut self,request:crate::project_set_ui::ImportRequest){
  if self.source_import.pending(){self.source_import.open=true;self.source_import.note="Finish or cancel the current import worker before choosing another source.".into();return;}
  let same=self.source_import.source.as_ref().is_some_and(|s|Arc::ptr_eq(s,&request.source))&&self.source_import.member==request.member;
  if !same{self.source_import=Panel{source:Some(request.source),member:request.member,system:String::new(),..Default::default()};}
  self.source_import.open=true;
 }
 pub(super) fn source_import_audit(&self)->serde_json::Value {
  let p=&self.source_import;
  serde_json::json!({"open":p.open,"pending":p.pending(),"cancelled":p.pending.as_ref().is_some_and(|j|j.cancelled),"member":p.member,"system":p.system,"system_error":p.system().err(),"summary":p.prepared.as_ref().map(|p|p.summary()),"target_current":p.prepared.as_ref().is_some_and(|p|p.target_current(&self.session.editor)&&self.source_import.scope_matches(p)),"acknowledgements":p.acknowledgements,"applied":p.applied,"applied_current":p.applied.as_ref().is_some_and(|a|a.target_snapshot_sha256==self.session.editor.snapshot.digest()),"note":p.note,"error":p.error})
 }
 pub(super) fn source_import_dialog(&mut self,ctx:&egui::Context){
  let mut panel=std::mem::take(&mut self.source_import);
  let done=panel.pending.as_ref().and_then(|p|match p.receiver.try_recv(){Ok(r)=>Some(r),Err(mpsc::TryRecvError::Empty)=>None,Err(mpsc::TryRecvError::Disconnected)=>Some(Err("Source worker disconnected; native work retained.".into()))});
  if panel.pending(){ctx.request_repaint_after(std::time::Duration::from_millis(100));}
  let busy=self.source_import_busy();let mut open=panel.open;let mut cancel=false;let mut close=false;let mut prepare=false;let mut apply=false;let mut history=None;
  let old_prepared=panel.prepared.take();
  if open{
   egui::Window::new("Review native source import").id(egui::Id::new("native-source-import")).open(&mut open).default_size(egui::vec2(1050.,850.)).vscroll(true).show(ctx,|ui|{
    ui.set_max_width(ui.available_width());ui.label("Review a supported frozen source member before one undoable native import. Original source files stay unchanged.");
    ui.label(format!("Source member: {}",panel.member));
    let before=panel.system.clone();ui.horizontal_wrapped(|ui|{ui.label("Destination System ID");let r=ui.add_enabled(!panel.pending(),egui::TextEdit::singleline(&mut panel.system).char_limit(20).desired_width(120.));self.bound("import:system",&r);
     let total=self.session.editor.snapshot.records.values().filter(|r|r.kind==RecordKind::System).count();let systems=self.session.editor.snapshot.records.values().filter(|r|r.kind==RecordKind::System).take(32).map(|r|(r.id,r.name.clone())).collect::<Vec<_>>();for(id,name)in systems{let r=ui.add_enabled(!panel.pending(),egui::Button::new(format!("System #{} · {}",id.0,name)).selected(panel.system==id.0.to_string()));self.bound(format!("import:system:{}",id.0),&r);if r.clicked(){panel.system=id.0.to_string();}}if total>32{ui.label("First32 Systems shown; enter an exact ID for another System.");}
    });let edited=panel.system!=before;
    let scope=panel.system();let system_valid=scope.as_ref().is_ok_and(|id|self.session.editor.snapshot.records.get(id).is_some_and(|r|r.kind==RecordKind::System));
    if !system_valid{ui.colored_label(ERROR,scope.as_ref().err().cloned().unwrap_or("Destination must be an existing System".into()));}
    ui.horizontal_wrapped(|ui|{
     let r=ui.add_enabled(!busy&&!panel.pending()&&!edited&&system_valid&&panel.source.is_some(),egui::Button::new("Prepare mapping"));self.bound("import:Prepare",&r);prepare=r.clicked();
     let r=ui.add_enabled(!busy&&!panel.pending()&&self.session.editor.can_undo(),egui::Button::new("Undo model"));self.bound("import:Undo",&r);if r.clicked(){history=Some(false);}
     let r=ui.add_enabled(!busy&&!panel.pending()&&self.session.editor.can_redo(),egui::Button::new("Redo model"));self.bound("import:Redo",&r);if r.clicked(){history=Some(true);}
     let r=ui.button("Cancel review");self.bound("import:Cancel",&r);cancel=r.clicked();let r=ui.button("Close");self.bound("import:Close",&r);if r.clicked(){close=true;}
    });
    let r=ui.colored_label(if panel.error{ERROR}else{INK},&panel.note);self.bound("import:status",&r);
    if busy{ui.label("Finish pending native file work or a gesture before preparing, applying or changing model history.");}
    if panel.pending(){ui.label("Source worker pending. Cancel or Close prevents its result from changing the native model.");}
    if let Some(p)=&old_prepared{
     let s=p.summary();let current=p.target_current(&self.session.editor)&&panel.scope_matches(p);
     ui.strong(if s.ready_for_review{if current{"Prepared mapping · review required"}else{"Historical mapping · target or scope changed"}}else{"Import refused · original bytes retained"});
     ui.label(format!("{} source correspondences · {} owned multiplicity literals represented",s.mappings.len(),s.represented_multiplicity_literals));
     ui.label(format!("Existing inspection: {} retained losses / {} omitted; {} diagnostics / {} omitted",s.inspection_loss_count,s.inspection_omitted_losses,s.inspection_diagnostic_count,s.inspection_omitted_diagnostics));
     for n in &s.notices{
      if n.blocking{ui.colored_label(ERROR,format!("{}: {}",n.code.replace('_'," "),n.detail));}else{
       let mut checked=panel.acknowledgements.contains(&n.code);let r=ui.add_enabled(!panel.pending()&&s.ready_for_review,egui::Checkbox::new(&mut checked,&n.detail));self.bound(format!("import:ack:{}",n.code),&r);if r.changed(){if checked{panel.acknowledgements.insert(n.code.clone());}else{panel.acknowledgements.remove(&n.code);}}
      }
     }
     egui::ScrollArea::vertical().id_salt("source-import-mappings").max_height(180.).show(ui,|ui|{for m in &s.mappings{let name=panel.source.as_ref().and_then(|s|s.external_node(&m.source)).map(|n|n.name.as_str()).unwrap_or("");let kind=match m.native_kind{mbse_model::types::tags::TargetKind::System=>"System",mbse_model::types::tags::TargetKind::ComponentDefinition=>"Component definition",mbse_model::types::tags::TargetKind::Part=>"Part",mbse_model::types::tags::TargetKind::Reference=>"Reference",_=>"Native item"};let source_id=match &m.source.identity{mbse_exchange::project_sets::LocalIdentity::External(k)=>crate::semantic_ui::key_label(k),_=>"source record".into()};ui.label(format!("{} · {} to {} #{}{}",source_id,name,kind,m.native.0,if m.source_kind=="SysmlBlockDeclaration"{" · documentary Block declaration"}else{""}));}});
     let can_apply=!busy&&!panel.pending()&&!edited&&current&&s.ready_for_review&&panel.acknowledgements==s.required_acknowledgements;
     let r=ui.add_enabled(can_apply,egui::Button::new("Apply native import"));self.bound("import:Apply",&r);apply=r.clicked();
     let r=ui.button("Copy review JSON");self.bound("import:Copy",&r);if r.clicked(){if let Ok(text)=serde_json::to_string_pretty(s){ctx.copy_text(text);}}
     let r=ui.collapsing("Source and target provenance / limits",|ui|{ui.label(format!("Source SHA-256: {}\nTarget model revision {} · raw revision {}\nTarget snapshot SHA-256: {}",s.source_primary_sha256.as_deref().unwrap_or("unavailable"),s.target_native.model_revision,s.target_native.raw_version,s.target_snapshot_sha256));for limit in &s.limits{ui.label(limit);}for l in &s.source_inspection_losses{ui.label(format!("Historical inspection {}: {}",l.code,l.detail));}});self.bound("import:provenance",&r.header_response);
    }
    if let Some(receipt)=&panel.applied{let current=receipt.target_snapshot_sha256==self.session.editor.snapshot.digest();ui.strong(if current{"Native import applied · receipt matches current model"}else{"Historical native import receipt · model has changed"});ui.label(format!("Applied model revision {} · source files checked at Unix millisecond {}",receipt.target_model_revision,receipt.checked_unix_milliseconds));}
   });
  }
  panel.prepared=old_prepared;open&=!close;panel.open=open;
  // Process queued Cancel/Close before publishing an already-ready completion.
  if cancel||!open{if let Some(p)=&mut panel.pending{p.cancelled=true;}if cancel{panel.prepared=None;panel.acknowledgements.clear();panel.note="Review canceled; native work and source files unchanged.".into();panel.error=false;panel.open=false;}}
  if let Some(done)=done{
   let cancelled=panel.pending.take().is_none_or(|p|p.cancelled)||cancel||!panel.open;
   if !cancelled{match done{
    Ok(Completion::Prepared(prepared))=>{panel.prepared=Some(prepared);panel.acknowledgements.clear();panel.note="Prepared from frozen source bytes. Review every disclosure before Apply.".into();panel.error=false;},
    Ok(Completion::Checked(checked))=>{if self.source_import_busy(){panel.note="Native workspace became busy; checked result discarded. Review retained; Apply again after pending work finishes.".into();panel.error=true;}else{match checked.apply_with_views(&mut self.session.editor,&mut self.views.views){Ok(applied)=>{panel.applied=Some(applied);panel.note="Native import applied as one model-history entry. Original source bytes unchanged.".into();panel.error=false;},Err(error)=>{panel.note=error;panel.error=true;}}}},
    Err(error)=>{panel.note=error;panel.error=true;},
   }}
  }
  if !cancel&&panel.open&&!panel.pending(){
   if prepare&&!self.source_import_busy(){if let(Ok(system),Some(source))=(panel.system(),panel.source.clone()){
    let snapshot=self.session.editor.snapshot.clone();let instance=self.session.editor.freshness().instance_id;let member=panel.member.clone();let(tx,rx)=mpsc::channel();panel.pending=Some(Pending{receiver:rx,cancelled:false});panel.note="Preparing a bounded native mapping from frozen source bytes…".into();panel.error=false;std::thread::spawn(move||{let result=mbse_model::stage1::Editor::new(snapshot,instance).map(|e|Completion::Prepared(source::prepare(source,&member,&e,system)));let _=tx.send(result);});
   }}else if apply&&!self.source_import_busy(){if let Some(p)=&panel.prepared{if p.target_current(&self.session.editor)&&panel.scope_matches(p){match p.review(&panel.acknowledgements){Ok(reviewed)=>{let(tx,rx)=mpsc::channel();panel.pending=Some(Pending{receiver:rx,cancelled:false});panel.note="Checking original source files before native Apply…".into();panel.error=false;std::thread::spawn(move||{let _=tx.send(reviewed.check_source().map(Completion::Checked));});},Err(error)=>{panel.note=error;panel.error=true;}}}}}
   else if let Some(redo)=history{if !self.source_import_busy(){let stamp=self.views.views.stamp(&self.session.editor);let action=if redo{mbse_model::native_views::NativeAction::ModelRedo}else{mbse_model::native_views::NativeAction::ModelUndo};match self.views.views.dispatch(&mut self.session.editor,&stamp,action){Ok(_)=>{panel.note="Native model history changed; prior mapping and receipt remain historical.".into();panel.error=false;},Err(error)=>{panel.note=error;panel.error=true;}}}}
  }
  self.source_import=panel;
 }
}

#[cfg(test)]
#[path="ui_source_import_tests.rs"]
mod tests;
