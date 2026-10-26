//! Sealed accepted-model clone/move candidates with explicit reference treatment.
use super::*;
use crate::stage1::{Editor, Freshness};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    CloneDefinition,
    MoveUsages,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub mode: Mode,
    pub definition: String,
    pub name: String,
    pub usages: String,
    pub owner: String,
    pub keep_external: bool,
}
impl Default for Form {
    fn default() -> Self {
        Self {
            mode: Mode::CloneDefinition,
            definition: String::new(),
            name: String::new(),
            usages: String::new(),
            owner: String::new(),
            keep_external: false,
        }
    }
}
#[derive(Clone, Debug)]
pub enum Action {
    SetForm(Form),
    Apply(Box<Plan>),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Reference {
    pub owner: Id,
    pub role: String,
    pub target: Id,
}
#[derive(Clone, Debug, Serialize)]
pub struct Summary {
    pub operation: String,
    pub id_mapping: BTreeMap<Id, Id>,
    pub moved_usages: Vec<Id>,
    pub moved_connectors: Vec<Id>,
    pub boundary_connectors: Vec<Id>,
    pub internal_remapped: Vec<Reference>,
    pub external_targets_retained: Vec<Reference>,
    pub incoming_targets_retained: Vec<Reference>,
    pub uninterpreted_locations: Vec<String>,
    pub retained_native_field_count: usize,
    pub exhaustive_global_reference_verdict: bool,
    pub refusal: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Plan {
    native: Freshness,
    digest: String,
    candidate: Snapshot,
    summary: Summary,
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
    pub fn summary(&self) -> &Summary {
        &self.summary
    }
    pub fn check(&self, e: &Editor) -> Result<(), String> {
        if self.native != e.freshness() || self.digest != e.snapshot.digest() {
            return Err("REFACTOR_PREVIEW_STALE_OR_FOREIGN_INSTANCE".into());
        }
        Ok(())
    }
}
pub fn form(s: &Snapshot) -> Result<Option<Form>, String> {
    Ok(journal(s)?.refactor)
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    if j.refactor.as_ref().is_some_and(|f| {
        [&f.definition, &f.name, &f.usages, &f.owner]
            .iter()
            .any(|s| s.len() > 4096)
    }) {
        return Err("REFACTOR_FORM_LIMIT".into());
    }
    Ok(())
}
fn reference(text: &str) -> Result<Id, String> {
    let text = text.trim();
    let id = text
        .parse::<u64>()
        .map_err(|_| "REFACTOR_CANONICAL_ID_REQUIRED")?;
    if id == 0 || id.to_string() != text {
        return Err("REFACTOR_CANONICAL_ID_REQUIRED".into());
    }
    Ok(Id(id))
}
fn references(s: &Store) -> Vec<Reference> {
    let mut rows = Vec::new();
    let mut add = |owner, role: &str, target| {
        rows.push(Reference {
            owner,
            role: role.into(),
            target,
        })
    };
    for d in s.definitions.values() {
        add(d.id, "System membership", Id(d.system.0));
    }
    for p in s.properties.values() {
        add(p.id, "Component owner", p.owner);
        add(p.id, "Value type", p.value_type);
    }
    for u in s.usages.values() {
        add(u.id, "Component owner", u.owner);
        add(u.id, "Usage type", u.target);
    }
    for i in s.interfaces.values() {
        add(i.id, "System membership", Id(i.system.0));
        add(i.id, "Interface payload", i.payload);
    }
    for p in s.ports.values() {
        add(p.id, "Component owner", p.owner);
        add(p.id, "Port contract", p.contract);
        if let Some(a) = p.assignment {
            add(p.id, "Assigned interface", a);
        }
    }
    for c in s.connectors.values() {
        for (role, target) in [
            ("Component owner", c.owner),
            ("Source Part", c.source.usage),
            ("Source Port", c.source.port),
            ("Target Part", c.target.usage),
            ("Target Port", c.target.port),
            ("Conveyed type", c.conveyed),
        ] {
            add(c.id, role, target);
        }
    }
    for (usage, values) in &s.overrides {
        for property in values.keys() {
            add(*usage, "Override property", *property);
        }
    }
    rows
}
fn empty_summary(snapshot: &Snapshot, mode: Mode) -> Summary {
    let field_count = snapshot.records.values().map(|r| r.fields.len()).sum();
    let mut outside: Vec<_> = snapshot
        .records
        .values()
        .flat_map(|r| {
            r.fields.keys().map(move |key| {
                format!(
                    "Native record#{} field {} retained outside typed reference scope",
                    r.id.0, key
                )
            })
        })
        .take(128)
        .collect();
    if let Some(e) = &snapshot.extensions {
        outside.extend(
            e.namespaces
                .keys()
                .filter(|k| k.as_str() != NAMESPACE && k.as_str() != DRAFTS)
                .map(|k| format!("Opaque namespace {k} retained unchanged")),
        );
    }
    Summary {
        operation: match mode {
            Mode::CloneDefinition => "Clone definition and owned accepted members",
            Mode::MoveUsages => "Move usages and enclosed connectors",
        }
        .into(),
        id_mapping: BTreeMap::new(),
        moved_usages: Vec::new(),
        moved_connectors: Vec::new(),
        boundary_connectors: Vec::new(),
        internal_remapped: Vec::new(),
        external_targets_retained: Vec::new(),
        incoming_targets_retained: Vec::new(),
        uninterpreted_locations: outside,
        retained_native_field_count: field_count,
        exhaustive_global_reference_verdict: false,
        refusal: None,
    }
}
fn owned(s: &Store, root: Id) -> BTreeSet<Id> {
    let mut ids = BTreeSet::from([root]);
    ids.extend(
        s.properties
            .values()
            .filter(|p| p.owner == root)
            .map(|p| p.id),
    );
    ids.extend(s.usages.values().filter(|p| p.owner == root).map(|p| p.id));
    ids.extend(s.ports.values().filter(|p| p.owner == root).map(|p| p.id));
    ids.extend(
        s.connectors
            .values()
            .filter(|p| p.owner == root)
            .map(|p| p.id),
    );
    ids
}
pub fn prepare(e: &Editor) -> Result<Prepared, String> {
    e.snapshot.validate()?;
    let f = form(&e.snapshot)?.ok_or("REFACTOR_FORM_UNAVAILABLE")?;
    let before = load(&e.snapshot)?;
    let mut store = before.clone();
    let mut candidate = e.snapshot.clone();
    let mut summary = empty_summary(&e.snapshot, f.mode);
    let refs = references(&before);
    match f.mode {
        Mode::CloneDefinition => {
            let root = reference(&f.definition)?;
            let definition = before
                .definitions
                .get(&root)
                .ok_or("REFACTOR_DEFINITION_MISSING_OR_KIND")?;
            name(&f.name)?;
            let members = owned(&before, root);
            if members.len() > 128 {
                return Err("REFACTOR_CLONE_MEMBER_LIMIT".into());
            }
            for id in &members {
                summary.id_mapping.insert(*id, allocate(&mut candidate)?);
            }
            let map = |id: Id| summary.id_mapping.get(&id).copied().unwrap_or(id);
            let mut copy = definition.clone();
            copy.id = map(root);
            copy.name = f.name.clone();
            store.definitions.insert(copy.id, copy);
            for p in before
                .properties
                .values()
                .filter(|p| members.contains(&p.id))
            {
                let mut p = p.clone();
                p.id = map(p.id);
                p.owner = map(p.owner);
                p.value_type = map(p.value_type);
                store.properties.insert(p.id, p);
            }
            for u in before.usages.values().filter(|u| members.contains(&u.id)) {
                let mut u = u.clone();
                u.id = map(u.id);
                u.owner = map(u.owner);
                u.target = map(u.target);
                store.usages.insert(u.id, u);
            }
            for p in before.ports.values().filter(|p| members.contains(&p.id)) {
                let mut p = p.clone();
                p.id = map(p.id);
                p.owner = map(p.owner);
                p.contract = map(p.contract);
                p.assignment = p.assignment.map(map);
                store.ports.insert(p.id, p);
            }
            for c in before
                .connectors
                .values()
                .filter(|c| members.contains(&c.id))
            {
                let mut c = c.clone();
                c.id = map(c.id);
                c.owner = map(c.owner);
                c.source.usage = map(c.source.usage);
                c.source.port = map(c.source.port);
                c.target.usage = map(c.target.usage);
                c.target.port = map(c.target.port);
                c.conveyed = map(c.conveyed);
                store.connectors.insert(c.id, c);
            }
            for (id, value) in &before.defaults {
                if members.contains(id) {
                    store.defaults.insert(map(*id), value.clone());
                }
            }
            for (usage, values) in &before.overrides {
                if members.contains(usage) {
                    store.overrides.insert(
                        map(*usage),
                        values.iter().map(|(p, v)| (map(*p), v.clone())).collect(),
                    );
                }
            }
            for r in refs {
                if members.contains(&r.owner) {
                    if members.contains(&r.target) {
                        summary.internal_remapped.push(Reference {
                            owner: map(r.owner),
                            role: r.role,
                            target: map(r.target),
                        });
                    } else {
                        summary.external_targets_retained.push(Reference {
                            owner: map(r.owner),
                            role: r.role,
                            target: r.target,
                        });
                    }
                } else if members.contains(&r.target) {
                    summary.incoming_targets_retained.push(r);
                }
            }
        }
        Mode::MoveUsages => {
            let owner = reference(&f.owner)?;
            if before
                .definitions
                .get(&owner)
                .is_none_or(|d| d.kind != Kind::Component)
            {
                return Err("REFACTOR_MOVE_OWNER_KIND".into());
            }
            let parsed: Vec<_> = f
                .usages
                .split(',')
                .map(reference)
                .collect::<Result<_, _>>()?;
            let selected: BTreeSet<_> = parsed.iter().copied().collect();
            if selected.is_empty() || selected.len() != parsed.len() || selected.len() > 64 {
                return Err("REFACTOR_MOVE_SELECTION_LIMIT_OR_DUPLICATE".into());
            }
            let mut changed = BTreeSet::new();
            for id in &selected {
                let usage = store
                    .usages
                    .get_mut(id)
                    .ok_or("REFACTOR_MOVE_USAGE_MISSING_OR_KIND")?;
                if usage.owner != owner {
                    changed.insert(*id);
                }
                usage.owner = owner;
                summary.moved_usages.push(*id);
            }
            for c in store.connectors.values_mut() {
                let source = changed.contains(&c.source.usage);
                let target = changed.contains(&c.target.usage);
                if source && target {
                    c.owner = owner;
                    summary.moved_connectors.push(c.id);
                } else if source || target {
                    summary.boundary_connectors.push(c.id);
                }
            }
            for r in refs {
                if changed.contains(&r.owner) || summary.moved_connectors.contains(&r.owner) {
                    if r.role == "Component owner" {
                        summary.internal_remapped.push(Reference {
                            owner: r.owner,
                            role: r.role,
                            target: owner,
                        });
                    } else {
                        summary.external_targets_retained.push(r);
                    }
                } else if selected.contains(&r.target) {
                    summary.incoming_targets_retained.push(r);
                }
            }
            if !summary.boundary_connectors.is_empty() {
                summary.refusal=Some("Move crosses connector ownership: select both endpoint Parts or cancel; no connector will be deleted or retargeted implicitly.".into());
            }
            if changed.is_empty() {
                summary.refusal = Some("Selected usages already have this owner.".into());
            }
        }
    }
    if !f.keep_external {
        summary.refusal=Some("Choose to keep existing external target identities before preparing an applicable preview.".into());
    }
    if summary.refusal.is_none() {
        if let Err(error) = store
            .validate()
            .and_then(|_| put(&mut candidate, NAMESPACE, &store))
            .and_then(|_| candidate.validate())
        {
            summary.refusal = Some(error);
        }
    }
    let plan = if summary.refusal.is_none() {
        Some(Plan {
            native: e.freshness(),
            digest: e.snapshot.digest(),
            candidate,
            summary: summary.clone(),
        })
    } else {
        None
    };
    Ok(Prepared { summary, plan })
}
pub(super) fn apply_inner(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    match action {
        Action::SetForm(form) => {
            let mut j = journal(snapshot)?;
            j.refactor = Some(form);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Apply(plan) => {
            if plan.digest != snapshot.digest() {
                return Err("REFACTOR_PREVIEW_STALE".into());
            }
            plan.candidate.validate()?;
            *snapshot = plan.candidate;
            Ok(Change::Model)
        }
    }
}
