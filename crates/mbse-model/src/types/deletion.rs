//! Native and typed semantic deletion with sealed native+presentation impact and unknown-impact holds.
use super::*;
use crate::{
    native_views::{Stamp, Views},
    stage1::{Editor, Freshness, RecordKind},
};
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub ids: String,
    pub cascade: bool,
}
#[derive(Clone, Debug)]
pub enum Action {
    SetForm(Form),
    Apply(Box<Plan>),
}
#[derive(Clone, Debug, Serialize)]
pub struct Removed {
    pub id: Id,
    pub kind: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct ValueEntry {
    pub usage: Option<Id>,
    pub property: Id,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct UnknownImpact {
    pub count: usize,
    pub locations: Vec<String>,
}
impl UnknownImpact {
    fn add(&mut self, text: String) {
        self.count += 1;
        if self.locations.len() < 128 {
            self.locations.push(text);
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Summary {
    pub roots: Vec<Id>,
    pub removed: Vec<Removed>,
    pub incoming: Vec<refactor::Reference>,
    pub removed_values: Vec<ValueEntry>,
    pub removed_profile_applications:Vec<String>,
    pub retained_raw_forms: usize,
    pub unknown: UnknownImpact,
    pub complete_supported_reference_inventory: bool,
    pub native_occurrences: Vec<String>,
    pub native_edges: Vec<String>,
    pub owned_contexts: Vec<super::deletion_native::OwnedContext>,
    pub port_occurrences: Vec<String>,
    pub connector_routes: Vec<String>,
    pub refusal: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Plan {
    native: Freshness,
    digest: String,
    views: Stamp,
    views_digest: String,
    candidate: Snapshot,
    summary: Summary,
    admitted: bool,
}
#[derive(Clone, Debug)]
pub struct Prepared {
    summary: Summary,
    plan: Option<Plan>,
}
impl Prepared {
    pub fn summary(&self) -> &Summary {
        &self.summary
    }
    pub fn plan(&self) -> Option<&Plan> {
        self.plan.as_ref()
    }
}
impl Plan {
    pub fn check(&self, e: &Editor, v: &Views) -> Result<(), String> {
        if self.native != e.freshness() || self.digest != e.snapshot.digest() {
            return Err("DELETION_PREVIEW_STALE_OR_FOREIGN_INSTANCE".into());
        }
        if self.views != v.stamp(e) || self.views_digest != v.document().digest()? {
            return Err("DELETION_PREVIEW_STALE_OR_FOREIGN_VIEWS".into());
        }
        Ok(())
    }
    pub(crate) fn admit_views(&mut self, e: &Editor, v: &Views) -> Result<(), String> {
        self.check(e, v)?;
        self.admitted = true;
        Ok(())
    }
    pub(crate) fn check_editor(&self, e: &Editor) -> Result<(), String> {
        if !self.admitted {
            return Err("DELETION_REQUIRES_COUPLED_VIEWS_TRANSACTION".into());
        }
        if self.native != e.freshness() || self.digest != e.snapshot.digest() {
            return Err("DELETION_PREVIEW_STALE_OR_FOREIGN_INSTANCE".into());
        }
        Ok(())
    }
    pub fn summary(&self) -> &Summary {
        &self.summary
    }
}
pub fn form(s: &Snapshot) -> Result<Option<Form>, String> {
    Ok(journal(s)?.deletion)
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    if j.deletion.as_ref().is_some_and(|f| f.ids.len() > 4096) {
        return Err("DELETION_FORM_LIMIT".into());
    }
    Ok(())
}
fn canonical_id(text: &str) -> Result<Id, String> {
    let text = text.trim();
    let n = text
        .parse::<u64>()
        .map_err(|_| "DELETION_CANONICAL_ID_REQUIRED")?;
    if n == 0 || n.to_string() != text {
        return Err("DELETION_CANONICAL_ID_REQUIRED".into());
    }
    Ok(Id(n))
}
fn simple_native_field(key: &str) -> bool {
    matches!(
        key,
        "purpose"
            | "outcome"
            | "assessmentMethod"
            | "conditions"
            | "uncertainty"
            | "interest"
            | "text"
            | "shortLabel"
    )
}
fn native_field_known(
    record: &crate::stage1::Record,
    key: &str,
    value: &Value,
    snapshot: &Snapshot,
) -> bool {
    if simple_native_field(key) {
        return value.is_string();
    }
    if matches!(key, "context" | "observations" | "inputs")
        && value.as_array().is_some_and(|a| a.is_empty())
    {
        return true;
    }
    match (record.kind, key) {
        (RecordKind::Capability, "contributesTo") => true,
        (RecordKind::Objective, "subject") => {
            value.is_null()
                || value
                    .as_object()
                    .is_some_and(|m| m.keys().all(|k| matches!(k.as_str(), "kind" | "id")))
        }
        (
            RecordKind::Measure,
            "forObjective"
            | "criterion"
            | "comparator"
            | "quantity"
            | "unit"
            | "calculation"
            | "_kernspanCalculationRawOwners",
        ) => true,
        (RecordKind::Expectation, "expressedBy" | "concernsSystem") => true,
        (RecordKind::Stakeholder, "affiliations") => value.as_object().is_some_and(|m| {
            m.iter().all(|(k, v)| {
                canonical_id(k).ok().is_some_and(|id| {
                    snapshot.records.get(&RecordId(id.0)).is_some_and(|r| {
                        r.kind == RecordKind::Organization && r.scope == record.scope
                    })
                }) && matches!(v.as_str(), Some("Unknown" | "Internal" | "External"))
            })
        }),
        (RecordKind::Stakeholder, "actors") => value.as_array().is_some_and(|a| {
            a.iter().all(|v| {
                v.as_u64().is_some_and(|id| {
                    snapshot
                        .records
                        .get(&RecordId(id))
                        .is_some_and(|r| r.kind == RecordKind::Actor && r.scope == record.scope)
                })
            })
        }),
        (RecordKind::Environment, "context") => value.as_array().is_some_and(|a| {
            a.iter().all(|v| {
                v.as_object().is_some_and(|m| {
                    m.keys().all(|k| {
                        matches!(
                            k.as_str(),
                            "id" | "type" | "scope" | "name" | "text" | "kind"
                        )
                    })
                }) && v["type"] == "Context"
                    && v["name"].is_string()
                    && v["text"].is_string()
                    && matches!(
                        v["kind"].as_str(),
                        Some("Description" | "Assumption" | "AcceptedConstraint")
                    )
            })
        }),
        _ => false,
    }
}
fn raw_native_field_known(field: &str) -> bool {
    field == "name"
        || simple_native_field(field)
        || matches!(
            field,
            "criterion"
                | "valueExpression"
                | "contextName"
                | "contextText"
                | "calc.expression"
                | "calc.output.kind"
                | "calc.output.unit"
        )
        || ["calc.input.", "calc.unit."].iter().any(|prefix| {
            field.strip_prefix(prefix).is_some_and(|name| {
                !name.is_empty()
                    && name.len() <= 64
                    && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
        })
}
/// This is the admitted document's current reference schemas, not a standards verdict.
pub fn unknown_impact(e: &Editor) -> UnknownImpact {
    let mut unknown = UnknownImpact::default();
    for record in e.snapshot.records.values() {
        for (key, value) in &record.fields {
            if !native_field_known(record, key, value, &e.snapshot) {
                unknown.add(format!(
                    "Native record#{} field {} has uninterpreted impact",
                    record.id.0, key
                ));
            }
        }
    }
    for raw in &e.snapshot.drafts {
        if !raw_native_field_known(&raw.key.field) {
            unknown.add(format!(
                "Native raw field {} on#{} has uninterpreted impact",
                raw.key.field, raw.key.record.0
            ));
        }
    }
    for (key, draft) in &e.snapshot.creation_drafts {
        for field in draft.fields.keys() {
            if field != "name" && !simple_native_field(field) {
                unknown.add(format!(
                    "Native creation form {} field {} has uninterpreted impact",
                    key, field
                ));
            }
        }
    }
    if let Some(ext) = &e.snapshot.extensions {
        for key in ext.namespaces.keys() {
            if key != NAMESPACE && key != DRAFTS {
                unknown.add(format!("Opaque namespace {} has uninterpreted impact", key));
            }
        }
    }
    if let Ok(s)=load(&e.snapshot){for activity in s.activities.values(){unknown.add(format!("Activity#{} references are outside this deletion inventory; use activity authoring and preserve the original.",activity.id.0));}}
    if journal(&e.snapshot).is_ok_and(|j|j.activity_flow.is_some()||!j.activity_forms.is_empty()){unknown.add("Retained activity proposals have uninterpreted deletion impact; preserve or explicitly discard it.".into());}
    if let Ok(s)=load(&e.snapshot){for machine in s.state_machines.values(){unknown.add(format!("StateMachine#{} references are outside this deletion inventory; use state-machine authoring and preserve the original.",machine.id.0));}}
    if journal(&e.snapshot).is_ok_and(|j|!j.state_machine_forms.is_empty()){unknown.add("Retained state-machine proposals have uninterpreted deletion impact; preserve or explicitly discard them.".into());}
    unknown
}
fn members(s: &Store) -> BTreeMap<Id, Removed> {
    let mut out = BTreeMap::new();
    for d in s.definitions.values() {
        out.insert(
            d.id,
            Removed {
                id: d.id,
                kind: format!("{:?} definition", d.kind),
                name: d.name.clone(),
            },
        );
    }
    for p in s.properties.values() {
        out.insert(
            p.id,
            Removed {
                id: p.id,
                kind: "Property".into(),
                name: p.name.clone(),
            },
        );
    }
    for u in s.usages.values() {
        out.insert(
            u.id,
            Removed {
                id: u.id,
                kind: format!("{:?} usage", u.kind),
                name: u.name.clone(),
            },
        );
    }
    for i in s.interfaces.values() {
        out.insert(
            i.id,
            Removed {
                id: i.id,
                kind: "Interface".into(),
                name: i.name.clone(),
            },
        );
    }
    for p in s.ports.values() {
        out.insert(
            p.id,
            Removed {
                id: p.id,
                kind: "Port".into(),
                name: p.name.clone(),
            },
        );
    }
    for c in s.connectors.values() {
        out.insert(
            c.id,
            Removed {
                id: c.id,
                kind: "Connector".into(),
                name: c.name.clone(),
            },
        );
    }
    out
}
pub fn prepare(e: &Editor, v: &Views) -> Result<Prepared, String> {
    e.snapshot.validate()?;
    v.document().validate(&e.snapshot)?;
    let f = form(&e.snapshot)?.ok_or("DELETION_FORM_UNAVAILABLE")?;
    let list: Vec<_> = f
        .ids
        .split(',')
        .map(canonical_id)
        .collect::<Result<_, _>>()?;
    let roots: BTreeSet<_> = list.iter().copied().collect();
    if roots.is_empty() || roots.len() > 64 || roots.len() != list.len() {
        return Err("DELETION_ROOT_SELECTION_LIMIT_OR_DUPLICATE".into());
    }
    let before = load(&e.snapshot)?;
    let mut all = members(&before);
    for r in e.snapshot.records.values() {
        all.insert(
            Id(r.id.0),
            Removed {
                id: Id(r.id.0),
                kind: format!("Native {}", r.kind.label()),
                name: r.name.clone(),
            },
        );
    }
    let native_refs = super::deletion_native::references(&e.snapshot)?;
    if roots.iter().any(|id| !all.contains_key(id)) {
        return Err("DELETION_TARGET_MISSING_OR_KIND".into());
    }
    let mut removed = roots.clone();
    loop {
        let old = removed.len();
        // Explicit policy: delete referencing native owners, including calculation consumers.
        // Typed property overrides are value entries, so they do not delete their Parts.
        for r in &native_refs {
            if removed.contains(&r.target) {
                removed.insert(r.owner);
            }
        }
        for d in before.definitions.values() {
            if removed.contains(&Id(d.system.0)) {
                removed.insert(d.id);
            }
        }
        for i in before.interfaces.values() {
            if removed.contains(&Id(i.system.0)) {
                removed.insert(i.id);
            }
        }
        for p in before.properties.values() {
            if removed.contains(&p.owner) || removed.contains(&p.value_type) {
                removed.insert(p.id);
            }
        }
        for u in before.usages.values() {
            if removed.contains(&u.owner) || removed.contains(&u.target) {
                removed.insert(u.id);
            }
        }
        for i in before.interfaces.values() {
            if removed.contains(&i.payload) {
                removed.insert(i.id);
            }
        }
        for p in before.ports.values() {
            if removed.contains(&p.owner)
                || removed.contains(&p.contract)
                || p.assignment.is_some_and(|id| removed.contains(&id))
            {
                removed.insert(p.id);
            }
        }
        for c in before.connectors.values() {
            if [
                c.owner,
                c.source.usage,
                c.source.port,
                c.target.usage,
                c.target.port,
                c.conveyed,
            ]
            .iter()
            .any(|id| removed.contains(id))
            {
                removed.insert(c.id);
            }
        }
        if removed.len() == old {
            break;
        }
    }
    let mut values = Vec::new();
    for property in before.defaults.keys() {
        if removed.contains(property) {
            values.push(ValueEntry {
                usage: None,
                property: *property,
            });
        }
    }
    for (usage, map) in &before.overrides {
        for property in map.keys() {
            if removed.contains(usage) || removed.contains(property) {
                values.push(ValueEntry {
                    usage: Some(*usage),
                    property: *property,
                });
            }
        }
    }
    let j = journal(&e.snapshot)?;
    let retained = j.forms.len()
        + j.ports.len()
        + j.connectors.len()
        + j.values.len()
        + usize::from(j.definition.is_some())
        + usize::from(j.usage.is_some())
        + usize::from(j.interface.is_some())
        + usize::from(j.refactor.is_some())
        + usize::from(j.profile_tags.is_some())
        + usize::from(j.activity_flow.is_some())
        + j.activity_forms.len()
        + j.state_machine_forms.len()
        + 1;
    let affected_ports: BTreeSet<_> = v
        .document()
        .typed_ports
        .values()
        .filter(|o| removed.contains(&o.port) || o.usage.is_some_and(|id| removed.contains(&id)))
        .map(|o| o.id)
        .collect();
    let unknown = unknown_impact(e);
    let mut summary = Summary {
        roots: roots.iter().copied().collect(),
        removed: removed.iter().map(|id| all[id].clone()).collect(),
        incoming: refactor::references(&before)
            .into_iter()
            .chain(native_refs)
            .filter(|r| removed.contains(&r.target))
            .collect(),
        removed_values: values,
        removed_profile_applications:before.profile_applications.iter().filter(|(_,a)|removed.contains(&a.target)).map(|(key,_)|key.clone()).collect(),
        retained_raw_forms: retained,
        complete_supported_reference_inventory: unknown.count == 0,
        unknown,
        owned_contexts: super::deletion_native::contexts(&e.snapshot, &removed),
        native_occurrences: v
            .document()
            .occurrences
            .values()
            .filter(|o| removed.contains(&Id(o.record.0)))
            .map(|o| o.id.get().to_string())
            .collect(),
        native_edges: v
            .document()
            .edges
            .values()
            .filter(|edge| {
                removed.contains(&Id(edge.key.owner.0)) || removed.contains(&Id(edge.key.target.0))
            })
            .map(|edge| edge.id.get().to_string())
            .collect(),
        port_occurrences: affected_ports
            .iter()
            .map(|id| id.get().to_string())
            .collect(),
        connector_routes: v
            .document()
            .connector_edges
            .values()
            .filter(|edge| {
                removed.contains(&edge.connector)
                    || affected_ports.contains(&edge.source)
                    || affected_ports.contains(&edge.target)
            })
            .map(|edge| edge.id.get().to_string())
            .collect(),
        refusal: None,
    };
    if summary.unknown.count > 0 {
        summary.refusal=Some("Unknown impact prevents a complete verdict. Delete is unavailable; original data is retained.".into());
    } else if removed.len() + summary.owned_contexts.len() > 128
        || summary.removed_values.len() > 128
        || summary.removed_profile_applications.len()>128
        || summary.incoming.len() > 512
    {
        summary.refusal = Some(
            "Deletion exceeds the review limit: 128 records plus owned Contexts, 128 value entries, 128 profile applications or 512 references.".into(),
        );
    } else if !f.cascade {
        summary.refusal = Some(
            "Choose the listed dependent-record and value-entry treatment before Delete.".into(),
        );
    }
    let mut candidate = e.snapshot.clone();
    if summary.refusal.is_none() {
        candidate
            .records
            .retain(|id, _| !removed.contains(&Id(id.0)));
        let mut s = before;
        s.definitions.retain(|id, _| !removed.contains(id));
        s.properties.retain(|id, _| !removed.contains(id));
        s.usages.retain(|id, _| !removed.contains(id));
        s.interfaces.retain(|id, _| !removed.contains(id));
        s.ports.retain(|id, _| !removed.contains(id));
        s.connectors.retain(|id, _| !removed.contains(id));
        s.defaults.retain(|id, _| !removed.contains(id));
        s.profile_applications.retain(|_,a|!removed.contains(&a.target));
        s.overrides.retain(|usage, map| {
            if removed.contains(usage) {
                return false;
            }
            map.retain(|property, _| !removed.contains(property));
            !map.is_empty()
        });
        if let Err(error) = s
            .validate()
            .and_then(|_| put(&mut candidate, NAMESPACE, &s))
            .and_then(|_| candidate.validate())
        {
            summary.refusal = Some(error);
        }
    }
    let plan = if summary.refusal.is_none() {
        Some(Plan {
            native: e.freshness(),
            digest: e.snapshot.digest(),
            views: v.stamp(e),
            views_digest: v.document().digest()?,
            candidate,
            summary: summary.clone(),
            admitted: false,
        })
    } else {
        None
    };
    Ok(Prepared { summary, plan })
}
pub(super) fn apply_inner(s: &mut Snapshot, action: Action) -> Result<Change, String> {
    match action {
        Action::SetForm(form) => {
            let mut j = journal(s)?;
            j.deletion = Some(form);
            put(s, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Apply(plan) => {
            if !plan.admitted || plan.digest != s.digest() {
                return Err("DELETION_UNADMITTED_OR_STALE".into());
            }
            plan.candidate.validate()?;
            *s = plan.candidate;
            Ok(Change::Model)
        }
    }
}
