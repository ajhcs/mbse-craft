//! Bounded Stage1 app-owned records, accepted values and independent raw journals.
//!
//! This profile is not a SysML interchange format or a calculation engine. Unknown
//! record properties remain JSON values; typed commands operate on a closed set.
use crate::stage1_calc::{self as calc, CalculationId, Definition, Input};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const CALCULATION_CONFIG_PREFIX: &str = "calc-config:";
const CALCULATION_CONFIG_PROFILE: &str = "kernspan.stage1.calc-config/1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct RecordId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct DocumentId(pub String);

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecordKind {
    System,
    Mission,
    Capability,
    Objective,
    Measure,
    Stakeholder,
    Expectation,
    Environment,
    Organization,
    Actor,
}

impl RecordKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Mission => "Mission",
            Self::Capability => "Capability",
            Self::Objective => "Objective",
            Self::Measure => "Measure",
            Self::Stakeholder => "Stakeholder",
            Self::Expectation => "Expectation",
            Self::Environment => "Environment",
            Self::Organization => "Organization",
            Self::Actor => "Actor",
        }
    }
}

/// Existing closed-profile affiliation meaning; never inferred from proximity.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AffiliationKind {
    Unknown,
    Internal,
    External,
}
impl AffiliationKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Internal => "Internal",
            Self::External => "External",
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextKind {
    Description,
    Assumption,
    AcceptedConstraint,
}
impl ContextKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Description => "Description",
            Self::Assumption => "Assumption",
            Self::AcceptedConstraint => "AcceptedConstraint",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub id: RecordId,
    pub kind: RecordKind,
    pub name: String,
    pub scope: RecordId,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubjectRef {
    System(RecordId),
    Mission(RecordId),
    Capability(RecordId),
}

impl SubjectRef {
    fn parts(self) -> (RecordId, RecordKind) {
        match self {
            Self::System(id) => (id, RecordKind::System),
            Self::Mission(id) => (id, RecordKind::Mission),
            Self::Capability(id) => (id, RecordKind::Capability),
        }
    }

    fn from_value(value: &Value) -> Result<Option<Self>, String> {
        if value.is_null() {
            return Ok(None);
        }
        let object = value
            .as_object()
            .ok_or("Subject must be a typed reference")?;
        let id = RecordId(
            object
                .get("id")
                .and_then(Value::as_u64)
                .ok_or("Missing subject ID")?,
        );
        match object.get("kind").and_then(Value::as_str) {
            Some("System") => Ok(Some(Self::System(id))),
            Some("Mission") => Ok(Some(Self::Mission(id))),
            Some("Capability") => Ok(Some(Self::Capability(id))),
            _ => Err("Subject kind must be System, Mission or Capability".into()),
        }
    }

    fn as_value(self) -> Value {
        let (id, kind) = self.parts();
        json!({"kind": kind.label(), "id": id.0})
    }
}

impl Record {
    pub fn text(&self, field: &str) -> &str {
        if field == "name" {
            &self.name
        } else {
            self.fields.get(field).and_then(Value::as_str).unwrap_or("")
        }
    }

    pub fn number(&self, field: &str) -> Option<f64> {
        self.fields.get(field).and_then(Value::as_f64)
    }

