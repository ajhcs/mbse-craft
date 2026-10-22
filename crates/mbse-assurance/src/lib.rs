//! Application-owned requirements/evidence register, never normative SysML admission.
use mbse_model::{
    stage1::Editor,
    stage1_calc::{self as calc, CalculationId, DocumentStamp, QuantityValue},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
pub const PROFILE: &str = "mbse-craft.requirements-evidence/1";
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_AUDIT_ENTRIES: usize = 20_000;
pub const MAX_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequirementId(pub u64);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceId(pub u64);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Maturity {
    Draft,
    InDevelopment,
    ReadyForReview,
    InReview,
    Approved,
    Disapproved,
    Deleted,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImplementationStatus {
    UnderReview,
    Waived,
    Pending,
    InProgress,
    Implemented,
    PartiallyImplemented,
    NotImplemented,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationStatus {
    NotStarted,
    InProgress,
    Completed,
    CompletedUnsuccessful,
    CompletedFailed,
    CompletedPassed,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectionStatus {
    Unknown,
    Supported,
    Partial,
    Unsupported,
    Failed,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceAnchor {
    pub format_contract: String,
    pub artifact_sha256: String,
    pub metamodel: String,
    pub source_element_id: String,
    pub native_document: Option<String>,
    pub native_record: Option<u64>,
    pub units: BTreeMap<String, String>,
    pub references: Vec<String>,
    pub package_integrity: ProjectionStatus,
    pub parsing: ProjectionStatus,
    pub reference_resolution: ProjectionStatus,
    pub semantic_projection: ProjectionStatus,
    pub editable_admission: ProjectionStatus,
    pub execution: ProjectionStatus,
    pub unknown: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: RequirementId,
    pub statement: String,
    pub rationale: String,
    pub source: String,
    pub owner: String,
    pub user_version: String,
    pub revision: u64,
    pub maturity: Maturity,
    pub implementation: ImplementationStatus,
    pub verification: VerificationStatus,
    pub anchor: Option<SourceAnchor>,
    pub unknown: BTreeMap<String, Value>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceOutcome {
    NotRun,
    Completed,
    Failed,
    Cancelled,
    Unsupported,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Adequacy {
    NotReviewed,
    Adequate,
    Inadequate,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceContext {
    pub model: DocumentStamp,
    pub config_hash: String,
    pub input_hash: String,
    pub tool_version: String,
}
impl EvidenceContext {
    pub fn from_editor(editor: &Editor, config: &Value) -> Result<Self, String> {
        let receipt = editor.evaluate_calculations()?;
        Ok(Self {
            model: editor.calculation_stamp(),
            config_hash: digest(config)?,
            input_hash: digest(&(
                receipt.definitions_digest,
                receipt.raw_digest,
                receipt.input_closure_digest,
            ))?,
            tool_version: calc::ALGORITHM_REVISION.into(),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub id: EvidenceId,
    pub requirement: RequirementId,
    pub requirement_revision: u64,
    pub requirement_hash: String,
    pub context: EvidenceContext,
    #[serde(default)]
    pub context_metadata: Option<Value>,
    #[serde(default)]
    pub evaluator_limits: Option<calc::Limits>,
    pub artifact_sha256: String,
    pub artifact_bytes: Vec<u8>,
    pub artifact_path: Option<PathBuf>,
    pub selected_calculation: Option<CalculationId>,
    pub selected_quantity: Option<QuantityValue>,
    pub outcome: EvidenceOutcome,
    #[serde(default)]
    pub global_calculation_success: Option<bool>,
    pub adequacy: Adequacy,
    pub reviewer: String,
    pub rationale: String,
    pub reviewed_digest: Option<String>,
    pub unknown: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VerificationDecision {
    pub decision_id: u64,
    pub requirement: RequirementId,
    pub evidence: EvidenceId,
    pub status: VerificationStatus,
    pub reviewer: String,
    pub rationale: String,
    pub evidence_digest: String,
    pub requirement_hash: String,
    pub context: EvidenceContext,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceReview {
    pub review_id: u64,
    pub evidence: EvidenceId,
    pub adequacy: Adequacy,
    pub reviewer: String,
    pub rationale: String,
    pub evidence_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawDraft {
    pub text: String,
    pub valid: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Register {
    pub requirements: BTreeMap<RequirementId, Requirement>,
    pub evidence: BTreeMap<EvidenceId, Evidence>,
    pub decisions: BTreeMap<RequirementId, VerificationDecision>,
    #[serde(default)]
    pub review_history: Vec<EvidenceReview>,
    #[serde(default)]
    pub decision_history: Vec<VerificationDecision>,
    pub drafts: BTreeMap<String, RawDraft>,
    pub next_id: u64,
    pub revision: u64,
    pub unknown: BTreeMap<String, Value>,
}
#[derive(Clone, Debug)]
pub struct Board {
    pub register: Register,
    past: Vec<Register>,
    future: Vec<Register>,
    // Runtime-only ownership proof. Deserialization never restores this map.
    trusted_calculations: BTreeMap<EvidenceId, String>,
    trusted_reviews: BTreeMap<EvidenceId, String>,
    trusted_decisions: BTreeMap<RequirementId, VerificationDecision>,
}
impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}
fn bytes_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|b| bytes_hash(&b))
        .map_err(|e| e.to_string())
}
fn requirement_hash(req: &Requirement) -> Result<String, String> {
    let mut req = req.clone();
    req.verification = VerificationStatus::NotStarted;
    digest(&req)
}
fn evidence_hash(e: &Evidence) -> Result<String, String> {
    let mut e = e.clone();
    e.adequacy = Adequacy::NotReviewed;
    e.reviewer.clear();
    e.rationale.clear();
    e.reviewed_digest = None;
    digest(&e)
}
fn retain_audit(target: &mut Register, current: &Register) {
    for review in &current.review_history {
        if !target
            .review_history
            .iter()
            .any(|v| v.review_id == review.review_id)
        {
            target.review_history.push(review.clone());
        }
    }
    for decision in &current.decision_history {
        if !target
            .decision_history
            .iter()
            .any(|v| v.decision_id == decision.decision_id)
        {
            target.decision_history.push(decision.clone());
        }
    }
    target.next_id = target.next_id.max(current.next_id);
}
impl Board {
    pub fn new() -> Self {
        Self {
            register: Register {
                requirements: BTreeMap::new(),
                evidence: BTreeMap::new(),
                decisions: BTreeMap::new(),
                review_history: vec![],
                decision_history: vec![],
                drafts: BTreeMap::new(),
                next_id: 1,
                revision: 0,
                unknown: BTreeMap::new(),
            },
            past: vec![],
            future: vec![],
            trusted_calculations: BTreeMap::new(),
            trusted_reviews: BTreeMap::new(),
            trusted_decisions: BTreeMap::new(),
        }
    }
    fn transaction<T>(
        &mut self,
        f: impl FnOnce(&mut Register) -> Result<T, String>,
    ) -> Result<T, String> {
        envelope_size(&self.register)?;
        let before = self.register.clone();
        let next_revision = before
            .revision
            .checked_add(1)
            .ok_or("ASSURANCE_REVISION_LIMIT")?;
        match f(&mut self.register) {
            Ok(v) => {
                if self.register != before {
                    self.register.revision = next_revision;
                    if let Err(error) = envelope_size(&self.register) {
                        self.register = before;
                        return Err(error);
                    }
                    self.past.push(before);
                    self.future.clear();
                }
                Ok(v)
            }
            Err(e) => {
                self.register = before;
                Err(e)
            }
        }
    }
    pub fn undo(&mut self) -> bool {
        if let Some(mut state) = self.past.last().cloned() {
            retain_audit(&mut state, &self.register);
            if envelope_size(&state).is_err() || validate_register(&state).is_err() {
                return false;
            }
            self.past.pop();
            self.future
                .push(std::mem::replace(&mut self.register, state));
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        if let Some(mut state) = self.future.last().cloned() {
            retain_audit(&mut state, &self.register);
            if envelope_size(&state).is_err() || validate_register(&state).is_err() {
                return false;
            }
            self.future.pop();
            self.past.push(std::mem::replace(&mut self.register, state));
            true
        } else {
            false
        }
    }
    pub fn set_new_statement_draft(&mut self, text: String) -> Result<(), String> {
        self.transaction(move |state| {
            if text.is_empty() {
                state.drafts.remove("new:statement");
            } else {
                state.drafts.insert(
                    "new:statement".into(),
                    RawDraft {
                        valid: !text.trim().is_empty(),
                        text,
                    },
                );
            }
            Ok(())
        })
    }
    pub fn add_requirement(&mut self, statement: String) -> Result<RequirementId, String> {
        self.transaction(|state| {
            if state.requirements.len() >= 10_000 {
                return Err("ASSURANCE_RECORD_LIMIT".into());
            }
            if statement.trim().is_empty() {
                return Err("ASSURANCE_STATEMENT_REQUIRED".into());
            }
            let id = RequirementId(state.next_id);
            state.next_id = state.next_id.checked_add(1).ok_or("ASSURANCE_ID_LIMIT")?;
            state.drafts.remove("new:statement");
            state.requirements.insert(
                id,
                Requirement {
                    id,
                    statement,
                    rationale: String::new(),
                    source: String::new(),
                    owner: String::new(),
                    user_version: String::new(),
                    revision: 0,
                    maturity: Maturity::Draft,
                    implementation: ImplementationStatus::UnderReview,
                    verification: VerificationStatus::NotStarted,
                    anchor: None,
                    unknown: BTreeMap::new(),
                },
            );
            Ok(id)
        })
    }
    pub fn edit_field(
        &mut self,
        id: RequirementId,
        field: &str,
        text: String,
    ) -> Result<bool, String> {
        self.transaction(|state| {
            let req = state
                .requirements
                .get_mut(&id)
                .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
            let value = match field {
                "statement" => &mut req.statement,
                "rationale" => &mut req.rationale,
                "source" => &mut req.source,
                "owner" => &mut req.owner,
                "user_version" => &mut req.user_version,
                _ => return Err("ASSURANCE_UNSUPPORTED_FIELD".into()),
            };
            let valid = field != "statement" || !text.trim().is_empty();
            state.drafts.insert(
                format!("{}:{field}", id.0),
                RawDraft {
                    text: text.clone(),
                    valid,
                },
            );
            if valid && *value != text {
                *value = text;
                req.revision = req
                    .revision
                    .checked_add(1)
                    .ok_or("ASSURANCE_REVISION_LIMIT")?;
            }
            Ok(valid)
        })
    }
    pub fn set_maturity(&mut self, id: RequirementId, value: Maturity) -> Result<(), String> {
        self.transaction(|s| {
            let r = s
                .requirements
                .get_mut(&id)
                .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
            if r.maturity != value {
                r.maturity = value;
                r.revision = r
                    .revision
                    .checked_add(1)
                    .ok_or("ASSURANCE_REVISION_LIMIT")?;
            }
            Ok(())
        })
    }
    pub fn set_implementation(
        &mut self,
        id: RequirementId,
        value: ImplementationStatus,
    ) -> Result<(), String> {
        self.transaction(|s| {
            let r = s
                .requirements
                .get_mut(&id)
                .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
            if r.implementation != value {
                r.implementation = value;
                r.revision = r
                    .revision
                    .checked_add(1)
                    .ok_or("ASSURANCE_REVISION_LIMIT")?;
            }
            Ok(())
        })
    }
    pub fn set_anchor(
        &mut self,
        id: RequirementId,
        anchor: Option<SourceAnchor>,
    ) -> Result<(), String> {
        self.transaction(|s| {
            let r = s
                .requirements
                .get_mut(&id)
                .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
            if r.anchor != anchor {
                r.anchor = anchor;
                r.revision = r
                    .revision
                    .checked_add(1)
                    .ok_or("ASSURANCE_REVISION_LIMIT")?;
            }
            Ok(())
        })
    }
    pub fn attach_calculation(
        &mut self,
        requirement: RequirementId,
        editor: &Editor,
        config: &Value,
        selected: CalculationId,
    ) -> Result<EvidenceId, String> {
        let requirement_record = self
            .register
            .requirements
            .get(&requirement)
            .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
        let selected_record = editor
            .snapshot
            .records
            .get(&mbse_model::stage1::RecordId(selected.0))
            .ok_or("ASSURANCE_SELECTED_MEASURE_MISSING")?;
        if selected_record.kind != mbse_model::stage1::RecordKind::Measure
            || !editor.snapshot.calculations()?.contains_key(&selected)
        {
            return Err("ASSURANCE_SELECTED_CALCULATION_MISSING".into());
        }
        if let Some(anchor) = &requirement_record.anchor {
            if anchor.format_contract == mbse_model::stage1_io::PROFILE
                && anchor.artifact_sha256 != editor.snapshot.digest()
            {
                return Err("ASSURANCE_NATIVE_ANCHOR_SOURCE_STALE".into());
            }
            if anchor
                .native_document
                .as_ref()
                .is_some_and(|id| id != &editor.snapshot.document_id.0)
            {
                return Err("ASSURANCE_ANCHOR_DOCUMENT_MISMATCH".into());
            }
            if let Some(id) = anchor.native_record {
                let source = editor
                    .snapshot
                    .records
                    .get(&mbse_model::stage1::RecordId(id))
                    .ok_or("ASSURANCE_ANCHOR_RECORD_MISSING")?;
                if source.scope != selected_record.scope {
                    return Err("ASSURANCE_CROSS_SYSTEM_EVIDENCE".into());
                }
            }
        }
        let receipt = editor.evaluate_calculations()?;
        let context = EvidenceContext::from_editor(editor, config)?;
        let selected_quantity = receipt
            .outcomes
            .get(&selected)
            .and_then(|o| o.value())
            .cloned();
        let outcome = if selected_quantity.is_some() {
            EvidenceOutcome::Completed
        } else {
            EvidenceOutcome::Failed
        };
        let bytes = serde_json::to_vec(&receipt).map_err(|e| e.to_string())?;
        let id = self.attach(
            requirement,
            context,
            bytes,
            None,
            Some(selected),
            selected_quantity,
            outcome,
            Some(receipt.successful),
            Some(config.clone()),
        )?;
        let proof = evidence_hash(
            self.register
                .evidence
                .get(&id)
                .ok_or("ASSURANCE_EVIDENCE_MISSING")?,
        )?;
        self.trusted_calculations.insert(id, proof);
        Ok(id)
    }
    pub fn attach_local_evidence(
        &mut self,
        requirement: RequirementId,
        context: EvidenceContext,
        path: &Path,
        outcome: EvidenceOutcome,
    ) -> Result<EvidenceId, String> {
        let bytes = mbse_model::local_input::read(path, 8 * 1024 * 1024, "ASSURANCE_EVIDENCE")?;
        self.attach(
            requirement,
            context,
            bytes,
            Some(path.to_owned()),
            None,
            None,
            outcome,
            None,
            None,
        )
    }
    pub fn attach_local_evidence_with_metadata(
        &mut self,
        requirement: RequirementId,
        context: EvidenceContext,
        metadata: &Value,
        path: &Path,
        outcome: EvidenceOutcome,
    ) -> Result<EvidenceId, String> {
        if digest(metadata)? != context.config_hash {
            return Err("ASSURANCE_CONTEXT_METADATA_HASH_MISMATCH".into());
        }
        let bytes = mbse_model::local_input::read(path, 8 * 1024 * 1024, "ASSURANCE_EVIDENCE")?;
        self.attach(
            requirement,
            context,
            bytes,
            Some(path.to_owned()),
            None,
            None,
            outcome,
            None,
            Some(metadata.clone()),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn attach(
        &mut self,
        requirement: RequirementId,
        context: EvidenceContext,
        artifact_bytes: Vec<u8>,
        artifact_path: Option<PathBuf>,
        selected_calculation: Option<CalculationId>,
        selected_quantity: Option<QuantityValue>,
        outcome: EvidenceOutcome,
        global_calculation_success: Option<bool>,
        context_metadata: Option<Value>,
    ) -> Result<EvidenceId, String> {
        if self.register.evidence.len() >= 10_000 {
            return Err("ASSURANCE_RECORD_LIMIT".into());
        }
        let aggregate = self
            .register
            .evidence
            .values()
            .try_fold(artifact_bytes.len(), |n, e| {
                n.checked_add(e.artifact_bytes.len())
            })
            .ok_or("ASSURANCE_ARTIFACT_BUDGET")?;
        if aggregate > MAX_ARTIFACT_BYTES {
            return Err("ASSURANCE_ARTIFACT_BUDGET".into());
        }
        let req = self
            .register
            .requirements
            .get(&requirement)
            .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
        let id = EvidenceId(self.register.next_id);
        let evidence = Evidence {
            id,
            requirement,
            requirement_revision: req.revision,
            requirement_hash: requirement_hash(req)?,
            context,
            context_metadata,
            evaluator_limits: selected_calculation.map(|_| calc::Limits::default()),
            artifact_sha256: bytes_hash(&artifact_bytes),
            artifact_bytes,
            artifact_path,
            selected_calculation,
            selected_quantity,
            outcome,
            global_calculation_success,
            adequacy: Adequacy::NotReviewed,
            reviewer: String::new(),
            rationale: String::new(),
            reviewed_digest: None,
            unknown: BTreeMap::new(),
        };
        // Stream-count the exact prospective envelope, including Vec<u8> JSON expansion,
        // before cloning the existing register into transaction/Undo history.
        envelope_size(&RegisterWithEvidence {
            register: &self.register,
            evidence: &evidence,
        })?;
        self.transaction(move |s| {
            s.next_id = s.next_id.checked_add(1).ok_or("ASSURANCE_ID_LIMIT")?;
            s.evidence.insert(id, evidence);
            Ok(id)
        })
    }

    pub fn evidence_staleness(
        &self,
        id: EvidenceId,
        current: &EvidenceContext,
    ) -> Result<Vec<String>, String> {
        let e = self
            .register
            .evidence
            .get(&id)
            .ok_or("ASSURANCE_EVIDENCE_MISSING")?;
        let req = self
            .register
            .requirements
            .get(&e.requirement)
            .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
        let mut reasons = vec![];
        if e.context_metadata
            .as_ref()
            .and_then(|v| digest(v).ok())
            .as_ref()
            != Some(&e.context.config_hash)
        {
            reasons.push("retained evidence-context metadata missing or hash mismatch".into());
        }
        if e.selected_calculation.is_some()
            && e.evaluator_limits.as_ref() != Some(&calc::Limits::default())
        {
            reasons.push(
                "recorded evaluator limits differ from the fixed native evaluator configuration"
                    .into(),
            );
        }
        if e.selected_calculation.is_some()
            || e.selected_quantity.is_some()
            || e.global_calculation_success.is_some()
        {
            if self.trusted_calculations.get(&id) != Some(&evidence_hash(e)?) {
                reasons.push("native evaluation has no matching evaluator-owned runtime proof; serialized stamps prove freshness only".into());
            }
        } else if e.artifact_path.is_none() {
            reasons.push("evidence has neither native runtime proof nor explicit external artifact provenance".into());
        }

        if let Some(anchor) = &req.anchor {
            if anchor.format_contract == mbse_model::stage1_io::PROFILE
                && (anchor.artifact_sha256 != current.model.snapshot_hash
                    || anchor.native_document.as_ref() != Some(&current.model.document_id))
            {
                reasons.push(
                    "native source anchor no longer matches the current exact document/snapshot"
                        .into(),
                );
            }
        }
        if req.revision != e.requirement_revision || requirement_hash(req)? != e.requirement_hash {
            reasons.push("requirement changed".into());
        }
        if &e.context != current {
            reasons.push("model/raw/input/tool/config stamp changed".into());
        }
        if self
            .register
            .drafts
            .iter()
            .any(|(k, v)| k.starts_with(&format!("{}:", req.id.0)) && !v.valid)
        {
            reasons.push("requirement has incomplete raw input".into());
        }
        if bytes_hash(&e.artifact_bytes) != e.artifact_sha256 {
            reasons.push("retained artifact changed".into());
        }
        if let Some(path) = &e.artifact_path {
            match mbse_model::local_input::read(path, 8 * 1024 * 1024, "ASSURANCE_EVIDENCE") {
                Ok(bytes) if bytes_hash(&bytes) == e.artifact_sha256 => (),
                _ => reasons.push("source artifact changed or unavailable".into()),
            }
        }
        Ok(reasons)
    }
    pub fn review_evidence(
        &mut self,
        id: EvidenceId,
        adequacy: Adequacy,
        reviewer: String,
        rationale: String,
    ) -> Result<(), String> {
        self.transaction(|s| {
            if adequacy != Adequacy::NotReviewed
                && (reviewer.trim().is_empty() || rationale.trim().is_empty())
            {
                return Err("ASSURANCE_EXPLICIT_REVIEW_REQUIRED".into());
            }
            let e = s
                .evidence
                .get_mut(&id)
                .ok_or("ASSURANCE_EVIDENCE_MISSING")?;
            e.adequacy = adequacy;
            e.reviewer = reviewer;
            e.rationale = rationale;
            e.reviewed_digest = if adequacy == Adequacy::NotReviewed {
                None
            } else {
                Some(evidence_hash(e)?)
            };
            if s.review_history.len() >= MAX_AUDIT_ENTRIES {
                return Err("ASSURANCE_AUDIT_LIMIT".into());
            }
            let review_id = s.next_id;
            s.next_id = s.next_id.checked_add(1).ok_or("ASSURANCE_ID_LIMIT")?;
            s.review_history.push(EvidenceReview {
                review_id,
                evidence: id,
                adequacy,
                reviewer: e.reviewer.clone(),
                rationale: e.rationale.clone(),
                evidence_digest: evidence_hash(e)?,
            });
            Ok(())
        })?;
        let review = self
            .register
            .review_history
            .last()
            .ok_or("ASSURANCE_REVIEW_MISSING")?;
        self.trusted_reviews.insert(id, digest(review)?);
        Ok(())
    }
    pub fn record_verification(
        &mut self,
        requirement: RequirementId,
        evidence: EvidenceId,
        status: VerificationStatus,
        current: &EvidenceContext,
        reviewer: String,
        rationale: String,
    ) -> Result<(), String> {
        if reviewer.trim().is_empty() || rationale.trim().is_empty() {
            return Err("ASSURANCE_EXPLICIT_DECISION_REQUIRED".into());
        }
        let stale = self.evidence_staleness(evidence, current)?;
        let e = self
            .register
            .evidence
            .get(&evidence)
            .ok_or("ASSURANCE_EVIDENCE_MISSING")?;
        if e.requirement != requirement {
            return Err("ASSURANCE_WRONG_REQUIREMENT".into());
        }
        if status == VerificationStatus::CompletedPassed
            && (!stale.is_empty()
                || e.outcome != EvidenceOutcome::Completed
                || e.adequacy != Adequacy::Adequate
                || e.reviewed_digest.as_ref() != Some(&evidence_hash(e)?)
                || self
                    .register
                    .review_history
                    .iter()
                    .rev()
                    .find(|r| r.evidence == evidence)
                    .and_then(|r| digest(r).ok())
                    .as_ref()
                    != self.trusted_reviews.get(&evidence)
                || !self
                    .register
                    .review_history
                    .iter()
                    .rev()
                    .find(|r| r.evidence == evidence)
                    .is_some_and(|r| {
                        r.adequacy == Adequacy::Adequate
                            && r.evidence_digest == evidence_hash(e).unwrap_or_default()
                            && r.reviewer == e.reviewer
                            && r.rationale == e.rationale
                    }))
        {
            return Err(format!("ASSURANCE_PASSED_REFUSED: current completed explicitly adequate evidence required; {stale:?}"));
        }
        let ehash = evidence_hash(e)?;
        self.transaction(|s| {
            let req = s
                .requirements
                .get_mut(&requirement)
                .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
            let rhash = requirement_hash(req)?;
            req.verification = status;
            if s.decision_history.len() >= MAX_AUDIT_ENTRIES {
                return Err("ASSURANCE_AUDIT_LIMIT".into());
            }
            let decision_id = s.next_id;
            s.next_id = s.next_id.checked_add(1).ok_or("ASSURANCE_ID_LIMIT")?;
            let decision = VerificationDecision {
                decision_id,
                requirement,
                evidence,
                status,
                reviewer,
                rationale,
                evidence_digest: ehash,
                requirement_hash: rhash,
                context: current.clone(),
            };
            s.decision_history.push(decision.clone());
            s.decisions.insert(requirement, decision);
            Ok(())
        })?;
        let decision = self
            .register
            .decisions
            .get(&requirement)
            .ok_or("ASSURANCE_DECISION_MISSING")?
            .clone();
        self.trusted_decisions.insert(requirement, decision);
        Ok(())
    }
    pub fn verification_current(
        &self,
        requirement: RequirementId,
        current: &EvidenceContext,
    ) -> bool {
        let Some(d) = self.register.decisions.get(&requirement) else {
            return false;
        };
        let Some(e) = self.register.evidence.get(&d.evidence) else {
            return false;
        };
        let Some(r) = self.register.requirements.get(&requirement) else {
            return false;
        };
        self.trusted_decisions.get(&requirement) == Some(d)
            && self
                .evidence_staleness(d.evidence, current)
                .is_ok_and(|v| v.is_empty())
            && requirement_hash(r).ok().as_ref() == Some(&d.requirement_hash)
            && evidence_hash(e).ok().as_ref() == Some(&d.evidence_digest)
            && d.context == *current
            && (d.status != VerificationStatus::CompletedPassed
                || (e.adequacy == Adequacy::Adequate
                    && e.reviewed_digest.as_ref() == Some(&d.evidence_digest)
                    && self
                        .register
                        .review_history
                        .iter()
                        .rev()
                        .find(|r| r.evidence == d.evidence)
                        .is_some_and(|r| {
                            digest(r).ok().as_ref() == self.trusted_reviews.get(&d.evidence)
                                && r.reviewer == e.reviewer
                                && r.rationale == e.rationale
                        })))
    }
    pub fn as_value(&self) -> Result<Value, String> {
        serde_json::to_value(&self.register).map_err(|e| e.to_string())
    }
}
// Counting serialization is bounded and allocates no encoded artifact copy.
struct SizeWriter(usize);
impl std::io::Write for SizeWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let next = self
            .0
            .checked_add(bytes.len())
            .filter(|n| *n <= MAX_BYTES)
            .ok_or_else(|| std::io::Error::other("ASSURANCE_DOCUMENT_LIMIT"))?;
        self.0 = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn envelope_size(register: &impl Serialize) -> Result<usize, String> {
    #[derive(Serialize)]
    struct View<'a, T: Serialize> {
        profile: &'static str,
        register: &'a T,
    }
    let mut writer = SizeWriter(0);
    serde_json::to_writer(
        &mut writer,
        &View {
            profile: PROFILE,
            register,
        },
    )
    .map_err(|e| format!("ASSURANCE_DOCUMENT_LIMIT: {e}"))?;
    Ok(writer.0)
}
struct RegisterWithEvidence<'a> {
    register: &'a Register,
    evidence: &'a Evidence,
}
impl Serialize for RegisterWithEvidence<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        struct Entries<'a>(&'a BTreeMap<EvidenceId, Evidence>, &'a Evidence);
        impl Serialize for Entries<'_> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(self.0.len() + 1))?;
                for (id, e) in self.0 {
                    map.serialize_entry(id, e)?;
                }
                map.serialize_entry(&self.1.id, self.1)?;
                map.end()
            }
        }
        let r = self.register;
        let next_id = r
            .next_id
            .checked_add(1)
            .ok_or_else(|| <S::Error as serde::ser::Error>::custom("ASSURANCE_ID_LIMIT"))?;
        let revision = r
            .revision
            .checked_add(1)
            .ok_or_else(|| <S::Error as serde::ser::Error>::custom("ASSURANCE_REVISION_LIMIT"))?;
        let mut state = serializer.serialize_struct("Register", 9)?;
        state.serialize_field("requirements", &r.requirements)?;
        state.serialize_field("evidence", &Entries(&r.evidence, self.evidence))?;
        state.serialize_field("decisions", &r.decisions)?;
        state.serialize_field("review_history", &r.review_history)?;
        state.serialize_field("decision_history", &r.decision_history)?;
        state.serialize_field("drafts", &r.drafts)?;
        state.serialize_field("next_id", &next_id)?;
        state.serialize_field("revision", &revision)?;
        state.serialize_field("unknown", &r.unknown)?;
        state.end()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    profile: String,
    register: Register,
}
impl Board {
    pub fn set_verification(
        &mut self,
        id: RequirementId,
        status: VerificationStatus,
    ) -> Result<(), String> {
        if status == VerificationStatus::CompletedPassed {
            return Err("ASSURANCE_PASSED_REQUIRES_EVIDENCE_DECISION".into());
        }
        self.transaction(|s| {
            let r = s
                .requirements
                .get_mut(&id)
                .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
            r.verification = status;
            s.decisions.remove(&id);
            Ok(())
        })
    }
    pub fn save_new(&self, path: &Path) -> Result<String, String> {
        self.save(path, None)
    }
    /// Replaces only previously owned exact bytes. A noncooperating writer can race
    /// final rename; no filesystem compare-and-swap or review authority is claimed.
    pub fn replace(&self, path: &Path, expected_sha256: &str) -> Result<String, String> {
        self.save(path, Some(expected_sha256))
    }
    fn save(&self, path: &Path, expected: Option<&str>) -> Result<String, String> {
        self.validate()?;
        let bytes = serde_json::to_vec(&Envelope {
            profile: PROFILE.into(),
            register: self.register.clone(),
        })
        .map_err(|e| e.to_string())?;
        if bytes.len() > MAX_BYTES {
            return Err("ASSURANCE_DOCUMENT_LIMIT".into());
        }
        let options = craft_atomic::WriteOptions::default()
            .with_new_file_permissions(craft_atomic::NewFilePermissions::Private);
        if let Some(expected) = expected {
            let prior = mbse_model::local_input::read(path, MAX_BYTES, "ASSURANCE")?;
            if bytes_hash(&prior) != expected {
                return Err("ASSURANCE_DISK_CONFLICT".into());
            }
            options
                .atomic_write(path, &bytes)
                .map_err(|e| e.to_string())?;
        } else {
            options.write_new(path, &bytes).map_err(|e| e.to_string())?;
        }
        let committed = mbse_model::local_input::read(path, MAX_BYTES, "ASSURANCE")?;
        if committed != bytes {
            return Err("ASSURANCE_POST_WRITE_CONFLICT".into());
        }
        Ok(bytes_hash(&bytes))
    }
    pub fn open(path: &Path) -> Result<(Self, String), String> {
        let bytes = mbse_model::local_input::read(path, MAX_BYTES, "ASSURANCE")?;
        let unique: UniqueValue =
            serde_json::from_slice(&bytes).map_err(|e| format!("ASSURANCE_JSON: {e}"))?;
        let env: Envelope = serde_json::from_value(unique.0).map_err(|e| e.to_string())?;
        if env.profile != PROFILE {
            return Err("ASSURANCE_UNSUPPORTED_PROFILE".into());
        }
        let board = Self {
            register: env.register,
            past: vec![],
            future: vec![],
            trusted_calculations: BTreeMap::new(),
            trusted_reviews: BTreeMap::new(),
            trusted_decisions: BTreeMap::new(),
        };
        board.validate()?;
        Ok((board, bytes_hash(&bytes)))
    }
    fn validate(&self) -> Result<(), String> {
        validate_register(&self.register)
    }
}
fn validate_register(s: &Register) -> Result<(), String> {
    if s.requirements.len() > 10000
        || s.evidence.len() > 10000
        || s.next_id == 0
        || s.review_history.len() > MAX_AUDIT_ENTRIES
        || s.decision_history.len() > MAX_AUDIT_ENTRIES
    {
        return Err("ASSURANCE_RECORD_LIMIT".into());
    }
    if s.evidence
        .values()
        .try_fold(0usize, |n, e| n.checked_add(e.artifact_bytes.len()))
        .is_none_or(|n| n > MAX_ARTIFACT_BYTES)
    {
        return Err("ASSURANCE_ARTIFACT_BUDGET".into());
    }
    let mut max = 0u64;
    let mut used = BTreeSet::new();
    for (id, r) in &s.requirements {
        if *id != r.id || id.0 == 0 || !used.insert(id.0) || r.statement.trim().is_empty() {
            return Err("ASSURANCE_REQUIREMENT_ID".into());
        }
        max = max.max(id.0);
    }
    for (id, e) in &s.evidence {
        if *id != e.id
            || id.0 == 0
            || !used.insert(id.0)
            || !s.requirements.contains_key(&e.requirement)
            || bytes_hash(&e.artifact_bytes) != e.artifact_sha256
            || e.context_metadata
                .as_ref()
                .is_some_and(|v| digest(v).ok().as_ref() != Some(&e.context.config_hash))
        {
            return Err("ASSURANCE_EVIDENCE_INTEGRITY".into());
        }
        max = max.max(id.0);
    }
    // Audit entries may reference undone objects, but their identities never recycle.
    for review in &s.review_history {
        if review.review_id == 0 || !used.insert(review.review_id) {
            return Err("ASSURANCE_REVIEW_ID".into());
        }
        max = max.max(review.review_id);
    }
    for decision in &s.decision_history {
        if decision.decision_id == 0 || !used.insert(decision.decision_id) {
            return Err("ASSURANCE_DECISION_ID".into());
        }
        max = max.max(decision.decision_id);
    }
    for (id, decision) in &s.decisions {
        if *id != decision.requirement
            || !s.requirements.contains_key(id)
            || !s.evidence.contains_key(&decision.evidence)
            || !s.decision_history.iter().any(|v| v == decision)
        {
            return Err("ASSURANCE_DECISION_CACHE_INTEGRITY".into());
        }
    }
    if s.next_id <= max {
        return Err("ASSURANCE_ID_HIGH_WATER".into());
    }
    Ok(())
}
// Reject duplicate JSON keys before typed interpretation, including unknown maps.
struct UniqueValue(Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unique-key JSON")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = vec![];
                while let Some(v) = a.next_element::<UniqueValue>()? {
                    values.push(v.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((k, v)) = a.next_entry::<String, UniqueValue>()? {
                    if values.insert(k, v.0).is_some() {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
