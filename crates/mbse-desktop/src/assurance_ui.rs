//! Native authoring and explicit review of an independent requirements sidecar.
use eframe::egui;
use mbse_assurance::{
    Adequacy, Board, EvidenceContext, EvidenceId, ImplementationStatus, Maturity, RequirementId,
    VerificationStatus,
};
use mbse_model::{
    stage1::{Editor, Freshness},
    stage1_calc::CalculationId,
};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

enum Completion {
    Loaded(Result<(Board, String), String>, PathBuf, serde_json::Value),
    Updated(
        Result<(Board, Option<EvidenceId>, String), String>,
        serde_json::Value,
        Freshness,
        serde_json::Value,
    ),
    Saved(Result<String, String>, PathBuf, serde_json::Value),
}
#[derive(Clone, PartialEq, Eq)]
struct AssessmentKey {
    model: Freshness,
    config: serde_json::Value,
    epoch: u64,
    requirement: Option<RequirementId>,
    evidence: Option<EvidenceId>,
}
struct Assessment {
    key: AssessmentKey,
    context: Result<EvidenceContext, String>,
    staleness: Result<Vec<String>, String>,
    verification_current: bool,
    completed: Instant,
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
    epoch: u64,
    assessment: Option<Assessment>,
    assessment_pending: Option<Receiver<Assessment>>,
}
impl AssuranceWorkspace {
    pub fn open_window(&mut self) {
        self.visible = true;
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    pub fn dirty(&self) -> bool {
        let value = self.board.as_value().ok();
        match &self.saved {
            Some(saved) => value.as_ref() != Some(saved),
            None => value != Board::new().as_value().ok(),
        }
    }
    /// A discard permit applies only to these exact register bytes, once no IO is pending.
    pub fn request_close(&mut self) -> bool {
        if self.pending.is_none()
            && (!self.dirty() || self.discard.as_ref() == self.board.as_value().ok().as_ref())
        {
            return true;
        }
        self.visible = true;
        self.close_prompt = true;
        false
    }
    pub fn request_save(&mut self) {
        self.save(false);
    }
    fn config(&self) -> Result<serde_json::Value, String> {
        if self.config.trim().is_empty() {
            Ok(serde_json::json!({"profile":"mbse-craft.assurance-ui/1"}))
        } else {
            serde_json::from_str(&self.config)
                .map_err(|e| format!("Configuration JSON is incomplete: {e}"))
        }
    }
    fn save(&mut self, fresh: bool) {
        if self.pending.is_some() {
            self.feedback = "A frozen register operation is pending.".into();
            return;
        }
        let path = PathBuf::from(self.path.trim());
        if !path.is_absolute() {
            self.feedback = "Choose an absolute register sidecar path.".into();
            return;
        }
        let expected = if fresh {
            None
        } else {
            match &self.owned {
                Some((owned, hash)) if owned == &path => Some(hash.clone()),
                _ => None,
            }
        };
        let board = self.board.clone();
        let Ok(frozen) = board.as_value() else {
            self.feedback = "Register serialization failed.".into();
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.feedback = "Saving frozen requirements register; newer edits remain unsaved.".into();
        std::thread::spawn(move || {
            let result = match expected {
                Some(hash) => board.replace(&path, &hash),
                None => board.save_new(&path),
            };
            let _ = tx.send(Completion::Saved(result, path, frozen));
        });
    }
    fn load(&mut self) {
        if self.pending.is_some() {
            return;
        }
        if self.dirty() && !self.replace_prompt {
            self.replace_prompt = true;
            return;
        }
        self.replace_prompt = false;
        let path = PathBuf::from(self.path.trim());
        if !path.is_absolute() {
            self.feedback = "Choose an absolute register sidecar path.".into();
            return;
        }
        let Ok(frozen) = self.board.as_value() else {
            self.feedback = "Register serialization failed; current work retained.".into();
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        std::thread::spawn(move || {
            let result = Board::open(&path);
            let _ = tx.send(Completion::Loaded(result, path, frozen));
        });
    }
    fn result<T>(&mut self, result: Result<T, String>, message: &str) {
        self.feedback = match result {
            Ok(_) => {
                self.epoch = self.epoch.saturating_add(1);
                message.into()
            }
            Err(e) => e,
        };
    }
    fn update_worker<F>(&mut self, editor: &Editor, operation: F)
    where
        F: FnOnce(
                &mut Board,
                &Editor,
                &serde_json::Value,
            ) -> Result<(Option<EvidenceId>, String), String>
            + Send
            + 'static,
    {
        if self.pending.is_some() {
            self.feedback = "A register operation is pending; current edits retained.".into();
            return;
        }
        let config = match self.config() {
            Ok(v) => v,
            Err(e) => {
                self.feedback = e;
                return;
            }
        };
        let frozen = match self.board.as_value() {
            Ok(v) => v,
            Err(e) => {
                self.feedback = e;
                return;
            }
        };
        let mut board = self.board.clone();
        let editor = editor.clone();
        let stamp = editor.freshness();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.feedback =
            "Checking frozen evidence on a worker; current work remains editable.".into();
        std::thread::spawn(move || {
            let result =
                operation(&mut board, &editor, &config).map(|(id, message)| (board, id, message));
            let _ = tx.send(Completion::Updated(result, frozen, stamp, config));
        });
    }
    fn assessment_key(&self, editor: &Editor, config: serde_json::Value) -> AssessmentKey {
        AssessmentKey {
            model: editor.freshness(),
            config,
            epoch: self.epoch,
            requirement: self.selected,
            evidence: self.evidence,
        }
    }
    fn assess(&mut self, editor: &Editor, config: &Result<serde_json::Value, String>) {
        if let Some(rx) = &self.assessment_pending {
            match rx.try_recv() {
                Ok(value) => {
                    self.assessment = Some(value);
                    self.assessment_pending = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.assessment_pending = None;
                    self.feedback =
                        "Evidence assessment worker disconnected; no current verdict asserted."
                            .into();
                }
                Err(mpsc::TryRecvError::Empty) => (),
            }
        }
        let Ok(config) = config else {
            return;
        };
        let key = self.assessment_key(editor, config.clone());
        if self
            .assessment
            .as_ref()
            .is_some_and(|a| a.key == key && a.completed.elapsed() < Duration::from_secs(1))
            || self.assessment_pending.is_some()
        {
            return;
        }
        let context_cache = self
            .assessment
            .as_ref()
            .filter(|a| a.key.model == key.model && a.key.config == key.config)
            .map(|a| a.context.clone());
        let board = self.board.clone();
        let editor = editor.clone();
        let (tx, rx) = mpsc::channel();
        self.assessment_pending = Some(rx);
        std::thread::spawn(move || {
            let context =
                context_cache.unwrap_or_else(|| EvidenceContext::from_editor(&editor, &key.config));
            let staleness = match (&context, key.evidence) {
                (Ok(c), Some(id)) => board.evidence_staleness(id, c),
                (Err(e), _) => Err(e.clone()),
                _ => Ok(vec![]),
            };
            let verification_current = match (&context, key.requirement) {
                (Ok(c), Some(id)) => board.verification_current(id, c),
                _ => false,
            };
            let _ = tx.send(Assessment {
                key,
                context,
                staleness,
                verification_current,
                completed: Instant::now(),
            });
        });
    }
    pub fn show(&mut self, ctx: &egui::Context, editor: &Editor) {
        let mut disconnected = false;
        let completion = self.pending.as_ref().and_then(|r| match r.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.feedback = "Register worker disconnected; current edits retained.".into();
                disconnected = true;
                None
            }
        });
        if disconnected {
            self.pending = None;
        }
        if let Some(completion) = completion {
            self.pending = None;
            match completion {
            Completion::Loaded(Ok((board,hash)),path,frozen) if self.board.as_value().ok().as_ref()==Some(&frozen)&&PathBuf::from(self.path.trim())==path=>{self.epoch=self.epoch.saturating_add(1);self.saved=board.as_value().ok();self.board=board;self.selected=self.board.register.requirements.keys().next().copied();self.evidence=None;self.owned=Some((path,hash));self.discard=None;self.statement=self.board.register.drafts.get("new:statement").map(|d|d.text.clone()).unwrap_or_default();self.feedback="Register reopened. Historical evidence is retained; reopening does not authenticate a native evaluator run.".into();},
            Completion::Loaded(Ok(_),_,_)=>self.feedback="Open result is stale because register edits changed after dispatch; live changes retained. Open again after preserving them.".into(),
            Completion::Loaded(Err(e),_,_)=>self.feedback=e,
            Completion::Updated(Ok((board,evidence,message)),frozen,stamp,config) if self.board.as_value().ok().as_ref()==Some(&frozen)&&editor.freshness()==stamp&&self.config().ok().as_ref()==Some(&config)=>{self.board=board;self.epoch=self.epoch.saturating_add(1);if evidence.is_some(){self.evidence=evidence;}self.feedback=message;},
            Completion::Updated(Ok(_),_,_,_)=>self.feedback="Evidence result is stale because the register, native model/raw input or configuration changed; live changes retained.".into(),
            Completion::Updated(Err(e),_,_,_)=>self.feedback=e,
            Completion::Saved(Ok(hash),path,frozen)=>{self.saved=Some(frozen);self.owned=Some((path,hash));self.feedback=if self.dirty(){"Frozen register saved; newer edits remain unsaved."}else{"Requirements register saved."}.into();},
            Completion::Saved(Err(e),_,_)=>self.feedback=e,
        }
        }
        if self.pending.is_some() || self.assessment_pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if !self.visible {
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(500));
        let config = self.config();
        self.assess(editor, &config);
        let assessment_key = config
            .as_ref()
            .ok()
            .map(|c| self.assessment_key(editor, c.clone()));
        let assessment = self
            .assessment
            .as_ref()
            .filter(|a| Some(&a.key) == assessment_key.as_ref());
        let current = assessment.map(|a| a.context.clone()).unwrap_or_else(|| {
            Err("Evidence freshness assessment pending; no current verification asserted.".into())
        });
        let staleness = assessment.map(|a| a.staleness.clone());
        let verification_current = assessment.is_some_and(|a| a.verification_current);
        let mut visible = true;
        egui::Window::new("Requirements and evidence").id(egui::Id::new("requirements-evidence")).open(&mut visible).default_size([1100.,820.]).vscroll(true).show(ctx,|ui|{
            ui.label("Independent application register · native model unchanged · approval, implementation and verification are separate decisions.");
            ui.horizontal(|ui|{ui.label(if self.dirty(){"Unsaved register changes"}else{"Register saved / empty"});if self.pending.is_some(){ui.spinner();}if ui.button("Undo register edit").clicked(){if self.board.undo(){self.epoch=self.epoch.saturating_add(1);}}if ui.button("Redo register edit").clicked(){if self.board.redo(){self.epoch=self.epoch.saturating_add(1);}}});
            ui.horizontal(|ui|{ui.label("Register path");ui.text_edit_singleline(&mut self.path);if ui.add_enabled(self.pending.is_none(),egui::Button::new("Open register")).clicked(){self.load();}if ui.add_enabled(self.pending.is_none(),egui::Button::new("Save register")).clicked(){self.save(false);}if ui.add_enabled(self.pending.is_none(),egui::Button::new("Save fresh copy")).clicked(){self.save(true);}});
            ui.label(&self.feedback);
            ui.separator();
            ui.horizontal(|ui|{ui.label("New requirement");if ui.text_edit_singleline(&mut self.statement).changed(){self.board.register.drafts.insert("new:statement".into(),mbse_assurance::RawDraft{text:self.statement.clone(),valid:!self.statement.trim().is_empty()});self.epoch=self.epoch.saturating_add(1);}if ui.button("Add requirement").clicked(){match self.board.add_requirement(self.statement.clone()){Ok(id)=>{self.epoch=self.epoch.saturating_add(1);self.selected=Some(id);self.statement.clear();self.board.register.drafts.remove("new:statement");self.feedback="Requirement added in Draft / UnderReview / NotStarted.".into();},Err(e)=>self.feedback=e}}});
            ui.columns(2,|columns|{
                egui::ScrollArea::vertical().id_salt("requirement-list").max_height(180.).show(&mut columns[0],|ui|{for(id,r)in &self.board.register.requirements{if ui.selectable_label(self.selected==Some(*id),format!("R{} · {}",id.0,r.statement)).clicked(){self.selected=Some(*id);self.evidence=None;}}});
                columns[1].collapsing("Exact evaluation configuration",|ui|{ui.label("JSON enters evidence freshness; it does not configure or execute an external engine.");ui.text_edit_multiline(&mut self.config);});
            });
            let Some(id)=self.selected else {ui.label("Add or select a requirement to edit and review evidence.");return;};
            let Some(req)=self.board.register.requirements.get(&id).cloned() else{return;};
            ui.heading(format!("Requirement R{}",id.0));
            ui.horizontal_wrapped(|ui|{
                if ui.button("Link selected native record").clicked(){
                    match editor.selected.and_then(|selected|editor.snapshot.records.get(&selected)) {
                        Some(record)=>{
                            let mut units=std::collections::BTreeMap::new();
                            for field in ["quantity","unit"]{if let Some(value)=record.fields.get(field).and_then(serde_json::Value::as_str){units.insert(field.into(),value.into());}}
                            let anchor=mbse_assurance::SourceAnchor{format_contract:mbse_model::stage1_io::PROFILE.into(),artifact_sha256:editor.snapshot.digest(),metamodel:"Kernspan app-owned Stage1 records; no normative requirement reclassification".into(),source_element_id:format!("record:{}",record.id.0),native_document:Some(editor.snapshot.document_id.0.clone()),native_record:Some(record.id.0),units,references:vec![],package_integrity:mbse_assurance::ProjectionStatus::Supported,parsing:mbse_assurance::ProjectionStatus::Supported,reference_resolution:mbse_assurance::ProjectionStatus::Partial,semantic_projection:mbse_assurance::ProjectionStatus::Partial,editable_admission:mbse_assurance::ProjectionStatus::Supported,execution:mbse_assurance::ProjectionStatus::Unknown,unknown:std::collections::BTreeMap::from([("nativeRecordKind".into(),serde_json::json!(record.kind.label())),("modelRevision".into(),serde_json::json!(editor.snapshot.model_revision)),("rawVersion".into(),serde_json::json!(editor.snapshot.raw_version))])};
                            let result=self.board.set_anchor(id,Some(anchor));self.result(result,"Exact selected native source revision linked; this remains an independent application requirement.");
                        },None=>self.feedback="Select an actual native record before linking; no source relation invented.".into(),
                    }
                }
                if ui.button("Clear source anchor").clicked(){let result=self.board.set_anchor(id,None);self.result(result,"Source anchor explicitly cleared; previous evidence may now be stale.");}
            });
            ui.collapsing("Exact source anchor and independent status claims",|ui|{match &self.board.register.requirements.get(&id).and_then(|r|r.anchor.as_ref()){Some(anchor)=>{ui.monospace(serde_json::to_string_pretty(anchor).unwrap_or_else(|error|error.to_string()));},None=>{ui.label("No source anchor recorded; no native or external normative classification inferred.");}}});

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
            if current.is_ok() {ui.label(if verification_current&&assessment_key.as_ref().is_some_and(|key|key.epoch==self.epoch){"Latest verification decision applicable at the latest background source/proof check; rechecked before any new decision"}else{"No currently applicable verification decision; historical decisions remain recorded"});}
            else if let Err(e)=&current{ui.colored_label(egui::Color32::DARK_RED,e);}
            ui.separator();
            ui.horizontal(|ui|{ui.label("Calculation measure ID");ui.add(egui::TextEdit::singleline(&mut self.calculation).desired_width(80.));if ui.add_enabled(self.pending.is_none(),egui::Button::new("Evaluate and attach receipt")).clicked(){match self.calculation.trim().parse::<u64>(){Ok(value)=>self.update_worker(editor,move|board,editor,config|{let id=board.attach_calculation(id,editor,config,CalculationId(value))?;Ok((Some(id),"Selected calculation receipt attached from the actual native engine; review and verification remain separate.".into()))}),Err(_)=>self.feedback="Enter a calculation Measure ID.".into()}}});
            ui.horizontal(|ui|{ui.label("Local artifact path");ui.text_edit_singleline(&mut self.local_evidence);if ui.add_enabled(self.pending.is_none(),egui::Button::new("Attach external artifact")).clicked(){let path=PathBuf::from(self.local_evidence.trim());if !path.is_absolute(){self.feedback="Choose an absolute local artifact path; no URLs or commands are executed.".into();}else{self.update_worker(editor,move|board,editor,config|{let context=EvidenceContext::from_editor(editor,config)?;let id=board.attach_local_evidence(id,context,&path,mbse_assurance::EvidenceOutcome::NotRun)?;Ok((Some(id),"Explicit external artifact attached as NotRun; no native-run authenticity inferred.".into()))});}}});
            egui::ScrollArea::vertical().id_salt("evidence-list").max_height(95.).show(ui,|ui|{for(eid,e)in &self.board.register.evidence{if e.requirement==id && ui.selectable_label(self.evidence==Some(*eid),format!("E{} · {:?} · {:?} · selected {:?}",eid.0,e.outcome,e.adequacy,e.selected_calculation)).clicked(){self.evidence=Some(*eid);}}});
            if let Some(eid)=self.evidence {if let Some(e)=self.board.register.evidence.get(&eid).cloned(){
                ui.monospace(format!("Artifact SHA-256: {}",e.artifact_sha256));
                if let Some(q)=&e.selected_quantity{ui.label(format!("Selected output: {} · quantity {:?} · unit {:?}",q.value,q.quantity.kind,q.quantity.unit));}
                ui.label(format!("Global native receipt successful: {:?}. Selected evidence outcome: {:?}. An unrelated global error remains in the full artifact.",e.global_calculation_success,e.outcome));
                ui.label(if e.selected_calculation.is_some(){"Native evaluator provenance requires evaluator-owned live proof. Reopened serialized receipts are historical until a fresh actual evaluation."}else{"External local artifact provenance; no native execution inferred."});
                match &staleness {Some(Ok(reasons))if reasons.is_empty()=>{ui.label("Current freshness/provenance checks match; explicit adequacy and verification remain separate.");},Some(Ok(reasons))=>{ui.colored_label(egui::Color32::DARK_RED,format!("Stale or untrusted: {}",reasons.join("; ")));},Some(Err(error))=>{ui.label(error);},None=>{ui.label("Evidence assessment pending; no current verdict asserted.");}}
                ui.collapsing("Intact evidence artifact and provenance",|ui|{ui.monospace(format!("{:?}",e.context));egui::ScrollArea::both().max_height(120.).show(ui,|ui|{let n=e.artifact_bytes.len().min(64*1024);ui.monospace(String::from_utf8_lossy(&e.artifact_bytes[..n]));if n<e.artifact_bytes.len(){ui.label("Preview truncated; complete original bytes retained.");}});});
                ui.horizontal(|ui|{ui.label("Reviewer");ui.text_edit_singleline(&mut self.reviewer);ui.label("Decision rationale");ui.text_edit_singleline(&mut self.rationale);});
                ui.horizontal_wrapped(|ui|{for (label,adequacy) in [("Review adequate",Adequacy::Adequate),("Review inadequate",Adequacy::Inadequate)]{if ui.add_enabled(self.pending.is_none(),egui::Button::new(label)).clicked(){let reviewer=self.reviewer.clone();let rationale=self.rationale.clone();self.update_worker(editor,move|board,_,_|{board.review_evidence(eid,adequacy,reviewer,rationale)?;Ok((Some(eid),"Explicit evidence review appended; native provenance and current applicability remain separate.".into()))});}}});
                ui.horizontal_wrapped(|ui|{for status in [VerificationStatus::InProgress,VerificationStatus::Completed,VerificationStatus::CompletedUnsuccessful,VerificationStatus::CompletedFailed,VerificationStatus::CompletedPassed]{if ui.add_enabled(self.pending.is_none(),egui::Button::new(format!("Record {status:?}"))).clicked(){let reviewer=self.reviewer.clone();let rationale=self.rationale.clone();self.update_worker(editor,move|board,editor,config|{let context=EvidenceContext::from_editor(editor,config)?;board.record_verification(id,eid,status,&context,reviewer,rationale)?;Ok((Some(eid),"Verification decision appended; current applicability is derived from exact source/proof checks.".into()))});}}});
            }}
            ui.collapsing("Immutable review and verification record history",|ui|{
                for review in &self.board.register.review_history {ui.label(format!("Review #{} · E{} · {:?} · {} · {}",review.review_id,review.evidence.0,review.adequacy,review.reviewer,review.rationale));}
                for decision in self.board.register.decision_history.iter().filter(|d|d.requirement==id) {ui.label(format!("Decision #{} · E{} · {:?} · {} · {}",decision.decision_id,decision.evidence.0,decision.status,decision.reviewer,decision.rationale));}
                ui.label("History records prior declarations; serialized stamps do not authenticate native runs or assert a currently applicable verdict.");
            });
        });
        self.visible = visible;
        if self.close_prompt {
            egui::Window::new("Preserve requirements register").collapsible(false).show(ctx,|ui|{ui.label("The independent register has unsaved changes or pending IO. Save it in Requirements and evidence, or explicitly discard these exact changes for this close.");if ui.button("Return to work").clicked(){self.close_prompt=false;self.discard=None;}if ui.add_enabled(self.pending.is_none(),egui::Button::new("Discard register changes for this close")).clicked(){self.discard=self.board.as_value().ok();self.close_prompt=false;self.feedback="Exact register changes permitted to be discarded on next application close.".into();}});
        }
        if self.replace_prompt {
            egui::Window::new("Open another register?").collapsible(false).show(ctx,|ui|{ui.label("Opening replaces unsaved register edits only after successful admission. Native model is retained.");if ui.button("Keep current register").clicked(){self.replace_prompt=false;}if ui.button("Discard and open register").clicked(){self.load();}});
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mbse_model::stage1::{DocumentId, Snapshot};
    fn editor() -> Editor {
        Editor::new(
            Snapshot::new(DocumentId("controller-test".into())),
            "instance".into(),
        )
        .unwrap()
    }
    #[test]
    fn delayed_load_does_not_replace_newer_register_edits() {
        let mut workspace = AssuranceWorkspace::default();
        workspace.path = "/workspace/example.json".into();
        let frozen = workspace.board.as_value().unwrap();
        let (tx, rx) = mpsc::channel();
        workspace.pending = Some(rx);
        let id = workspace
            .board
            .add_requirement("New work after dispatch".into())
            .unwrap();
        tx.send(Completion::Loaded(
            Ok((Board::new(), "test hash".into())),
            PathBuf::from("/workspace/example.json"),
            frozen,
        ))
        .unwrap();
        workspace.show(&egui::Context::default(), &editor());
        assert_eq!(
            workspace.board.register.requirements[&id].statement,
            "New work after dispatch"
        );
        assert!(workspace.feedback.contains("stale"));
        assert!(workspace.dirty());
    }
    #[test]
    fn older_save_ack_keeps_new_edits_dirty_and_close_permission_is_exact() {
        let mut workspace = AssuranceWorkspace::default();
        let id = workspace.board.add_requirement("Accepted".into()).unwrap();
        let frozen = workspace.board.as_value().unwrap();
        let (tx, rx) = mpsc::channel();
        workspace.pending = Some(rx);
        workspace
            .board
            .edit_field(id, "statement", "Newer retained work".into())
            .unwrap();
        tx.send(Completion::Saved(
            Ok("older hash".into()),
            PathBuf::from("/workspace/old.json"),
            frozen,
        ))
        .unwrap();
        workspace.show(&egui::Context::default(), &editor());
        assert!(workspace.dirty());
        workspace.discard = workspace.board.as_value().ok();
        assert!(workspace.request_close());
        workspace
            .board
            .edit_field(id, "statement", "".into())
            .unwrap();
        assert!(!workspace.request_close());
        assert_eq!(
            workspace.board.register.requirements[&id].statement,
            "Newer retained work"
        );
    }
    #[test]
    fn disconnected_worker_clears_pending_without_acknowledging_save() {
        let mut workspace = AssuranceWorkspace::default();
        workspace
            .board
            .add_requirement("Retain this".into())
            .unwrap();
        let (tx, rx) = mpsc::channel();
        workspace.pending = Some(rx);
        drop(tx);
        workspace.show(&egui::Context::default(), &editor());
        assert!(workspace.pending.is_none());
        assert!(workspace.dirty());
        assert!(workspace.feedback.contains("disconnected"));
    }
}
