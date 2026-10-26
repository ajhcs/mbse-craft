//! Exact app-owned profile identities and primitive literal metadata, never reclassification.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TargetKind {
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
    ComponentDefinition,
    ValueDefinition,
    Property,
    Part,
    Reference,
    Interface,
    Port,
    Connector,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub namespace: String,
    pub version: String,
}
impl Identity {
    pub fn key(&self) -> String {
        format!("{}|{}", self.namespace, self.version)
    }
    fn validate(&self) -> Result<(), String> {
        if self.namespace.is_empty()
            || self.namespace.len() > 256
            || !self.namespace.contains(':')
            || !self
                .namespace
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b":/._-".contains(&c))
            || self.version.is_empty()
            || self.version.len() > 64
            || !self
                .version
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-+".contains(&c))
        {
            return Err("PROFILE_EXACT_IDENTITY_REQUIRED".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stereotype {
    pub applies_to: BTreeSet<TargetKind>,
    pub tags: BTreeMap<String, Scalar>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub identity: Identity,
    pub stereotypes: BTreeMap<String, Stereotype>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Application {
    pub target: Id,
    pub expected: TargetKind,
    pub profile: Identity,
    pub stereotype: String,
    pub values: BTreeMap<String, values::Literal>,
}
impl Application {
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.target.0,
            self.profile.key(),
            self.stereotype
        )
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub profile_json: String,
    pub target: String,
    pub namespace: String,
    pub version: String,
    pub stereotype: String,
    pub values_json: String,
}
#[derive(Clone, Debug)]
pub enum Action {
    SetForm(Form),
    Register,
    Apply,
    Remove,
}
fn identifier(text: &str) -> Result<(), String> {
    if text.is_empty()
        || text.len() > 128
        || !text
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err("PROFILE_TAG_IDENTIFIER".into());
    }
    Ok(())
}
fn profile(p: &Profile) -> Result<(), String> {
    p.identity.validate()?;
    if p.stereotypes.is_empty() || p.stereotypes.len() > 32 {
        return Err("PROFILE_STEREOTYPE_LIMIT".into());
    }
    for (name, s) in &p.stereotypes {
        identifier(name)?;
        if s.applies_to.is_empty() || s.tags.len() > 32 {
            return Err("PROFILE_APPLICABILITY_OR_TAG_LIMIT".into());
        }
        for key in s.tags.keys() {
            identifier(key)?;
        }
    }
    Ok(())
}
pub(super) fn validate_store(store: &Store) -> Result<(), String> {
    if store.profiles.len() > 16 || store.profile_applications.len() > 256 {
        return Err("PROFILE_REGISTRY_OR_APPLICATION_LIMIT".into());
    }
    for (key, p) in &store.profiles {
        profile(p)?;
        if key != &p.identity.key() {
            return Err("PROFILE_IDENTITY_KEY_MISMATCH".into());
        }
    }
    for (key, a) in &store.profile_applications {
        a.profile.validate()?;
        identifier(&a.stereotype)?;
        if a.target.0 == 0 || key != &a.key() {
            return Err("PROFILE_APPLICATION_IDENTITY_MISMATCH".into());
        }
        let p = store
            .profiles
            .get(&a.profile.key())
            .ok_or("PROFILE_EXACT_NAMESPACE_VERSION_UNKNOWN")?;
        let s = p
            .stereotypes
            .get(&a.stereotype)
            .ok_or("PROFILE_EXACT_STEREOTYPE_UNKNOWN")?;
        if !s.applies_to.contains(&a.expected) {
            return Err("PROFILE_TARGET_KIND_REFUSED".into());
        }
        if a.values.keys().ne(s.tags.keys()) {
            return Err("PROFILE_EXACT_TAG_KEYS_REQUIRED".into());
        }
        for (key, value) in &a.values {
            if value.kind != s.tags[key] {
                return Err("PROFILE_LITERAL_KIND_MISMATCH".into());
            }
            let admitted = values::literal(value.kind, &value.text)?;
            if admitted.text != value.text {
                return Err("PROFILE_CANONICAL_LITERAL_REQUIRED".into());
            }
        }
    }
    Ok(())
}
pub fn target_kind(snapshot: &Snapshot, store: &Store, id: Id) -> Result<TargetKind, String> {
    use crate::stage1::RecordKind as N;
    if let Some(r) = snapshot.records.get(&RecordId(id.0)) {
        return Ok(match r.kind {
            N::System => TargetKind::System,
            N::Mission => TargetKind::Mission,
            N::Capability => TargetKind::Capability,
            N::Objective => TargetKind::Objective,
            N::Measure => TargetKind::Measure,
            N::Stakeholder => TargetKind::Stakeholder,
            N::Expectation => TargetKind::Expectation,
            N::Environment => TargetKind::Environment,
            N::Organization => TargetKind::Organization,
            N::Actor => TargetKind::Actor,
        });
    }
    if let Some(d) = store.definitions.get(&id) {
        return Ok(if d.kind == Kind::Component {
            TargetKind::ComponentDefinition
        } else {
            TargetKind::ValueDefinition
        });
    }
    if store.properties.contains_key(&id) {
        return Ok(TargetKind::Property);
    }
    if let Some(u) = store.usages.get(&id) {
        return Ok(if u.kind == UsageKind::Part {
            TargetKind::Part
        } else {
            TargetKind::Reference
        });
    }
    if store.interfaces.contains_key(&id) {
        return Ok(TargetKind::Interface);
    }
    if store.ports.contains_key(&id) {
        return Ok(TargetKind::Port);
    }
    if store.connectors.contains_key(&id) {
        return Ok(TargetKind::Connector);
    }
    Err("PROFILE_TARGET_MISSING_OR_UNSUPPORTED_KIND".into())
}
pub(super) fn validate_targets(snapshot: &Snapshot, store: &Store) -> Result<(), String> {
    for a in store.profile_applications.values() {
        if target_kind(snapshot, store, a.target)? != a.expected {
            return Err("PROFILE_TARGET_KIND_CHANGED".into());
        }
    }
    Ok(())
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    if let Some(f) = &j.profile_tags {
        if f.profile_json.len() > 32768
            || f.values_json.len() > 32768
            || [&f.target, &f.namespace, &f.version, &f.stereotype]
                .iter()
                .any(|s| s.len() > 4096)
        {
            return Err("PROFILE_RAW_FORM_LIMIT".into());
        }
    }
    Ok(())
}
pub fn form(s: &Snapshot) -> Result<Option<Form>, String> {
    Ok(journal(s)?.profile_tags)
}
fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    if text.len() > 32768 {
        return Err("PROFILE_JSON_LIMIT".into());
    }
    let unique: crate::stage1_io::UniqueValue =
        serde_json::from_str(text).map_err(|e| format!("PROFILE_JSON: {e}"))?;
    serde_json::from_value(unique.0).map_err(|e| format!("PROFILE_SCHEMA: {e}"))
}
fn application(s: &Snapshot, store: &Store, f: &Form) -> Result<Application, String> {
    let n = f
        .target
        .parse::<u64>()
        .map_err(|_| "PROFILE_CANONICAL_TARGET_REQUIRED")?;
    if n == 0 || n.to_string() != f.target {
        return Err("PROFILE_CANONICAL_TARGET_REQUIRED".into());
    }
    let target = Id(n);
    Ok(Application {
        target,
        expected: target_kind(s, store, target)?,
        profile: Identity {
            namespace: f.namespace.clone(),
            version: f.version.clone(),
        },
        stereotype: f.stereotype.clone(),
        values: parse(&f.values_json)?,
    })
}
pub(super) fn apply_inner(s: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut store = load(s)?;
    match action {
        Action::SetForm(f) => {
            let mut j = journal(s)?;
            j.profile_tags = Some(f);
            put(s, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Register => {
            let f = form(s)?.ok_or("PROFILE_FORM_UNAVAILABLE")?;
            let p: Profile = parse(&f.profile_json)?;
            profile(&p)?;
            let key = p.identity.key();
            if store.profiles.get(&key).is_some_and(|old| old != &p) {
                return Err("PROFILE_EXACT_IDENTITY_CONFLICT_IMMUTABLE".into());
            }
            store.profiles.insert(key, p);
            store.validate()?;
            put(s, NAMESPACE, &store)?;
            Ok(Change::Model)
        }
        Action::Apply => {
            let f = form(s)?.ok_or("PROFILE_FORM_UNAVAILABLE")?;
            let a = application(s, &store, &f)?;
            store.profile_applications.insert(a.key(), a);
            store.validate()?;
            validate_targets(s, &store)?;
            put(s, NAMESPACE, &store)?;
            Ok(Change::Model)
        }
        Action::Remove => {
            let f = form(s)?.ok_or("PROFILE_FORM_UNAVAILABLE")?;
            let n = f
                .target
                .parse::<u64>()
                .map_err(|_| "PROFILE_CANONICAL_TARGET_REQUIRED")?;
            if n == 0 || n.to_string() != f.target {
                return Err("PROFILE_CANONICAL_TARGET_REQUIRED".into());
            }
            let identity = Identity {
                namespace: f.namespace,
                version: f.version,
            };
            identity.validate()?;
            identifier(&f.stereotype)?;
            let key = format!("{}|{}|{}", n, identity.key(), f.stereotype);
            if store.profile_applications.remove(&key).is_none() {
                return Err("PROFILE_EXACT_APPLICATION_UNKNOWN".into());
            }
            put(s, NAMESPACE, &store)?;
            Ok(Change::Model)
        }
    }
}
/// Literal metadata has one target attachment and no hidden model-reference semantics.
pub(super) fn copy_applications(
    store: &mut Store,
    before: &Store,
    map: &BTreeMap<Id, Id>,
) -> Vec<String> {
    let mut copied = Vec::new();
    for a in before.profile_applications.values() {
        if let Some(id) = map.get(&a.target) {
            let mut copy = a.clone();
            copy.target = *id;
            let key = copy.key();
            store.profile_applications.insert(key.clone(), copy);
            copied.push(key);
        }
    }
    copied
}
/// UI read-only eligibility uses the same candidate validation as publication.
pub fn preview(s: &Snapshot, action: Action) -> Result<(), String> {
    let mut candidate = s.clone();
    super::apply(&mut candidate, super::Action::ProfileTag(action))?;
    Ok(())
}
