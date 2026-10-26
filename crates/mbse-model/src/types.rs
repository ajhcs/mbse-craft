//! Bounded app-owned component/value definitions, typed properties and raw forms.
use crate::{
    extensions::Extensions,
    stage1::{RecordId, RecordKind, Snapshot},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub mod connectors;
pub mod ports;
pub const NAMESPACE: &str = "mbse-craft.types/1";
pub const DRAFTS: &str = "mbse-craft.type-drafts/1";
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scalar {
    Boolean,
    Integer,
    Real,
    Text,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Component,
    Value(Scalar),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: Id,
    pub system: RecordId,
    pub name: String,
    pub kind: Kind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Multiplicity {
    pub lower: u32,
    pub upper: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Property {
    pub id: Id,
    pub owner: Id,
    pub name: String,
    pub value_type: Id,
    pub multiplicity: Multiplicity,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub owner: Id,
    pub name: String,
    pub value_type: String,
    pub lower: String,
    pub upper: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageKind {
    Part,
    Reference,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    pub id: Id,
    pub name: String,
    pub owner: Id,
    pub target: Id,
    pub kind: UsageKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageForm {
    pub id: Option<Id>,
    pub name: String,
    pub owner: String,
    pub target: String,
    pub kind: UsageKind,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
    pub profile: String,
    pub schema_version: u32,
    pub definitions: BTreeMap<Id, Definition>,
    pub properties: BTreeMap<Id, Property>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub usages: BTreeMap<Id, Usage>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub interfaces: BTreeMap<Id, ports::Interface>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ports: BTreeMap<Id, ports::Port>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub connectors: BTreeMap<Id, connectors::Connector>,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            profile: NAMESPACE.into(),
            schema_version: 1,
            definitions: BTreeMap::new(),
            properties: BTreeMap::new(),
            usages: BTreeMap::new(),
            interfaces: BTreeMap::new(),
            ports: BTreeMap::new(),
            connectors: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionForm {
    pub system: RecordId,
    pub name: String,
    pub value: bool,
    pub scalar: Scalar,
}
impl DefinitionForm {
    pub fn kind(&self) -> Kind {
        if self.value {
            Kind::Value(self.scalar)
        } else {
            Kind::Component
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    profile: String,
    schema_version: u32,
    forms: BTreeMap<Id, Form>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    definition: Option<DefinitionForm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    usage: Option<UsageForm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interface: Option<ports::InterfaceForm>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    ports: BTreeMap<Id, ports::PortForm>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    connectors: BTreeMap<Id, connectors::Form>,
}
impl Default for Journal {
    fn default() -> Self {
        Self {
            profile: DRAFTS.into(),
            schema_version: 1,
            forms: BTreeMap::new(),
            definition: None,
            usage: None,
            interface: None,
            ports: BTreeMap::new(),
            connectors: BTreeMap::new(),
        }
    }
}
fn canonical(value: &Value, field: &str) -> Result<(), String> {
    let map = value
        .get(field)
        .and_then(Value::as_object)
        .ok_or("TYPE_NAMESPACE_SHAPE")?;
    if map.keys().any(|key| {
        key.parse::<u64>()
            .ok()
            .is_none_or(|n| n == 0 || n.to_string() != *key)
    }) {
        return Err("TYPE_NONCANONICAL_ID".into());
    }
    Ok(())
}
fn parse(value: &Value) -> Result<Store, String> {
    canonical(value, "definitions")?;
    canonical(value, "properties")?;
    if value.get("usages").is_some() {
        canonical(value, "usages")?;
    }
    for field in ["interfaces", "ports", "connectors"] {
        if value.get(field).is_some() {
            canonical(value, field)?;
        }
    }
    let store: Store =
        serde_json::from_value(value.clone()).map_err(|e| format!("TYPE_SCHEMA: {e}"))?;
    store.validate()?;
    Ok(store)
}
fn parse_journal(value: &Value) -> Result<Journal, String> {
    canonical(value, "forms")?;
    if value.get("ports").is_some() {
        canonical(value, "ports")?;
    }
    if value.get("connectors").is_some() {
        canonical(value, "connectors")?;
    }
    let j: Journal =
        serde_json::from_value(value.clone()).map_err(|e| format!("TYPE_DRAFT_SCHEMA: {e}"))?;
    if j.profile != DRAFTS || j.schema_version != 1 || j.forms.len() > 512 {
        return Err("TYPE_DRAFT_VERSION_OR_LIMIT".into());
    }
    if j.definition
        .as_ref()
        .is_some_and(|f| f.system.0 == 0 || f.name.len() > 4096)
    {
        return Err("TYPE_DEFINITION_DRAFT_SHAPE_OR_LIMIT".into());
    }
    if j.usage.as_ref().is_some_and(|f| {
        f.id.is_some_and(|id| id.0 == 0)
            || [&f.name, &f.owner, &f.target]
                .iter()
                .any(|v| v.len() > 4096)
    }) {
        return Err("TYPE_USAGE_DRAFT_SHAPE_OR_LIMIT".into());
    }
    for f in j.forms.values() {
        if f.owner.0 == 0
            || [&f.name, &f.value_type, &f.lower, &f.upper]
                .iter()
                .any(|s| s.len() > 4096)
        {
            return Err("TYPE_DRAFT_SHAPE_OR_LIMIT".into());
        }
    }
    ports::validate_journal(&j)?;
    connectors::validate_journal(&j)?;
    Ok(j)
}
pub fn validate_namespace(name: &str, value: &Value) -> Result<(), String> {
    match name {
        NAMESPACE => parse(value).map(|_| ()),
        DRAFTS => parse_journal(value).map(|_| ()),
        _ => Err(format!("MBSE_EXTENSIONS_REQUIRED_UNSUPPORTED: {name}")),
    }
}
fn active<'a>(snapshot: &'a Snapshot, name: &str) -> Result<Option<&'a Value>, String> {
    let e = snapshot
        .extensions
        .as_ref()
        .ok_or("TYPE_MIGRATION_REQUIRED: open a protected migrated native copy first")?;
    if let Some(v) = e.namespaces.get(name) {
        if !e.required_namespaces.iter().any(|s| s == name) {
            return Err("TYPE_OPAQUE_NAMESPACE_COLLISION: retained optional data requires explicit migration".into());
        }
        Ok(Some(v))
    } else {
        Ok(None)
    }
}
pub fn load(snapshot: &Snapshot) -> Result<Store, String> {
    active(snapshot, NAMESPACE)?
        .map(parse)
        .transpose()
        .map(|s| s.unwrap_or_default())
}
fn journal(snapshot: &Snapshot) -> Result<Journal, String> {
    active(snapshot, DRAFTS)?
        .map(parse_journal)
        .transpose()
        .map(|j| j.unwrap_or_default())
}
pub fn forms(snapshot: &Snapshot) -> Result<BTreeMap<Id, Form>, String> {
    Ok(journal(snapshot)?.forms)
}
pub fn definition_form(snapshot: &Snapshot) -> Result<Option<DefinitionForm>, String> {
    Ok(journal(snapshot)?.definition)
}
pub fn usage_form(snapshot: &Snapshot) -> Result<Option<UsageForm>, String> {
    Ok(journal(snapshot)?.usage)
}
pub fn drafts_value(value: &Option<Extensions>) -> Option<Value> {
    value
        .as_ref()
        .filter(|e| e.required_namespaces.iter().any(|n| n == DRAFTS))
        .and_then(|e| e.namespaces.get(DRAFTS))
        .cloned()
}

fn put<T: Serialize>(snapshot: &mut Snapshot, name: &str, value: &T) -> Result<(), String> {
    active(snapshot, name)?;
    let e = snapshot
        .extensions
        .as_mut()
        .ok_or("TYPE_MIGRATION_REQUIRED")?;
    e.namespaces.insert(
        name.into(),
        serde_json::to_value(value).map_err(|e| e.to_string())?,
    );
    if !e.required_namespaces.iter().any(|s| s == name) {
        e.required_namespaces.push(name.into());
        e.required_namespaces.sort();
    }
    Ok(())
}
fn name(text: &str) -> Result<(), String> {
    if text.trim().is_empty() || text.len() > 256 || text.chars().any(char::is_control) {
        Err("TYPE_NAME: use1..256 bytes without control characters".into())
    } else {
        Ok(())
    }
}
fn number(text: &str) -> Result<u32, String> {
    let text = text.trim();
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err("TYPE_MULTIPLICITY: use a canonical nonnegative integer".into());
    }
    text.parse().map_err(|_| "TYPE_MULTIPLICITY_RANGE".into())
}
impl Multiplicity {
    pub fn parse(lower: &str, upper: &str) -> Result<Self, String> {
        let m = Self {
            lower: number(lower)?,
            upper: if upper.trim() == "*" {
                None
            } else {
                Some(number(upper)?)
            },
        };
        m.validate()?;
        Ok(m)
    }
    fn validate(&self) -> Result<(), String> {
        if self.upper.is_some_and(|u| u < self.lower) {
            Err("TYPE_MULTIPLICITY: lower exceeds upper".into())
        } else {
            Ok(())
        }
    }
}
impl Store {
    pub fn validate(&self) -> Result<(), String> {
        if self.profile != NAMESPACE
            || self.schema_version != 1
            || self.definitions.len()
                + self.properties.len()
                + self.usages.len()
                + self.interfaces.len()
                + self.ports.len()
                + self.connectors.len()
                > 512
        {
            return Err("TYPE_VERSION_OR_LIMIT".into());
        }
        let mut ids = BTreeSet::new();
        for (id, d) in &self.definitions {
            if id != &d.id || id.0 == 0 || !ids.insert(*id) {
                return Err("TYPE_DEFINITION_ID".into());
            }
            name(&d.name)?;
        }
        for (id, p) in &self.properties {
            if id != &p.id || id.0 == 0 || !ids.insert(*id) {
                return Err("TYPE_PROPERTY_ID".into());
            }
            name(&p.name)?;
            p.multiplicity.validate()?;
            let owner = self
                .definitions
                .get(&p.owner)
                .ok_or("TYPE_OWNER_MISSING_OR_WRONG_KIND")?;
            let value = self
                .definitions
                .get(&p.value_type)
                .ok_or("TYPE_REFERENCE_MISSING_OR_WRONG_KIND")?;
            if owner.kind != Kind::Component || !matches!(value.kind, Kind::Value(_)) {
                return Err("TYPE_REFERENCE_WRONG_KIND".into());
            }
            if owner.system != value.system {
                return Err("TYPE_CROSS_SYSTEM_REFERENCE_UNSUPPORTED".into());
            }
        }
        for (id, u) in &self.usages {
            if id != &u.id || id.0 == 0 || !ids.insert(*id) {
                return Err("TYPE_USAGE_ID".into());
            }
            name(&u.name)?;
            let owner = self
                .definitions
                .get(&u.owner)
                .ok_or("TYPE_USAGE_OWNER_MISSING")?;
            let target = self
                .definitions
                .get(&u.target)
                .ok_or("TYPE_USAGE_TARGET_MISSING")?;
            if owner.kind != Kind::Component || target.kind != Kind::Component {
                return Err("TYPE_USAGE_WRONG_KIND".into());
            }
            if owner.system != target.system {
                return Err("TYPE_USAGE_CROSS_SYSTEM_UNSUPPORTED".into());
            }
        }
        ports::validate_store(self, &mut ids)?;
        connectors::validate_store(self, &mut ids)?;
        // Topological admission bounds work to definitions plus Part edges;
        // reused targets never expand into every possible occurrence path.
        let mut incoming: BTreeMap<Id, usize> =
            self.definitions.keys().map(|id| (*id, 0)).collect();
        let mut edges: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
        for u in self.usages.values().filter(|u| u.kind == UsageKind::Part) {
            *incoming.get_mut(&u.target).unwrap() += 1;
            edges.entry(u.owner).or_default().push(u.target);
        }
        let mut pending: Vec<Id> = incoming
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = 0;
        while let Some(owner) = pending.pop() {
            visited += 1;
            if let Some(targets) = edges.get(&owner) {
                for target in targets {
                    let count = incoming.get_mut(target).unwrap();
                    *count -= 1;
                    if *count == 0 {
                        pending.push(*target);
                    }
                }
            }
        }
        if visited != self.definitions.len() {
            return Err("TYPE_CONTAINMENT_CYCLE".into());
        }
        Ok(())
    }
}
/// Extend the native allocator/identity checks without reclassifying Stage1 records.
pub fn validate_snapshot(snapshot: &Snapshot, native: &BTreeSet<RecordId>) -> Result<u64, String> {
    let Some(e) = &snapshot.extensions else {
        return Ok(0);
    };
    let mut ids: BTreeSet<u64> = native.iter().map(|id| id.0).collect();
    for draft in &snapshot.drafts {
        ids.insert(draft.key.record.0);
        if let Some(item) = draft.key.item {
            ids.insert(item.0);
        }
    }
    for draft in snapshot.creation_drafts.values() {
        ids.insert(draft.owner.0);
    }
    let native_reserved = ids.clone();
    let mut high = 0;
    let store = if e.required_namespaces.iter().any(|n| n == NAMESPACE) {
        load(snapshot)?
    } else {
        Store::default()
    };
    for d in store.definitions.values() {
        let system = snapshot
            .records
            .get(&d.system)
            .ok_or("TYPE_SYSTEM_MISSING")?;
        if system.kind != RecordKind::System {
            return Err("TYPE_SYSTEM_WRONG_KIND".into());
        }
    }
    for id in store
        .definitions
        .keys()
        .chain(store.properties.keys())
        .chain(store.usages.keys())
        .chain(store.interfaces.keys())
        .chain(store.ports.keys())
        .chain(store.connectors.keys())
    {
        if !ids.insert(id.0) {
            return Err("TYPE_NATIVE_ID_COLLISION".into());
        }
        high = high.max(id.0)
    }
    if e.required_namespaces.iter().any(|n| n == DRAFTS) {
        let j = journal(snapshot)?;
        if let Some(form) = j.interface {
            high = high.max(form.system.0);
        }
        for (id, form) in j.ports {
            if store.definitions.contains_key(&id)
                || store.properties.contains_key(&id)
                || store.usages.contains_key(&id)
                || store.interfaces.contains_key(&id)
                || store.connectors.contains_key(&id)
                || connectors::forms(snapshot)?.contains_key(&id)
                || native_reserved.contains(&id.0)
                || forms(snapshot)?.contains_key(&id)
                || usage_form(snapshot)?.is_some_and(|f| f.id == Some(id))
            {
                return Err("PORT_DRAFT_ID_COLLISION".into());
            }
            high = high.max(id.0).max(form.owner.0);
        }
        for (id, form) in connectors::forms(snapshot)? {
            if store.definitions.contains_key(&id)
                || store.properties.contains_key(&id)
                || store.usages.contains_key(&id)
                || store.interfaces.contains_key(&id)
                || store.ports.contains_key(&id)
                || native_reserved.contains(&id.0)
                || forms(snapshot)?.contains_key(&id)
                || ports::forms(snapshot)?.contains_key(&id)
                || usage_form(snapshot)?.is_some_and(|f| f.id == Some(id))
            {
                return Err("CONNECTOR_DRAFT_ID_COLLISION".into());
            }
            high = high.max(id.0).max(form.owner.0);
        }
        if let Some(definition) = definition_form(snapshot)? {
            high = high.max(definition.system.0);
        }
        if let Some(form) = usage_form(snapshot)? {
            if let Some(id) = form.id {
                if store.definitions.contains_key(&id)
                    || store.properties.contains_key(&id)
                    || store.interfaces.contains_key(&id)
                    || store.ports.contains_key(&id)
                    || store.connectors.contains_key(&id)
                    || connectors::forms(snapshot)?.contains_key(&id)
                    || ports::forms(snapshot)?.contains_key(&id)
                    || native_reserved.contains(&id.0)
                {
                    return Err("TYPE_USAGE_DRAFT_ID_COLLISION".into());
                }
                high = high.max(id.0);
            }
        }
        for (id, f) in forms(snapshot)? {
            if store.definitions.contains_key(&id)
                || store.usages.contains_key(&id)
                || store.ports.contains_key(&id)
                || store.interfaces.contains_key(&id)
                || store.connectors.contains_key(&id)
                || connectors::forms(snapshot)?.contains_key(&id)
                || ports::forms(snapshot)?.contains_key(&id)
                || usage_form(snapshot)?.is_some_and(|f| f.id == Some(id))
                || native_reserved.contains(&id.0)
            {
                return Err("TYPE_DRAFT_ID_COLLISION".into());
            }
            high = high.max(id.0).max(f.owner.0);
        }
    }
    Ok(high)
}
#[derive(Clone, Debug)]
pub enum Action {
    Connector(connectors::Action),
    Port(ports::Action),
    SetUsageForm {
        form: UsageForm,
    },
    ApplyUsage,
    SetDefinitionForm {
        form: DefinitionForm,
    },
    AddDefinition {
        system: RecordId,
        name: String,
        kind: Kind,
    },
    BeginProperty {
        owner: Id,
    },
    SetForm {
        id: Id,
        form: Form,
    },
    ApplyProperty {
        id: Id,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Model,
    Raw,
}
fn allocate(snapshot: &mut Snapshot) -> Result<Id, String> {
    let id = Id(snapshot.next_id);
    snapshot.next_id = snapshot.next_id.checked_add(1).ok_or("TYPE_ID_EXHAUSTED")?;
    Ok(id)
}
pub fn apply(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut next = snapshot.clone();
    let change = apply_inner(&mut next, action)?;
    next.validate()?;
    *snapshot = next;
    Ok(change)
}
fn apply_inner(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut store = load(snapshot)?;
    match action {
        Action::Connector(action) => connectors::apply_inner(snapshot, action),
        Action::Port(action) => ports::apply_inner(snapshot, action),
        Action::SetUsageForm { form } => {
            let mut j = journal(snapshot)?;
            j.usage = Some(form);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::ApplyUsage => {
            let mut j = journal(snapshot)?;
            let mut f = j.usage.clone().ok_or("TYPE_USAGE_DRAFT_UNAVAILABLE")?;
            name(&f.name)?;
            let reference = |text: &str| -> Result<Id, String> {
                let id = text
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| "TYPE_USAGE_REFERENCE_ID")?;
                if id == 0 || id.to_string() != text.trim() {
                    return Err("TYPE_USAGE_REFERENCE_ID".into());
                }
                Ok(Id(id))
            };
            let owner = reference(&f.owner)?;
            let target = reference(&f.target)?;
            let id = if let Some(id) = f.id {
                if !store.usages.contains_key(&id) {
                    return Err("TYPE_USAGE_DETACHED_DRAFT: create a new usage explicitly".into());
                }
                id
            } else {
                allocate(snapshot)?
            };
            store.usages.insert(
                id,
                Usage {
                    id,
                    name: f.name.clone(),
                    owner,
                    target,
                    kind: f.kind,
                },
            );
            store.validate()?;
            f.id = Some(id);
            j.usage = Some(f);
            put(snapshot, NAMESPACE, &store)?;
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Model)
        }
        Action::SetDefinitionForm { form } => {
            let mut j = journal(snapshot)?;
            j.definition = Some(form);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::AddDefinition {
            system,
            name: label,
            kind,
        } => {
            name(&label)?;
            if snapshot
                .records
                .get(&system)
                .is_none_or(|r| r.kind != RecordKind::System)
            {
                return Err("TYPE_SYSTEM_WRONG_KIND".into());
            }
            let mut journal = journal(snapshot)?;
            if let Some(form) = journal.definition.as_mut() {
                if form.system == system && form.name == label && form.kind() == kind {
                    form.name.clear();
                }
            }
            let id = allocate(snapshot)?;
            store.definitions.insert(
                id,
                Definition {
                    id,
                    system,
                    name: label,
                    kind,
                },
            );
            put(snapshot, NAMESPACE, &store)?;
            if journal.definition.is_some() {
                put(snapshot, DRAFTS, &journal)?;
            }
            Ok(Change::Model)
        }
        Action::BeginProperty { owner } => {
            if store
                .definitions
                .get(&owner)
                .is_none_or(|d| d.kind != Kind::Component)
            {
                return Err("TYPE_OWNER_WRONG_KIND".into());
            }
            let id = allocate(snapshot)?;
            let mut journal = journal(snapshot)?;
            journal.forms.insert(
                id,
                Form {
                    owner,
                    name: String::new(),
                    value_type: String::new(),
                    lower: "1".into(),
                    upper: "1".into(),
                },
            );
            put(snapshot, DRAFTS, &journal)?;
            Ok(Change::Raw)
        }
        Action::SetForm { id, form } => {
            let mut journal = journal(snapshot)?;
            if !journal.forms.contains_key(&id) && !store.properties.contains_key(&id) {
                return Err("TYPE_DRAFT_UNAVAILABLE".into());
            }
            journal.forms.insert(id, form);
            put(snapshot, DRAFTS, &journal)?;
            Ok(Change::Raw)
        }
        Action::ApplyProperty { id } => {
            let form = forms(snapshot)?
                .get(&id)
                .cloned()
                .ok_or("TYPE_DRAFT_UNAVAILABLE")?;
            name(&form.name)?;
            let value_type = Id(form
                .value_type
                .trim()
                .parse()
                .map_err(|_| "TYPE_REFERENCE_ID")?);
            let multiplicity = Multiplicity::parse(&form.lower, &form.upper)?;
            store.properties.insert(
                id,
                Property {
                    id,
                    owner: form.owner,
                    name: form.name,
                    value_type,
                    multiplicity,
                },
            );
            store.validate()?;
            put(snapshot, NAMESPACE, &store)?;
            Ok(Change::Model)
        }
    }
}
/// Model history excludes current definition/property journals; IDs stay in Snapshot.next_id.
pub fn accepted_extensions(value: &Option<Extensions>) -> Option<Extensions> {
    let mut value = value.clone();
    if let Some(e) = value.as_mut() {
        if e.required_namespaces.iter().any(|n| n == DRAFTS) {
            e.namespaces.remove(DRAFTS);
            e.required_namespaces.retain(|n| n != DRAFTS);
        }
    }
    value
}
pub fn restore_accepted(current: &mut Option<Extensions>, mut restored: Option<Extensions>) {
    if let Some(e) = current.as_ref() {
        if e.required_namespaces.iter().any(|n| n == DRAFTS) {
            if let Some(journal) = e.namespaces.get(DRAFTS) {
                let r = restored.get_or_insert_with(Extensions::default);
                r.namespaces.insert(DRAFTS.into(), journal.clone());
                if !r.required_namespaces.iter().any(|n| n == DRAFTS) {
                    r.required_namespaces.push(DRAFTS.into());
                    r.required_namespaces.sort();
                }
            }
        }
    }
    *current = restored;
}

pub fn preview_usage(snapshot: &Snapshot) -> Result<Usage, String> {
    let mut candidate = snapshot.clone();
    apply(&mut candidate, Action::ApplyUsage)?;
    let id = usage_form(&candidate)?
        .and_then(|f| f.id)
        .ok_or("TYPE_USAGE_DRAFT_UNAVAILABLE")?;
    load(&candidate)?
        .usages
        .get(&id)
        .cloned()
        .ok_or("TYPE_USAGE_UNAVAILABLE".into())
}

pub fn preview(snapshot: &Snapshot, id: Id) -> Result<Property, String> {
    let mut candidate = snapshot.clone();
    apply(&mut candidate, Action::ApplyProperty { id })?;
    load(&candidate)?
        .properties
        .get(&id)
        .cloned()
        .ok_or("TYPE_PROPERTY_UNAVAILABLE".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage1::{Command, DocumentId, Editor};
    fn editor() -> Editor {
        let mut s = Snapshot::new(DocumentId("native:typed-fixture".into()));
        s.extensions = Some(Extensions::default());
        Editor::new(s, "typed:test".into()).unwrap()
    }
    fn run(e: &mut Editor, a: Action) -> Result<Id, String> {
        let id = Id(e.snapshot.next_id);
        e.command(&e.freshness(), Command::Typed(a))?;
        Ok(id)
    }
    fn definitions(e: &mut Editor) -> (Id, Id) {
        let c = run(
            e,
            Action::AddDefinition {
                system: RecordId(1),
                name: "Subsystem".into(),
                kind: Kind::Component,
            },
        )
        .unwrap();
        let v = run(
            e,
            Action::AddDefinition {
                system: RecordId(1),
                name: "Distance".into(),
                kind: Kind::Value(Scalar::Real),
            },
        )
        .unwrap();
        (c, v)
    }
    fn property(e: &mut Editor, c: Id, v: Id) -> Id {
        let id = run(e, Action::BeginProperty { owner: c }).unwrap();
        run(
            e,
            Action::SetForm {
                id,
                form: Form {
                    owner: c,
                    name: "length".into(),
                    value_type: v.0.to_string(),
                    lower: "1".into(),
                    upper: "1".into(),
                },
            },
        )
        .unwrap();
        run(e, Action::ApplyProperty { id }).unwrap();
        id
    }
    #[test]
    fn wrong_kind_and_invalid_multiplicities_refuse_atomically_with_raw_form_retained() {
        let mut e = editor();
        let (c, v) = definitions(&mut e);
        let id = property(&mut e, c, v);
        let accepted = load(&e.snapshot).unwrap();
        for form in [
            Form {
                owner: c,
                name: "wrong type".into(),
                value_type: c.0.to_string(),
                lower: "1".into(),
                upper: "1".into(),
            },
            Form {
                owner: v,
                name: "wrong owner".into(),
                value_type: v.0.to_string(),
                lower: "1".into(),
                upper: "1".into(),
            },
            Form {
                owner: c,
                name: "pending".into(),
                value_type: v.0.to_string(),
                lower: "2".into(),
                upper: "1".into(),
            },
            Form {
                owner: c,
                name: "pending".into(),
                value_type: v.0.to_string(),
                lower: "1".into(),
                upper: "1..".into(),
            },
        ] {
            run(
                &mut e,
                Action::SetForm {
                    id,
                    form: form.clone(),
                },
            )
            .unwrap();
            let before = e.snapshot.clone();
            assert!(run(&mut e, Action::ApplyProperty { id }).is_err());
            assert_eq!(e.snapshot, before);
            assert_eq!(load(&e.snapshot).unwrap(), accepted);
            assert_eq!(forms(&e.snapshot).unwrap()[&id], form);
        }
    }
    #[test]
    fn model_undo_redo_retains_newer_property_forms_and_monotonic_native_allocator() {
        let mut e = editor();
        let (c, v) = definitions(&mut e);
        let id = property(&mut e, c, v);
        let next = e.snapshot.next_id;
        let form = Form {
            owner: c,
            name: "new unfinished text".into(),
            value_type: v.0.to_string(),
            lower: "1".into(),
            upper: "1..".into(),
        };
        run(
            &mut e,
            Action::SetForm {
                id,
                form: form.clone(),
            },
        )
        .unwrap();
        assert!(e.undo_model(&e.freshness()).unwrap());
        assert!(!load(&e.snapshot).unwrap().properties.contains_key(&id));
        assert_eq!(forms(&e.snapshot).unwrap()[&id], form);
        assert_eq!(e.snapshot.next_id, next);
        assert!(e.redo_model(&e.freshness()).unwrap());
        assert_eq!(load(&e.snapshot).unwrap().properties[&id].value_type, v);
        assert_eq!(forms(&e.snapshot).unwrap()[&id], form);
        while e.can_undo() {
            e.undo_model(&e.freshness()).unwrap();
        }
        assert!(load(&e.snapshot).unwrap().definitions.is_empty());
        assert_eq!(forms(&e.snapshot).unwrap()[&id], form);
        assert!(e.snapshot.next_id >= next);
        let new = run(
            &mut e,
            Action::AddDefinition {
                system: RecordId(1),
                name: "Fresh identity".into(),
                kind: Kind::Component,
            },
        )
        .unwrap();
        assert!(new.0 >= next);
    }
    #[test]
    fn optional_known_key_is_not_reinterpreted_and_native_opaque_data_is_unchanged() {
        let mut e = editor();
        let original = serde_json::json!({"future":"opaque meaning"});
        e.snapshot
            .extensions
            .as_mut()
            .unwrap()
            .namespaces
            .insert(NAMESPACE.into(), original.clone());
        let before = e.snapshot.clone();
        assert!(
            run(
                &mut e,
                Action::AddDefinition {
                    system: RecordId(1),
                    name: "No overwrite".into(),
                    kind: Kind::Component
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot, before);
        assert_eq!(
            e.snapshot.extensions.unwrap().namespaces[NAMESPACE],
            original
        );
    }
    #[test]
    fn identity_collision_noncanonical_keys_and_future_typed_schema_refuse() {
        let mut e = editor();
        let (c, _) = definitions(&mut e);
        let mut s = e.snapshot.clone();
        let ext = s.extensions.as_mut().unwrap();
        let data = ext.namespaces.get_mut(NAMESPACE).unwrap();
        let mut definition = data["definitions"][c.0.to_string()].clone();
        definition["id"] = serde_json::json!(1);
        data["definitions"]
            .as_object_mut()
            .unwrap()
            .insert("1".into(), definition);
        assert!(s.validate().unwrap_err().contains("COLLISION"));
        let mut s = e.snapshot.clone();
        let data = s
            .extensions
            .as_mut()
            .unwrap()
            .namespaces
            .get_mut(NAMESPACE)
            .unwrap();
        let value = data["definitions"]
            .as_object_mut()
            .unwrap()
            .remove(&c.0.to_string())
            .unwrap();
        data["definitions"]
            .as_object_mut()
            .unwrap()
            .insert(format!("0{}", c.0), value);
        assert!(s.validate().unwrap_err().contains("NONCANONICAL"));
        let mut s = e.snapshot.clone();
        s.extensions
            .as_mut()
            .unwrap()
            .namespaces
            .get_mut(NAMESPACE)
            .unwrap()["schema_version"] = serde_json::json!(99);
        assert!(s.validate().is_err());
    }
    #[test]
    fn direct_apply_is_atomic_and_stale_editor_tickets_cannot_mutate_types() {
        let mut e = editor();
        let (c, v) = definitions(&mut e);
        let ticket = e.freshness();
        let id = property(&mut e, c, v);
        let before = e.snapshot.clone();
        assert!(
            e.command(&ticket, Command::Typed(Action::ApplyProperty { id }))
                .is_err()
        );
        assert_eq!(e.snapshot, before);
        let mut snapshot = before.clone();
        assert!(
            apply(
                &mut snapshot,
                Action::AddDefinition {
                    system: RecordId(9999),
                    name: "bad".into(),
                    kind: Kind::Component
                }
            )
            .is_err()
        );
        assert_eq!(snapshot, before);
        for (lo, hi) in [
            ("2", "1"),
            ("01", "1"),
            ("-1", "*"),
            ("1", "1.."),
            ("4294967296", "*"),
        ] {
            assert!(Multiplicity::parse(lo, hi).is_err());
        }
        assert_eq!(
            Multiplicity::parse("0", "*").unwrap(),
            Multiplicity {
                lower: 0,
                upper: None
            }
        );
    }
    #[test]
    fn actual_native_save_reopen_retains_typed_ids_and_unfinished_forms_with_empty_runtime_history()
    {
        use crate::stage1_io::{Session, execute_write};
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        session.editor.snapshot.extensions = Some(Extensions::default());
        let (c, v) = definitions(&mut session.editor);
        let id = property(&mut session.editor, c, v);
        let form = Form {
            owner: c,
            name: "unfinished rename".into(),
            value_type: v.0.to_string(),
            lower: "1".into(),
            upper: "1..".into(),
        };
        run(
            &mut session.editor,
            Action::SetForm {
                id,
                form: form.clone(),
            },
        )
        .unwrap();
        let expected = session.editor.snapshot.clone();
        let path = area.path().join("typed.kernspan");
        let job = session.begin_save(&path).unwrap();
        assert!(session.complete(execute_write(job)));
        let reopened = Session::open(area.path(), &path).unwrap();
        assert_eq!(reopened.editor.snapshot, expected);
        assert_eq!(forms(&reopened.editor.snapshot).unwrap()[&id], form);
        assert_eq!(
            load(&reopened.editor.snapshot).unwrap().properties[&id].value_type,
            v
        );
        assert!(!reopened.editor.can_undo() && !reopened.editor.can_redo());
    }
    #[test]
    fn invalid_definition_draft_survives_property_edits_history_and_native_reopen() {
        use crate::stage1_io::{Session, execute_write};
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        session.editor.snapshot.extensions = Some(Extensions::default());
        let (c, v) = definitions(&mut session.editor);
        let draft = DefinitionForm {
            system: RecordId(1),
            name: "x".repeat(257),
            value: true,
            scalar: Scalar::Integer,
        };
        run(
            &mut session.editor,
            Action::SetDefinitionForm {
                form: draft.clone(),
            },
        )
        .unwrap();
        let property = property(&mut session.editor, c, v);
        assert_eq!(
            definition_form(&session.editor.snapshot).unwrap(),
            Some(draft.clone())
        );
        let before = session.editor.snapshot.clone();
        assert!(
            run(
                &mut session.editor,
                Action::AddDefinition {
                    system: draft.system,
                    name: draft.name.clone(),
                    kind: draft.kind()
                }
            )
            .is_err()
        );
        assert_eq!(session.editor.snapshot, before);
        assert!(
            session
                .editor
                .undo_model(&session.editor.freshness())
                .unwrap()
        );
        assert_eq!(
            definition_form(&session.editor.snapshot).unwrap(),
            Some(draft.clone())
        );
        assert!(
            session
                .editor
                .redo_model(&session.editor.freshness())
                .unwrap()
        );
        assert_eq!(
            definition_form(&session.editor.snapshot).unwrap(),
            Some(draft.clone())
        );
        assert!(
            forms(&session.editor.snapshot)
                .unwrap()
                .contains_key(&property)
        );
        let path = area.path().join("definition-draft.kernspan");
        let job = session.begin_save(&path).unwrap();
        assert!(session.complete(execute_write(job)));
        let reopened = Session::open(area.path(), &path).unwrap();
        assert_eq!(reopened.editor.snapshot, session.editor.snapshot);
        assert_eq!(
            definition_form(&reopened.editor.snapshot).unwrap(),
            Some(draft)
        );
    }
    #[test]
    fn adding_definition_consumes_only_matching_name_and_advances_raw_revision() {
        let mut e = editor();
        let draft = DefinitionForm {
            system: RecordId(1),
            name: "Pending value".into(),
            value: true,
            scalar: Scalar::Text,
        };
        run(
            &mut e,
            Action::SetDefinitionForm {
                form: draft.clone(),
            },
        )
        .unwrap();
        let unchanged = e.snapshot.clone();
        run(
            &mut e,
            Action::SetDefinitionForm {
                form: draft.clone(),
            },
        )
        .unwrap();
        assert_eq!(e.snapshot, unchanged);
        run(
            &mut e,
            Action::AddDefinition {
                system: RecordId(1),
                name: "Another definition".into(),
                kind: Kind::Component,
            },
        )
        .unwrap();
        assert_eq!(definition_form(&e.snapshot).unwrap(), Some(draft.clone()));
        let raw = e.snapshot.raw_version;
        let model = e.snapshot.model_revision;
        run(
            &mut e,
            Action::AddDefinition {
                system: draft.system,
                name: draft.name.clone(),
                kind: draft.kind(),
            },
        )
        .unwrap();
        assert_eq!(e.snapshot.raw_version, raw + 1);
        assert_eq!(e.snapshot.model_revision, model + 1);
        let mut expected = draft;
        expected.name.clear();
        assert_eq!(
            definition_form(&e.snapshot).unwrap(),
            Some(expected.clone())
        );
        assert!(e.undo_model(&e.freshness()).unwrap());
        assert_eq!(definition_form(&e.snapshot).unwrap(), Some(expected));
        assert!(
            load(&e.snapshot)
                .unwrap()
                .definitions
                .values()
                .all(|d| d.name != "Pending value")
        );
    }
    #[test]
    fn repeated_apply_and_identical_raw_form_are_noops_in_the_model_history() {
        let mut e = editor();
        let (c, v) = definitions(&mut e);
        let id = property(&mut e, c, v);
        let expected = e.snapshot.clone();
        run(&mut e, Action::ApplyProperty { id }).unwrap();
        assert_eq!(e.snapshot, expected);
        let form = forms(&e.snapshot).unwrap()[&id].clone();
        run(&mut e, Action::SetForm { id, form }).unwrap();
        assert_eq!(e.snapshot, expected);
        assert!(e.undo_model(&e.freshness()).unwrap());
        assert!(!load(&e.snapshot).unwrap().properties.contains_key(&id));
    }
}

#[cfg(test)]
mod p16_tests;

#[cfg(test)]
mod ports_tests;

#[cfg(test)]
mod connectors_tests;