    pub fn links(&self) -> Vec<RecordId> {
        self.fields
            .get("contributesTo")
            .and_then(Value::as_array)
            .map(|links| {
                links
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(RecordId)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn subject(&self) -> Option<SubjectRef> {
        self.fields
            .get("subject")
            .and_then(|v| SubjectRef::from_value(v).ok())
            .flatten()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawKey {
    pub record: RecordId,
    pub field: String,
    pub item: Option<RecordId>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Validity {
    Valid,
    Incomplete,
    Invalid,
    Unsupported,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RawDraft {
    pub key: RawKey,
    pub text: String,
    pub version: u64,
    pub validity: Validity,
    pub unit_context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_generation: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CalculationRawOwners {
    expression: u64,
    inputs: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CreationDraft {
    pub owner: RecordId,
    pub kind: RecordKind,
    pub fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub extensions:Option<crate::extensions::Extensions>,
    pub document_id: DocumentId,
    pub records: BTreeMap<RecordId, Record>,
    pub drafts: Vec<RawDraft>,
    pub creation_drafts: BTreeMap<String, CreationDraft>,
    pub next_id: u64,
    pub model_revision: u64,
    pub raw_version: u64,
}

impl Snapshot {
    pub fn new(document_id: DocumentId) -> Self {
        let system = Record {
            id: RecordId(1),
            kind: RecordKind::System,
            name: String::new(),
            scope: RecordId(1),
            fields: BTreeMap::from([("purpose".into(), json!(""))]),
        };
        Self {
            extensions:None,
            document_id,
            records: BTreeMap::from([(system.id, system)]),
            drafts: Vec::new(),
            creation_drafts: BTreeMap::new(),
            next_id: 1000,
            model_revision: 0,
            raw_version: 0,
        }
    }

    /// Encodes the explicitly selected illustrative reference fixture, not an import
    /// adapter for arbitrary SysML. Every non-identity record property is retained.
    pub fn from_fixture(document_id: DocumentId, fixture: &Value) -> Result<Self, String> {
        let mut snapshot = Self::new(document_id);
        snapshot.records.clear();
        let records = fixture
            .get("records")
            .and_then(Value::as_array)
            .ok_or("Fixture must contain a records array")?;
        for value in records {
            let mut fields = value
                .as_object()
                .ok_or("Fixture record must be an object")?
                .clone();
            let id = RecordId(
                fields
                    .remove("id")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing record ID")?,
            );
            let kind: RecordKind =
                serde_json::from_value(fields.remove("type").ok_or("Missing record type")?)
                    .map_err(|_| "Unsupported fixture record type")?;
            let name = fields
                .remove("name")
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or("Missing record name")?;
            let scope = RecordId(
                fields
                    .remove("scope")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing record scope")?,
            );
            let record = Record {
                id,
                kind,
                name,
                scope,
                fields: fields.into_iter().collect(),
            };
            if snapshot.records.insert(id, record).is_some() {
                return Err("Duplicate fixture record ID".into());
            }
        }
        if let Some(raw) = fixture.get("rawDrafts") {
            for value in raw.as_array().ok_or("rawDrafts must be an array")? {
                let record = RecordId(
                    value
                        .get("recordId")
                        .and_then(Value::as_u64)
                        .ok_or("Missing draft record ID")?,
                );
                let field = value
                    .get("field")
                    .and_then(Value::as_str)
                    .ok_or("Missing draft field")?
                    .to_owned();
                let text = value
                    .get("rawText")
                    .and_then(Value::as_str)
                    .ok_or("Missing draft text")?
                    .to_owned();
                let item = value
                    .get("itemId")
                    .map(|v| v.as_u64().map(RecordId).ok_or("Invalid draft item ID"))
                    .transpose()?;
                let validity = match value.get("validity").and_then(Value::as_str) {
                    Some("valid") => Validity::Valid,
                    Some("incomplete") => Validity::Incomplete,
                    Some("invalid") => Validity::Invalid,
                    Some(
                        "unsupported-expression" | "unsupported-numeric-range" | "unsupported",
                    ) => Validity::Unsupported,
                    _ => return Err("Unknown fixture draft validity".into()),
                };
                snapshot.raw_version = increment(snapshot.raw_version, "raw version")?;
                let unit_context = value
                    .get("unitId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| {
                        snapshot
                            .records
                            .get(&record)
                            .and_then(|r| r.fields.get("unit"))
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    });
                snapshot.drafts.push(RawDraft {
                    key: RawKey {
                        record,
                        field,
                        item,
                    },
                    text,
                    version: snapshot.raw_version,
                    validity,
                    unit_context,
                    owner_generation: None,
                });
            }
        }
        let max_id = snapshot
            .all_ids()?
            .into_iter()
            .map(|id| id.0)
            .max()
            .unwrap_or(0);
        snapshot.next_id = 1000.max(increment(max_id, "record ID")?);
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn digest(&self) -> String {
        // No non-finite numbers can be represented in serde_json::Value.
        let bytes = serde_json::to_vec(self).expect("Stage1 snapshot is serializable");
        format!("{:x}", Sha256::digest(bytes))
    }

    fn all_ids(&self) -> Result<BTreeSet<RecordId>, String> {
        let mut ids = BTreeSet::new();
        for (key, record) in &self.records {
            if *key != record.id || record.id.0 == 0 || !ids.insert(record.id) {
                return Err("Record identity is zero, duplicated or mismatched".into());
            }
            for field in ["context", "observations", "inputs"] {
                if let Some(items) = record.fields.get(field) {
                    for item in items
                        .as_array()
                        .ok_or_else(|| format!("{field} must be an array"))?
                    {
                        let id = RecordId(
                            item.get("id")
                                .and_then(Value::as_u64)
                                .ok_or("Nested item needs an ID")?,
                        );
                        if id.0 == 0 || !ids.insert(id) {
                            return Err("Nested identity is zero or duplicated".into());
                        }
                        if item.get("scope").and_then(Value::as_u64) != Some(record.scope.0) {
                            return Err("Nested item must retain its owner's system scope".into());
                        }
                    }
                }
            }
        }
        Ok(ids)
    }

    /// Checks this app profile's structural integrity, never SysML conformance.
    /// Detached raw and creation journals are intentional after model Undo.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(extensions)=&self.extensions{extensions.validate()?;}
        if self.document_id.0.is_empty() || self.records.is_empty() {
            return Err("A document identity and at least one System are required".into());
        }
        let ids = self.all_ids()?;
        let extension_high=crate::types::validate_snapshot(self,&ids)?;
        if !self.records.values().any(|r| r.kind == RecordKind::System) {
            return Err("Document has no System".into());
        }
        for record in self.records.values() {
            if ["id", "type", "name", "scope"]
                .iter()
                .any(|key| record.fields.contains_key(*key))
            {
                return Err("Reserved record properties must use typed fields".into());
            }
            let system = self.record(record.scope, RecordKind::System)?;
            if system.scope != system.id
                || (record.kind == RecordKind::System && record.scope != record.id)
            {
                return Err("System scope must refer to itself".into());
            }
            for field in [
                "purpose",
                "outcome",
                "assessmentMethod",
                "conditions",
                "uncertainty",
                "interest",
                "text",
                "shortLabel",
            ] {
                if record.fields.get(field).is_some_and(|v| !v.is_string()) {
                    return Err(format!("{field} must be text"));
                }
            }
            if let Some(definition) = record_calculation(record)? {
                validate_calculation_owner(self, record, &definition)?;
                if record.fields.contains_key("_kernspanCalculationRawOwners") {
                    let owners = raw_owners(record)?;
                    if owners.expression > self.raw_version
                        || owners
                            .inputs
                            .values()
                            .any(|generation| *generation > self.raw_version)
                        || owners.inputs.keys().ne(definition.bindings.keys())
                    {
                        return Err("Invalid calculation raw ownership generations".into());
                    }
                }
            } else if record.fields.contains_key("_kernspanCalculationRawOwners") {
                return Err("Calculation raw owners require an explicit definition".into());
            }
            if let Some(source) = record.fields.get("_kernspanSysmlSource") {
                if record.kind != RecordKind::System {
                    return Err("Imported source archive belongs to System".into());
                }
                let source: crate::stage1_sysml::RetainedSysmlSource =
                    serde_json::from_value(source.clone())
                        .map_err(|error| format!("STAGE1_SOURCE_ARCHIVE: {error}"))?;
                if source.sha256 != format!("{:x}", Sha256::digest(source.source_utf8.as_bytes()))
                    || source.profile != crate::stage1_sysml::PROFILE
                    || source.parser_revision != crate::PARSER_REV
                {
                    return Err("Imported source archive identity/hash mismatch".into());
                }
                if source.source_utf8.len() > crate::boundary::MAX_SOURCE
                    || source.symbols.len() > 10_000
                    || source.logical_key.starts_with('/')
                    || source.logical_key.contains('\\')
                    || source
                        .logical_key
                        .split('/')
                        .any(|part| part.is_empty() || part == ".." || part == ".")
                    || !source.logical_key.ends_with(".sysml")
                {
                    return Err("Imported source archive key/size limit".into());
                }
                for symbol in &source.symbols {
                    if symbol.qualified_name.is_empty()
                        || symbol.qualified_name.iter().any(String::is_empty)
                        || symbol.declaration.byte_start >= symbol.declaration.byte_end
                        || source
                            .source_utf8
                            .get(symbol.declaration.byte_start..symbol.declaration.byte_end)
                            .is_none()
                        || symbol.value.as_ref().is_some_and(|span| {
                            span.byte_start > span.byte_end
                                || source
                                    .source_utf8
                                    .get(span.byte_start..span.byte_end)
                                    .is_none()
                        })
                    {
                        return Err("Imported source archive span/name bounds".into());
                    }
                }
            }
            match record.kind {
                RecordKind::Capability => {
                    if let Some(value) = record.fields.get("contributesTo") {
                        let links = value.as_array().ok_or("Mission links must be an array")?;
                        let mut unique = BTreeSet::new();
                        for link in links {
                            let id =
                                RecordId(link.as_u64().ok_or("Mission link must be a record ID")?);
                            if !unique.insert(id) {
                                return Err("Mission links must be unique".into());
                            }
                            let target = self.record(id, RecordKind::Mission)?;
                            same_scope(record, target)?;
                        }
                    }
                }
                RecordKind::Objective => {
                    if let Some(value) = record.fields.get("subject") {
                        if let Some(subject) = SubjectRef::from_value(value)? {
                            let (id, kind) = subject.parts();
                            same_scope(record, self.record(id, kind)?)?;
                        }
                    }
                }
                RecordKind::Measure => {
                    let objective = RecordId(
                        record
                            .fields
                            .get("forObjective")
                            .and_then(Value::as_u64)
                            .ok_or("Measure needs an Objective owner")?,
                    );
                    same_scope(record, self.record(objective, RecordKind::Objective)?)?;
                    if let Some(value) = record.fields.get("criterion") {
                        if !value.is_null() && value.as_f64().is_none_or(|n| !n.is_finite()) {
                            return Err(
                                "Accepted criterion must be absent or finite numeric".into()
                            );
                        }
                    }
                    if let Some(value) = record.fields.get("comparator") {
                        if !matches!(
                            value.as_str(),
                            Some("GreaterOrEqual" | "LessOrEqual" | "Equal")
                        ) {
                            return Err("Unsupported criterion comparator".into());
                        }
                    }
                    let quantity = record.fields.get("quantity").filter(|v| !v.is_null());
                    let unit = record.fields.get("unit").filter(|v| !v.is_null());
                    match (quantity, unit) {
                        (None, None) => {
                            if record.number("criterion").is_some() {
                                return Err("Numeric criterion needs quantity and unit".into());
                            }
                        }
                        (Some(q), Some(u))
                            if q.as_str().zip(u.as_str()).is_some_and(|(q, u)| {
                                unit_scale(u).is_some_and(|(dimension, _)| dimension == q)
                            }) => {}
                        _ => return Err("Incompatible quantity and unit".into()),
                    }
                }
                RecordKind::Expectation => {
                    if let Some(value) = record.fields.get("expressedBy") {
                        let id = RecordId(
                            value
                                .as_u64()
                                .ok_or("Expectation source must be a Stakeholder ID")?,
                        );
                        same_scope(record, self.record(id, RecordKind::Stakeholder)?)?;
                    }
                    if let Some(value) = record.fields.get("concernsSystem") {
                        if value.as_u64() != Some(record.scope.0) {
                            return Err("Expectation must concern its same System".into());
                        }
                    }
                }
                _ => {}
            }
        }
        let mut raw_keys = BTreeSet::new();
        let mut high_water = ids.iter().map(|id| id.0).max().unwrap_or(0);
        for definition in self.calculations()?.values() {
            high_water = high_water.max(
                calc::dependencies(definition)
                    .iter()
                    .map(|id| id.0)
                    .max()
                    .unwrap_or(0),
            );
        }
        for draft in &self.drafts {
            if draft.key.record.0 == 0
                || draft.key.field.is_empty()
                || !raw_keys.insert((draft.key.clone(), draft.owner_generation))
                || draft.version > self.raw_version
                || draft
                    .owner_generation
                    .is_some_and(|generation| generation > draft.version)
                || (!draft.key.field.starts_with("calc.") && draft.owner_generation.is_some())
            {
                return Err("Malformed or duplicate raw journal entry".into());
            }
            // Detached and unknown journals retain their declared semantics.
            // Known Measure fields have deterministic native classifiers, so a
            // file cannot promote incomplete text to a false valid state.
            if draft.key.item.is_none()
                && self
                    .records
                    .get(&draft.key.record)
                    .is_some_and(|record| record.kind == RecordKind::Measure)
            {
                let expected = match draft.key.field.as_str() {
                    "criterion" => Some(
                        numeric(&draft.text).map_or_else(|validity| validity, |_| Validity::Valid),
                    ),
                    "valueExpression" => Some(Validity::Unsupported),
                    field
                        if field.starts_with("calc.")
                            && self.calculation_generation(&draft.key)
                                == draft.owner_generation
                            && draft.owner_generation.is_some() =>
                    {
                        let definition = record_calculation(&self.records[&draft.key.record])?
                            .ok_or("Missing calculation owner")?;
                        calc::raw_validity(&definition, draft, &calc::Limits::default())
                    }
                    _ if self.calculation_config_field(&draft.key).is_some() => {
                        if draft.unit_context.is_some() {
                            return Err("Calculation configuration text has no unit context".into());
                        }
                        Some(Validity::Valid)
                    }
                    _ => None,
                };
                if expected.is_some_and(|validity| validity != draft.validity) {
                    return Err("Raw journal validity contradicts its typed field text".into());
                }
            }
            high_water = high_water
                .max(draft.key.record.0)
                .max(draft.key.item.map_or(0, |id| id.0));
        }
        for (id, draft) in &self.creation_drafts {
            if id.is_empty() || draft.owner.0 == 0 {
                return Err("Malformed creation journal entry".into());
            }
            high_water = high_water.max(draft.owner.0);
        }
        high_water=high_water.max(extension_high);
        if self.next_id <= high_water {
            return Err("Record allocator must exceed all retained identities".into());
        }
        Ok(())
    }

    pub fn calculations(&self) -> Result<BTreeMap<CalculationId, Definition>, String> {
        let mut definitions = BTreeMap::new();
        for record in self.records.values() {
            if let Some(definition) = record_calculation(record)? {
                definitions.insert(definition.id, definition);
            }
        }
        Ok(definitions)
    }

    /// Only the current slot generation enters evaluation. Older journal entries
    /// retain their exact semantic key and text for history and recovery.
    pub fn calculation_raw_state(&self) -> calc::CalculationRawState {
        let mut result: calc::CalculationRawState = BTreeMap::new();
        for draft in &self.drafts {
            if self.calculation_generation(&draft.key) == draft.owner_generation
                && draft.owner_generation.is_some()
            {
                result
                    .entry(CalculationId(draft.key.record.0))
                    .or_default()
                    .push(draft.clone());
            }
        }
        result
    }

    /// Identifies the retained configuration belonging to the current expression
    /// owner. Removing/recreating a calculation never reuses its former draft.
    pub fn calculation_config_id(&self, measure: RecordId) -> Option<String> {
        let generation = self.calculation_generation(&RawKey {
            record: measure,
            field: "calc.expression".into(),
            item: None,
        })?;
        Some(format!(
            "{CALCULATION_CONFIG_PREFIX}{}:{generation}",
            measure.0
        ))
    }

    /// Captures ownership of either a literal or an output-reference binding.
    /// Imported definitions without native ownership metadata use generation 0.
    pub fn calculation_binding_generation(&self, measure: RecordId, name: &str) -> Option<u64> {
        let record = self.records.get(&measure)?;
        if record.kind != RecordKind::Measure {
            return None;
        }
        let definition = record_calculation(record).ok()??;
        definition.bindings.get(name)?;
        Some(*raw_owners(record).ok()?.inputs.get(name).unwrap_or(&0))
    }

    fn calculation_config_field<'a>(&self, key: &'a RawKey) -> Option<(&'a str, &'a str)> {
        if key.item.is_some() || !key.field.starts_with(CALCULATION_CONFIG_PREFIX) {
            return None;
        }
        let (id, field) = key.field.rsplit_once(':')?;
        if !matches!(field, "name" | "decimal")
            || self.calculation_config_id(key.record).as_deref() != Some(id)
        {
            return None;
        }
        let draft = self.creation_drafts.get(id)?;
        (draft.owner == key.record
            && draft.kind == RecordKind::Measure
            && draft.fields.get("profile").map(String::as_str) == Some(CALCULATION_CONFIG_PROFILE))
        .then_some((id, field))
    }

    fn calculation_generation(&self, key: &RawKey) -> Option<u64> {
        if key.item.is_some() {
            return None;
        }
        let record = self.records.get(&key.record)?;
        if record.kind != RecordKind::Measure {
            return None;
        }
        let definition = record_calculation(record).ok()??;
        let owners = raw_owners(record).ok()?;
        if key.field == "calc.expression" {
            return Some(owners.expression);
        }
        let name = key.field.strip_prefix("calc.input.")?;
        if !matches!(definition.bindings.get(name), Some(Input::Literal(_))) {
            return None;
        }
        Some(*owners.inputs.get(name).unwrap_or(&0))
    }

    fn record(&self, id: RecordId, kind: RecordKind) -> Result<&Record, String> {
        let record = self.records.get(&id).ok_or("Missing record")?;
        if record.kind != kind {
            return Err("Wrong record kind".into());
        }
        Ok(record)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Freshness {
    pub document_id: DocumentId,
    pub instance_id: String,
    pub model_revision: u64,
    pub raw_version: u64,
}

#[derive(Clone, Debug)]
pub enum Command {
    Typed(crate::types::Action),
    SetAffiliation {
        stakeholder: RecordId,
        organization: RecordId,
        relationship: Option<AffiliationKind>,
    },
    AddContext {
        environment: RecordId,
        name: String,
        kind: ContextKind,
    },
    SetContextKind {
        environment: RecordId,
        item: RecordId,
        kind: ContextKind,
    },
    EditText {
        key: RawKey,
        text: String,
    },
    Add {
        kind: RecordKind,
        owner: RecordId,
        name: String,
    },
    CreateAndLinkMission {
        capability: RecordId,
        name: String,
    },
    ToggleMission {
        capability: RecordId,
        mission: RecordId,
    },
    SetSubject {
        objective: RecordId,
        subject: Option<SubjectRef>,
    },
    SetQuantity {
        measure: RecordId,
        quantity: String,
        unit: String,
    },
    SetForObjective {
        measure: RecordId,
        objective: RecordId,
    },
    PutCalculation {
        measure: RecordId,
        definition: Option<Definition>,
    },
    SetCalculationInputUnit {
        measure: RecordId,
        binding: String,
        unit: calc::UnitId,
    },
    PutCreationDraft {
        id: String,
        draft: CreationDraft,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditOutcome {
    pub accepted: bool,
    pub created: Option<RecordId>,
}

#[derive(Clone, Debug)]
struct TextState {
    text: String,
    unit_context: Option<String>,
}

#[derive(Clone,Debug,PartialEq)]
struct ModelState{records:BTreeMap<RecordId,Record>,extensions:Option<crate::extensions::Extensions>}
impl ModelState{fn capture(snapshot:&Snapshot)->Self{Self{records:snapshot.records.clone(),extensions:crate::types::accepted_extensions(&snapshot.extensions)}}}

#[derive(Clone, Debug)]
pub struct Editor {
    pub snapshot: Snapshot,
    pub selected: Option<RecordId>,
    instance_id: String,
    past: Vec<ModelState>,
    future: Vec<ModelState>,
    text_past: BTreeMap<(RawKey, Option<u64>), Vec<TextState>>,
    text_future: BTreeMap<(RawKey, Option<u64>), Vec<TextState>>,
    active_field: Option<RawKey>,
}

impl Editor {
    pub fn new(snapshot: Snapshot, instance_id: String) -> Result<Self, String> {
        snapshot.validate()?;
        if instance_id.is_empty() {
            return Err("Editor instance identity is required".into());
        }
        let selected = snapshot.records.keys().next().copied();
        Ok(Self {
            snapshot,
            selected,
            instance_id,
            past: Vec::new(),
            future: Vec::new(),
            text_past: BTreeMap::new(),
            text_future: BTreeMap::new(),
            active_field: None,
        })
    }

    pub fn freshness(&self) -> Freshness {
        Freshness {
            document_id: self.snapshot.document_id.clone(),
            instance_id: self.instance_id.clone(),
            model_revision: self.snapshot.model_revision,
            raw_version: self.snapshot.raw_version,
        }
    }

    fn check(&self, expected: &Freshness) -> Result<(), String> {
        if self.freshness() != *expected {
            return Err(
                "Stale document instance, model revision or raw version; current work preserved"
                    .into(),
            );
        }
        Ok(())
    }

    /// All refusals, including post-command integrity checks, are atomic.
    pub fn command(
        &mut self,
        expected: &Freshness,
        command: Command,
    ) -> Result<EditOutcome, String> {
        self.check(expected)?;
        let before = self.clone();
        match self.apply_command(command).and_then(|outcome| {
            self.snapshot.validate()?;
            Ok(outcome)
        }) {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }

    pub fn select(&mut self, id: RecordId) -> Result<(), String> {
        if !self.contains_id(id) {
            return Err("Selected record is unavailable; raw draft retained".into());
        }
        self.selected = Some(id);
        self.finish_field();
        Ok(())
    }

    pub fn finish_field(&mut self) {
        self.active_field = None;
    }
    pub fn can_text_history(&self, key: &RawKey, redo: bool) -> bool {
        if !self.field_available(key) { return false; }
        let generation = if key.field.starts_with("calc.") {
            self.snapshot.calculation_generation(key)
        } else { None };
        let history = if redo { &self.text_future } else { &self.text_past };
        history.get(&(key.clone(), generation)).is_some_and(|entries| !entries.is_empty())
    }
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
    pub fn raw(&self, key: &RawKey) -> Option<&RawDraft> {
        if key.field.starts_with(CALCULATION_CONFIG_PREFIX)
            && self.snapshot.calculation_config_field(key).is_none()
        {
            return None;
        }
        let generation = if key.field.starts_with("calc.") {
            self.snapshot.calculation_generation(key)?
        } else {
            0
        };
        self.snapshot.drafts.iter().find(|draft| {
            draft.key == *key
                && draft.owner_generation
                    == if key.field.starts_with("calc.") {
                        Some(generation)
                    } else {
                        None
                    }
        })
    }
    pub fn selection_resolved(&self) -> bool {
        self.selected.is_some_and(|id| self.contains_id(id))
    }

    pub fn display_text(&self, key: &RawKey) -> String {
        self.raw(key)
            .map(|draft| draft.text.clone())
            .unwrap_or_else(|| self.accepted_text(key))
    }

    fn contains_id(&self, id: RecordId) -> bool {
        self.snapshot.records.contains_key(&id)
            || self.snapshot.records.values().any(|record| {
                ["context", "observations", "inputs"].iter().any(|field| {
                    record
                        .fields
                        .get(*field)
                        .and_then(Value::as_array)
                        .is_some_and(|items| {
                            items
                                .iter()
                                .any(|item| item.get("id").and_then(Value::as_u64) == Some(id.0))
                        })
                })
            })
    }

    fn accepted_text(&self, key: &RawKey) -> String {
        if let Some((id, field)) = self.snapshot.calculation_config_field(key) {
            return self.snapshot.creation_drafts[id]
                .fields
                .get(field)
                .cloned()
                .unwrap_or_default();
        }
        let Some(record) = self.snapshot.records.get(&key.record) else {
            return String::new();
        };
        if key.field == "calc.expression" {
            return record_calculation(record)
                .ok()
                .flatten()
                .map(|definition| calc::format_expression(&definition.expression))
                .unwrap_or_default();
        }
        if let Some(name) = key.field.strip_prefix("calc.input.") {
            return record_calculation(record)
                .ok()
                .flatten()
                .and_then(|definition| definition.bindings.get(name).cloned())
                .and_then(|input| match input {
                    Input::Literal(literal) => Some(literal.decimal),
                    _ => None,
                })
                .unwrap_or_default();
        }
        if key.field == "name" {
            return record.name.clone();
        }
        if matches!(key.field.as_str(), "contextText" | "contextName") {
            return record
                .fields
                .get("context")
                .and_then(Value::as_array)
                .and_then(|items| {
                    items.iter().find(|item| {
                        item.get("id").and_then(Value::as_u64) == key.item.map(|id| id.0)
                    })
                })
                .and_then(|item| {
                    item.get(if key.field == "contextName" {
                        "name"
                    } else {
                        "text"
                    })
                })
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
        }
        match record.fields.get(&key.field) {
            Some(Value::String(text)) => text.clone(),
            Some(Value::Number(number)) => number.to_string(),
            _ => String::new(),
        }
    }

    fn apply_command(&mut self, command: Command) -> Result<EditOutcome, String> {
        let mut created = None;
        let accepted = match command {
            Command::Typed(action)=>{
                let before=ModelState::capture(&self.snapshot);
                let raw_before=self.snapshot.extensions.clone();let allocator_before=self.snapshot.next_id;
                match crate::types::apply(&mut self.snapshot,action)?{
                    crate::types::Change::Model=>{if before.records!=self.snapshot.records||before.extensions!=crate::types::accepted_extensions(&self.snapshot.extensions){self.past.push(before);self.future.clear();self.snapshot.model_revision=increment(self.snapshot.model_revision,"model revision")?;}},
                    crate::types::Change::Raw=>{if raw_before!=self.snapshot.extensions||allocator_before!=self.snapshot.next_id{self.snapshot.raw_version=increment(self.snapshot.raw_version,"raw version")?;}}
                }
                self.finish_field();true
            }

            Command::EditText { key, text } => {
                self.validate_key(&key)?;
                self.write_raw(&key, text.clone(), true, None)?;
                self.accept_raw(&key, &text)?
            }
            Command::PutCreationDraft { id, draft } => {
                if id.is_empty()
                    || !self.snapshot.records.contains_key(&draft.owner)
                    || draft.kind == RecordKind::System
                {
                    return Err(
                        "Creation draft needs an existing owner and supported category".into(),
                    );
                }
                if id.starts_with(CALCULATION_CONFIG_PREFIX)
                    || draft.fields.get("profile").map(String::as_str)
                        == Some(CALCULATION_CONFIG_PROFILE)
                {
                    let matches_owner = |candidate: &CreationDraft| {
                        candidate.owner == draft.owner
                            && candidate.kind == RecordKind::Measure
                            && candidate.fields.get("profile").map(String::as_str)
                                == Some(CALCULATION_CONFIG_PROFILE)
                    };
                    if self.snapshot.calculation_config_id(draft.owner).as_deref()
                        != Some(id.as_str())
                        || !matches_owner(&draft)
                        || self
                            .snapshot
                            .creation_drafts
                            .get(&id)
                            .is_some_and(|existing| !matches_owner(existing))
                    {
                        return Err("Calculation configuration owner/profile is unavailable or collides with retained data".into());
                    }
                }
                self.snapshot.creation_drafts.insert(id, draft);
                self.snapshot.raw_version = increment(self.snapshot.raw_version, "raw version")?;
                self.finish_field();
                true
            }
            command => {
                let before = self.snapshot.records.clone();
                match command {
                    Command::SetAffiliation {
                        stakeholder,
                        organization,
                        relationship,
                    } => {
                        let owner = self.snapshot.record(stakeholder, RecordKind::Stakeholder)?;
                        same_scope(
                            owner,
                            self.snapshot
                                .record(organization, RecordKind::Organization)?,
                        )?;
                        if owner
                            .fields
                            .get("affiliations")
                            .is_some_and(|v| !v.is_object())
                        {
                            return Err(
                                "Opaque affiliations are preserved; editing requires an object"
                                    .into(),
                            );
                        }
                        let entries = self
                            .snapshot
                            .records
                            .get_mut(&stakeholder)
                            .expect("checked owner")
                            .fields
                            .entry("affiliations".into())
                            .or_insert_with(|| json!({}))
                            .as_object_mut()
                            .expect("checked affiliations");
                        if let Some(kind) = relationship {
                            entries.insert(organization.0.to_string(), json!(kind.label()));
                        } else {
                            entries.remove(&organization.0.to_string());
                        }
                    }
                    Command::AddContext {
                        environment,
                        name,
                        kind,
                    } => {
                        let owner = self.snapshot.record(environment, RecordKind::Environment)?;
                        let scope = owner.scope;
                        let name = name.trim().to_owned();
                        if name.is_empty() {
                            return Err("Name the context; creation draft retained".into());
                        }
                        if owner.fields.get("context").is_some_and(|v| !v.is_array()) {
                            return Err(
                                "Opaque context is preserved; editing requires an array".into()
                            );
                        }
                        let id = RecordId(self.snapshot.next_id);
                        self.snapshot.next_id =
                            increment(self.snapshot.next_id, "context identity")?;
                        self.snapshot.records.get_mut(&environment).expect("checked Environment")
                            .fields.entry("context".into()).or_insert_with(|| json!([]))
                            .as_array_mut().expect("checked context").push(json!({"id":id.0,"type":"Context","name":name,"scope":scope.0,"kind":kind.label(),"text":""}));
                        created = Some(id);
                    }
                    Command::SetContextKind {
                        environment,
                        item,
                        kind,
                    } => {
                        self.validate_context(environment, item)?;
                        let context = self
                            .snapshot
                            .records
                            .get_mut(&environment)
                            .expect("checked Environment")
                            .fields
                            .get_mut("context")
                            .and_then(Value::as_array_mut)
                            .expect("checked context")
                            .iter_mut()
                            .find(|v| v["id"].as_u64() == Some(item.0))
                            .expect("checked item");
                        context
                            .as_object_mut()
                            .expect("checked object")
                            .insert("kind".into(), json!(kind.label()));
                    }
                    Command::Add { kind, owner, name } => {
                        created = Some(self.add(kind, owner, name)?);
                    }
                    Command::CreateAndLinkMission { capability, name } => {
                        self.snapshot.record(capability, RecordKind::Capability)?;
                        let id = self.add(RecordKind::Mission, capability, name)?;
                        self.toggle_mission(capability, id)?;
                        created = Some(id);
                    }
                    Command::ToggleMission {
                        capability,
                        mission,
                    } => {
                        self.toggle_mission(capability, mission)?;
                    }
                    Command::SetSubject { objective, subject } => {
                        let record = self.snapshot.record(objective, RecordKind::Objective)?;
                        if let Some(subject) = subject {
                            let (id, kind) = subject.parts();
                            same_scope(record, self.snapshot.record(id, kind)?)?;
                        }
                        self.snapshot
                            .records
                            .get_mut(&objective)
                            .expect("checked objective")
                            .fields
                            .insert(
                                "subject".into(),
                                subject.map_or(Value::Null, SubjectRef::as_value),
                            );
                    }
                    Command::SetQuantity {
                        measure,
                        quantity,
                        unit,
                    } => {
                        self.set_quantity(measure, quantity, unit)?;
                    }
                    Command::PutCalculation {
                        measure,
                        definition,
                    } => {
                        self.put_calculation(measure, definition)?;
                    }
                    Command::SetCalculationInputUnit {
                        measure,
                        binding,
                        unit,
                    } => {
                        self.set_calculation_input_unit(measure, &binding, unit)?;
                    }
                    Command::SetForObjective { measure, objective } => {
                        let record = self.snapshot.record(measure, RecordKind::Measure)?;
                        let target = self.snapshot.record(objective, RecordKind::Objective)?;
                        same_scope(record, target)?;
                        self.snapshot
                            .records
                            .get_mut(&measure)
                            .expect("checked measure")
                            .fields
                            .insert("forObjective".into(), json!(objective.0));
                    }
                    _ => unreachable!("text and creation commands handled above"),
                }
                self.commit_records(before, None)?;
                self.finish_field();
                true
            }
        };
        Ok(EditOutcome { accepted, created })
    }

    /// Availability is current semantic ownership, including draft generations.
    /// Retained historical text may still exist when a field is unavailable.
    pub fn field_available(&self, key: &RawKey) -> bool {
        self.validate_key(key).is_ok()
    }

    /// Stable ownership token for the active calculation slot, independent of edits.
    /// Ordinary fields use None; callers must also check field_available.
    pub fn field_generation(&self, key: &RawKey) -> Option<u64> {
        self.snapshot.calculation_generation(key)
    }

    fn validate_context(&self, environment: RecordId, item: RecordId) -> Result<(), String> {
        let owner = self.snapshot.record(environment, RecordKind::Environment)?;
        let context = owner
            .fields
            .get("context")
            .and_then(Value::as_array)
            .and_then(|items| items.iter().find(|v| v["id"].as_u64() == Some(item.0)))
            .ok_or("Context item is missing")?;
        if context["type"] != "Context"
            || context["scope"].as_u64() != Some(owner.scope.0)
            || !context["name"].is_string()
            || !context["text"].is_string()
            || !matches!(
                context["kind"].as_str(),
                Some("Description" | "Assumption" | "AcceptedConstraint")
            )
        {
            return Err("Opaque context is preserved; editing requires a supported Context".into());
        }
        Ok(())
    }

    fn validate_key(&self, key: &RawKey) -> Result<(), String> {
        let record = self
            .snapshot
            .records
            .get(&key.record)
            .ok_or("Record is unavailable; detached raw draft retained")?;
        if matches!(key.field.as_str(), "contextText" | "contextName") {
            return self.validate_context(
                key.record,
                key.item.ok_or("Context field needs an item identity")?,
            );
        }
        if key.item.is_some() {
            return Err("This field has no nested item binding".into());
        }
        if key.field.starts_with(CALCULATION_CONFIG_PREFIX) {
            return if self.snapshot.calculation_config_field(key).is_some() {
                Ok(())
            } else {
                Err(
                    "Calculation configuration field is unavailable; retained draft is unchanged"
                        .into(),
                )
            };
        }
        if key.field.starts_with("calc.") {
            if self.snapshot.calculation_generation(key).is_some() {
                return Ok(());
            }
            return Err("Calculation field is unavailable; detached raw input is retained".into());
        }
        let allowed = key.field == "name"
            || match record.kind {
                RecordKind::System => key.field == "purpose",
                RecordKind::Objective => key.field == "outcome",
                RecordKind::Measure => matches!(
                    key.field.as_str(),
                    "criterion"
                        | "valueExpression"
                        | "assessmentMethod"
                        | "conditions"
                        | "uncertainty"
                ),
                RecordKind::Stakeholder => key.field == "interest",
                RecordKind::Expectation => key.field == "text",
                _ => false,
            };
        if allowed {
            Ok(())
        } else {
            Err(
                "Field does not belong to this record type or has no supported typed operation"
                    .into(),
            )
        }
    }

    fn write_raw(
        &mut self,
        key: &RawKey,
        text: String,
        push: bool,
        unit_context: Option<String>,
    ) -> Result<(), String> {
        let record = self
            .snapshot
            .records
            .get(&key.record)
            .ok_or("Record is unavailable; raw draft retained")?;
        let unit = if let Some(name) = key.field.strip_prefix("calc.input.") {
            record_calculation(record)?
                .and_then(|definition| definition.bindings.get(name).cloned())
                .and_then(|input| match input {
                    Input::Literal(literal) => {
                        Some(calc::unit_key(literal.quantity.unit).to_owned())
                    }
                    _ => None,
                })
        } else if key.field == "calc.expression" || key.field.starts_with(CALCULATION_CONFIG_PREFIX)
        {
            None
        } else {
            record
                .fields
                .get("unit")
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        let owner_generation = if key.field.starts_with("calc.") {
            self.snapshot.calculation_generation(key)
        } else {
            None
        };
        if push {
            let previous = self
                .raw(key)
                .map(|draft| TextState {
                    text: draft.text.clone(),
                    unit_context: draft.unit_context.clone().or_else(|| unit.clone()),
                })
                .unwrap_or_else(|| TextState {
                    text: self.accepted_text(key),
                    unit_context: unit.clone(),
                });
            self.text_past
                .entry((key.clone(), owner_generation))
                .or_default()
                .push(previous);
            self.text_future.remove(&(key.clone(), owner_generation));
        }
        self.snapshot.raw_version = increment(self.snapshot.raw_version, "raw version")?;
        let draft = RawDraft {
            key: key.clone(),
            text,
            version: self.snapshot.raw_version,
            validity: Validity::Invalid,
            unit_context: unit_context.or(unit),
            owner_generation,
        };
        if let Some(index) = self
            .snapshot
            .drafts
            .iter()
            .position(|draft| draft.key == *key && draft.owner_generation == owner_generation)
        {
            self.snapshot.drafts[index] = draft;
        } else {
            self.snapshot.drafts.push(draft);
        }
        Ok(())
    }

    fn accept_raw(&mut self, key: &RawKey, text: &str) -> Result<bool, String> {
        self.validate_key(key)?;
        if let Some((id, field)) = self.snapshot.calculation_config_field(key) {
            let (id, field) = (id.to_owned(), field.to_owned());
            self.snapshot
                .creation_drafts
                .get_mut(&id)
                .expect("validated configuration")
                .fields
                .insert(field, text.to_owned());
            self.snapshot
                .drafts
                .iter_mut()
                .find(|draft| draft.key == *key && draft.owner_generation.is_none())
                .expect("configuration raw written first")
                .validity = Validity::Valid;
            return Ok(true);
        }
        if key.field.starts_with("calc.") {
            return self.accept_calculation_raw(key, text);
        }
        let (value, validity) = if key.field == "valueExpression" {
            (None, Validity::Unsupported)
        } else if key.field == "criterion" {
            match numeric(text) {
                Ok(value) => (
                    Some(value.map_or(Value::Null, |number| json!(number))),
                    Validity::Valid,
                ),
                Err(validity) => (None, validity),
            }
        } else {
            (Some(json!(text)), Validity::Valid)
        };
        self.snapshot
            .drafts
            .iter_mut()
            .find(|draft| draft.key == *key)
            .expect("raw written first")
            .validity = validity;
        let Some(value) = value else {
            return Ok(false);
        };
        let before = self.snapshot.records.clone();
        let record = self
            .snapshot
            .records
            .get_mut(&key.record)
            .expect("validated record");
        if key.field == "name" {
            record.name = text.to_owned();
        } else if matches!(key.field.as_str(), "contextText" | "contextName") {
            let item = record
                .fields
                .get_mut("context")
                .and_then(Value::as_array_mut)
                .and_then(|items| {
                    items.iter_mut().find(|item| {
                        item.get("id").and_then(Value::as_u64) == key.item.map(|id| id.0)
                    })
                })
                .expect("validated context item");
            item.as_object_mut().expect("context object").insert(
                if key.field == "contextName" {
                    "name"
                } else {
                    "text"
                }
                .into(),
                value,
            );
        } else {
            record.fields.insert(key.field.clone(), value);
        }
        self.commit_records(before, Some(key.clone()))?;
        Ok(true)
    }

    fn commit_records(
        &mut self,
        before: BTreeMap<RecordId, Record>,
        field: Option<RawKey>,
    ) -> Result<(), String> {
        if before != self.snapshot.records {
            if field.is_none() || field != self.active_field {
                self.past.push(ModelState{records:before,extensions:crate::types::accepted_extensions(&self.snapshot.extensions)});
            }
            self.future.clear();
            self.snapshot.model_revision =
                increment(self.snapshot.model_revision, "model revision")?;
            self.active_field = field;
        }
        Ok(())
    }

    pub fn calculation_stamp(&self) -> calc::DocumentStamp {
        calc::DocumentStamp {
            document_id: self.snapshot.document_id.0.clone(),
            instance_id: self.instance_id.clone(),
            model_revision: self.snapshot.model_revision,
            raw_version: self.snapshot.raw_version,
            snapshot_hash: self.snapshot.digest(),
        }
    }

    pub fn evaluate_calculations(&self) -> Result<calc::EvaluationReceipt, String> {
        Ok(calc::evaluate(
            &self.snapshot.calculations()?,
            &self.snapshot.calculation_raw_state(),
            &self.calculation_stamp(),
            &calc::Limits::default(),
        ))
    }

    fn set_calculation_input_unit(
        &mut self,
        measure: RecordId,
        binding: &str,
        unit: calc::UnitId,
    ) -> Result<(), String> {
        let record = self.snapshot.record(measure, RecordKind::Measure)?;
        let mut definition = record_calculation(record)?.ok_or("Missing calculation")?;
        let Some(Input::Literal(previous)) = definition.bindings.get(binding) else {
            return Err("Input is not an authored literal".into());
        };
        if previous.quantity.unit == unit {
            return Ok(());
        }
        let key = RawKey {
            record: measure,
            field: format!("calc.input.{binding}"),
            item: None,
        };
        let raw = self.raw(&key).cloned();
        if let Some(raw) = &raw {
            calc::classify_raw(&definition, raw, &calc::Limits::default()).map_err(|error| {
                format!("Correct the current input before changing its unit: {error:?}")
            })?;
        }
        let converted =
            calc::convert_literal(previous, unit).map_err(|error| format!("{error:?}"))?;
        let converted_raw = raw
            .as_ref()
            .map(|raw| {
                calc::convert_literal(
                    &calc::Literal {
                        decimal: raw.text.clone(),
                        quantity: previous.quantity,
                    },
                    unit,
                )
                .map_err(|error| format!("{error:?}"))
            })
            .transpose()?;
        if let Some(raw) = &raw {
            let history_key = (key.clone(), raw.owner_generation);
            self.text_past
                .entry(history_key.clone())
                .or_default()
                .push(TextState {
                    text: raw.text.clone(),
                    unit_context: raw
                        .unit_context
                        .clone()
                        .or_else(|| Some(calc::unit_key(previous.quantity.unit).into())),
                });
            self.text_future.remove(&history_key);
        }
        definition
            .bindings
            .insert(binding.to_owned(), Input::Literal(converted));
        self.snapshot
            .records
            .get_mut(&measure)
            .expect("checked owner")
            .fields
            .insert(
                "calculation".into(),
                serde_json::to_value(definition).map_err(|error| error.to_string())?,
            );
        if let Some(literal) = converted_raw {
            self.write_raw(
                &key,
                literal.decimal.clone(),
                false,
                Some(calc::unit_key(unit).into()),
            )?;
            self.accept_calculation_raw(&key, &literal.decimal)?;
        }
        Ok(())
    }

    fn convert_calculation_current_raw(
        &mut self,
        measure: RecordId,
        binding: &str,
        old: &calc::Literal,
        new: &calc::Literal,
    ) -> Result<(), String> {
        let key = RawKey {
            record: measure,
            field: format!("calc.input.{binding}"),
            item: None,
        };
        let Some(draft) = self.raw(&key).cloned() else {
            return Ok(());
        };
        if draft
            .unit_context
            .as_deref()
            .is_some_and(|context| context != calc::unit_key(old.quantity.unit))
        {
            return Ok(());
        }
        let literal = calc::Literal {
            decimal: draft.text.clone(),
            quantity: old.quantity,
        };
        if calc::validate_literal(&literal).is_err() {
            return Ok(());
        }
        // A unit transaction must restore the exact accepted source spelling
        // when no subsequent raw edit diverged. Independent live input is
        // converted physically and remains visibly different from acceptance.
        let converted = if draft.text == old.decimal {
            new.clone()
        } else {
            calc::convert_literal(&literal, new.quantity.unit)
                .map_err(|error| format!("{error:?}"))?
        };
        self.snapshot.raw_version = increment(self.snapshot.raw_version, "raw version")?;
        let current = self
            .snapshot
            .drafts
            .iter_mut()
            .find(|value| value.key == key && value.owner_generation == draft.owner_generation)
            .expect("active journal");
        current.text = converted.decimal;
        current.unit_context = Some(calc::unit_key(new.quantity.unit).into());
        current.version = self.snapshot.raw_version;
        Ok(())
    }

    fn put_calculation(
        &mut self,
        measure: RecordId,
        definition: Option<Definition>,
    ) -> Result<(), String> {
        let record = self.snapshot.record(measure, RecordKind::Measure)?;
        let previous = record_calculation(record)?;
        if previous == definition {
            return Ok(());
        }
        if let Some(definition) = &definition {
            validate_calculation_owner(&self.snapshot, record, definition)?;
        }
        let previous_owners = raw_owners(record)?;
        if let Some(definition) = &definition {
            if let Some(id) = calc::dependencies(definition).iter().map(|id| id.0).max() {
                self.snapshot.next_id = self
                    .snapshot
                    .next_id
                    .max(increment(id, "calculation output identity")?);
            }
        }
        self.snapshot.raw_version = increment(self.snapshot.raw_version, "raw version")?;
        let generation = self.snapshot.raw_version;
        let record = self
            .snapshot
            .records
            .get_mut(&measure)
            .expect("validated measure");
        if let Some(definition) = definition {
            let expression = if previous
                .as_ref()
                .is_some_and(|old| old.expression == definition.expression)
            {
                previous_owners.expression
            } else {
                generation
            };
            let inputs = definition
                .bindings
                .iter()
                .map(|(name, input)| {
                    let epoch = if previous.as_ref().and_then(|old| old.bindings.get(name))
                        == Some(input)
                    {
                        *previous_owners.inputs.get(name).unwrap_or(&0)
                    } else {
                        generation
                    };
                    (name.clone(), epoch)
                })
                .collect();
            record.fields.insert(
                "calculation".into(),
                serde_json::to_value(definition).map_err(|e| e.to_string())?,
            );
            record.fields.insert(
                "_kernspanCalculationRawOwners".into(),
                serde_json::to_value(CalculationRawOwners { expression, inputs })
                    .map_err(|e| e.to_string())?,
            );
        } else {
            record.fields.remove("calculation");
            record.fields.remove("_kernspanCalculationRawOwners");
        }
        Ok(())
    }

    fn accept_calculation_raw(&mut self, key: &RawKey, text: &str) -> Result<bool, String> {
        let mut definition = record_calculation(
            self.snapshot
                .records
                .get(&key.record)
                .ok_or("Missing Measure")?,
        )?
        .ok_or("Missing calculation")?;
        let generation = self.snapshot.calculation_generation(key);
        let draft = self
            .snapshot
            .drafts
            .iter()
            .find(|d| d.key == *key && d.owner_generation == generation)
            .ok_or("Missing active calculation draft")?
            .clone();
        let validity = calc::raw_validity(&definition, &draft, &calc::Limits::default())
            .unwrap_or(Validity::Invalid);
        self.snapshot
            .drafts
            .iter_mut()
            .find(|d| d.key == *key && d.owner_generation == generation)
            .expect("checked draft")
            .validity = validity;
        if validity != Validity::Valid {
            return Ok(false);
        }
        if key.field == "calc.expression" {
            definition.expression =
                calc::parse(text, &calc::Limits::default()).map_err(|e| format!("{e:?}"))?;
        } else if let Some(name) = key.field.strip_prefix("calc.input.") {
            let Some(Input::Literal(literal)) = definition.bindings.get_mut(name) else {
                return Err("Not a literal input".into());
            };
            if draft.unit_context.as_deref() != Some(calc::unit_key(literal.quantity.unit)) {
                return Ok(false);
            }
            literal.decimal = text.to_owned();
            calc::validate_literal(literal).map_err(|e| format!("{e:?}"))?;
        }
        let before = self.snapshot.records.clone();
        self.snapshot
            .records
            .get_mut(&key.record)
            .expect("checked owner")
            .fields
            .insert(
                "calculation".into(),
                serde_json::to_value(definition).map_err(|e| e.to_string())?,
            );
        self.commit_records(before, Some(key.clone()))?;
        Ok(true)
    }

    fn add(&mut self, kind: RecordKind, owner: RecordId, name: String) -> Result<RecordId, String> {
        if kind == RecordKind::System {
            return Err(
                "New System is a document workflow; no Add System operation is bound".into(),
            );
        }
        let parent = self.snapshot.records.get(&owner).ok_or("Missing owner")?;
        let scope = parent.scope;
        if kind == RecordKind::Measure && parent.kind != RecordKind::Objective {
            return Err("Measure owner must be an Objective".into());
        }
        if kind == RecordKind::Expectation && parent.kind != RecordKind::Stakeholder {
            return Err("Expectation owner must be a Stakeholder".into());
        }
        let name = name.trim().to_owned();
        if name.is_empty() {
            return Err("Name the new item; creation draft retained".into());
        }
        let id = RecordId(self.snapshot.next_id);
        self.snapshot.next_id = increment(self.snapshot.next_id, "record ID")?;
        let fields = match kind {
            RecordKind::Capability => BTreeMap::from([("contributesTo".into(), json!([]))]),
            RecordKind::Objective => BTreeMap::from([("subject".into(), Value::Null)]),
            RecordKind::Measure => BTreeMap::from([
                ("forObjective".into(), json!(owner.0)),
                ("quantity".into(), Value::Null),
                ("unit".into(), Value::Null),
                ("criterion".into(), Value::Null),
                ("comparator".into(), json!("GreaterOrEqual")),
                ("assessmentMethod".into(), json!("")),
                ("observations".into(), json!([])),
            ]),
            RecordKind::Stakeholder => BTreeMap::from([
                ("affiliations".into(), json!({})),
                ("actors".into(), json!([])),
            ]),
            RecordKind::Environment => {
                BTreeMap::from([("context".into(), json!([])), ("inputs".into(), json!([]))])
            }
            RecordKind::Expectation => BTreeMap::from([
                ("expressedBy".into(), json!(owner.0)),
                ("concernsSystem".into(), json!(scope.0)),
            ]),
            _ => BTreeMap::new(),
        };
        self.snapshot.records.insert(
            id,
            Record {
                id,
                kind,
                name,
                scope,
                fields,
            },
        );
        Ok(id)
    }

    fn toggle_mission(&mut self, capability: RecordId, mission: RecordId) -> Result<(), String> {
        let cap = self.snapshot.record(capability, RecordKind::Capability)?;
        let target = self.snapshot.record(mission, RecordKind::Mission)?;
        same_scope(cap, target)?;
        let mut links = cap.links();
        if let Some(index) = links.iter().position(|id| *id == mission) {
            links.remove(index);
        } else {
            links.push(mission);
        }
        self.snapshot
            .records
            .get_mut(&capability)
            .expect("checked capability")
            .fields
            .insert("contributesTo".into(), json!(links));
        Ok(())
    }

    fn set_quantity(
        &mut self,
        measure: RecordId,
        quantity: String,
        unit: String,
    ) -> Result<(), String> {
        let record = self.snapshot.record(measure, RecordKind::Measure)?;
        let key = RawKey {
            record: measure,
            field: "criterion".into(),
            item: None,
        };
        let old_unit = record
            .fields
            .get("unit")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let criterion = record
            .fields
            .get("criterion")
            .filter(|v| !v.is_null())
            .cloned();
        let initial_metadata = old_unit.is_none()
            && record.fields.get("quantity").is_none_or(Value::is_null)
            && criterion.is_none();
        if !initial_metadata
            && self
                .raw(&key)
                .is_some_and(|draft| draft.validity != Validity::Valid)
        {
            return Err("Resolve criterion draft before changing units; raw retained".into());
        }
        let (dimension, _) = unit_scale(&unit).ok_or("Unsupported unit")?;
        if dimension != quantity {
            return Err("Incompatible quantity and unit".into());
        }
        if criterion.is_some() && record.text("quantity") != quantity {
            return Err("Resolve criterion before changing dimension".into());
        }
        let converted = if let Some(criterion) = criterion {
            let text = convert_units(
                &criterion.to_string(),
                old_unit.as_deref().ok_or("Criterion has no unit")?,
                &unit,
            )?;
            let number = numeric(&text)
                .map_err(|_| "Unit conversion outside supported numeric range")?
                .ok_or("Missing converted criterion")?;
            Some(json!(number))
        } else {
            None
        };
        let record = self
            .snapshot
            .records
            .get_mut(&measure)
            .expect("checked measure");
        record.fields.insert("quantity".into(), json!(quantity));
        record.fields.insert("unit".into(), json!(unit));
        if let Some(value) = converted {
            record.fields.insert("criterion".into(), value);
        }
        if let Some(old) = old_unit {
            self.convert_current_raw(measure, &old, &unit)?;
        } else if initial_metadata {
            if let Some(index) = self
                .snapshot
                .drafts
                .iter()
                .position(|draft| draft.key == key)
            {
                // The first metadata selection supplies context only. It must
                // not reinterpret or accept any retained numeric draft.
                self.snapshot.raw_version = increment(self.snapshot.raw_version, "raw version")?;
                self.snapshot.drafts[index].unit_context = Some(unit);
                self.snapshot.drafts[index].version = self.snapshot.raw_version;
            }
        }
        Ok(())
    }

    fn convert_current_raw(
        &mut self,
        measure: RecordId,
        old: &str,
        new: &str,
    ) -> Result<(), String> {
        let key = RawKey {
            record: measure,
            field: "criterion".into(),
            item: None,
        };
        let Some(index) = self
            .snapshot
            .drafts
            .iter()
            .position(|draft| draft.key == key && draft.validity == Validity::Valid)
        else {
            return Ok(());
        };
        if unit_scale(old).is_none() || unit_scale(new).is_none() {
            return Ok(());
        }
        let text = self.snapshot.drafts[index].text.clone();
        let converted = if text.trim().is_empty() {
            text
        } else {
            convert_units(&text, old, new)?
        };
        self.snapshot.raw_version = increment(self.snapshot.raw_version, "raw version")?;
        let draft = &mut self.snapshot.drafts[index];
        draft.text = converted;
        draft.unit_context = Some(new.into());
        draft.version = self.snapshot.raw_version;
        Ok(())
    }

    pub fn undo_model(&mut self, expected: &Freshness) -> Result<bool, String> {
        self.model_history(expected, false)
    }
    pub fn redo_model(&mut self, expected: &Freshness) -> Result<bool, String> {
        self.model_history(expected, true)
    }

    fn model_history(&mut self, expected: &Freshness, redo: bool) -> Result<bool, String> {
        self.check(expected)?;
        let before = self.clone();
        let result = (|| {
            let records = if redo {
                self.future.pop()
            } else {
                self.past.pop()
            };
            let Some(records) = records else {
                return Ok(false);
            };
            let previous_extensions=crate::types::accepted_extensions(&self.snapshot.extensions);
            let previous = std::mem::replace(&mut self.snapshot.records, records.records);
            crate::types::restore_accepted(&mut self.snapshot.extensions,records.extensions);
            let previous_state=ModelState{records:previous.clone(),extensions:previous_extensions};
            if redo {self.past.push(previous_state);}else{self.future.push(previous_state);}
            self.snapshot.model_revision =
                increment(self.snapshot.model_revision, "model revision")?;
            let conversions: Vec<_> = self
                .snapshot
                .records
                .iter()
                .filter_map(|(id, record)| {
                    if record.kind != RecordKind::Measure {
                        return None;
                    }
                    let old = previous.get(id)?.fields.get("unit")?.as_str()?;
                    let new = record.fields.get("unit")?.as_str()?;
                    (old != new).then(|| (*id, old.to_owned(), new.to_owned()))
                })
                .collect();
            for (id, old, new) in conversions {
                self.convert_current_raw(id, &old, &new)?;
            }
            let mut calculation_units = Vec::new();
            for (id, record) in &self.snapshot.records {
                let Some(new_definition) = record_calculation(record)? else {
                    continue;
                };
                let Some(old_record) = previous.get(id) else {
                    continue;
                };
                let Some(old_definition) = record_calculation(old_record)? else {
                    continue;
                };
                let old_owners = raw_owners(old_record)?;
                let new_owners = raw_owners(record)?;
                for (name, input) in &new_definition.bindings {
                    if let (Input::Literal(new), Some(Input::Literal(old))) =
                        (input, old_definition.bindings.get(name))
                    {
                        if old.quantity.unit != new.quantity.unit
                            && old_owners.inputs.get(name) == new_owners.inputs.get(name)
                        {
                            calculation_units.push((*id, name.clone(), old.clone(), new.clone()));
                        }
                    }
                }
            }
            for (id, name, old, new) in calculation_units {
                self.convert_calculation_current_raw(id, &name, &old, &new)?;
            }
            self.finish_field();
            self.snapshot.validate()?;
            Ok(true)
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    pub fn undo_text(&mut self, expected: &Freshness, key: &RawKey) -> Result<bool, String> {
        self.text_history(expected, key, false)
    }
    pub fn redo_text(&mut self, expected: &Freshness, key: &RawKey) -> Result<bool, String> {
        self.text_history(expected, key, true)
    }

    fn text_history(
        &mut self,
        expected: &Freshness,
        key: &RawKey,
        redo: bool,
    ) -> Result<bool, String> {
        self.check(expected)?;
        let before = self.clone();
        let result = (|| {
            self.validate_key(key)?;
            let history_key = (
                key.clone(),
                if key.field.starts_with("calc.") {
                    self.snapshot.calculation_generation(key)
                } else {
                    None
                },
            );
            let previous = if redo {
                self.text_future.get_mut(&history_key).and_then(Vec::pop)
            } else {
                self.text_past.get_mut(&history_key).and_then(Vec::pop)
            };
            let Some(mut previous) = previous else {
                return Ok(false);
            };
            let known_unit = if let Some(name) = key.field.strip_prefix("calc.input.") {
                record_calculation(&self.snapshot.records[&key.record])?
                    .and_then(|definition| definition.bindings.get(name).cloned())
                    .and_then(|input| match input {
                        Input::Literal(literal) => {
                            Some(calc::unit_key(literal.quantity.unit).to_owned())
                        }
                        _ => None,
                    })
            } else if key.field.starts_with("calc.")
                || key.field.starts_with(CALCULATION_CONFIG_PREFIX)
            {
                None
            } else {
                self.snapshot
                    .records
                    .get(&key.record)
                    .and_then(|record| record.fields.get("unit"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            };
            let current = self
                .raw(key)
                .map(|draft| TextState {
                    text: draft.text.clone(),
                    unit_context: draft.unit_context.clone().or_else(|| known_unit.clone()),
                })
                .unwrap_or_else(|| TextState {
                    text: self.accepted_text(key),
                    unit_context: None,
                });
            if redo {
                self.text_past
                    .entry(history_key.clone())
                    .or_default()
                    .push(current);
            } else {
                self.text_future
                    .entry(history_key)
                    .or_default()
                    .push(current);
            }
            if key.field == "criterion" {
                let current_unit = self
                    .snapshot
                    .records
                    .get(&key.record)
                    .and_then(|record| record.fields.get("unit"))
                    .and_then(Value::as_str);
                if let Some((old, new)) = previous.unit_context.as_deref().zip(current_unit) {
                    if old != new && matches!(numeric(&previous.text), Ok(Some(_))) {
                        // A finite old-unit value may not be representable in the
                        // current unit. Refuse atomically, including the history
                        // pop/push, rather than accepting its unconverted digits.
                        previous.text = convert_units(&previous.text, old, new)?;
                        previous.unit_context = Some(new.into());
                    }
                }
            }
            if let Some(name) = key.field.strip_prefix("calc.input.") {
                let definition = record_calculation(&self.snapshot.records[&key.record])?
                    .ok_or("Missing calculation")?;
                if let Some(Input::Literal(current)) = definition.bindings.get(name) {
                    if let Some(old_unit) = previous
                        .unit_context
                        .as_deref()
                        .and_then(calc::unit_from_key)
                    {
                        if old_unit != current.quantity.unit {
                            let literal = calc::Literal {
                                decimal: previous.text.clone(),
                                quantity: calc::QuantityType {
                                    kind: calc::unit_kind(old_unit),
                                    unit: old_unit,
                                },
                            };
                            if calc::validate_literal(&literal).is_ok() {
                                previous.text =
                                    calc::convert_literal(&literal, current.quantity.unit)
                                        .map_err(|error| format!("{error:?}"))?
                                        .decimal;
                                previous.unit_context =
                                    Some(calc::unit_key(current.quantity.unit).into());
                            }
                        }
                    }
                }
            }
            self.write_raw(key, previous.text.clone(), false, previous.unit_context)?;
            self.accept_raw(key, &previous.text)?;
            self.snapshot.validate()?;
            Ok(true)
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }
}

fn record_calculation(record: &Record) -> Result<Option<Definition>, String> {
    record
        .fields
        .get("calculation")
        .map(|value| {
            serde_json::from_value(value.clone())
                .map_err(|error| format!("STAGE1_CALCULATION_PAYLOAD: {error}"))
        })
        .transpose()
}
fn raw_owners(record: &Record) -> Result<CalculationRawOwners, String> {
    record
        .fields
        .get("_kernspanCalculationRawOwners")
        .map_or_else(
            || Ok(CalculationRawOwners::default()),
            |value| {
                serde_json::from_value(value.clone())
                    .map_err(|error| format!("STAGE1_CALC_RAW_OWNERS: {error}"))
            },
        )
}
fn validate_calculation_owner(
    snapshot: &Snapshot,
    record: &Record,
    definition: &Definition,
) -> Result<(), String> {
    if record.kind != RecordKind::Measure
        || definition.id.0 != record.id.0
        || definition.owner != record.id
    {
        return Err("Calculation identity must equal its owning Measure".into());
    }
    calc::validate(definition, &calc::Limits::default()).map_err(|error| format!("{error:?}"))?;
    for input in definition.bindings.values() {
        if let Input::Output(id) = input {
            // Missing dependencies remain a visible evaluation error. Existing
            // endpoints still cannot cross System scope or masquerade as criteria.
            if let Some(target) = snapshot.records.get(&RecordId(id.0)) {
                if target.kind != RecordKind::Measure {
                    return Err("Calculation output target must be a Measure".into());
                }
                same_scope(record, target)?;
            }
        }
    }
    Ok(())
}

fn same_scope(a: &Record, b: &Record) -> Result<(), String> {
    if a.scope != b.scope {
        Err("Reference must belong to the same System".into())
    } else {
        Ok(())
    }
}

fn increment(value: u64, name: &str) -> Result<u64, String> {
    value
        .checked_add(1)
        .ok_or_else(|| format!("{name} exhausted; current work preserved"))
}

fn unit_scale(unit: &str) -> Option<(&'static str, i32)> {
    match unit {
        "percent" => Some(("dimensionless", -2)),
        "one" => Some(("dimensionless", 0)),
        "metre" => Some(("length", 0)),
        _ => None,
    }
}

/// Plain finite decimal only. Empty means absent; nonzero underflow is refused.
fn numeric(text: &str) -> Result<Option<f64>, Validity> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let incomplete = text.ends_with(['e', 'E', '.', '+', '-']);
    if !decimal_syntax(text) {
        return Err(if incomplete {
            Validity::Incomplete
        } else {
            Validity::Invalid
        });
    }
    let number: f64 = text.parse().map_err(|_| Validity::Unsupported)?;
    let mantissa = text.split(['e', 'E']).next().unwrap_or(text);
    let nonzero = mantissa.bytes().any(|byte| matches!(byte, b'1'..=b'9'));
    if !number.is_finite() || (number == 0.0 && nonzero) {
        return Err(Validity::Unsupported);
    }
    Ok(Some(number))
}

fn decimal_syntax(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let mut digits = 0;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
        digits += 1;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return false;
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if start == index {
            return false;
        }
    }
    index == bytes.len()
}

/// Convert by decimal exponent rather than multiplying binary floating values.
/// The three supported units are a bounded reference adapter, not a unit engine.
fn convert_units(text: &str, old: &str, new: &str) -> Result<String, String> {
    let (old_dimension, old_scale) = unit_scale(old).ok_or("Unsupported original unit")?;
    let (new_dimension, new_scale) = unit_scale(new).ok_or("Unsupported destination unit")?;
    if old_dimension != new_dimension {
        return Err("Incompatible unit dimensions".into());
    }
    numeric(text).map_err(|_| "Complete a supported numeric criterion first")?;
    let text = text.trim();
    if text.is_empty() {
        return Ok(String::new());
    }
    let negative = text.starts_with('-');
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    let (mantissa, exponent) = unsigned
        .split_once(['e', 'E'])
        .map_or((unsigned, 0i64), |(m, e)| {
            (m, e.parse::<i64>().unwrap_or(i64::MAX))
        });
    let mut digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    if digits.bytes().all(|byte| byte == b'0') {
        return Ok(if negative { "-0".into() } else { "0".into() });
    }
    let decimal = mantissa.find('.').unwrap_or(mantissa.len()) as i64;
    let position = decimal
        .checked_add(exponent)
        .and_then(|n| n.checked_add(i64::from(old_scale - new_scale)))
        .ok_or("Unit conversion outside supported range")?;
    if !(-4096..=4096).contains(&position) || digits.len() > 8192 {
        return Err("Unit conversion outside supported range".into());
    }
    let mut result = if position <= 0 {
        format!("0.{}{}", "0".repeat((-position) as usize), digits)
    } else if position as usize >= digits.len() {
        digits.push_str(&"0".repeat(position as usize - digits.len()));
        digits
    } else {
        digits.insert(position as usize, '.');
        digits
    };
    if result.contains('.') {
        while result.ends_with('0') {
            result.pop();
        }
        if result.ends_with('.') {
            result.pop();
        }
    }
    let trimmed = result.trim_start_matches('0');
    result = if trimmed.is_empty() {
        "0".into()
    } else if trimmed.starts_with('.') {
        format!("0{trimmed}")
    } else {
        trimmed.into()
    };
    if negative {
        result.insert(0, '-');
    }
    numeric(&result).map_err(|_| "Unit conversion outside supported numeric range")?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(record: u64, field: &str) -> RawKey {
        RawKey {
            record: RecordId(record),
            field: field.into(),
            item: None,
        }
    }

    fn fixture_value() -> Value {
        json!({
            "records": [
                {"id":1,"type":"System","name":"Inspection support system","scope":1,"purpose":"Support reviewable inspection decisions."},
                {"id":100,"type":"Mission","name":"Support dependable inspection decisions.","scope":1,"shortLabel":"Support inspection decisions"},
                {"id":200,"type":"Capability","name":"Retain inspection outcomes","scope":1,"contributesTo":[100]},
                {"id":300,"type":"Objective","name":"Keep outcomes reviewable","scope":1,"subject":{"kind":"Capability","id":200},"outcome":"Keep each inspection outcome reviewable."},
                {"id":301,"type":"Measure","name":"Recorded outcomes / inspected items","scope":1,"forObjective":300,"quantity":"dimensionless","unit":"percent","criterion":95,"comparator":"GreaterOrEqual","assessmentMethod":"Inspect retained reports","observations":[],"conditions":"Staffed inspection shifts","uncertainty":"Coverage of missing records unknown"},
                {"id":400,"type":"Stakeholder","name":"Operator","scope":1,"affiliations":{},"actors":[]},
                {"id":401,"type":"Stakeholder","name":"Quality reviewer","scope":1,"affiliations":{"900":"Internal"},"actors":[]},
                {"id":500,"type":"Expectation","name":"Reviewer expectation","scope":1,"expressedBy":401,"concernsSystem":1,"text":"Find the recorded outcome and evidence."},
                {"id":600,"type":"Environment","name":"Staffed inspection bench","scope":1,"context":[
                    {"id":601,"type":"Context","name":"Operating context","scope":1,"kind":"Description","text":"Inspection bench during staffed shifts."},
                    {"id":602,"type":"Context","name":"Reviewer assumption","scope":1,"kind":"Assumption","text":"A reviewer is available during the shift."}
                ],"inputs":[]},
                {"id":900,"type":"Organization","name":"Operating organization","scope":1}
            ],
            "rawDrafts":[{"recordId":301,"field":"criterion","rawText":"9e","validity":"incomplete","acceptedNumeric":95,"unitId":"percent"}]
        })
    }

    fn editor() -> Editor {
        Editor::new(
            Snapshot::from_fixture(DocumentId("fixture".into()), &fixture_value()).unwrap(),
            "instance-a".into(),
        )
        .unwrap()
    }

    fn run(editor: &mut Editor, command: Command) -> EditOutcome {
        editor.command(&editor.freshness(), command).unwrap()
    }

    fn edit(editor: &mut Editor, record: u64, field: &str, text: &str) -> EditOutcome {
        run(
            editor,
            Command::EditText {
                key: key(record, field),
                text: text.into(),
            },
        )
    }

    #[test]
    fn new_document_is_blank_and_reopen_resets_only_interactive_history() {
        let snapshot = Snapshot::new(DocumentId("new".into()));
        assert_eq!(snapshot.records.len(), 1);
        assert_eq!(snapshot.records[&RecordId(1)].name, "");
        assert_eq!(snapshot.records[&RecordId(1)].text("purpose"), "");
        assert_eq!(snapshot.next_id, 1000);
        let mut editor = Editor::new(snapshot, "instance".into()).unwrap();
        edit(&mut editor, 1, "purpose", "A retained purpose");
        assert!(editor.can_undo());
        let saved = editor.snapshot.clone();
        let reopened = Editor::new(saved.clone(), "new instance".into()).unwrap();
        assert_eq!(reopened.snapshot, saved);
        assert!(!reopened.can_undo());
        assert!(!reopened.can_redo());
        assert_eq!(
            reopened.display_text(&key(1, "purpose")),
            "A retained purpose"
        );
    }

    #[test]
    fn fixture_preserves_all_record_properties_and_nested_bytes_as_json_values() {
        let mut fixture = fixture_value();
        fixture["records"][4]["unknownFutureProperty"] =
            json!({"raw":"Σ <script>\r\n", "nested":[null,false,13]});
        let snapshot = Snapshot::from_fixture(DocumentId("fixture".into()), &fixture).unwrap();
        assert_eq!(snapshot.records.len(), 10);
        for value in fixture["records"].as_array().unwrap() {
            let record = &snapshot.records[&RecordId(value["id"].as_u64().unwrap())];
            let mut restored: serde_json::Map<String, Value> =
                record.fields.clone().into_iter().collect();
            restored.insert("id".into(), json!(record.id.0));
            restored.insert("type".into(), json!(record.kind.label()));
            restored.insert("name".into(), json!(record.name));
            restored.insert("scope".into(), json!(record.scope.0));
            assert_eq!(Value::Object(restored), *value);
        }
        assert_eq!(
            snapshot.records[&RecordId(301)].number("criterion"),
            Some(95.0)
        );
        assert_eq!(snapshot.drafts[0].text, "9e");
        assert_eq!(snapshot.drafts[0].validity, Validity::Incomplete);
        assert_eq!(
            snapshot.records[&RecordId(300)].subject(),
            Some(SubjectRef::Capability(RecordId(200)))
        );
        assert_eq!(
            snapshot.records[&RecordId(200)].links(),
            vec![RecordId(100)]
        );
        let bytes = serde_json::to_vec(&snapshot).unwrap();
        let restored: Snapshot = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored, snapshot);
        assert_eq!(restored.digest(), snapshot.digest());
    }

    #[test]
    fn every_freshness_component_refuses_atomically() {
        let mut editor = editor();
        for component in 0..4 {
            let mut expected = editor.freshness();
            match component {
                0 => expected.document_id.0.push('x'),
                1 => expected.instance_id.push('x'),
                2 => expected.model_revision += 1,
                _ => expected.raw_version += 1,
            }
            let before = editor.snapshot.clone();
            assert!(editor
                .command(
                    &expected,
                    Command::EditText {
                        key: key(1, "name"),
                        text: "lost".into()
                    }
                )
                .is_err());
            assert!(editor.undo_model(&expected).is_err());
            assert!(editor.redo_model(&expected).is_err());
            assert!(editor.undo_text(&expected, &key(301, "criterion")).is_err());
            assert_eq!(editor.snapshot, before);
            assert!(!editor.can_undo());
        }
    }

    #[test]
    fn model_undo_groups_a_field_session_and_navigation_closes_the_group() {
        let mut editor = Editor::new(Snapshot::new(DocumentId("new".into())), "i".into()).unwrap();
        edit(&mut editor, 1, "name", "A");
        edit(&mut editor, 1, "name", "AB");
        edit(&mut editor, 1, "name", "ABC");
        assert_eq!(editor.snapshot.model_revision, 3);
        editor.select(RecordId(1)).unwrap();
        edit(&mut editor, 1, "name", "ABCD");
        assert!(editor.undo_model(&editor.freshness()).unwrap());
        assert_eq!(editor.snapshot.records[&RecordId(1)].name, "ABC");
        assert_eq!(editor.display_text(&key(1, "name")), "ABCD");
        assert!(editor.undo_model(&editor.freshness()).unwrap());
        assert_eq!(editor.snapshot.records[&RecordId(1)].name, "");
        assert!(!editor.can_undo());
        assert!(editor.redo_model(&editor.freshness()).unwrap());
        assert_eq!(editor.snapshot.records[&RecordId(1)].name, "ABC");
        assert_eq!(editor.snapshot.model_revision, 7);
    }

    #[test]
    fn accepted_numeric_and_exact_text_history_are_independent() {
        let mut editor = editor();
        let criterion = key(301, "criterion");
        edit(&mut editor, 301, "criterion", "095.00");
        assert!(!edit(&mut editor, 301, "criterion", "9e").accepted);
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].number("criterion"),
            Some(95.0)
        );
        assert_eq!(
            editor.raw(&criterion).unwrap().validity,
            Validity::Incomplete
        );
        assert!(editor.undo_text(&editor.freshness(), &criterion).unwrap());
        assert_eq!(editor.display_text(&criterion), "095.00");
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].number("criterion"),
            Some(95.0)
        );
        assert!(editor.redo_text(&editor.freshness(), &criterion).unwrap());
        assert_eq!(editor.display_text(&criterion), "9e");
        assert!(edit(&mut editor, 301, "criterion", "").accepted);
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].number("criterion"),
            None
        );
        assert!(edit(&mut editor, 301, "criterion", "0").accepted);
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].number("criterion"),
            Some(0.0)
        );
        assert_eq!(editor.display_text(&criterion), "0");
    }

    #[test]
    fn numeric_range_and_incomplete_inputs_never_replace_accepted_value() {
        let mut editor = editor();
        for (text, validity) in [
            ("1e9999", Validity::Unsupported),
            ("1e-9999", Validity::Unsupported),
            ("NaN", Validity::Invalid),
            ("inf", Validity::Invalid),
            ("9e-", Validity::Incomplete),
            ("-", Validity::Incomplete),
            ("recorded / inspected * 100", Validity::Invalid),
        ] {
            assert!(!edit(&mut editor, 301, "criterion", text).accepted);
            assert_eq!(
                editor.raw(&key(301, "criterion")).unwrap().validity,
                validity
            );
            assert_eq!(
                editor.snapshot.records[&RecordId(301)].number("criterion"),
                Some(95.0)
            );
            assert_eq!(editor.display_text(&key(301, "criterion")), text);
        }
        assert!(edit(&mut editor, 301, "criterion", "-0e-9999").accepted);
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].number("criterion"),
            Some(-0.0)
        );
        assert!(numeric(".5").is_ok());
        assert_eq!(numeric("9."), Ok(Some(9.0)));
        assert_eq!(numeric("3e+2"), Ok(Some(300.0)));
        assert_eq!(numeric("0x10"), Err(Validity::Invalid));
    }

    #[test]
    fn detached_selected_raw_and_creation_journals_survive_undo_and_reopen() {
        let mut editor = Editor::new(Snapshot::new(DocumentId("doc".into())), "i".into()).unwrap();
        let mission = run(
            &mut editor,
            Command::Add {
                kind: RecordKind::Mission,
                owner: RecordId(1),
                name: "Original".into(),
            },
        )
        .created
        .unwrap();
        editor.select(mission).unwrap();
        edit(&mut editor, mission.0, "name", "Raw retained");
        run(
            &mut editor,
            Command::PutCreationDraft {
                id: "next-mission".into(),
                draft: CreationDraft {
                    owner: mission,
                    kind: RecordKind::Mission,
                    fields: BTreeMap::from([("name".into(), "Partial new mission".into())]),
                },
            },
        );
        assert!(editor.undo_model(&editor.freshness()).unwrap());
        assert!(editor.undo_model(&editor.freshness()).unwrap());
        assert_eq!(editor.selected, Some(mission));
        assert!(!editor.selection_resolved());
        assert_eq!(editor.display_text(&key(mission.0, "name")), "Raw retained");
        assert!(editor.snapshot.creation_drafts.contains_key("next-mission"));
        let retained = editor.snapshot.clone();
        let mut reopened = Editor::new(retained.clone(), "reopened".into()).unwrap();
        assert_eq!(reopened.snapshot, retained);
        assert!(!reopened.can_undo());
        let new_id = run(
            &mut reopened,
            Command::Add {
                kind: RecordKind::Mission,
                owner: RecordId(1),
                name: "Different".into(),
            },
        )
        .created
        .unwrap();
        assert!(new_id > mission);
        assert_eq!(
            reopened.display_text(&key(mission.0, "name")),
            "Raw retained"
        );
        assert!(reopened
            .command(
                &reopened.freshness(),
                Command::EditText {
                    key: key(mission.0, "name"),
                    text: "wrong".into()
                }
            )
            .is_err());
    }

    #[test]
    fn create_and_link_is_one_undoable_transaction_and_allocators_never_rewind() {
        let mut editor = editor();
        let created = run(
            &mut editor,
            Command::CreateAndLinkMission {
                capability: RecordId(200),
                name: " New mission ".into(),
            },
        )
        .created
        .unwrap();
        assert_eq!(editor.snapshot.records[&created].name, "New mission");
        assert!(editor.snapshot.records[&RecordId(200)]
            .links()
            .contains(&created));
        let next_id = editor.snapshot.next_id;
        assert!(editor.undo_model(&editor.freshness()).unwrap());
        assert!(!editor.snapshot.records.contains_key(&created));
        assert!(!editor.snapshot.records[&RecordId(200)]
            .links()
            .contains(&created));
        assert_eq!(editor.snapshot.next_id, next_id);
        assert!(editor.redo_model(&editor.freshness()).unwrap());
        assert!(editor.snapshot.records.contains_key(&created));
        assert!(editor.undo_model(&editor.freshness()).unwrap());
        let different = run(
            &mut editor,
            Command::Add {
                kind: RecordKind::Mission,
                owner: RecordId(1),
                name: "Another".into(),
            },
        )
        .created
        .unwrap();
        assert!(different > created);
        assert!(!editor.can_redo());
    }

    #[test]
    fn wrong_kind_wrong_system_and_missing_reference_mutations_are_atomic() {
        let mut snapshot =
            Snapshot::from_fixture(DocumentId("scopes".into()), &fixture_value()).unwrap();
        snapshot.records.insert(
            RecordId(2),
            Record {
                id: RecordId(2),
                kind: RecordKind::System,
                name: "Other System".into(),
                scope: RecordId(2),
                fields: BTreeMap::new(),
            },
        );
        snapshot.records.insert(
            RecordId(101),
            Record {
                id: RecordId(101),
                kind: RecordKind::Mission,
                name: "Other Mission".into(),
                scope: RecordId(2),
                fields: BTreeMap::new(),
            },
        );
        let mut editor = Editor::new(snapshot, "i".into()).unwrap();
        for command in [
            Command::ToggleMission {
                capability: RecordId(200),
                mission: RecordId(101),
            },
            Command::ToggleMission {
                capability: RecordId(200),
                mission: RecordId(300),
            },
            Command::ToggleMission {
                capability: RecordId(200),
                mission: RecordId(999),
            },
            Command::SetSubject {
                objective: RecordId(300),
                subject: Some(SubjectRef::Mission(RecordId(101))),
            },
            Command::SetSubject {
                objective: RecordId(300),
                subject: Some(SubjectRef::Mission(RecordId(200))),
            },
            Command::Add {
                kind: RecordKind::Measure,
                owner: RecordId(200),
                name: "Bad owner".into(),
            },
            Command::Add {
                kind: RecordKind::Expectation,
                owner: RecordId(200),
                name: "Bad owner".into(),
            },
            Command::CreateAndLinkMission {
                capability: RecordId(200),
                name: "  ".into(),
            },
        ] {
            let before = editor.snapshot.clone();
            assert!(editor.command(&editor.freshness(), command).is_err());
            assert_eq!(editor.snapshot, before);
        }
        run(
            &mut editor,
            Command::ToggleMission {
                capability: RecordId(200),
                mission: RecordId(100),
            },
        );
        assert!(editor.snapshot.records[&RecordId(200)].links().is_empty());
        run(
            &mut editor,
            Command::ToggleMission {
                capability: RecordId(200),
                mission: RecordId(100),
            },
        );
        assert_eq!(
            editor.snapshot.records[&RecordId(200)].links(),
            vec![RecordId(100)]
        );
        run(
            &mut editor,
            Command::SetSubject {
                objective: RecordId(300),
                subject: Some(SubjectRef::System(RecordId(1))),
            },
        );
        assert_eq!(
            editor.snapshot.records[&RecordId(300)].subject(),
            Some(SubjectRef::System(RecordId(1)))
        );
    }

    #[test]
    fn invalid_criterion_refuses_units_and_valid_units_convert_raw_with_history() {
        let mut editor = editor();
        let before = editor.snapshot.clone();
        assert!(editor
            .command(
                &editor.freshness(),
                Command::SetQuantity {
                    measure: RecordId(301),
                    quantity: "dimensionless".into(),
                    unit: "one".into()
                }
            )
            .is_err());
        assert_eq!(editor.snapshot, before);
        edit(&mut editor, 301, "criterion", "95");
        edit(&mut editor, 301, "criterion", "90");
        editor.finish_field();
        run(
            &mut editor,
            Command::SetQuantity {
                measure: RecordId(301),
                quantity: "dimensionless".into(),
                unit: "one".into(),
            },
        );
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].number("criterion"),
            Some(0.9)
        );
        assert_eq!(editor.display_text(&key(301, "criterion")), "0.9");
        assert!(editor
            .undo_text(&editor.freshness(), &key(301, "criterion"))
            .unwrap());
        assert_eq!(editor.display_text(&key(301, "criterion")), "0.95");
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].number("criterion"),
            Some(0.95)
        );
        assert!(editor
            .redo_text(&editor.freshness(), &key(301, "criterion"))
            .unwrap());
        assert_eq!(editor.display_text(&key(301, "criterion")), "0.9");
        assert_eq!(convert_units("95", "percent", "one").unwrap(), "0.95");
        assert_eq!(convert_units("0.95", "one", "percent").unwrap(), "95");
        assert_eq!(
            convert_units("+009.500e1", "percent", "one").unwrap(),
            "0.95"
        );
        assert!(convert_units("95", "percent", "metre").is_err());
    }

    #[test]
    fn initial_measure_metadata_selection_retains_reopened_incomplete_criterion() {
        let mut editor = editor();
        let measure = run(
            &mut editor,
            Command::Add {
                kind: RecordKind::Measure,
                owner: RecordId(300),
                name: "New measure".into(),
            },
        )
        .created
        .unwrap();
        assert!(!edit(&mut editor, measure.0, "criterion", "9e").accepted);
        let criterion = key(measure.0, "criterion");
        let snapshot = editor.snapshot.clone();
        let mut reopened = Editor::new(snapshot, "reopened".into()).unwrap();
        let previous_raw_version = reopened.snapshot.raw_version;
        run(
            &mut reopened,
            Command::SetQuantity {
                measure,
                quantity: "dimensionless".into(),
                unit: "percent".into(),
            },
        );
        assert_eq!(reopened.snapshot.records[&measure].text("unit"), "percent");
        assert_eq!(
            reopened.snapshot.records[&measure].number("criterion"),
            None
        );
        assert_eq!(reopened.display_text(&criterion), "9e");
        let raw = reopened.raw(&criterion).unwrap();
        assert_eq!(raw.validity, Validity::Incomplete);
        assert_eq!(raw.unit_context.as_deref(), Some("percent"));
        assert!(raw.version > previous_raw_version);
        let retained = reopened.snapshot.clone();
        // Once metadata exists, incomplete input still blocks a unit change.
        assert!(reopened
            .command(
                &reopened.freshness(),
                Command::SetQuantity {
                    measure,
                    quantity: "dimensionless".into(),
                    unit: "one".into()
                }
            )
            .is_err());
        assert_eq!(reopened.snapshot, retained);
    }

    #[test]
    fn native_raw_validity_cannot_contradict_a_known_field_classifier() {
        let mut snapshot = editor().snapshot;
        assert_eq!(snapshot.drafts[0].text, "9e");
        snapshot.drafts[0].validity = Validity::Valid;
        let bytes = serde_json::to_vec(&snapshot).unwrap();
        let malformed: Snapshot = serde_json::from_slice(&bytes).unwrap();
        let before = malformed.clone();
        assert!(malformed.validate().is_err());
        assert_eq!(malformed, before);
        assert!(Editor::new(malformed, "refused".into()).is_err());
        let mut working = editor();
        edit(
            &mut working,
            301,
            "valueExpression",
            "recorded / inspected * 100",
        );
        working
            .snapshot
            .drafts
            .iter_mut()
            .find(|draft| draft.key.field == "valueExpression")
            .unwrap()
            .validity = Validity::Valid;
        assert!(working.snapshot.validate().is_err());
        // Detached and unknown-field journals do not acquire invented semantics.
        let mut snapshot = editor().snapshot;
        snapshot.drafts[0].key.record = RecordId(999);
        snapshot.drafts[0].validity = Validity::Valid;
        assert!(snapshot.validate().is_ok());
        snapshot.drafts[0].key.record = RecordId(301);
        snapshot.drafts[0].key.field = "futureCustomField".into();
        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn text_unit_conversion_overflow_refuses_with_all_history_unchanged() {
        let mut editor = editor();
        let criterion = key(301, "criterion");
        edit(&mut editor, 301, "criterion", "95");
        run(
            &mut editor,
            Command::SetQuantity {
                measure: RecordId(301),
                quantity: "dimensionless".into(),
                unit: "one".into(),
            },
        );
        edit(&mut editor, 301, "criterion", "1e308");
        edit(&mut editor, 301, "criterion", "1");
        run(
            &mut editor,
            Command::SetQuantity {
                measure: RecordId(301),
                quantity: "dimensionless".into(),
                unit: "percent".into(),
            },
        );
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].number("criterion"),
            Some(100.0)
        );
        assert_eq!(editor.display_text(&criterion), "100");
        let before = editor.snapshot.clone();
        let history = editor.text_past[&(criterion.clone(), None)]
            .iter()
            .map(|state| (state.text.clone(), state.unit_context.clone()))
            .collect::<Vec<_>>();
        let model_past = editor.past.clone();
        let model_future = editor.future.clone();
        let active_field = editor.active_field.clone();
        assert!(editor.text_future.is_empty());
        // 1e308 in unit one would overflow when represented in percent.
        assert!(editor.undo_text(&editor.freshness(), &criterion).is_err());
        assert_eq!(editor.snapshot, before);
        assert_eq!(
            editor.text_past[&(criterion.clone(), None)]
                .iter()
                .map(|state| (state.text.clone(), state.unit_context.clone()))
                .collect::<Vec<_>>(),
            history
        );
        assert!(editor.text_future.is_empty());
        assert_eq!(editor.past, model_past);
        assert_eq!(editor.future, model_future);
        assert_eq!(editor.active_field, active_field);
        // Repeating the request still refuses: the failed Undo did not pop it.
        assert!(editor.undo_text(&editor.freshness(), &criterion).is_err());
        assert_eq!(editor.snapshot, before);
    }

    #[test]
    fn model_unit_undo_converts_only_current_valid_raw() {
        let mut editor = editor();
        edit(&mut editor, 301, "criterion", "95");
        editor.finish_field();
        run(
            &mut editor,
            Command::SetQuantity {
                measure: RecordId(301),
                quantity: "dimensionless".into(),
                unit: "one".into(),
            },
        );
        assert_eq!(editor.display_text(&key(301, "criterion")), "0.95");
        assert!(editor.undo_model(&editor.freshness()).unwrap());
        assert_eq!(editor.display_text(&key(301, "criterion")), "95");
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].number("criterion"),
            Some(95.0)
        );
        assert!(editor.redo_model(&editor.freshness()).unwrap());
        edit(&mut editor, 301, "criterion", "9e");
        assert!(editor.undo_model(&editor.freshness()).unwrap());
        assert_eq!(editor.display_text(&key(301, "criterion")), "9e");
        assert_eq!(
            editor
                .raw(&key(301, "criterion"))
                .unwrap()
                .unit_context
                .as_deref(),
            Some("one")
        );
        assert_eq!(
            editor.snapshot.records[&RecordId(301)].text("unit"),
            "percent"
        );
    }

