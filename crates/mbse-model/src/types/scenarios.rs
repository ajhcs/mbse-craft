//! Bounded documentary sequences over exact existing native/typed identities.
use super::*;
pub const MAX_ORDER: u32 = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantKind {
    Native(RecordKind),
    Component,
    Usage(UsageKind),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    Synchronous,
    Asynchronous,
    Reply,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lifeline {
    pub id: Id,
    pub name: String,
    pub participant: Id,
    pub expected: ParticipantKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub id: Id,
    pub name: String,
    pub source: Id,
    pub target: Id,
    pub order: u32,
    pub operand: Option<Id>,
    pub kind: MessageKind,
    pub reply_to: Option<Id>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operand {
    pub id: Id,
    pub name: String,
    pub order: u32,
    pub guard: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alternative {
    pub id: Id,
    pub name: String,
    pub order: u32,
    pub operands: BTreeMap<Id, Operand>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: Id,
    pub system: RecordId,
    pub name: String,
    pub lifelines: BTreeMap<Id, Lifeline>,
    pub messages: BTreeMap<Id, Message>,
    pub alternatives: BTreeMap<Id, Alternative>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Form {
    Scenario {
        id: Option<Id>,
        system: String,
        name: String,
    },
    Lifeline {
        scenario: Id,
        id: Option<Id>,
        name: String,
        participant: String,
        expected: ParticipantKind,
    },
    Message {
        scenario: Id,
        id: Option<Id>,
        name: String,
        source: String,
        target: String,
        order: String,
        operand: String,
        message_kind: MessageKind,
        reply_to: String,
    },
    Alternative {
        scenario: Id,
        id: Option<Id>,
        name: String,
        order: String,
    },
    Operand {
        scenario: Id,
        id: Option<Id>,
        name: String,
        alternative: String,
        order: String,
        guard: String,
    },
}
#[derive(Clone, Debug)]
pub enum Action {
    Set(Form),
    Apply(String),
    Discard(String),
    Remove { scenario: Id, element: Id },
}
impl Form {
    pub fn id(&self) -> Option<Id> {
        match self {
            Self::Scenario { id, .. }
            | Self::Lifeline { id, .. }
            | Self::Message { id, .. }
            | Self::Alternative { id, .. }
            | Self::Operand { id, .. } => *id,
        }
    }
    pub fn scenario(&self) -> Option<Id> {
        match self {
            Self::Scenario { .. } => None,
            Self::Lifeline { scenario, .. }
            | Self::Message { scenario, .. }
            | Self::Alternative { scenario, .. }
            | Self::Operand { scenario, .. } => Some(*scenario),
        }
    }
    pub fn key(&self) -> String {
        let family = match self {
            Self::Scenario { .. } => "scenario",
            Self::Lifeline { .. } => "lifeline",
            Self::Message { .. } => "message",
            Self::Alternative { .. } => "alternative",
            Self::Operand { .. } => "operand",
        };
        let id = self.id().map_or("new".into(), |id| id.0.to_string());
        self.scenario().map_or_else(
            || format!("{family}:{id}"),
            |s| format!("{family}:{}:{id}", s.0),
        )
    }
    fn set_id(&mut self, new: Id) {
        match self {
            Self::Scenario { id, .. }
            | Self::Lifeline { id, .. }
            | Self::Message { id, .. }
            | Self::Alternative { id, .. }
            | Self::Operand { id, .. } => *id = Some(new),
        }
    }
    fn raw(&self) -> Vec<&str> {
        match self {
            Self::Scenario { system, name, .. } => vec![system, name],
            Self::Lifeline {
                name, participant, ..
            } => vec![name, participant],
            Self::Message {
                name,
                source,
                target,
                order,
                operand,
                reply_to,
                ..
            } => vec![name, source, target, order, operand, reply_to],
            Self::Alternative { name, order, .. } => vec![name, order],
            Self::Operand {
                name,
                alternative,
                order,
                guard,
                ..
            } => vec![name, alternative, order, guard],
        }
    }
    fn matching(&self, s: &Store, id: Id) -> bool {
        match self {
            Self::Scenario { .. } => s.scenarios.contains_key(&id),
            Self::Lifeline { scenario, .. } => s
                .scenarios
                .get(scenario)
                .is_some_and(|a| a.lifelines.contains_key(&id)),
            Self::Message { scenario, .. } => s
                .scenarios
                .get(scenario)
                .is_some_and(|a| a.messages.contains_key(&id)),
            Self::Alternative { scenario, .. } => s
                .scenarios
                .get(scenario)
                .is_some_and(|a| a.alternatives.contains_key(&id)),
            Self::Operand { scenario, .. } => s.scenarios.get(scenario).is_some_and(|a| {
                a.alternatives
                    .values()
                    .any(|alt| alt.operands.contains_key(&id))
            }),
        }
    }
}
pub fn forms(s: &Snapshot) -> Result<BTreeMap<String, Form>, String> {
    Ok(journal(s)?.scenario_forms)
}
pub fn ids(s: &Store) -> impl Iterator<Item = Id> + '_ {
    s.scenarios.values().flat_map(|a| {
        std::iter::once(a.id)
            .chain(a.lifelines.keys().copied())
            .chain(a.messages.keys().copied())
            .chain(
                a.alternatives
                    .values()
                    .flat_map(|alt| std::iter::once(alt.id).chain(alt.operands.keys().copied())),
            )
    })
}
pub(super) fn target_high(s: &Store) -> u64 {
    s.scenarios
        .values()
        .flat_map(|a| a.lifelines.values())
        .map(|l| l.participant.0)
        .max()
        .unwrap_or(0)
}
pub(super) fn canonical_maps(v: &Value) -> Result<(), String> {
    if let Some(map) = v.get("scenarios") {
        canonical(v, "scenarios")?;
        for a in map.as_object().ok_or("SCENARIO_SHAPE")?.values() {
            for key in ["lifelines", "messages", "alternatives"] {
                canonical(a, key)?;
            }
            for alt in a["alternatives"]
                .as_object()
                .ok_or("SCENARIO_ALTERNATIVE_SHAPE")?
                .values()
            {
                canonical(alt, "operands")?;
            }
        }
    }
    Ok(())
}
fn identity(key: &Id, id: Id, all: &mut BTreeSet<Id>) -> Result<(), String> {
    if *key != id || id.0 == 0 || !all.insert(id) {
        Err("SCENARIO_ID_COLLISION_OR_MAP_KEY".into())
    } else {
        Ok(())
    }
}
fn rank(order: u32) -> Result<(), String> {
    if (1..=MAX_ORDER).contains(&order) {
        Ok(())
    } else {
        Err("SCENARIO_ORDER_1_TO_4096".into())
    }
}
fn guard(text: &str) -> Result<(), String> {
    if text.trim().is_empty()
        || text.len() > 4096
        || text
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        Err("SCENARIO_OPERAND_GUARD_REQUIRED_OR_LIMIT".into())
    } else {
        Ok(())
    }
}
pub(super) fn validate_store(s: &Store, all: &mut BTreeSet<Id>) -> Result<(), String> {
    for (key, a) in &s.scenarios {
        identity(key, a.id, all)?;
        name(&a.name)?;
        if a.system.0 == 0 {
            return Err("SCENARIO_SYSTEM_ID".into());
        }
        for (key, l) in &a.lifelines {
            identity(key, l.id, all)?;
            name(&l.name)?;
            if l.participant.0 == 0 {
                return Err("SCENARIO_PARTICIPANT_ID".into());
            }
        }
        let mut main = BTreeSet::new();
        let mut operand_ids = BTreeSet::new();
        for (key, alt) in &a.alternatives {
            identity(key, alt.id, all)?;
            name(&alt.name)?;
            rank(alt.order)?;
            if !main.insert(alt.order) {
                return Err("SCENARIO_MAIN_ORDER_COLLISION".into());
            }
            let mut ranks = BTreeSet::new();
            for (key, o) in &alt.operands {
                identity(key, o.id, all)?;
                name(&o.name)?;
                guard(&o.guard)?;
                rank(o.order)?;
                if !ranks.insert(o.order) {
                    return Err("SCENARIO_OPERAND_ORDER_COLLISION".into());
                }
                operand_ids.insert(o.id);
            }
        }
        let mut orders: BTreeMap<Option<Id>, BTreeSet<u32>> = BTreeMap::from([(None, main)]);
        for (key, m) in &a.messages {
            identity(key, m.id, all)?;
            name(&m.name)?;
            rank(m.order)?;
            if !a.lifelines.contains_key(&m.source) || !a.lifelines.contains_key(&m.target) {
                return Err("SCENARIO_MESSAGE_LIFELINE_MISSING_OR_FOREIGN".into());
            }
            if m.operand.is_some_and(|o| !operand_ids.contains(&o)) {
                return Err("SCENARIO_MESSAGE_OPERAND_MISSING_OR_FOREIGN".into());
            }
            if !orders.entry(m.operand).or_default().insert(m.order) {
                return Err("SCENARIO_MESSAGE_ORDER_COLLISION".into());
            }
            if m.kind == MessageKind::Reply {
                let call = m
                    .reply_to
                    .and_then(|id| a.messages.get(&id))
                    .ok_or("SCENARIO_REPLY_CALL_MISSING_OR_FOREIGN")?;
                if call.kind != MessageKind::Synchronous
                    || call.source != m.target
                    || call.target != m.source
                    || call.operand != m.operand
                    || call.order >= m.order
                {
                    return Err("SCENARIO_REPLY_REQUIRES_PRIOR_REVERSED_CALL_IN_SAME_SCOPE".into());
                }
            } else if m.reply_to.is_some() {
                return Err("SCENARIO_REPLY_REFERENCE_ON_NON_REPLY".into());
            }
        }
    }
    Ok(())
}
pub(super) fn validate_targets(snapshot: &Snapshot, s: &Store) -> Result<(), String> {
    for a in s.scenarios.values() {
        if snapshot
            .records
            .get(&a.system)
            .is_none_or(|r| r.kind != RecordKind::System)
        {
            return Err("SCENARIO_SYSTEM_MISSING_OR_WRONG_KIND".into());
        }
    }
    Ok(())
}
/// Exact current binding, never a name match or replacement identity.
pub fn participant_status(
    snapshot: &Snapshot,
    s: &Store,
    a: &Scenario,
    l: &Lifeline,
) -> Result<(), String> {
    if snapshot
        .records
        .get(&a.system)
        .is_none_or(|r| r.kind != RecordKind::System)
    {
        return Err("SCENARIO_SYSTEM_MISSING_OR_WRONG_KIND".into());
    }
    let scope = match l.expected {
        ParticipantKind::Native(expected) => {
            let r = snapshot
                .records
                .get(&RecordId(l.participant.0))
                .ok_or("SCENARIO_PARTICIPANT_NATIVE_MISSING")?;
            if r.kind != expected {
                return Err("SCENARIO_PARTICIPANT_NATIVE_WRONG_KIND".into());
            }
            r.scope
        }
        ParticipantKind::Component => {
            let d = s
                .definitions
                .get(&l.participant)
                .ok_or("SCENARIO_PARTICIPANT_COMPONENT_MISSING")?;
            if d.kind != Kind::Component {
                return Err("SCENARIO_PARTICIPANT_COMPONENT_WRONG_KIND".into());
            }
            d.system
        }
        ParticipantKind::Usage(expected) => {
            let u = s
                .usages
                .get(&l.participant)
                .ok_or("SCENARIO_PARTICIPANT_USAGE_MISSING")?;
            if u.kind != expected {
                return Err("SCENARIO_PARTICIPANT_USAGE_WRONG_KIND".into());
            }
            s.definitions
                .get(&u.owner)
                .ok_or("SCENARIO_PARTICIPANT_USAGE_OWNER_MISSING")?
                .system
        }
    };
    if scope != a.system {
        Err("SCENARIO_PARTICIPANT_FOREIGN_SYSTEM".into())
    } else {
        Ok(())
    }
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    if j.scenario_forms.len() > 128 {
        return Err("SCENARIO_FORM_COUNT_LIMIT".into());
    }
    let mut ids = BTreeSet::new();
    for (key, f) in &j.scenario_forms {
        if *key != f.key()
            || f.id().is_some_and(|id| id.0 == 0 || !ids.insert(id))
            || f.scenario().is_some_and(|id| id.0 == 0)
            || f.raw().iter().any(|s| s.len() > 4096)
        {
            return Err("SCENARIO_FORM_KEY_ID_OR_LIMIT".into());
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
        .chain(state_machines::ids(s))
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
        .chain(j.state_machine_forms.values().filter_map(|f| f.id()))
        .collect();
    if other.iter().any(|id| owned.contains(id)) {
        return Err("SCENARIO_OTHER_DRAFT_ID_COLLISION".into());
    }
    let mut high = 0;
    for f in j.scenario_forms.values() {
        high = high.max(f.scenario().map_or(0, |id| id.0));
        if let Some(id) = f.id() {
            if other.contains(&id)
                || ((native.contains(&id.0) || occupied.contains(&id)) && !f.matching(s, id))
            {
                return Err("SCENARIO_FORM_FOREIGN_ID_COLLISION".into());
            }
            high = high.max(id.0);
        }
    }
    Ok(high)
}
fn reference(raw: &str) -> Result<Id, String> {
    let n = raw
        .parse::<u64>()
        .map_err(|_| "SCENARIO_CANONICAL_REFERENCE")?;
    if n == 0 || n.to_string() != raw {
        Err("SCENARIO_CANONICAL_REFERENCE".into())
    } else {
        Ok(Id(n))
    }
}
fn order(raw: &str) -> Result<u32, String> {
    let n = raw.parse::<u32>().map_err(|_| "SCENARIO_CANONICAL_ORDER")?;
    if n.to_string() != raw {
        return Err("SCENARIO_CANONICAL_ORDER".into());
    }
    rank(n)?;
    Ok(n)
}
pub(super) fn apply_inner(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut j = journal(snapshot)?;
    match action {
        Action::Set(f) => {
            j.scenario_forms.insert(f.key(), f);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Discard(key) => {
            if j.scenario_forms.remove(&key).is_none() {
                return Err("SCENARIO_FORM_UNAVAILABLE".into());
            }
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Apply(key) => {
            let mut f = j
                .scenario_forms
                .get(&key)
                .cloned()
                .ok_or("SCENARIO_FORM_UNAVAILABLE")?;
            let mut s = load(snapshot)?;
            if f.id().is_some_and(|id| !f.matching(&s, id)) {
                return Err("SCENARIO_DETACHED_EDIT_CREATE_EXPLICITLY".into());
            }
            let id = if let Some(id) = f.id() {
                id
            } else {
                allocate(snapshot)?
            };
            match &f {
                Form::Scenario { system, name, .. } => {
                    let system = RecordId(reference(system)?.0);
                    let a = s.scenarios.entry(id).or_insert(Scenario {
                        id,
                        system,
                        name: name.clone(),
                        lifelines: BTreeMap::new(),
                        messages: BTreeMap::new(),
                        alternatives: BTreeMap::new(),
                    });
                    a.name = name.clone();
                    a.system = system;
                }
                Form::Lifeline {
                    scenario,
                    name,
                    participant,
                    expected,
                    ..
                } => {
                    let a = s
                        .scenarios
                        .get(scenario)
                        .ok_or("SCENARIO_MISSING_OR_FOREIGN")?;
                    let l = Lifeline {
                        id,
                        name: name.clone(),
                        participant: reference(participant)?,
                        expected: *expected,
                    };
                    participant_status(snapshot, &s, a, &l)?;
                    s.scenarios
                        .get_mut(scenario)
                        .unwrap()
                        .lifelines
                        .insert(id, l);
                }
                Form::Message {
                    scenario,
                    name,
                    source,
                    target,
                    order: raw,
                    operand,
                    message_kind,
                    reply_to,
                    ..
                } => {
                    let m = Message {
                        id,
                        name: name.clone(),
                        source: reference(source)?,
                        target: reference(target)?,
                        order: order(raw)?,
                        operand: if operand.is_empty() {
                            None
                        } else {
                            Some(reference(operand)?)
                        },
                        kind: *message_kind,
                        reply_to: if reply_to.is_empty() {
                            None
                        } else {
                            Some(reference(reply_to)?)
                        },
                    };
                    s.scenarios
                        .get_mut(scenario)
                        .ok_or("SCENARIO_MISSING_OR_FOREIGN")?
                        .messages
                        .insert(id, m);
                }
                Form::Alternative {
                    scenario,
                    name,
                    order: raw,
                    ..
                } => {
                    let a = s
                        .scenarios
                        .get_mut(scenario)
                        .ok_or("SCENARIO_MISSING_OR_FOREIGN")?;
                    let order = order(raw)?;
                    let alt = a.alternatives.entry(id).or_insert(Alternative {
                        id,
                        name: name.clone(),
                        order,
                        operands: BTreeMap::new(),
                    });
                    alt.name = name.clone();
                    alt.order = order;
                }
                Form::Operand {
                    scenario,
                    name,
                    alternative,
                    order: raw,
                    guard,
                    ..
                } => {
                    let a = s
                        .scenarios
                        .get_mut(scenario)
                        .ok_or("SCENARIO_MISSING_OR_FOREIGN")?;
                    let alt = reference(alternative)?;
                    if f.id().is_some()
                        && a.alternatives
                            .values()
                            .any(|other| other.id != alt && other.operands.contains_key(&id))
                    {
                        return Err("SCENARIO_OPERAND_OWNER_CHANGE_UNSUPPORTED".into());
                    }
                    a.alternatives
                        .get_mut(&alt)
                        .ok_or("SCENARIO_OPERAND_ALTERNATIVE_MISSING_OR_FOREIGN")?
                        .operands
                        .insert(
                            id,
                            Operand {
                                id,
                                name: name.clone(),
                                order: order(raw)?,
                                guard: guard.clone(),
                            },
                        );
                }
            }
            s.validate()?;
            f.set_id(id);
            j.scenario_forms.remove(&key);
            j.scenario_forms.insert(f.key(), f);
            put(snapshot, NAMESPACE, &s)?;
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Model)
        }
        Action::Remove { scenario, element } => {
            let mut s = load(snapshot)?;
            if scenario == element {
                if s.scenarios.remove(&scenario).is_none() {
                    return Err("SCENARIO_MISSING".into());
                }
            } else {
                let a = s.scenarios.get_mut(&scenario).ok_or("SCENARIO_MISSING")?;
                if a.messages.values().any(|m| {
                    m.source == element
                        || m.target == element
                        || m.operand == Some(element)
                        || m.reply_to == Some(element)
                }) {
                    return Err("SCENARIO_ELEMENT_IN_USE".into());
                }
                if a.alternatives
                    .get(&element)
                    .is_some_and(|a| !a.operands.is_empty())
                {
                    return Err("SCENARIO_ALTERNATIVE_IN_USE".into());
                }
                let mut found = a.lifelines.remove(&element).is_some()
                    || a.messages.remove(&element).is_some()
                    || a.alternatives.remove(&element).is_some();
                if !found {
                    for alt in a.alternatives.values_mut() {
                        found |= alt.operands.remove(&element).is_some();
                    }
                }
                if !found {
                    return Err("SCENARIO_ELEMENT_MISSING_OR_FOREIGN".into());
                }
            }
            put(snapshot, NAMESPACE, &s)?;
            Ok(Change::Model)
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub elements: Vec<Id>,
    pub message: String,
}
/// Current documentary diagnostics only, never a simulation or path truth verdict.
pub fn diagnostics(
    snapshot: &Snapshot,
    s: &Store,
    scenario: Id,
) -> Result<Vec<Diagnostic>, String> {
    let a = s.scenarios.get(&scenario).ok_or("SCENARIO_MISSING")?;
    let mut out = Vec::new();
    let mut add = |code: &str, elements: Vec<Id>, message: String| {
        out.push(Diagnostic {
            code: code.into(),
            elements,
            message,
        })
    };
    let mut unresolved = BTreeSet::new();
    for l in a.lifelines.values() {
        if let Err(error) = participant_status(snapshot, s, a, l) {
            unresolved.insert(l.id);
            add(
                "SCENARIO_PARTICIPANT_UNRESOLVED",
                vec![l.id, l.participant],
                error,
            );
        }
        if !a
            .messages
            .values()
            .any(|m| m.source == l.id || m.target == l.id)
        {
            add(
                "SCENARIO_LIFELINE_UNUSED",
                vec![l.id],
                "No message uses this documentary participant.".into(),
            );
        }
    }
    let mut aliases: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
    for l in a.lifelines.values() {
        aliases.entry(l.participant).or_default().push(l.id);
    }
    for (target, lines) in aliases.iter().filter(|(_, lines)| lines.len() > 1) {
        add(
            "SCENARIO_PARTICIPANT_ALIASES",
            lines.clone(),
            format!(
                "Multiple documentary lifelines reference exact participant #{}; separate runtime instances are not inferred.",
                target.0
            ),
        );
    }
    for m in a.messages.values() {
        if unresolved.contains(&m.source) || unresolved.contains(&m.target) {
            add("SCENARIO_MESSAGE_PARTICIPANT_UNRESOLVED",vec![m.id,m.source,m.target],"One or both exact participant bindings are unavailable or incompatible; no replacement is inferred.".into());
        }
    }
    if a.messages.is_empty() {
        add(
            "SCENARIO_MESSAGES_MISSING",
            vec![a.id],
            "No messages authored; no execution or completeness is inferred.".into(),
        );
    }
    for alt in a.alternatives.values() {
        if alt.operands.len() < 2 {
            add(
                "SCENARIO_ALTERNATIVE_OPERANDS_INCOMPLETE",
                vec![alt.id],
                "At least two guarded operands are needed to document alternatives.".into(),
            );
        }
        let mut guards: BTreeMap<&str, Vec<Id>> = BTreeMap::new();
        for o in alt.operands.values() {
            guards.entry(o.guard.trim()).or_default().push(o.id);
            if !a.messages.values().any(|m| m.operand == Some(o.id)) {
                add("SCENARIO_OPERAND_EMPTY",vec![alt.id,o.id],"No messages in this guarded operand; guard text is retained without evaluation.".into());
            }
        }
        for ids in guards.values().filter(|ids| ids.len() > 1) {
            add("SCENARIO_DUPLICATE_GUARD",ids.clone(),"Identical guard text on multiple operands; exclusivity or precedence is not inferred.".into());
        }
    }
    Ok(out)
}

/// Stable authored order only; neither execution order nor causal proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Fragment {
    Message(Id),
    Alternative(Id),
}
pub fn main_sequence(a: &Scenario) -> Vec<Fragment> {
    let mut rows: Vec<_> = a
        .messages
        .values()
        .filter(|m| m.operand.is_none())
        .map(|m| (m.order, Fragment::Message(m.id)))
        .chain(
            a.alternatives
                .values()
                .map(|alt| (alt.order, Fragment::Alternative(alt.id))),
        )
        .collect();
    rows.sort_by_key(|(rank, _)| *rank);
    rows.into_iter().map(|(_, fragment)| fragment).collect()
}
pub fn ordered_operands(alt: &Alternative) -> Vec<&Operand> {
    let mut rows: Vec<_> = alt.operands.values().collect();
    rows.sort_by_key(|o| o.order);
    rows
}
pub fn operand_sequence(a: &Scenario, operand: Id) -> Result<Vec<Id>, String> {
    if !a
        .alternatives
        .values()
        .any(|alt| alt.operands.contains_key(&operand))
    {
        return Err("SCENARIO_OPERAND_MISSING_OR_FOREIGN".into());
    }
    let mut rows: Vec<_> = a
        .messages
        .values()
        .filter(|m| m.operand == Some(operand))
        .collect();
    rows.sort_by_key(|m| m.order);
    Ok(rows.into_iter().map(|m| m.id).collect())
}
