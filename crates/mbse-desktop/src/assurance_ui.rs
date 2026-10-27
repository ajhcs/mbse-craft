//! Native authoring and explicit review of an independent requirements sidecar.
use eframe::egui;
use mbse_assurance::{
    Adequacy, Board, EvidenceContext, EvidenceId, EvidenceOutcome, ImplementationStatus, Maturity,
    RequirementId, VerificationStatus,
};
use mbse_model::{
    stage1::Editor,
    stage1_calc::{CalculationId, DocumentStamp},
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

enum Completion {
    Loaded(Result<(Board, String), String>, PathBuf, serde_json::Value),
    Updated(
        Result<(Board, Option<EvidenceId>, String), String>,
        serde_json::Value,
        DocumentStamp,
        serde_json::Value,
    ),
    Saved(Result<String, String>, PathBuf, serde_json::Value),
}
#[derive(Clone, PartialEq, Eq)]
struct AssessmentKey {
    model: DocumentStamp,
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
    new_identifier:String,
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
    audit: BTreeMap<String, [f32; 4]>,
}
impl AssuranceWorkspace {
    pub fn audit(&self) -> BTreeMap<String, [f32; 4]> {
        self.audit.clone()
    }
    pub fn authoring_audit(&self)->serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.requirement-authoring-ui/1","visible":self.visible,"selected":self.selected.map(|id|id.0),"requirements":self.board.register.requirements,"drafts":self.board.register.drafts,"next_id":self.board.register.next_id,"revision":self.board.register.revision,"hierarchy":self.board.hierarchy().ok(),"pending":self.pending.is_some(),"dirty":self.dirty(),"feedback":self.feedback})
    }
    pub fn open_window(&mut self) {
        self.visible = true;
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    fn unadmitted_form(&self) -> bool {
        self.new_identifier!=self.board.register.drafts.get("new:identifier").map(|d|d.text.as_str()).unwrap_or("") || self.statement
            != self
                .board
                .register
                .drafts
                .get("new:statement")
                .map(|d| d.text.as_str())
                .unwrap_or("")
    }
    fn close_state(&self) -> Option<serde_json::Value> {
        Some(
            serde_json::json!({"register":self.board.as_value().ok()?,"newStatementForm":self.statement,"newIdentifierForm":self.new_identifier}),
        )
    }
    fn history(&mut self, undo: bool) {
        if self.unadmitted_form() {
            self.feedback="New requirement fields are retained only in the form. Reduce or clear it before register history navigation; text-field Undo remains available.".into();
            return;
        }
        if if undo {
            self.board.undo()
        } else {
            self.board.redo()
        } {
            self.epoch = self.epoch.saturating_add(1);
            self.statement = self
                .board
                .register
                .drafts
                .get("new:statement")
                .map(|d| d.text.clone())
                .unwrap_or_default();
            self.new_identifier=self.board.register.drafts.get("new:identifier").map(|d|d.text.clone()).unwrap_or_default();
        } else {
            self.feedback="Register history cannot move within the current integrity and document budget; work retained.".into();
        }
    }
    pub fn dirty(&self) -> bool {
        let value = self.board.as_value().ok();
        self.unadmitted_form()
            || match &self.saved {
                Some(saved) => value.as_ref() != Some(saved),
                None => value != Board::new().as_value().ok(),
            }
    }
    /// A discard permit applies only to these exact register bytes, once no IO is pending.
    pub fn request_close(&mut self) -> bool {
        if self.pending.is_none()
            && (!self.dirty()
                || self
                    .discard
                    .as_ref()
                    .zip(self.close_state().as_ref())
                    .is_some_and(|(a, b)| a == b))
        {
            return true;
        }
        self.visible = true;
        self.close_prompt = true;
        false
    }
    pub fn cancel_close(&mut self) {
        self.discard = None;
        self.close_prompt = false;
    }
    pub fn request_save(&mut self) {
        self.save(false);
    }
    fn config(&self) -> Result<serde_json::Value, String> {
        if self.config.trim().is_empty() {
            Ok(serde_json::json!({"profile":"mbse-craft.assurance-ui/1"}))
        } else {
            serde_json::from_str(&self.config)
                .map_err(|e| format!("Evidence-context metadata JSON is incomplete: {e}"))
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
        let Some(frozen) = self.close_state() else {
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
        let frozen = match self.close_state() {
            Some(v) => v,
            None => {
                self.feedback = "Register serialization failed; live work retained.".into();
                return;
            }
        };
        let mut board = self.board.clone();
        let editor = editor.clone();
        let stamp = editor.calculation_stamp();
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
            model: editor.calculation_stamp(),
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
            Completion::Loaded(Ok((board,hash)),path,frozen) if self.close_state().as_ref()==Some(&frozen)&&PathBuf::from(self.path.trim())==path=>{self.epoch=self.epoch.saturating_add(1);self.saved=board.as_value().ok();self.board=board;self.selected=self.board.register.requirements.keys().next().copied();self.evidence=None;self.owned=Some((path,hash));self.discard=None;self.statement=self.board.register.drafts.get("new:statement").map(|d|d.text.clone()).unwrap_or_default();self.new_identifier=self.board.register.drafts.get("new:identifier").map(|d|d.text.clone()).unwrap_or_default();self.feedback="Register reopened. Historical evidence is retained; reopening does not authenticate a native evaluator run.".into();},
            Completion::Loaded(Ok(_),_,_)=>self.feedback="Open result is stale because register edits changed after dispatch; live changes retained. Open again after preserving them.".into(),
            Completion::Loaded(Err(e),_,_)=>self.feedback=e,
            Completion::Updated(Ok((board,evidence,message)),frozen,stamp,config) if self.close_state().as_ref()==Some(&frozen)&&editor.calculation_stamp()==stamp&&self.config().ok().as_ref()==Some(&config)=>{self.board=board;self.epoch=self.epoch.saturating_add(1);if let Some(eid)=evidence{if self.board.register.evidence.get(&eid).is_some_and(|e|Some(e.requirement)==self.selected){self.evidence=Some(eid);}}self.feedback=message;},
            Completion::Updated(Ok(_),_,_,_)=>self.feedback="Evidence result is stale because the register, native model/raw input or configuration changed; live changes retained.".into(),
            Completion::Updated(Err(e),_,_,_)=>self.feedback=e,
            Completion::Saved(Ok(hash),path,frozen)=>{self.saved=Some(frozen);self.owned=Some((path,hash));self.feedback=if self.dirty(){"Frozen register saved; newer edits remain unsaved."}else{"Requirements register saved."}.into();},
            Completion::Saved(Err(e),_,_)=>self.feedback=e,
        }
        }
        if self.pending.is_some() || self.assessment_pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        self.audit.clear();
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
        let mut audit = BTreeMap::new();
        let mut visible = true;
        let mut close_editor=false;
        egui::Window::new("Requirements and evidence").id(egui::Id::new("requirements-evidence")).open(&mut visible).default_size([1100.,820.]).vscroll(true).show(ctx,|ui|{
            ui.label("Independent application register · native model unchanged · approval, implementation and verification are separate decisions.");
            ui.horizontal(|ui|{ui.label(if self.dirty(){"Unsaved register changes"}else{"Register saved / empty"});if self.pending.is_some(){ui.spinner();}if mark(&mut audit,"assurance:Undo",ui.button("Undo register edit")).clicked(){self.history(true);}if mark(&mut audit,"assurance:Redo",ui.button("Redo register edit")).clicked(){self.history(false);}if mark(&mut audit,"assurance:Close",ui.button("Close requirements editor")).clicked(){close_editor=true;}});
            ui.horizontal(|ui|{ui.label("Register path");mark(&mut audit,"assurance:path",ui.text_edit_singleline(&mut self.path));if mark(&mut audit,"assurance:Open",ui.add_enabled(self.pending.is_none(),egui::Button::new("Open register"))).clicked(){self.load();}if mark(&mut audit,"assurance:Save",ui.add_enabled(self.pending.is_none(),egui::Button::new("Save register"))).clicked(){self.save(false);}if mark(&mut audit,"assurance:Save fresh",ui.add_enabled(self.pending.is_none(),egui::Button::new("Save fresh copy"))).clicked(){self.save(true);}});
            ui.label(&self.feedback);
            ui.separator();
            ui.horizontal_wrapped(|ui|{ui.label("New identifier");if mark(&mut audit,"assurance:new-identifier",ui.add(egui::TextEdit::singleline(&mut self.new_identifier).desired_width(110.).char_limit(128).id(egui::Id::new("new-requirement-identifier")))).changed(){let result=self.board.set_new_identifier_draft(self.new_identifier.clone());self.result(result,"Identifier draft retained; accepted identifiers must be exact and unique.");}ui.label("New requirement");if mark(&mut audit,"assurance:new-statement",ui.add(egui::TextEdit::singleline(&mut self.statement).id(egui::Id::new("new-requirement-statement")))).changed(){let result=self.board.set_new_statement_draft(self.statement.clone());self.result(result,"New requirement draft retained. If admission fails, its text stays in the form and remains unsaved.");}for(child,key,label)in[(false,"assurance:Add","Add root requirement"),(true,"assurance:Add child","Add child of selected")]{let r=mark(&mut audit,key,ui.add_enabled(!child||self.selected.is_some(),egui::Button::new(label)));if r.clicked(){let parent=if child{self.selected}else{None};match self.board.add_identified_requirement(self.statement.clone(),self.new_identifier.clone(),parent){Ok(id)=>{self.epoch=self.epoch.saturating_add(1);self.selected=Some(id);self.statement.clear();self.new_identifier.clear();self.feedback="Requirement added in Draft / Under review / Not started; parent status is not inherited.".into();},Err(e)=>self.feedback=e}}}});
            ui.columns(2,|columns|{
                egui::ScrollArea::vertical().id_salt("requirement-list").max_height(180.).show(&mut columns[0],|ui|{match self.board.hierarchy(){Ok(rows)=>{for row in rows.iter().take(256){let r=&self.board.register.requirements[&row.id];ui.horizontal(|ui|{ui.add_space(row.depth.min(16)as f32*12.);let short:String=r.statement.chars().take(120).collect();let identifier=if r.identifier.is_empty(){"identifier unset"}else{r.identifier.as_str()};let label=format!("{} · local #{} · depth {} · {}{}",identifier,row.id.0,row.depth,short,if r.statement.chars().nth(120).is_some(){" [shortened]"}else{""});if mark(&mut audit,format!("assurance:select:{}",row.id.0),ui.selectable_label(self.selected==Some(row.id),label)).clicked(){self.selected=Some(row.id);self.evidence=None;}});}if rows.len()>256{ui.label(format!("Showing first256 of{} hierarchy rows; complete register retained.",rows.len()));}},Err(e)=>{ui.colored_label(egui::Color32::DARK_RED,e);}}});
                columns[1].collapsing("Evidence-context metadata (JSON)",|ui|{ui.label("Canonical JSON is retained with its hash for evidence context. These values do not configure the evaluator. Native execution uses the fixed algorithm and Limits::default() recorded with each receipt.");ui.text_edit_multiline(&mut self.config);});
            });
            let Some(id)=self.selected else {ui.label("Add or select a requirement to edit and review evidence.");return;};
            let Some(req)=self.board.register.requirements.get(&id).cloned() else{return;};
            ui.heading(format!("Requirement {} · local #{}",if req.identifier.is_empty(){"identifier unset"}else{req.identifier.as_str()},id.0));
            let identifier_key=format!("{}:identifier",id.0);let mut identifier_text=self.board.register.drafts.get(&identifier_key).map(|d|d.text.clone()).unwrap_or_else(||req.identifier.clone());ui.horizontal(|ui|{ui.label("Exact requirement identifier");if mark(&mut audit,"assurance:field:identifier",ui.add(egui::TextEdit::singleline(&mut identifier_text).char_limit(128).id(egui::Id::new(("requirement-identifier",id.0))))).changed(){let result=self.board.edit_identifier(id,identifier_text);self.result(result,"Identifier draft retained; duplicates/invalid text leave accepted identifier and stable internal links unchanged.");}});if self.board.register.drafts.get(&identifier_key).is_some_and(|d|!d.valid){ui.colored_label(egui::Color32::DARK_RED,format!("Unapplied identifier draft; accepted identifier remains {}",req.identifier));}

            ui.label("Parenting organizes the requirement hierarchy; it does not derive another statement or roll up approval, implementation or verification.");
            let parent_key=format!("{}:parent",id.0);let mut parent_text=self.board.register.drafts.get(&parent_key).map(|d|d.text.clone()).unwrap_or_else(||req.parent.map_or(String::new(),|p|p.0.to_string()));
            ui.horizontal(|ui|{ui.label("Parent local ID (empty means root)");if mark(&mut audit,"assurance:field:parent",ui.add(egui::TextEdit::singleline(&mut parent_text).char_limit(128).id(egui::Id::new(("requirement-parent",id.0))))).changed(){let result=self.board.edit_parent(id,parent_text);self.result(result,"Parent draft retained; only an existing acyclic parent is accepted. Lifecycle states remain independent.");}});
            if self.board.register.drafts.get(&parent_key).is_some_and(|d|!d.valid){ui.colored_label(egui::Color32::DARK_RED,"Unapplied parent draft: use an existing canonical requirement ID that does not create a cycle, or leave empty for a root.");}
            ui.label(req.parent.map_or("Accepted parent: root".into(),|p|format!("Accepted parent local #{}",p.0)));

            ui.horizontal_wrapped(|ui|{
                if mark(&mut audit,"assurance:Link source",ui.button("Link selected native record")).clicked(){
                    match editor.selected.and_then(|selected|editor.snapshot.records.get(&selected)) {
                        Some(record)=>{
                            let mut units=std::collections::BTreeMap::new();
                            for field in ["quantity","unit"]{if let Some(value)=record.fields.get(field).and_then(serde_json::Value::as_str){units.insert(field.into(),value.into());}}
                            let anchor=mbse_assurance::SourceAnchor{format_contract:mbse_model::stage1_io::PROFILE.into(),artifact_sha256:editor.snapshot.digest(),metamodel:"Kernspan app-owned Stage1 records; no normative requirement reclassification".into(),source_element_id:format!("record:{}",record.id.0),native_document:Some(editor.snapshot.document_id.0.clone()),native_record:Some(record.id.0),units,references:vec![],package_integrity:mbse_assurance::ProjectionStatus::Supported,parsing:mbse_assurance::ProjectionStatus::Supported,reference_resolution:mbse_assurance::ProjectionStatus::Partial,semantic_projection:mbse_assurance::ProjectionStatus::Partial,editable_admission:mbse_assurance::ProjectionStatus::Supported,execution:mbse_assurance::ProjectionStatus::Unknown,unknown:std::collections::BTreeMap::from([("nativeRecordKind".into(),serde_json::json!(record.kind.label())),("modelRevision".into(),serde_json::json!(editor.snapshot.model_revision)),("rawVersion".into(),serde_json::json!(editor.snapshot.raw_version))])};
                            let result=self.board.set_anchor(id,Some(anchor));self.result(result,"Exact selected native source revision linked; this remains an independent application requirement.");
                        },None=>self.feedback="Select an actual native record before linking; no source relation invented.".into(),
                    }
                }
                if mark(&mut audit,"assurance:Clear source",ui.button("Clear source anchor")).clicked(){let result=self.board.set_anchor(id,None);self.result(result,"Source anchor explicitly cleared; previous evidence may now be stale.");}
            });
            ui.collapsing("Exact source anchor and independent status claims",|ui|{ui.monospace(format!("Exact maturity: {:?}\nExact implementation: {:?}\nExact verification: {:?}",req.maturity,req.implementation,req.verification));match &self.board.register.requirements.get(&id).and_then(|r|r.anchor.as_ref()){Some(anchor)=>{ui.monospace(serde_json::to_string_pretty(anchor).unwrap_or_else(|error|error.to_string()));},None=>{ui.label("No source anchor recorded; no native or external normative classification inferred.");}}});

            for(field,label,value)in[("statement","Statement",req.statement.clone()),("rationale","Rationale",req.rationale.clone()),("source","Source",req.source.clone()),("owner","Owner",req.owner.clone()),("user_version","Version",req.user_version.clone())]{
                let mut text=self.board.register.drafts.get(&format!("{}:{field}",id.0)).map(|d|d.text.clone()).unwrap_or(value);
                ui.horizontal(|ui|{ui.label(label);if mark(&mut audit,format!("assurance:field:{field}"),ui.add(egui::TextEdit::singleline(&mut text).desired_width(720.).id(egui::Id::new(("requirement-field",id.0,field))))).changed(){let result=self.board.edit_field(id,field,text);let message=if matches!(&result,Ok(false)){"Unapplied invalid statement draft retained; accepted statement is unchanged."}else{"Requirement text updated; existing evidence may be stale."};self.result(result,message);}});
            }
            if self.board.register.drafts.get(&format!("{}:statement",id.0)).is_some_and(|d|!d.valid){ui.colored_label(egui::Color32::DARK_RED,format!("Unapplied statement draft. Accepted statement remains: {}",req.statement));}
            ui.horizontal_wrapped(|ui|{
                let mut maturity=req.maturity;
                let combo=egui::ComboBox::from_id_salt("maturity").selected_text(format!("Maturity: {}",maturity_label(maturity))).show_ui(ui,|ui|{for v in [Maturity::Draft,Maturity::InDevelopment,Maturity::ReadyForReview,Maturity::InReview,Maturity::Approved,Maturity::Disapproved,Maturity::Deleted]{let r=ui.selectable_value(&mut maturity,v,maturity_label(v)).on_hover_text(maturity_hint(v));mark(&mut audit,format!("assurance:maturity:{v:?}"),r);}});
                mark(&mut audit,"assurance:maturity",combo.response.on_hover_text(maturity_hint(maturity)));
                if maturity!=req.maturity{let result=self.board.set_maturity(id,maturity);self.result(result,"Maturity updated independently.");}
                let mut implementation=req.implementation;
                let combo=egui::ComboBox::from_id_salt("implementation").selected_text(format!("Implementation: {}",implementation_label(implementation))).show_ui(ui,|ui|{for v in [ImplementationStatus::UnderReview,ImplementationStatus::Waived,ImplementationStatus::Pending,ImplementationStatus::InProgress,ImplementationStatus::Implemented,ImplementationStatus::PartiallyImplemented,ImplementationStatus::NotImplemented]{let r=ui.selectable_value(&mut implementation,v,implementation_label(v)).on_hover_text(implementation_hint(v));mark(&mut audit,format!("assurance:implementation:{v:?}"),r);}});
                mark(&mut audit,"assurance:implementation",combo.response.on_hover_text(implementation_hint(implementation)));
                if implementation!=req.implementation{let result=self.board.set_implementation(id,implementation);self.result(result,"Implementation updated independently.");}
                ui.label(format!("Recorded verification: {}",verification_label(req.verification))).on_hover_text(verification_hint(req.verification));
            });
            if current.is_ok() {ui.label(if verification_current&&assessment_key.as_ref().is_some_and(|key|key.epoch==self.epoch){"Latest verification decision applicable at the latest background source/proof check; rechecked before any new decision"}else{"No currently applicable verification decision; historical decisions remain recorded"});}
            else if let Err(e)=&current{ui.colored_label(egui::Color32::DARK_RED,e);}
            ui.separator();
            ui.horizontal(|ui|{ui.label("Calculation measure ID");mark(&mut audit,"assurance:calculation",ui.add(egui::TextEdit::singleline(&mut self.calculation).desired_width(80.)));if mark(&mut audit,"assurance:Evaluate",ui.add_enabled(self.pending.is_none(),egui::Button::new("Evaluate and attach receipt"))).clicked(){match self.calculation.trim().parse::<u64>(){Ok(value)=>self.update_worker(editor,move|board,editor,config|{let id=board.attach_calculation(id,editor,config,CalculationId(value))?;Ok((Some(id),"Selected calculation receipt attached from the actual native engine; review and verification remain separate.".into()))}),Err(_)=>self.feedback="Enter a calculation Measure ID.".into()}}});
            ui.horizontal(|ui|{ui.label("Local artifact path");mark(&mut audit,"assurance:artifact-path",ui.text_edit_singleline(&mut self.local_evidence));if mark(&mut audit,"assurance:Attach external",ui.add_enabled(self.pending.is_none(),egui::Button::new("Attach external artifact"))).clicked(){let path=PathBuf::from(self.local_evidence.trim());if !path.is_absolute(){self.feedback="Choose an absolute local artifact path; no URLs or commands are executed.".into();}else{self.update_worker(editor,move|board,editor,config|{let context=EvidenceContext::from_editor(editor,config)?;let id=board.attach_local_evidence_with_metadata(id,context,config,&path,mbse_assurance::EvidenceOutcome::NotRun)?;Ok((Some(id),"External artifact attached with outcome not run; no native-run authenticity inferred.".into()))});}}});
            egui::ScrollArea::vertical().id_salt("evidence-list").max_height(95.).show(ui,|ui|{for(eid,e)in &self.board.register.evidence{if e.requirement==id && mark(&mut audit,format!("assurance:evidence:{}",eid.0),ui.selectable_label(self.evidence==Some(*eid),format!("E{} · {} · {} · {}",eid.0,outcome_label(e.outcome),adequacy_label(e.adequacy),selected_calculation_label(e.selected_calculation))).on_hover_text(format!("{} {}",outcome_hint(e.outcome),adequacy_hint(e.adequacy)))).clicked(){self.evidence=Some(*eid);}}});
            if let Some(eid)=self.evidence {if let Some(e)=self.board.register.evidence.get(&eid).filter(|e|e.requirement==id).cloned(){
                ui.label(selected_calculation_label(e.selected_calculation));
                ui.monospace(format!("Artifact SHA-256: {}",e.artifact_sha256));
                if let Some(q)=&e.selected_quantity{ui.label(format!("Selected output: {} {} · quantity {:?}",q.value,mbse_model::stage1_calc::unit_label(q.quantity.unit),q.quantity.kind));}
                ui.label(format!("Global receipt: {}. Selected evidence outcome: {}. The full artifact retains every result and diagnostic.",global_receipt_label(e.global_calculation_success),outcome_label(e.outcome))).on_hover_text(outcome_hint(e.outcome));if e.global_calculation_success==Some(false){ui.colored_label(egui::Color32::DARK_RED,"Global calculation failed. This evidence describes only the selected output; no overall success is inferred.");}
                if let Some(metadata)=&e.context_metadata{if mark(&mut audit,"assurance:Use recorded metadata",ui.button("Use recorded evidence context metadata")).clicked(){self.config=serde_json::to_string_pretty(metadata).unwrap_or_default();self.feedback="Recorded canonical metadata selected explicitly; historical native proof remains historical until actual evaluation.".into();}}
                ui.label(if e.selected_calculation.is_some(){"Native evaluator provenance requires evaluator-owned live proof. Reopened serialized receipts are historical until a fresh actual evaluation."}else{"External local artifact provenance; no native execution inferred."});
                match &staleness {Some(Ok(reasons))if reasons.is_empty()=>{ui.label("Current freshness/provenance checks match; explicit adequacy and verification remain separate.");},Some(Ok(reasons))=>{ui.colored_label(egui::Color32::DARK_RED,format!("Stale or untrusted: {}",reasons.join("; ")));},Some(Err(error))=>{ui.label(error);},None=>{ui.label("Evidence assessment pending; no current verdict asserted.");}}
                ui.collapsing("Intact evidence artifact and provenance",|ui|{ui.monospace(format!("Exact selected calculation: {:?}\nExact global receipt flag: {:?}\nExact outcome: {:?}\nExact adequacy: {:?}\nExact selected quantity: {:?}",e.selected_calculation,e.global_calculation_success,e.outcome,e.adequacy,e.selected_quantity));ui.monospace(format!("{:?}",e.context));ui.label("Recorded canonical evidence-context metadata:");ui.monospace(serde_json::to_string_pretty(&e.context_metadata).unwrap_or_default());ui.label("Actual fixed native evaluator limits (not controlled by metadata):");ui.monospace(serde_json::to_string_pretty(&e.evaluator_limits).unwrap_or_default());egui::ScrollArea::both().max_height(120.).show(ui,|ui|{let n=e.artifact_bytes.len().min(64*1024);ui.monospace(String::from_utf8_lossy(&e.artifact_bytes[..n]));if n<e.artifact_bytes.len(){ui.label("Preview truncated; complete original bytes retained.");}});});
                ui.horizontal(|ui|{ui.label("Reviewer");mark(&mut audit,"assurance:reviewer",ui.text_edit_singleline(&mut self.reviewer));ui.label("Decision rationale");mark(&mut audit,"assurance:decision-rationale",ui.text_edit_singleline(&mut self.rationale));});
                ui.horizontal_wrapped(|ui|{for (label,adequacy) in [("Review adequate",Adequacy::Adequate),("Review inadequate",Adequacy::Inadequate)]{if mark(&mut audit,format!("assurance:{label}"),ui.add_enabled(self.pending.is_none(),egui::Button::new(label)).on_hover_text(adequacy_hint(adequacy))).clicked(){let reviewer=self.reviewer.clone();let rationale=self.rationale.clone();self.update_worker(editor,move|board,_,_|{board.review_evidence(eid,adequacy,reviewer,rationale)?;Ok((Some(eid),"Explicit evidence review appended; native provenance and current applicability remain separate.".into()))});}}});
                ui.horizontal_wrapped(|ui|{for status in [VerificationStatus::InProgress,VerificationStatus::Completed,VerificationStatus::CompletedUnsuccessful,VerificationStatus::CompletedFailed,VerificationStatus::CompletedPassed]{if mark(&mut audit,format!("assurance:Record {status:?}"),ui.add_enabled(self.pending.is_none(),egui::Button::new(format!("Record {}",verification_label(status)))).on_hover_text(verification_hint(status))).clicked(){let reviewer=self.reviewer.clone();let rationale=self.rationale.clone();self.update_worker(editor,move|board,editor,config|{let context=EvidenceContext::from_editor(editor,config)?;board.record_verification(id,eid,status,&context,reviewer,rationale)?;Ok((Some(eid),"Verification decision appended; current applicability is derived from exact source/proof checks.".into()))});}}});
            }}
            ui.collapsing("Immutable review and verification record history",|ui|{
                for review in &self.board.register.review_history {ui.label(format!("Review #{} · E{} · {} · {} · {}",review.review_id,review.evidence.0,adequacy_label(review.adequacy),review.reviewer,review.rationale));}
                for decision in self.board.register.decision_history.iter().filter(|d|d.requirement==id) {ui.label(format!("Decision #{} · E{} · {} · {} · {}",decision.decision_id,decision.evidence.0,verification_label(decision.status),decision.reviewer,decision.rationale));}
                ui.collapsing("Exact review and decision enum values",|ui|{for review in &self.board.register.review_history{ui.monospace(format!("Review #{} · E{} · {:?}",review.review_id,review.evidence.0,review.adequacy));}for decision in self.board.register.decision_history.iter().filter(|d|d.requirement==id){ui.monospace(format!("Decision #{} · E{} · {:?}",decision.decision_id,decision.evidence.0,decision.status));}});
                ui.label("History records prior declarations; serialized stamps do not authenticate native runs or assert a currently applicable verdict.");
            });
        });
        self.visible = visible&&!close_editor;
        if self.close_prompt {
            egui::Window::new("Preserve requirements register").collapsible(false).show(ctx,|ui|{ui.label("The independent register has unsaved changes or pending IO. Save it in Requirements and evidence, or explicitly discard these exact changes for this close.");if mark(&mut audit,"assurance:Return",ui.button("Return to work")).clicked(){self.close_prompt=false;self.discard=None;}if mark(&mut audit,"assurance:Discard close",ui.add_enabled(self.pending.is_none(),egui::Button::new("Discard register changes for this close"))).clicked(){self.discard=self.close_state();self.close_prompt=false;self.feedback="Exact register changes permitted to be discarded on next application close.".into();}});
        }
        if self.replace_prompt {
            egui::Window::new("Open another register?").collapsible(false).show(ctx,|ui|{ui.label("Opening replaces unsaved register edits only after successful admission. Native model is retained.");if mark(&mut audit,"assurance:Keep current",ui.button("Keep current register")).clicked(){self.replace_prompt=false;}if mark(&mut audit,"assurance:Discard and open",ui.button("Discard and open register")).clicked(){self.load();}});
        }
        self.audit = audit;
    }
}

// Display copy only. Raw enums and IDs remain unchanged in commands, audit keys and provenance.
fn selected_calculation_label(value: Option<CalculationId>) -> String {
    match value {
        Some(id) => format!("Selected calculation measure #{}", id.0),
        None => "No selected calculation measure".into(),
    }
}
fn global_receipt_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "succeeded",
        Some(false) => "failed",
        None => "not applicable",
    }
}
fn maturity_label(value: Maturity) -> &'static str {
    match value {
        Maturity::Draft => "Draft",
        Maturity::InDevelopment => "In development",
        Maturity::ReadyForReview => "Ready for review",
        Maturity::InReview => "In review",
        Maturity::Approved => "Approved",
        Maturity::Disapproved => "Disapproved",
        Maturity::Deleted => "Deleted",
    }
}
fn maturity_hint(value: Maturity) -> &'static str {
    match value{Maturity::Draft=>"Initial requirement wording; implementation and verification remain separate.",Maturity::InDevelopment=>"The requirement wording is being developed.",Maturity::ReadyForReview=>"Prepared for review; no review result is implied.",Maturity::InReview=>"Requirement review is underway.",Maturity::Approved=>"Requirement maturity approval is recorded; this does not mark implementation or verification as passed.",Maturity::Disapproved=>"The requirement maturity decision is disapproved.",Maturity::Deleted=>"The requirement is marked deleted in its recorded lifecycle."}
}
fn implementation_label(value: ImplementationStatus) -> &'static str {
    match value {
        ImplementationStatus::UnderReview => "Under review",
        ImplementationStatus::Waived => "Waived",
        ImplementationStatus::Pending => "Pending",
        ImplementationStatus::InProgress => "In progress",
        ImplementationStatus::Implemented => "Implemented",
        ImplementationStatus::PartiallyImplemented => "Partially implemented",
        ImplementationStatus::NotImplemented => "Not implemented",
    }
}
fn implementation_hint(value: ImplementationStatus) -> &'static str {
    match value {
        ImplementationStatus::UnderReview => {
            "Implementation status is under review; maturity and verification remain separate."
        }
        ImplementationStatus::Waived => {
            "Implementation is recorded as waived; no verification result is implied."
        }
        ImplementationStatus::Pending => "Implementation work is pending.",
        ImplementationStatus::InProgress => "Implementation work is underway.",
        ImplementationStatus::Implemented => {
            "Implementation is recorded as complete; verification remains independent."
        }
        ImplementationStatus::PartiallyImplemented => {
            "Implementation is recorded as only partly complete."
        }
        ImplementationStatus::NotImplemented => "Implementation is recorded as not implemented.",
    }
}
fn verification_label(value: VerificationStatus) -> &'static str {
    match value {
        VerificationStatus::NotStarted => "Not started",
        VerificationStatus::InProgress => "In progress",
        VerificationStatus::Completed => "Completed",
        VerificationStatus::CompletedUnsuccessful => "Completed unsuccessful",
        VerificationStatus::CompletedFailed => "Completed failed",
        VerificationStatus::CompletedPassed => "Completed passed",
    }
}
fn verification_hint(value: VerificationStatus) -> &'static str {
    match value{VerificationStatus::NotStarted=>"Verification is recorded as not started.",VerificationStatus::InProgress=>"Verification is underway; this label records no completed verdict.",VerificationStatus::Completed=>"Completion is recorded without a specific unsuccessful, failed or passed label.",VerificationStatus::CompletedUnsuccessful=>"An unsuccessful completion is recorded. This remains distinct from completed failed and completed passed.",VerificationStatus::CompletedFailed=>"A failed completion is recorded. This remains distinct from completed unsuccessful and completed passed.",VerificationStatus::CompletedPassed=>"An explicit passed decision, subject to the existing evidence, adequacy and freshness/provenance checks."}
}
fn adequacy_label(value: Adequacy) -> &'static str {
    match value {
        Adequacy::NotReviewed => "Not reviewed",
        Adequacy::Adequate => "Adequate",
        Adequacy::Inadequate => "Inadequate",
    }
}
fn adequacy_hint(value: Adequacy) -> &'static str {
    match value{Adequacy::NotReviewed=>"Evidence has not been explicitly reviewed for adequacy.",Adequacy::Adequate=>"The reviewer records the evidence as adequate; a verification decision remains separate.",Adequacy::Inadequate=>"The reviewer records the evidence as inadequate; requirement maturity remains separate."}
}
fn outcome_label(value: EvidenceOutcome) -> &'static str {
    match value {
        EvidenceOutcome::NotRun => "Not run",
        EvidenceOutcome::Completed => "Completed",
        EvidenceOutcome::Failed => "Failed",
        EvidenceOutcome::Cancelled => "Cancelled",
        EvidenceOutcome::Unsupported => "Unsupported",
    }
}
fn outcome_hint(value: EvidenceOutcome) -> &'static str {
    match value{EvidenceOutcome::NotRun=>"This evidence is marked not run; attaching a local artifact does not execute it.",EvidenceOutcome::Completed=>"The evidence records a completed outcome; this is not a verification pass or a global receipt verdict.",EvidenceOutcome::Failed=>"The evidence records a failure; no successful result is implied.",EvidenceOutcome::Cancelled=>"The evidence activity is recorded as cancelled.",EvidenceOutcome::Unsupported=>"The evidence outcome is marked unsupported; no successful native result is implied."}
}