    #[test]
    fn unsupported_expression_is_retained_and_never_evaluated_or_accepted() {
        let mut editor = editor();
        let revision = editor.snapshot.model_revision;
        assert!(
            !edit(
                &mut editor,
                301,
                "valueExpression",
                "recorded / inspected * 100"
            )
            .accepted
        );
        assert_eq!(editor.snapshot.model_revision, revision);
        assert_eq!(
            editor.raw(&key(301, "valueExpression")).unwrap().validity,
            Validity::Unsupported
        );
        assert!(!editor.snapshot.records[&RecordId(301)]
            .fields
            .contains_key("valueExpression"));
        assert_eq!(
            editor.display_text(&key(301, "valueExpression")),
            "recorded / inspected * 100"
        );
        let reopened = Editor::new(editor.snapshot.clone(), "reopen".into()).unwrap();
        assert_eq!(
            reopened.display_text(&key(301, "valueExpression")),
            "recorded / inspected * 100"
        );
    }

    #[test]
    fn purpose_and_nested_context_edits_do_not_alias_other_fields() {
        let mut editor = editor();
        let mission_name = editor.snapshot.records[&RecordId(100)].name.clone();
        edit(&mut editor, 1, "purpose", "New purpose");
        assert_eq!(
            editor.snapshot.records[&RecordId(1)].text("purpose"),
            "New purpose"
        );
        assert_eq!(editor.snapshot.records[&RecordId(100)].name, mission_name);
        let context = RawKey {
            record: RecordId(600),
            field: "contextText".into(),
            item: Some(RecordId(602)),
        };
        run(
            &mut editor,
            Command::EditText {
                key: context.clone(),
                text: "Assumption remains documentary".into(),
            },
        );
        assert_eq!(
            editor.display_text(&context),
            "Assumption remains documentary"
        );
        let items = editor.snapshot.records[&RecordId(600)].fields["context"]
            .as_array()
            .unwrap();
        assert_eq!(items[0]["text"], "Inspection bench during staffed shifts.");
        assert_eq!(items[1]["kind"], "Assumption");
        let before = editor.snapshot.clone();
        assert!(editor
            .command(
                &editor.freshness(),
                Command::EditText {
                    key: key(100, "purpose"),
                    text: "wrong".into()
                }
            )
            .is_err());
        assert_eq!(editor.snapshot, before);
    }

