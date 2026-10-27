//! Keyed raw activity proposals; existing Journal and Editor own durability/history.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Form {
    Activity {
        id: Option<Id>,
        system: String,
        name: String,
    },
    Partition {
        activity: Id,
        id: Option<Id>,
        name: String,
    },
    Node {
        activity: Id,
        id: Option<Id>,
        name: String,
        node_kind: NodeKind,
        partition: String,
    },
    Pin {
        activity: Id,
        id: Option<Id>,
        owner: String,
        name: String,
        direction: Direction,
        value_type: String,
    },
    Flow {
        form: FlowForm,
    },
}
#[derive(Clone, Debug)]
pub enum Action {
    Set(Form),
    Apply(String),
    Discard(String),
}
impl Form {
    pub fn id(&self) -> Option<Id> {
        match self {
            Self::Activity { id, .. }
            | Self::Partition { id, .. }
            | Self::Node { id, .. }
            | Self::Pin { id, .. } => *id,
            Self::Flow { form } => form.id,
        }
    }
    fn scope(&self) -> Option<Id> {
        match self {
            Self::Activity { .. } => None,
            Self::Partition { activity, .. }
            | Self::Node { activity, .. }
            | Self::Pin { activity, .. } => Some(*activity),
            Self::Flow { form } => Some(form.activity),
        }
    }
    pub fn key(&self) -> String {
        let id = self.id().map_or("new".into(), |id| id.0.to_string());
        let family = match self {
            Self::Activity { .. } => "activity",
            Self::Partition { .. } => "partition",
            Self::Node { .. } => "node",
            Self::Pin { .. } => "pin",
            Self::Flow { .. } => "flow",
        };
        self.scope().map_or_else(
            || format!("{family}:{id}"),
            |scope| format!("{family}:{}:{id}", scope.0),
        )
    }
    fn set_id(&mut self, new: Id) {
        match self {
            Self::Activity { id, .. }
            | Self::Partition { id, .. }
            | Self::Node { id, .. }
            | Self::Pin { id, .. } => *id = Some(new),
            Self::Flow { form } => form.id = Some(new),
        }
    }
    fn raw(&self) -> Vec<&str> {
        match self {
            Self::Activity { system, name, .. } => vec![system, name],
            Self::Partition { name, .. } => vec![name],
            Self::Node {
                name, partition, ..
            } => vec![name, partition],
            Self::Pin {
                owner,
                name,
                value_type,
                ..
            } => vec![owner, name, value_type],
            Self::Flow { form } => vec![&form.name, &form.source, &form.target, &form.guard],
        }
    }
    fn matching(&self, s: &Store, id: Id) -> bool {
        match self {
            Self::Activity { .. } => s.activities.contains_key(&id),
            Self::Partition { activity, .. } => s
                .activities
                .get(activity)
                .is_some_and(|a| a.partitions.contains_key(&id)),
            Self::Node { activity, .. } => s
                .activities
                .get(activity)
                .is_some_and(|a| a.nodes.contains_key(&id)),
            Self::Pin { activity, .. } => s
                .activities
                .get(activity)
                .is_some_and(|a| a.pins.contains_key(&id)),
            Self::Flow { form } => s
                .activities
                .get(&form.activity)
                .is_some_and(|a| a.flows.contains_key(&id)),
        }
    }
    fn concrete(&self) -> Result<super::Action, String> {
        Ok(match self {
            Self::Activity { id, system, name } => super::Action::Activity {
                id: *id,
                system: RecordId(reference(system)?.0),
                name: name.clone(),
            },
            Self::Partition { activity, id, name } => super::Action::Partition {
                activity: *activity,
                id: *id,
                name: name.clone(),
            },
            Self::Node {
                activity,
                id,
                name,
                node_kind,
                partition,
            } => super::Action::Node {
                activity: *activity,
                id: *id,
                name: name.clone(),
                kind: *node_kind,
                partition: if partition.is_empty() {
                    None
                } else {
                    Some(reference(partition)?)
                },
            },
            Self::Pin {
                activity,
                id,
                owner,
                name,
                direction,
                value_type,
            } => super::Action::Pin {
                activity: *activity,
                id: *id,
                owner: reference(owner)?,
                name: name.clone(),
                direction: *direction,
                value_type: reference(value_type)?,
            },
            Self::Flow { form } => super::Action::SetFlowForm(form.clone()),
        })
    }
}
fn effective(j: &Journal) -> Result<BTreeMap<String, Form>, String> {
    let mut map = j.activity_forms.clone();
    if let Some(f) = &j.activity_flow {
        let form = Form::Flow { form: f.clone() };
        let key = form.key();
        if map.get(&key).is_some_and(|old| old != &form) {
            return Err("ACTIVITY_CONFLICTING_LEGACY_PROPOSAL".into());
        }
        map.insert(key, form);
    }
    Ok(map)
}
pub fn load(snapshot: &Snapshot) -> Result<BTreeMap<String, Form>, String> {
    effective(&journal(snapshot)?)
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    let forms = effective(j)?;
    if forms.len() > 128 {
        return Err("ACTIVITY_FORM_COUNT_LIMIT".into());
    }
    let mut ids = BTreeSet::new();
    for (key, f) in forms {
        if key != f.key()
            || f.id().is_some_and(|id| id.0 == 0 || !ids.insert(id))
            || f.scope().is_some_and(|id| id.0 == 0)
            || f.raw().iter().any(|s| s.len() > 4096)
        {
            return Err("ACTIVITY_FORM_KEY_ID_OR_LIMIT".into());
        }
    }
    Ok(())
}
pub(super) fn validate_ids(s: &Store, j: &Journal, native: &BTreeSet<u64>) -> Result<u64, String> {
    let owned: BTreeSet<_> = super::ids(s).collect();
    let others: BTreeSet<_> = j
        .forms
        .keys()
        .chain(j.ports.keys())
        .chain(j.connectors.keys())
        .copied()
        .chain(j.usage.as_ref().and_then(|f| f.id))
        .collect();
    let mut high = 0;
    for f in effective(j)?.values() {
        high = high.max(f.scope().map_or(0, |id| id.0));
        if let Some(id) = f.id() {
            let occupied = native.contains(&id.0)
                || s.definitions.contains_key(&id)
                || s.properties.contains_key(&id)
                || s.usages.contains_key(&id)
                || s.interfaces.contains_key(&id)
                || s.ports.contains_key(&id)
                || s.connectors.contains_key(&id)
                || owned.contains(&id);
            if others.contains(&id) || (occupied && !f.matching(s, id)) {
                return Err("ACTIVITY_FORM_FOREIGN_ID_COLLISION".into());
            }
            high = high.max(id.0);
        }
    }
    Ok(high)
}
pub(super) fn apply(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut j = journal(snapshot)?;
    j.activity_forms = effective(&j)?;
    j.activity_flow = None;
    match action {
        Action::Set(form) => {
            j.activity_forms.insert(form.key(), form);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Discard(key) => {
            if j.activity_forms.remove(&key).is_none() {
                return Err("ACTIVITY_FORM_UNAVAILABLE".into());
            }
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Apply(key) => {
            let mut form = j
                .activity_forms
                .get(&key)
                .cloned()
                .ok_or("ACTIVITY_FORM_UNAVAILABLE")?;
            let action = form.concrete()?;
            let allocated = form.id().unwrap_or(Id(snapshot.next_id));
            // Replace the consumed slot inside this atomic candidate before the
            // legacy flow bridge stages its accepted-ID proposal at the quota.
            j.activity_forms.remove(&key);
            put(snapshot, DRAFTS, &j)?;
            super::apply_inner(snapshot, action)?;
            if matches!(form, Form::Flow { .. }) {
                super::apply_inner(snapshot, super::Action::ApplyFlow)?;
            }
            let mut after = journal(snapshot)?;
            after.activity_flow = None;
            after.activity_forms.remove(&key);
            form.set_id(allocated);
            after.activity_forms.insert(form.key(), form);
            put(snapshot, DRAFTS, &after)?;
            Ok(Change::Model)
        }
    }
}
