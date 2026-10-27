//! Finite documentary activity authoring under the existing native type authority.
use super::*;
pub mod forms;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Initial,
    Action,
    Decision,
    Merge,
    Final,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    In,
    Out,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FlowKind {
    Control,
    Object,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Guard {
    Boolean(bool),
    Else,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Partition {
    pub id: Id,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: Id,
    pub name: String,
    pub kind: NodeKind,
    pub partition: Option<Id>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub id: Id,
    pub owner: Id,
    pub name: String,
    pub direction: Direction,
    pub value_type: Id,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flow {
    pub id: Id,
    pub name: String,
    pub kind: FlowKind,
    pub source: Id,
    pub target: Id,
    pub guard: Option<Guard>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activity {
    pub id: Id,
    pub system: RecordId,
    pub name: String,
    pub partitions: BTreeMap<Id, Partition>,
    pub nodes: BTreeMap<Id, Node>,
    pub pins: BTreeMap<Id, Pin>,
    pub flows: BTreeMap<Id, Flow>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowForm {
    pub activity: Id,
    pub id: Option<Id>,
    pub name: String,
    pub kind: FlowKind,
    pub source: String,
    pub target: String,
    pub guard: String,
}
#[derive(Clone, Debug)]
pub enum Action {
    Form(forms::Action),
    Activity {
        id: Option<Id>,
        system: RecordId,
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
        kind: NodeKind,
        partition: Option<Id>,
    },
    Pin {
        activity: Id,
        id: Option<Id>,
        owner: Id,
        name: String,
        direction: Direction,
        value_type: Id,
    },
    SetFlowForm(FlowForm),
    ClearFlowForm,
    ApplyFlow,
    Remove {
        activity: Id,
        element: Id,
    },
}
/// Accepted IDs only, including owned members; never allocate a parallel ID domain.
pub fn ids(s: &Store) -> impl Iterator<Item = Id> + '_ {
    s.activities.values().flat_map(|a| {
        std::iter::once(a.id)
            .chain(a.partitions.keys().copied())
            .chain(a.nodes.keys().copied())
            .chain(a.pins.keys().copied())
            .chain(a.flows.keys().copied())
    })
}
pub(super) fn count(s: &Store) -> usize {
    ids(s).count()
}
pub(super) fn canonical_maps(v: &Value) -> Result<(), String> {
    if let Some(activities) = v.get("activities") {
        canonical(v, "activities")?;
        for a in activities.as_object().ok_or("ACTIVITY_MAP_SHAPE")?.values() {
            for f in ["partitions", "nodes", "pins", "flows"] {
                canonical(a, f)?;
            }
        }
    }
    Ok(())
}
pub fn flow_form(snapshot: &Snapshot) -> Result<Option<FlowForm>, String> {
    Ok(journal(snapshot)?.activity_flow)
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    forms::validate_journal(j)?;
    if let Some(f) = &j.activity_flow {
        if f.activity.0 == 0
            || f.id.is_some_and(|id| id.0 == 0)
            || [&f.name, &f.source, &f.target, &f.guard]
                .iter()
                .any(|s| s.len() > 4096)
        {
            return Err("ACTIVITY_FORM_SHAPE_OR_LIMIT".into());
        }
    }
    Ok(())
}
fn identity(key: &Id, id: Id, ids: &mut BTreeSet<Id>) -> Result<(), String> {
    if *key != id || id.0 == 0 || !ids.insert(id) {
        Err("ACTIVITY_DUPLICATE_OR_MISMATCHED_ID".into())
    } else {
        Ok(())
    }
}
pub(super) fn validate_store(s: &Store, ids: &mut BTreeSet<Id>) -> Result<(), String> {
    for (key, a) in &s.activities {
        identity(key, a.id, ids)?;
        name(&a.name)?;
        if a.system.0 == 0 {
            return Err("ACTIVITY_SYSTEM_ID".into());
        }
        for (key, p) in &a.partitions {
            identity(key, p.id, ids)?;
            name(&p.name)?;
        }
        let mut initial = 0;
        for (key, n) in &a.nodes {
            identity(key, n.id, ids)?;
            name(&n.name)?;
            if n.partition
                .is_some_and(|id| !a.partitions.contains_key(&id))
            {
                return Err("ACTIVITY_PARTITION_MISSING_OR_FOREIGN".into());
            }
            if n.kind == NodeKind::Initial {
                initial += 1;
            }
        }
        if initial > 1 {
            return Err("ACTIVITY_MULTIPLE_INITIAL_NODES".into());
        }
        for (key, p) in &a.pins {
            identity(key, p.id, ids)?;
            name(&p.name)?;
            if a.nodes
                .get(&p.owner)
                .is_none_or(|n| n.kind != NodeKind::Action)
            {
                return Err("ACTIVITY_PIN_OWNER_MISSING_OR_NOT_ACTION".into());
            }
            let ty = s
                .definitions
                .get(&p.value_type)
                .ok_or("ACTIVITY_PIN_TYPE_MISSING")?;
            if !matches!(ty.kind, Kind::Value(_)) || ty.system != a.system {
                return Err("ACTIVITY_PIN_TYPE_KIND_OR_SYSTEM".into());
            }
        }
        let mut endpoints = BTreeSet::new();
        let mut branches = BTreeSet::new();
        for (key, f) in &a.flows {
            identity(key, f.id, ids)?;
            name(&f.name)?;
            if !endpoints.insert((f.kind, f.source, f.target)) {
                return Err("ACTIVITY_DUPLICATE_FLOW_ENDPOINTS".into());
            }
            match f.kind {
                FlowKind::Control => {
                    let src = a
                        .nodes
                        .get(&f.source)
                        .ok_or("ACTIVITY_CONTROL_SOURCE_MISSING_OR_FOREIGN")?;
                    let dst = a
                        .nodes
                        .get(&f.target)
                        .ok_or("ACTIVITY_CONTROL_TARGET_MISSING_OR_FOREIGN")?;
                    if src.kind == NodeKind::Final || dst.kind == NodeKind::Initial {
                        return Err("ACTIVITY_CONTROL_DIRECTION".into());
                    }
                    if src.kind == NodeKind::Decision {
                        let g = f.guard.ok_or("ACTIVITY_DECISION_GUARD_REQUIRED")?;
                        if !branches.insert((f.source, g)) {
                            return Err("ACTIVITY_DUPLICATE_DECISION_GUARD".into());
                        }
                    } else if f.guard.is_some() {
                        return Err("ACTIVITY_GUARD_ONLY_ON_DECISION".into());
                    }
                }
                FlowKind::Object => {
                    if f.guard.is_some() {
                        return Err("ACTIVITY_OBJECT_GUARD_UNSUPPORTED".into());
                    }
                    let src = a
                        .pins
                        .get(&f.source)
                        .ok_or("ACTIVITY_OBJECT_SOURCE_MISSING_OR_FOREIGN")?;
                    let dst = a
                        .pins
                        .get(&f.target)
                        .ok_or("ACTIVITY_OBJECT_TARGET_MISSING_OR_FOREIGN")?;
                    if src.direction != Direction::Out || dst.direction != Direction::In {
                        return Err("ACTIVITY_OBJECT_PIN_DIRECTION".into());
                    }
                    if src.value_type != dst.value_type {
                        return Err("ACTIVITY_OBJECT_PAYLOAD_IDENTITY".into());
                    }
                }
            }
        }
    }
    Ok(())
}
pub(super) fn validate_targets(snapshot: &Snapshot, s: &Store) -> Result<(), String> {
    for a in s.activities.values() {
        if snapshot
            .records
            .get(&a.system)
            .is_none_or(|r| r.kind != RecordKind::System)
        {
            return Err("ACTIVITY_SYSTEM_MISSING_OR_WRONG_KIND".into());
        }
    }
    Ok(())
}
/// Detached journals survive Undo, but cannot appropriate another accepted identity.
pub(super) fn validate_draft_ids(
    s: &Store,
    j: &Journal,
    native: &BTreeSet<u64>,
) -> Result<u64, String> {
    let owned: BTreeSet<_> = ids(s).collect();
    let other = j
        .forms
        .keys()
        .chain(j.ports.keys())
        .chain(j.connectors.keys())
        .copied()
        .chain(j.usage.as_ref().and_then(|f| f.id));
    if other.into_iter().any(|id| owned.contains(&id)) {
        return Err("ACTIVITY_OTHER_DRAFT_ID_COLLISION".into());
    }
    let form_high=forms::validate_ids(s,j,native)?;
    let Some(f) = &j.activity_flow else {
        return Ok(form_high);
    };
    if let Some(id) = f.id {
        let matching = s
            .activities
            .get(&f.activity)
            .is_some_and(|a| a.flows.contains_key(&id));
        let occupied = native.contains(&id.0)
            || s.definitions.contains_key(&id)
            || s.properties.contains_key(&id)
            || s.usages.contains_key(&id)
            || s.interfaces.contains_key(&id)
            || s.ports.contains_key(&id)
            || s.connectors.contains_key(&id)
            || owned.contains(&id)
            || j.forms.contains_key(&id)
            || j.ports.contains_key(&id)
            || j.connectors.contains_key(&id)
            || j.usage.as_ref().is_some_and(|u| u.id == Some(id));
        if occupied && !matching {
            return Err("ACTIVITY_FLOW_DRAFT_ID_COLLISION".into());
        }
    }
    Ok(form_high.max(f.activity.0.max(f.id.map_or(0, |id| id.0))))
}
fn reference(raw: &str) -> Result<Id, String> {
    let n = raw
        .parse::<u64>()
        .map_err(|_| "ACTIVITY_CANONICAL_ENDPOINT_ID")?;
    if n == 0 || n.to_string() != raw {
        return Err("ACTIVITY_CANONICAL_ENDPOINT_ID".into());
    }
    Ok(Id(n))
}
fn guard(raw: &str) -> Result<Option<Guard>, String> {
    match raw {
        "" => Ok(None),
        "true" => Ok(Some(Guard::Boolean(true))),
        "false" => Ok(Some(Guard::Boolean(false))),
        "else" => Ok(Some(Guard::Else)),
        _ => Err(
            "ACTIVITY_LITERAL_GUARD: use empty, true, false or else; unsupported text retained"
                .into(),
        ),
    }
}
fn id_for(snapshot: &mut Snapshot, id: Option<Id>, exists: bool) -> Result<Id, String> {
    if let Some(id) = id {
        if !exists {
            return Err("ACTIVITY_DETACHED_EDIT: create explicitly instead".into());
        }
        Ok(id)
    } else {
        allocate(snapshot)
    }
}
pub(super) fn apply_inner(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut s = load(snapshot)?;
    match action {
        Action::Form(action)=>return forms::apply(snapshot,action),
        Action::Remove { activity, element } => {
            if element == activity {
                if s.activities.remove(&activity).is_none() {
                    return Err("ACTIVITY_MISSING".into());
                }
            } else {
                let a = s.activities.get_mut(&activity).ok_or("ACTIVITY_MISSING")?;
                let removed = a.flows.remove(&element).is_some()
                    || a.pins.remove(&element).is_some()
                    || a.nodes.remove(&element).is_some()
                    || a.partitions.remove(&element).is_some();
                if !removed {
                    return Err("ACTIVITY_ELEMENT_MISSING_OR_FOREIGN".into());
                }
            }
        }
        Action::ClearFlowForm => {
            let mut j = journal(snapshot)?;
            j.activity_flow = None;
            put(snapshot, DRAFTS, &j)?;
            return Ok(Change::Raw);
        }
        Action::SetFlowForm(f) => {
            let mut j = journal(snapshot)?;
            j.activity_flow = Some(f);
            put(snapshot, DRAFTS, &j)?;
            return Ok(Change::Raw);
        }
        Action::Activity {
            id,
            system,
            name: label,
        } => {
            name(&label)?;
            let id = id_for(
                snapshot,
                id,
                id.is_some_and(|id| s.activities.contains_key(&id)),
            )?;
            if let Some(a) = s.activities.get_mut(&id) {
                a.name = label;
                a.system = system;
            } else {
                s.activities.insert(
                    id,
                    Activity {
                        id,
                        system,
                        name: label,
                        partitions: BTreeMap::new(),
                        nodes: BTreeMap::new(),
                        pins: BTreeMap::new(),
                        flows: BTreeMap::new(),
                    },
                );
            }
        }
        Action::Partition {
            activity,
            id,
            name: label,
        } => {
            let a = s.activities.get_mut(&activity).ok_or("ACTIVITY_MISSING")?;
            let id = id_for(
                snapshot,
                id,
                id.is_some_and(|id| a.partitions.contains_key(&id)),
            )?;
            a.partitions.insert(id, Partition { id, name: label });
        }
        Action::Node {
            activity,
            id,
            name: label,
            kind,
            partition,
        } => {
            let a = s.activities.get_mut(&activity).ok_or("ACTIVITY_MISSING")?;
            let id = id_for(snapshot, id, id.is_some_and(|id| a.nodes.contains_key(&id)))?;
            a.nodes.insert(
                id,
                Node {
                    id,
                    name: label,
                    kind,
                    partition,
                },
            );
        }
        Action::Pin {
            activity,
            id,
            owner,
            name: label,
            direction,
            value_type,
        } => {
            let a = s.activities.get_mut(&activity).ok_or("ACTIVITY_MISSING")?;
            let id = id_for(snapshot, id, id.is_some_and(|id| a.pins.contains_key(&id)))?;
            a.pins.insert(
                id,
                Pin {
                    id,
                    owner,
                    name: label,
                    direction,
                    value_type,
                },
            );
        }
        Action::ApplyFlow => {
            let mut j = journal(snapshot)?;
            let mut f = j
                .activity_flow
                .clone()
                .ok_or("ACTIVITY_FLOW_FORM_UNAVAILABLE")?;
            let a = s
                .activities
                .get_mut(&f.activity)
                .ok_or("ACTIVITY_MISSING")?;
            let id = id_for(
                snapshot,
                f.id,
                f.id.is_some_and(|id| a.flows.contains_key(&id)),
            )?;
            let flow = Flow {
                id,
                name: f.name.clone(),
                kind: f.kind,
                source: reference(&f.source)?,
                target: reference(&f.target)?,
                guard: guard(&f.guard)?,
            };
            a.flows.insert(id, flow);
            s.validate()?;
            f.id = Some(id);
            j.activity_flow = Some(f);
            put(snapshot, NAMESPACE, &s)?;
            put(snapshot, DRAFTS, &j)?;
            return Ok(Change::Model);
        }
    }
    put(snapshot, NAMESPACE, &s)?;
    Ok(Change::Model)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub activity: Id,
    pub element: Option<Id>,
    pub message: &'static str,
}
/// Read-only authoring completeness, not execution, reachability or liveness proof.
pub fn diagnostics(s: &Store, activity: Id) -> Result<Vec<Diagnostic>, String> {
    let a = s.activities.get(&activity).ok_or("ACTIVITY_MISSING")?;
    let mut out = Vec::new();
    for (kind, code, message) in [
        (
            NodeKind::Initial,
            "ACTIVITY.NO_INITIAL",
            "No initial node; activity is incomplete.",
        ),
        (
            NodeKind::Final,
            "ACTIVITY.NO_FINAL",
            "No final node; activity is incomplete.",
        ),
    ] {
        if !a.nodes.values().any(|n| n.kind == kind) {
            out.push(Diagnostic {
                code,
                activity,
                element: None,
                message,
            });
        }
    }
    for n in a.nodes.values() {
        if !a
            .flows
            .values()
            .any(|f| f.kind == FlowKind::Control && (f.source == n.id || f.target == n.id))
        {
            out.push(Diagnostic {
                code: "ACTIVITY.UNCONNECTED_NODE",
                activity,
                element: Some(n.id),
                message: "Node has no control flow; no execution or liveness is inferred.",
            });
        }
    }
    Ok(out)
}