    #[test]
    fn malformed_fixture_or_native_snapshot_is_refused_without_repair() {
        let fixture = fixture_value();
        for malformed in [
            {
                let mut value = fixture.clone();
                value["records"][2]["contributesTo"] = json!([100, 100]);
                value
            },
            {
                let mut value = fixture.clone();
                value["records"][3]["subject"] = json!({"kind":"Measure","id":301});
                value
            },
            {
                let mut value = fixture.clone();
                value["records"][8]["context"][0]["id"] = json!(1);
                value
            },
            {
                let mut value = fixture.clone();
                value["records"][4]["criterion"] = json!("95");
                value
            },
            {
                let mut value = fixture.clone();
                value["records"][4]["unit"] = json!("metre");
                value
            },
        ] {
            assert!(Snapshot::from_fixture(DocumentId("bad".into()), &malformed).is_err());
        }
        let mut snapshot = editor().snapshot;
        snapshot.next_id = 602;
        assert!(snapshot.validate().is_err());
        let mut snapshot = editor().snapshot;
        snapshot.drafts.push(snapshot.drafts[0].clone());
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn revision_exhaustion_refuses_without_mutating_records_or_journals() {
        let mut editor = editor();
        editor.snapshot.model_revision = u64::MAX;
        let before = editor.snapshot.clone();
        assert!(editor
            .command(
                &editor.freshness(),
                Command::EditText {
                    key: key(1, "name"),
                    text: "changed".into()
                }
            )
            .is_err());
        assert_eq!(editor.snapshot, before);
        editor.snapshot.model_revision = 0;
        editor.snapshot.raw_version = u64::MAX;
        let before = editor.snapshot.clone();
        assert!(editor
            .command(
                &editor.freshness(),
                Command::EditText {
                    key: key(1, "name"),
                    text: "changed".into()
                }
            )
            .is_err());
        assert_eq!(editor.snapshot, before);
        assert!(!editor.can_undo());
    }
}
