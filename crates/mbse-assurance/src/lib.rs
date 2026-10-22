//! Application-owned requirements/evidence register, never normative SysML admission.
use mbse_model::{
    stage1::Editor,
    stage1_calc::{self as calc, CalculationId, DocumentStamp, QuantityValue},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
pub const PROFILE: &str = "mbse-craft.requirements-evidence/1";
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
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
pub struct Evidence {
    pub id: EvidenceId,
    pub requirement: RequirementId,
    pub requirement_revision: u64,
    pub requirement_hash: String,
    pub context: EvidenceContext,
    pub artifact_sha256: String,
    pub artifact_bytes: Vec<u8>,
    pub artifact_path: Option<PathBuf>,
    pub selected_calculation: Option<CalculationId>,
    pub selected_quantity: Option<QuantityValue>,
    pub outcome: EvidenceOutcome,
    pub adequacy: Adequacy,
    pub reviewer: String,
    pub rationale: String,
    pub reviewed_digest: Option<String>,
    pub unknown: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VerificationDecision {
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
pub struct RawDraft {
    pub text: String,
    pub valid: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Register {
    pub requirements: BTreeMap<RequirementId, Requirement>,
    pub evidence: BTreeMap<EvidenceId, Evidence>,
    pub decisions: BTreeMap<RequirementId, VerificationDecision>,
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
impl Board {
    pub fn new() -> Self {
        Self {
            register: Register {
                requirements: BTreeMap::new(),
                evidence: BTreeMap::new(),
                decisions: BTreeMap::new(),
                drafts: BTreeMap::new(),
                next_id: 1,
                revision: 0,
                unknown: BTreeMap::new(),
            },
            past: vec![],
            future: vec![],
        }
    }
    fn transaction<T>(
        &mut self,
        f: impl FnOnce(&mut Register) -> Result<T, String>,
    ) -> Result<T, String> {
        let before = self.register.clone();
        let next_revision = before
            .revision
            .checked_add(1)
            .ok_or("ASSURANCE_REVISION_LIMIT")?;
        match f(&mut self.register) {
            Ok(v) => {
                if self.register != before {
                    self.register.revision = next_revision;
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
        if let Some(state) = self.past.pop() {
            self.future
                .push(std::mem::replace(&mut self.register, state));
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        if let Some(state) = self.future.pop() {
            self.past.push(std::mem::replace(&mut self.register, state));
            true
        } else {
            false
        }
    }
    pub fn add_requirement(&mut self, statement: String) -> Result<RequirementId, String> {
        self.transaction(|state| {
            if statement.trim().is_empty() {
                return Err("ASSURANCE_STATEMENT_REQUIRED".into());
            }
            let id = RequirementId(state.next_id);
            state.next_id = state.next_id.checked_add(1).ok_or("ASSURANCE_ID_LIMIT")?;
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
        let receipt = editor.evaluate_calculations()?;
        let context = EvidenceContext::from_editor(editor, config)?;
        let selected_quantity = receipt
            .outcomes
            .get(&selected)
            .and_then(|o| o.value())
            .cloned();
        let outcome = if receipt.successful && selected_quantity.is_some() {
            EvidenceOutcome::Completed
        } else {
            EvidenceOutcome::Failed
        };
        let bytes = serde_json::to_vec(&receipt).map_err(|e| e.to_string())?;
        self.attach(
            requirement,
            context,
            bytes,
            None,
            Some(selected),
            selected_quantity,
            outcome,
        )
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
    ) -> Result<EvidenceId, String> {
        self.transaction(|s| {
            let req = s
                .requirements
                .get(&requirement)
                .ok_or("ASSURANCE_REQUIREMENT_MISSING")?;
            let requirement_hash = requirement_hash(req)?;
            let requirement_revision = req.revision;
            let id = EvidenceId(s.next_id);
            s.next_id = s.next_id.checked_add(1).ok_or("ASSURANCE_ID_LIMIT")?;
            s.evidence.insert(
                id,
                Evidence {
                    id,
                    requirement,
                    requirement_revision,
                    requirement_hash,
                    context,
                    artifact_sha256: bytes_hash(&artifact_bytes),
                    artifact_bytes,
                    artifact_path,
                    selected_calculation,
                    selected_quantity,
                    outcome,
                    adequacy: Adequacy::NotReviewed,
                    reviewer: String::new(),
                    rationale: String::new(),
                    reviewed_digest: None,
                    unknown: BTreeMap::new(),
                },
            );
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
            Ok(())
        })
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
                || e.reviewed_digest.as_ref() != Some(&evidence_hash(e)?))
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
            s.decisions.insert(
                requirement,
                VerificationDecision {
                    requirement,
                    evidence,
                    status,
                    reviewer,
                    rationale,
                    evidence_digest: ehash,
                    requirement_hash: rhash,
                    context: current.clone(),
                },
            );
            Ok(())
        })
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
        self.evidence_staleness(d.evidence, current)
            .is_ok_and(|v| v.is_empty())
            && requirement_hash(r).ok().as_ref() == Some(&d.requirement_hash)
            && evidence_hash(e).ok().as_ref() == Some(&d.evidence_digest)
            && d.context == *current
            && (d.status != VerificationStatus::CompletedPassed
                || (e.adequacy == Adequacy::Adequate
                    && e.reviewed_digest.as_ref() == Some(&d.evidence_digest)))
    }
    pub fn as_value(&self) -> Result<Value, String> {
        serde_json::to_value(&self.register).map_err(|e| e.to_string())
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
        };
        board.validate()?;
        Ok((board, bytes_hash(&bytes)))
    }
    fn validate(&self) -> Result<(), String> {
        let s = &self.register;
        if s.requirements.len() > 10000 || s.evidence.len() > 10000 || s.next_id == 0 {
            return Err("ASSURANCE_RECORD_LIMIT".into());
        }
        let mut max = 0u64;
        for (id, r) in &s.requirements {
            if *id != r.id || id.0 == 0 {
                return Err("ASSURANCE_REQUIREMENT_ID".into());
            }
            max = max.max(id.0);
        }
        for (id, e) in &s.evidence {
            if *id != e.id
                || id.0 == 0
                || !s.requirements.contains_key(&e.requirement)
                || bytes_hash(&e.artifact_bytes) != e.artifact_sha256
            {
                return Err("ASSURANCE_EVIDENCE_INTEGRITY".into());
            }
            max = max.max(id.0);
        }
        if s.next_id <= max {
            return Err("ASSURANCE_ID_HIGH_WATER".into());
        }
        Ok(())
    }
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
