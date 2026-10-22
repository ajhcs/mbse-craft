//! Native authoring and explicit review of an independent requirements sidecar.
use eframe::egui;
use mbse_assurance::{Adequacy, Board, EvidenceContext, EvidenceId, ImplementationStatus, Maturity, RequirementId, VerificationStatus};
use mbse_model::{stage1::Editor, stage1_calc::CalculationId};
use std::{path::PathBuf, sync::mpsc::{self, Receiver}, time::Duration};

enum Completion {
    Loaded(Result<(Board, String), String>, PathBuf),
    Saved(Result<String, String>, PathBuf, serde_json::Value),
}
#[derive(Default)]
pub struct AssuranceWorkspace {
    board: Board,
    visible: bool,
    selected: Option<RequirementId>,
    evidence: Option<EvidenceId>,
    statement: String,
    path: String,
    owned: Option<(PathBuf, String)>,
    saved: Option<serde_json::Value>,
    pending: Option<Receiver<Completion>>,
    feedback: String,
    reviewer: String,
    rationale: String,
    calculation: String,
    local_evidence: String,
    config: String,
    close_prompt: bool,
    discard: Option<serde_json::Value>,
    replace_prompt: bool,
}
impl AssuranceWorkspace {
    pub fn open_window(&mut self) { self.visible = true; }
    pub fn is_visible(&self) -> bool { self.visible }
    pub fn dirty(&self) -> bool {
        let value = self.board.as_value().ok();
        match &self.saved { Some(saved) => value.as_ref() != Some(saved), None => value != Board::new().as_value().ok() }
    }
    /// A discard permit applies only to these exact register bytes, once no IO is pending.
    pub fn request_close(&mut self) -> bool {
        if self.pending.is_none() && (!self.dirty() || self.discard.as_ref() == self.board.as_value().ok().as_ref()) { return true; }
        self.visible = true;
        self.close_prompt = true;
        false
    }
    pub fn request_save(&mut self) { self.save(false); }
    fn config(&self) -> Result<serde_json::Value,String> {
        if self.config.trim().is_empty() { Ok(serde_json::json!({"profile":"mbse-craft.assurance-ui/1"})) }
        else { serde_json::from_str(&self.config).map_err(|e| format!("Configuration JSON is incomplete: {e}")) }
    }
    fn save(&mut self, fresh: bool) {
        if self.pending.is_some() { self.feedback="A frozen register operation is pending.".into(); return; }
        let path=PathBuf::from(self.path.trim());
        if !path.is_absolute() { self.feedback="Choose an absolute register sidecar path.".into(); return; }
        let expected=if fresh {None} else {match &self.owned {Some((owned,hash)) if owned==&path=>Some(hash.clone()), _=>None}};
        let board=self.board.clone();
        let Ok(frozen)=board.as_value() else {self.feedback="Register serialization failed.".into();return;};
        let (tx,rx)=mpsc::channel(); self.pending=Some(rx);
        self.feedback="Saving frozen requirements register; newer edits remain unsaved.".into();
        std::thread::spawn(move||{let result=match expected {Some(hash)=>board.replace(&path,&hash),None=>board.save_new(&path)};let _=tx.send(Completion::Saved(result,path,frozen));});
    }
    fn load(&mut self) {
        if self.pending.is_some(){return;}
        if self.dirty() && !self.replace_prompt {self.replace_prompt=true; return;}
        self.replace_prompt=false;
        let path=PathBuf::from(self.path.trim());
        if !path.is_absolute(){self.feedback="Choose an absolute register sidecar path.".into();return;}
        let(tx,rx)=mpsc::channel();self.pending=Some(rx);
        std::thread::spawn(move||{let result=Board::open(&path);let _=tx.send(Completion::Loaded(result,path));});
    }
    fn result<T>(&mut self, result:Result<T,String>, message:&str) {
        self.feedback=match result {Ok(_)=>message.into(),Err(e)=>e};
    }
    pub fn show(&mut self,ctx:&egui::Context,editor:&Editor) {
        let completion=self.pending.as_ref().and_then(|r|match r.try_recv(){Ok(v)=>Some(v),Err(mpsc::TryRecvError::Empty)=>None,Err(mpsc::TryRecvError::Disconnected)=>{self.feedback="Register worker disconnected; current edits retained.".into();None}});
        if let Some(completion)=completion {self.pending=None;match completion {
            Completion::Loaded(Ok((board,hash)),path)=>{self.saved=board.as_value().ok();self.board=board;self.selected=self.board.register.requirements.keys().next().copied();self.evidence=None;self.owned=Some((path,hash));self.discard=None;self.feedback="Register reopened. Historical evidence is retained; reopening does not authenticate a native evaluator run.".into();},
            Completion::Loaded(Err(e),_)=>self.feedback=e,
            Completion::Saved(Ok(hash),path,frozen)=>{self.saved=Some(frozen);self.owned=Some((path,hash));self.feedback=if self.dirty(){"Frozen register saved; newer edits remain unsaved."}else{"Requirements register saved."}.into();},
            Completion::Saved(Err(e),_,_)=>self.feedback=e,
        }}
        if self.pending.is_some(){ctx.request_repaint_after(Duration::from_millis(100));}
        if !self.visible{return;}
        let config=self.config();
        let current=config.as_ref().map_err(Clone::clone).and_then(|c|EvidenceContext::from_editor(editor,c));
        let mut visible=true;
        egui::Window::new("Requirements and evidence").id(egui::Id::new("requirements-evidence")).open(&mut visible).default_size([1100.,820.]).vscroll(true).show(ctx,|ui|{
            ui.label("Independent application register · native model unchanged · approval, implementation and verification are separate decisions.");
            ui.horizontal(|ui|{ui.label(if self.dirty(){"Unsaved register changes"}else{"Register saved / empty"});if self.pending.is_some(){ui.spinner();}if ui.button("Undo register edit").clicked(){self.board.undo();}if ui.button("Redo register edit").clicked(){self.board.redo();}});
            ui.horizontal(|ui|{ui.label("Register path");ui.text_edit_singleline(&mut self.path);if ui.add_enabled(self.pending.is_none(),egui::Button::new("Open register")).clicked(){self.load();}if ui.add_enabled(self.pending.is_none(),egui::Button::new("Save register")).clicked(){self.save(false);}if ui.add_enabled(self.pending.is_none(),egui::Button::new("Save fresh copy")).clicked(){self.save(true);}});
            ui.label(&self.feedback);
            ui.separator();
            ui.horizontal(|ui|{ui.label("New requirement");ui.text_edit_singleline(&mut self.statement);if ui.button("Add requirement").clicked(){match self.board.add_requirement(self.statement.clone()){Ok(id)=>{self.selected=Some(id);self.statement.clear();self.feedback="Requirement added in Draft / UnderReview / NotStarted.".into();},Err(e)=>self.feedback=e}}});
            ui.columns(2,|columns|{
                egui::ScrollArea::vertical().id_salt("requirement-list").max_height(180.).show(&mut columns[0],|ui|{for(id,r)in &self.board.register.requirements{if ui.selectable_label(self.selected==Some(*id),format!("R{} · {}",id.0,r.statement)).clicked(){self.selected=Some(*id);self.evidence=None;}}});
                columns[1].collapsing("Exact evaluation configuration",|ui|{ui.label("JSON enters evidence freshness; it does not configure or execute an external engine.");ui.text_edit_multiline(&mut self.config);});
            });
            let Some(id)=self.selected else {ui.label("Add or select a requirement to edit and review evidence.");return;};
            let Some(req)=self.board.register.requirements.get(&id).cloned() else{return;};
            ui.heading(format!("Requirement R{}",id.0));
            for(field,label,value)in[("statement","Statement",req.statement.clone()),("rationale","Rationale",req.rationale.clone()),("source","Source",req.source.clone()),("owner","Owner",req.owner.clone()),("user_version","Version",req.user_version.clone())]{
                let mut text=self.board.register.drafts.get(&format!("{}:{field}",id.0)).map(|d|d.text.clone()).unwrap_or(value);
                ui.horizontal(|ui|{ui.label(label);if ui.add(egui::TextEdit::singleline(&mut text).desired_width(720.)).changed(){let result=self.board.edit_field(id,field,text);self.result(result,"Requirement text updated; existing evidence may be stale.");}});
            }
            ui.horizontal_wrapped(|ui|{
                let mut maturity=req.maturity;
                egui::ComboBox::from_id_salt("maturity").selected_text(format!("Maturity: {maturity:?}")).show_ui(ui,|ui|{for v in [Maturity::Draft,Maturity::InDevelopment,Maturity::ReadyForReview,Maturity::InReview,Maturity::Approved,Maturity::Disapproved,Maturity::Deleted]{ui.selectable_value(&mut maturity,v,format!("{v:?}"));}});
                if maturity!=req.maturity{let result=self.board.set_maturity(id,maturity);self.result(result,"Maturity updated independently.");}
                let mut implementation=req.implementation;
                egui::ComboBox::from_id_salt("implementation").selected_text(format!("Implementation: {implementation:?}")).show_ui(ui,|ui|{for v in [ImplementationStatus::UnderReview,ImplementationStatus::Waived,ImplementationStatus::Pending,ImplementationStatus::InProgress,ImplementationStatus::Implemented,ImplementationStatus::PartiallyImplemented,ImplementationStatus::NotImplemented]{ui.selectable_value(&mut implementation,v,format!("{v:?}"));}});
                if implementation!=req.implementation{let result=self.board.set_implementation(id,implementation);self.result(result,"Implementation updated independently.");}
                ui.label(format!("Recorded verification: {:?}",req.verification));
            });
            if let Ok(current)=&current {ui.label(if self.board.verification_current(id,current){"Latest verification decision currently applicable"}else{"No currently applicable verification decision; historical decisions remain recorded"});}
            else if let Err(e)=&current{ui.colored_label(egui::Color32::DARK_RED,e);}
            ui.separator();
            ui.horizontal(|ui|{ui.label("Calculation measure ID");ui.add(egui::TextEdit::singleline(&mut self.calculation).desired_width(80.));if ui.button("Evaluate and attach receipt").clicked(){let result=self.calculation.trim().parse::<u64>().map_err(|_|"Enter a calculation measure ID.".into()).and_then(|v|config.as_ref().map_err(Clone::clone).and_then(|c|self.board.attach_calculation(id,editor,c,CalculationId(v))));match result {Ok(e)=>{self.evidence=Some(e);self.feedback="Evaluator receipt attached; adequacy and verification remain separate.".into();},Err(e)=>self.feedback=e}}});
            ui.horizontal(|ui|{ui.label("Local artifact path");ui.text_edit_singleline(&mut self.local_evidence);if ui.button("Attach external artifact").clicked(){let result=current.as_ref().map_err(Clone::clone).and_then(|c|self.board.attach_local_evidence(id,c.clone(),&PathBuf::from(self.local_evidence.trim()),mbse_assurance::EvidenceOutcome::NotRun));match result {Ok(e)=>{self.evidence=Some(e);self.feedback="Inert external artifact attached with NotRun outcome; no native execution authority.".into();},Err(e)=>self.feedback=e}}});
            egui::ScrollArea::vertical().id_salt("evidence-list").max_height(95.).show(ui,|ui|{for(eid,e)in &self.board.register.evidence{if e.requirement==id && ui.selectable_label(self.evidence==Some(*eid),format!("E{} · {:?} · {:?} · selected {:?}",eid.0,e.outcome,e.adequacy,e.selected_calculation)).clicked(){self.evidence=Some(*eid);}}});
            if let Some(eid)=self.evidence {if let Some(e)=self.board.register.evidence.get(&eid).cloned(){
                ui.monospace(format!("Artifact SHA-256: {}\nSelected output: {:?}",e.artifact_sha256,e.selected_quantity));
                if let Ok(current)=&current{match self.board.evidence_staleness(eid,current){Ok(reasons)if reasons.is_empty()=>{ui.label("Freshness closure matches. Authenticity and adequacy are separate gates.");},Ok(reasons)=>{ui.colored_label(egui::Color32::DARK_RED,format!("Stale: {}",reasons.join("; ")));},Err(e)=>{ui.label(e);}}}
                ui.collapsing("Intact evidence artifact and provenance",|ui|{ui.monospace(format!("{:?}",e.context));egui::ScrollArea::both().max_height(120.).show(ui,|ui|{let n=e.artifact_bytes.len().min(64*1024);ui.monospace(String::from_utf8_lossy(&e.artifact_bytes[..n]));if n<e.artifact_bytes.len(){ui.label("Preview truncated; complete original bytes retained.");}});});
                ui.horizontal(|ui|{ui.label("Reviewer");ui.text_edit_singleline(&mut self.reviewer);ui.label("Decision rationale");ui.text_edit_singleline(&mut self.rationale);});
                ui.horizontal_wrapped(|ui|{for (label,adequacy) in [("Review adequate",Adequacy::Adequate),("Review inadequate",Adequacy::Inadequate)]{if ui.button(label).clicked(){let result=self.board.review_evidence(eid,adequacy,self.reviewer.clone(),self.rationale.clone());self.result(result,"Explicit evidence review recorded.");}}});
                ui.horizontal_wrapped(|ui|{for status in [VerificationStatus::InProgress,VerificationStatus::Completed,VerificationStatus::CompletedUnsuccessful,VerificationStatus::CompletedFailed,VerificationStatus::CompletedPassed]{if ui.button(format!("Record {status:?}")).clicked(){let result=current.as_ref().map_err(Clone::clone).and_then(|c|self.board.record_verification(id,eid,status,c,self.reviewer.clone(),self.rationale.clone()));self.result(result,"Verification decision recorded; current applicability is derived.");}}});
            }}
        });
        self.visible=visible;
        if self.close_prompt{egui::Window::new("Preserve requirements register").collapsible(false).show(ctx,|ui|{ui.label("The independent register has unsaved changes or pending IO. Save it in Requirements and evidence, or explicitly discard these exact changes for this close.");if ui.button("Return to work").clicked(){self.close_prompt=false;self.discard=None;}if ui.add_enabled(self.pending.is_none(),egui::Button::new("Discard register changes for this close")).clicked(){self.discard=self.board.as_value().ok();self.close_prompt=false;self.feedback="Exact register changes permitted to be discarded on next application close.".into();}});}
        if self.replace_prompt{egui::Window::new("Open another register?").collapsible(false).show(ctx,|ui|{ui.label("Opening replaces unsaved register edits only after successful admission. Native model is retained.");if ui.button("Keep current register").clicked(){self.replace_prompt=false;}if ui.button("Discard and open register").clicked(){self.load();}});}
    }
}
