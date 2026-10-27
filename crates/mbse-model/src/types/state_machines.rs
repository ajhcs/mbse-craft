//! Flat app-owned state-machine authoring in the existing native type authority.
//! No execution, hierarchy, orthogonal regions, action code or normative UML claim.
pub use super::activities::Guard;
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateKind {
    Initial,
    State,
    Final,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub id: Id,
    pub name: String,
    pub kind: StateKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub id: Id,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub id: Id,
    pub name: String,
    pub source: Id,
    pub target: Id,
    pub event: Option<Id>,
    pub guard: Option<Guard>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    pub id: Id,
    pub system: RecordId,
    pub name: String,
    pub states: BTreeMap<Id, State>,
    pub events: BTreeMap<Id, Event>,
    pub transitions: BTreeMap<Id, Transition>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Form {
    Machine {
        id: Option<Id>,
        system: String,
        name: String,
    },
    State {
        machine: Id,
        id: Option<Id>,
        name: String,
        state_kind: StateKind,
    },
    Event {
        machine: Id,
        id: Option<Id>,
        name: String,
    },
    Transition {
        machine: Id,
        id: Option<Id>,
        name: String,
        source: String,
        target: String,
        event: String,
        guard: String,
    },
}
#[derive(Clone, Debug)]
pub enum Action {
    Set(Form),
    Apply(String),
    Discard(String),
    Remove { machine: Id, element: Id },
}
impl Form {
    pub fn id(&self) -> Option<Id> {
        match self {
            Self::Machine { id, .. }
            | Self::State { id, .. }
            | Self::Event { id, .. }
            | Self::Transition { id, .. } => *id,
        }
    }
    pub fn machine(&self) -> Option<Id> {
        match self {
            Self::Machine { .. } => None,
            Self::State { machine, .. }
            | Self::Event { machine, .. }
            | Self::Transition { machine, .. } => Some(*machine),
        }
    }
    pub fn key(&self) -> String {
        let family = match self {
            Self::Machine { .. } => "machine",
            Self::State { .. } => "state",
            Self::Event { .. } => "event",
            Self::Transition { .. } => "transition",
        };
        let id = self.id().map_or("new".into(), |id| id.0.to_string());
        self.machine().map_or_else(
            || format!("{family}:{id}"),
            |a| format!("{family}:{}:{id}", a.0),
        )
    }
    fn set_id(&mut self, new: Id) {
        match self {
            Self::Machine { id, .. }
            | Self::State { id, .. }
            | Self::Event { id, .. }
            | Self::Transition { id, .. } => *id = Some(new),
        }
    }
    fn raw(&self) -> Vec<&str> {
        match self {
            Self::Machine { system, name, .. } => vec![system, name],
            Self::State { name, .. } | Self::Event { name, .. } => vec![name],
            Self::Transition {
                name,
                source,
                target,
                event,
                guard,
                ..
            } => vec![name, source, target, event, guard],
        }
    }
    fn matching(&self, s: &Store, id: Id) -> bool {
        match self {
            Self::Machine { .. } => s.state_machines.contains_key(&id),
            Self::State { machine, .. } => s
                .state_machines
                .get(machine)
                .is_some_and(|a| a.states.contains_key(&id)),
            Self::Event { machine, .. } => s
                .state_machines
                .get(machine)
                .is_some_and(|a| a.events.contains_key(&id)),
            Self::Transition { machine, .. } => s
                .state_machines
                .get(machine)
                .is_some_and(|a| a.transitions.contains_key(&id)),
        }
    }
}
pub fn forms(snapshot: &Snapshot) -> Result<BTreeMap<String, Form>, String> {
    Ok(journal(snapshot)?.state_machine_forms)
}
pub fn ids(s: &Store) -> impl Iterator<Item = Id> + '_ {
    s.state_machines.values().flat_map(|a| {
        std::iter::once(a.id)
            .chain(a.states.keys().copied())
            .chain(a.events.keys().copied())
            .chain(a.transitions.keys().copied())
    })
}
pub(super) fn canonical_maps(v: &Value) -> Result<(), String> {
    if let Some(map) = v.get("state_machines") {
        canonical(v, "state_machines")?;
        for a in map.as_object().ok_or("SM_SHAPE")?.values() {
            for k in ["states", "events", "transitions"] {
                canonical(a, k)?;
            }
        }
    }
    Ok(())
}
fn identity(key: &Id, id: Id, all: &mut BTreeSet<Id>) -> Result<(), String> {
    if *key != id || id.0 == 0 || !all.insert(id) {
        return Err("SM_ID_COLLISION_OR_MAP_KEY".into());
    }
    Ok(())
}
pub(super) fn validate_store(s: &Store, all: &mut BTreeSet<Id>) -> Result<(), String> {
    for (key, a) in &s.state_machines {
        identity(key, a.id, all)?;
        name(&a.name)?;
        if a.system.0 == 0 {
            return Err("SM_SYSTEM_ID".into());
        }
        let mut initial = 0;
        for (key, n) in &a.states {
            identity(key, n.id, all)?;
            name(&n.name)?;
            if n.kind == StateKind::Initial {
                initial += 1;
            }
        }
        if initial > 1 {
            return Err("SM_MULTIPLE_INITIAL_STATES".into());
        }
        for (key, e) in &a.events {
            identity(key, e.id, all)?;
            name(&e.name)?;
        }
        for (key, t) in &a.transitions {
            identity(key, t.id, all)?;
            name(&t.name)?;
            let src = a
                .states
                .get(&t.source)
                .ok_or("SM_TRANSITION_SOURCE_MISSING_OR_FOREIGN")?;
            let dst = a
                .states
                .get(&t.target)
                .ok_or("SM_TRANSITION_TARGET_MISSING_OR_FOREIGN")?;
            if src.kind == StateKind::Final || dst.kind == StateKind::Initial {
                return Err("SM_TRANSITION_DIRECTION".into());
            }
            if t.event.is_some_and(|id| !a.events.contains_key(&id)) {
                return Err("SM_EVENT_MISSING_OR_FOREIGN".into());
            }
            if src.kind == StateKind::Initial && (t.event.is_some() || t.guard.is_some()) {
                return Err("SM_INITIAL_REQUIRES_UNGUARDED_COMPLETION".into());
            }
        }
    }
    Ok(())
}
pub(super) fn validate_targets(snapshot: &Snapshot, s: &Store) -> Result<(), String> {
    for a in s.state_machines.values() {
        if snapshot
            .records
            .get(&a.system)
            .is_none_or(|r| r.kind != RecordKind::System)
        {
            return Err("SM_SYSTEM_MISSING_OR_WRONG_KIND".into());
        }
    }
    Ok(())
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    if j.state_machine_forms.len() > 128 {
        return Err("SM_FORM_COUNT_LIMIT".into());
    }
    let mut ids = BTreeSet::new();
    for (key, f) in &j.state_machine_forms {
        if *key != f.key()
            || f.id().is_some_and(|id| id.0 == 0 || !ids.insert(id))
            || f.machine().is_some_and(|id| id.0 == 0)
            || f.raw().iter().any(|raw| raw.len() > 4096)
        {
            return Err("SM_FORM_KEY_ID_OR_LIMIT".into());
        }
    }
    Ok(())
}
pub(super) fn validate_draft_ids(
    s: &Store,
    j: &Journal,
    native: &BTreeSet<u64>,
) -> Result<u64, String> {
    let owned: BTreeSet<_> = ids(s).collect();
    let occupied: BTreeSet<_> = s
        .definitions
        .keys()
        .chain(s.properties.keys())
        .chain(s.usages.keys())
        .chain(s.interfaces.keys())
        .chain(s.ports.keys())
        .chain(s.connectors.keys())
        .copied()
        .chain(activities::ids(s))
        .chain(ids(s))
        .collect();
    let other: BTreeSet<_> = j
        .forms
        .keys()
        .chain(j.ports.keys())
        .chain(j.connectors.keys())
        .copied()
        .chain(j.usage.as_ref().and_then(|f| f.id))
        .chain(j.activity_flow.as_ref().and_then(|f| f.id))
        .chain(j.activity_forms.values().filter_map(|f| f.id()))
        .collect();
    if other.iter().any(|id| owned.contains(id)) {
        return Err("SM_OTHER_DRAFT_ID_COLLISION".into());
    }
    let mut high = 0;
    for f in j.state_machine_forms.values() {
        high = high.max(f.machine().map_or(0, |id| id.0));
        if let Some(id) = f.id() {
            if other.contains(&id)
                || ((native.contains(&id.0) || occupied.contains(&id)) && !f.matching(s, id))
            {
                return Err("SM_FORM_FOREIGN_ID_COLLISION".into());
            }
            high = high.max(id.0);
        }
    }
    Ok(high)
}
fn reference(raw: &str) -> Result<Id, String> {
    let n = raw.parse::<u64>().map_err(|_| "SM_CANONICAL_REFERENCE")?;
    if n == 0 || n.to_string() != raw {
        return Err("SM_CANONICAL_REFERENCE".into());
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
            "SM_LITERAL_GUARD: unsupported text retained; use empty, true, false or else".into(),
        ),
    }
}
pub(super) fn apply_inner(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut j = journal(snapshot)?;
    match action {
        Action::Set(f) => {
            j.state_machine_forms.insert(f.key(), f);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Discard(key) => {
            if j.state_machine_forms.remove(&key).is_none() {
                return Err("SM_FORM_UNAVAILABLE".into());
            }
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Apply(key) => {
            let mut f = j
                .state_machine_forms
                .get(&key)
                .cloned()
                .ok_or("SM_FORM_UNAVAILABLE")?;
            let mut s = load(snapshot)?;
            if let Some(id) = f.id() {
                if !f.matching(&s, id) {
                    return Err("SM_DETACHED_EDIT: create explicitly instead".into());
                }
            }
            let id = if let Some(id) = f.id() {
                id
            } else {
                allocate(snapshot)?
            };
            match &f {
                Form::Machine {
                    system,
                    name: label,
                    ..
                } => {
                    name(label)?;
                    let system = RecordId(reference(system)?.0);
                    if let Some(a) = s.state_machines.get_mut(&id) {
                        a.system = system;
                        a.name = label.clone();
                    } else {
                        s.state_machines.insert(
                            id,
                            Machine {
                                id,
                                system,
                                name: label.clone(),
                                states: BTreeMap::new(),
                                events: BTreeMap::new(),
                                transitions: BTreeMap::new(),
                            },
                        );
                    }
                }
                Form::State {
                    machine,
                    name: label,
                    state_kind,
                    ..
                } => {
                    name(label)?;
                    s.state_machines
                        .get_mut(machine)
                        .ok_or("SM_MACHINE_MISSING_OR_FOREIGN")?
                        .states
                        .insert(
                            id,
                            State {
                                id,
                                name: label.clone(),
                                kind: *state_kind,
                            },
                        );
                }
                Form::Event {
                    machine,
                    name: label,
                    ..
                } => {
                    name(label)?;
                    s.state_machines
                        .get_mut(machine)
                        .ok_or("SM_MACHINE_MISSING_OR_FOREIGN")?
                        .events
                        .insert(
                            id,
                            Event {
                                id,
                                name: label.clone(),
                            },
                        );
                }
                Form::Transition {
                    machine,
                    name: label,
                    source,
                    target,
                    event,
                    guard: raw_guard,
                    ..
                } => {
                    name(label)?;
                    let t = Transition {
                        id,
                        name: label.clone(),
                        source: reference(source)?,
                        target: reference(target)?,
                        event: if event.is_empty() {
                            None
                        } else {
                            Some(reference(event)?)
                        },
                        guard: guard(raw_guard)?,
                    };
                    s.state_machines
                        .get_mut(machine)
                        .ok_or("SM_MACHINE_MISSING_OR_FOREIGN")?
                        .transitions
                        .insert(id, t);
                }
            }
            j.state_machine_forms.remove(&key);
            f.set_id(id);
            j.state_machine_forms.insert(f.key(), f);
            put(snapshot, NAMESPACE, &s)?;
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Model)
        }
        Action::Remove { machine, element } => {
            let mut s = load(snapshot)?;
            if machine == element {
                if s.state_machines.remove(&machine).is_none() {
                    return Err("SM_MACHINE_MISSING".into());
                }
            } else {
                let a = s
                    .state_machines
                    .get_mut(&machine)
                    .ok_or("SM_MACHINE_MISSING")?;
                if !(a.states.remove(&element).is_some()
                    || a.events.remove(&element).is_some()
                    || a.transitions.remove(&element).is_some())
                {
                    return Err("SM_ELEMENT_MISSING_OR_FOREIGN".into());
                }
            }
            put(snapshot, NAMESPACE, &s)?;
            Ok(Change::Model)
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub elements: Vec<Id>,
    pub message: String,
}
/// Authored completeness/ambiguity only; never reachability, execution or liveness.
pub fn diagnostics(s: &Store, machine: Id) -> Result<Vec<Diagnostic>, String> {
    let a = s.state_machines.get(&machine).ok_or("SM_MACHINE_MISSING")?;
    let mut out = Vec::new();
    let mut add = |code: &str, elements: Vec<Id>, message: &str| {
        out.push(Diagnostic {
            code: code.into(),
            elements,
            message: message.into(),
        })
    };
    if !a.states.values().any(|n| n.kind == StateKind::Initial) {
        add(
            "SM_INITIAL_MISSING",
            vec![a.id],
            "Machine has no Initial state; no run can be inferred.",
        );
    }
    if !a.states.values().any(|n| n.kind == StateKind::Final) {
        add(
            "SM_FINAL_MISSING",
            vec![a.id],
            "No Final state is authored; termination is not established.",
        );
    }
    let mut groups: BTreeMap<(Id, Option<Id>), Vec<&Transition>> = BTreeMap::new();
    for t in a.transitions.values() {
        groups.entry((t.source, t.event)).or_default().push(t);
    }
    for group in groups.values() {
        let enabled: Vec<_> = group
            .iter()
            .filter(|t| t.guard.is_none() || t.guard == Some(Guard::Boolean(true)))
            .map(|t| t.id)
            .collect();
        let fallback: Vec<_> = group
            .iter()
            .filter(|t| t.guard == Some(Guard::Else))
            .map(|t| t.id)
            .collect();
        if enabled.len() > 1 {
            add("SM_AMBIGUOUS_TRANSITIONS",enabled,"Competing unconditional/literal-true transitions; no selection policy or execution is inferred.");
        }
        if fallback.len() > 1 {
            add(
                "SM_AMBIGUOUS_ELSE",
                fallback,
                "Multiple Else transitions compete for the same state/event.",
            );
        }
    }
    let mut names: BTreeMap<&str, Vec<Id>> = BTreeMap::new();
    for e in a.events.values() {
        names.entry(&e.name).or_default().push(e.id);
    }
    for ids in names.values().filter(|ids| ids.len() > 1) {
        add(
            "SM_DUPLICATE_EVENT_NAME",
            ids.clone(),
            "Events share a label; transition references still bind exact event identities.",
        );
    }
    for state in a.states.values().filter(|n| n.kind != StateKind::Final) {
        if !a.transitions.values().any(|t| t.source == state.id) {
            add(
                "SM_NO_OUTGOING_TRANSITION",
                vec![state.id],
                "No outgoing transition; no progress is established.",
            );
        }
    }
    Ok(out)
}