fn mark(
    audit: &mut BTreeMap<String, [f32; 4]>,
    key: impl Into<String>,
    response: egui::Response,
) -> egui::Response {
    let r = response.rect.intersect(response.interact_rect);
    audit.insert(key.into(), [r.min.x, r.min.y, r.max.x, r.max.y]);
    response
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
    fn delayed_attachment_keeps_new_requirement_selection_and_evidence() {
        let mut workspace = AssuranceWorkspace::default();
        let editor = editor();
        let first = workspace
            .board
            .add_requirement("First requirement".into())
            .unwrap();
        let second = workspace
            .board
            .add_requirement("Second requirement".into())
            .unwrap();
        let config = workspace.config().unwrap();
        let context = EvidenceContext::from_editor(&editor, &config).unwrap();
        let path = std::env::temp_dir().join(format!("mbse-selection-{}.txt", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(&mut file, b"Synthetic selection regression").unwrap();
        drop(file);
        let current = workspace
            .board
            .attach_local_evidence(second, context.clone(), &path, EvidenceOutcome::NotRun)
            .unwrap();
        workspace.selected = Some(first);
        let frozen = workspace.close_state().unwrap();
        let mut returned = workspace.board.clone();
        let late = returned
            .attach_local_evidence(first, context, &path, EvidenceOutcome::NotRun)
            .unwrap();
        std::fs::remove_file(&path).unwrap();
        let (tx, rx) = mpsc::channel();
        workspace.pending = Some(rx);
        workspace.selected = Some(second);
        workspace.evidence = Some(current);
        tx.send(Completion::Updated(
            Ok((returned, Some(late), "Attached".into())),
            frozen,
            editor.calculation_stamp(),
            config,
        ))
        .unwrap();
        workspace.show(&egui::Context::default(), &editor);
        assert_eq!(workspace.selected, Some(second));
        assert_eq!(workspace.evidence, Some(current));
        assert_eq!(workspace.board.register.evidence[&late].requirement, first);
        assert!(workspace.pending.is_none());
    }
    #[test]
    fn delayed_load_does_not_replace_newer_register_edits() {
        let mut workspace = AssuranceWorkspace::default();
        workspace.path = "/workspace/example.json".into();
        let frozen = workspace.close_state().unwrap();
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
        workspace.discard = workspace.close_state();
        assert!(workspace.request_close());
        workspace.cancel_close();
        assert!(!workspace.request_close());
        workspace.discard = workspace.close_state();
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
    fn delayed_evidence_cannot_apply_to_changed_bytes_with_unchanged_revisions() {
        let mut workspace = AssuranceWorkspace::default();
        let mut editor = editor();
        let frozen = workspace.close_state().unwrap();
        let stamp = editor.calculation_stamp();
        let config = workspace.config().unwrap();
        let mut returned = workspace.board.clone();
        returned
            .add_requirement("Should never replace current work".into())
            .unwrap();
        let (tx, rx) = mpsc::channel();
        workspace.pending = Some(rx);
        tx.send(Completion::Updated(
            Ok((returned, None, "late".into())),
            frozen,
            stamp.clone(),
            config,
        ))
        .unwrap();
        editor
            .snapshot
            .records
            .get_mut(&mbse_model::stage1::RecordId(1))
            .unwrap()
            .name = "Changed exact bytes".into();
        assert_eq!(
            editor.calculation_stamp().model_revision,
            stamp.model_revision
        );
        workspace.show(&egui::Context::default(), &editor);
        assert!(workspace.board.register.requirements.is_empty());
        assert!(workspace.feedback.contains("stale"));
    }
    #[test]
    fn unadmitted_form_text_binds_dirty_close_and_delayed_open() {
        let mut workspace = AssuranceWorkspace::default();
        workspace.path = "/workspace/source.json".into();
        workspace.saved = workspace.board.as_value().ok();
        let frozen = workspace.close_state().unwrap();
        let (tx, rx) = mpsc::channel();
        workspace.pending = Some(rx);
        workspace.statement = "Rejected form text retained outside the accepted register".into();
        tx.send(Completion::Loaded(
            Ok((Board::new(), "hash".into())),
            PathBuf::from("/workspace/source.json"),
            frozen,
        ))
        .unwrap();
        workspace.show(&egui::Context::default(), &editor());
        assert!(workspace.feedback.contains("stale"));
        assert!(workspace.dirty());
        workspace.discard = workspace.close_state();
        assert!(workspace.request_close());
        workspace.statement.push_str(" changed");
        assert!(!workspace.request_close());
        let retained = workspace.statement.clone();
        workspace.history(true);
        assert_eq!(workspace.statement, retained);
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

#[cfg(test)]
mod hierarchy_tests{
 use super::*;
 #[test]
 fn rendered_root_child_cycle_draft_and_register_history_preserve_native_model(){
  use mbse_model::stage1::{DocumentId,Snapshot};let editor=Editor::new(Snapshot::new(DocumentId("p23:render".into())),"p23:render".into()).unwrap();let native=editor.snapshot.clone();let mut w=AssuranceWorkspace::default();w.open_window();let ctx=egui::Context::default();let render=|w:&mut AssuranceWorkspace,events|{ctx.run_ui(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(2200.,1600.))),focused:true,events,..Default::default()},|_|w.show(&ctx,&editor)).drop_without_applying_deltas();};for _ in 0..3{render(&mut w,vec![]);}let click=|w:&mut AssuranceWorkspace,key:&str|{let r=w.audit[key];let pos=egui::pos2((r[0]+r[2])/2.,(r[1]+r[3])/2.);for pressed in [true,false]{render(w,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]);}render(w,vec![]);};
  click(&mut w,"assurance:new-identifier");render(&mut w,vec![egui::Event::Text("R-1".into())]);click(&mut w,"assurance:new-statement");render(&mut w,vec![egui::Event::Text("Root requirement".into())]);click(&mut w,"assurance:Add");assert_eq!(w.selected,Some(RequirementId(1)));click(&mut w,"assurance:new-identifier");render(&mut w,vec![egui::Event::Text("R-2".into())]);click(&mut w,"assurance:new-statement");render(&mut w,vec![egui::Event::Text("Child requirement".into())]);click(&mut w,"assurance:Add child");assert_eq!(w.selected,Some(RequirementId(2)));assert_eq!(w.board.register.requirements[&RequirementId(2)].parent,Some(RequirementId(1)));click(&mut w,"assurance:select:1");ctx.memory_mut(|m|m.request_focus(egui::Id::new(("requirement-parent",1u64))));render(&mut w,vec![egui::Event::Text("2".into())]);render(&mut w,vec![]);assert_eq!(w.board.register.requirements[&RequirementId(1)].parent,None);assert_eq!(w.board.register.drafts["1:parent"].text,"2");assert!(!w.board.register.drafts["1:parent"].valid);assert_eq!(ctx.memory(|m|m.focused()),Some(egui::Id::new(("requirement-parent",1u64))));let state=w.board.register.clone();click(&mut w,"assurance:Undo");assert!(!w.board.register.drafts.contains_key("1:parent"));click(&mut w,"assurance:Redo");assert_eq!(w.board.register,state);let state=w.board.register.clone();click(&mut w,"assurance:Close");assert!(!w.visible);assert_eq!(w.board.register,state);assert!(!w.request_close());assert_eq!(editor.snapshot,native);
 }
}
