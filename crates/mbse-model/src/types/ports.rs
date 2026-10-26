//! App-owned one-payload interface contracts; no external language equivalence.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    In,
    Out,
    InOut,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    pub id: Id,
    pub system: RecordId,
    pub name: String,
    pub payload: Id,
    pub direction: Direction,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub id: Id,
    pub owner: Id,
    pub name: String,
    pub contract: Id,
    pub assignment: Option<Id>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceForm {
    pub system: RecordId,
    pub name: String,
    pub payload: String,
    pub direction: Direction,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortForm {
    pub owner: Id,
    pub name: String,
    pub contract: String,
    pub assignment: String,
}
#[derive(Clone, Debug)]
pub enum Action {
    SetInterfaceForm(InterfaceForm),
    AddInterface,
    BeginPort { owner: Id },
    SetPortForm { id: Id, form: PortForm },
    ApplyPort { id: Id },
}
pub fn interface_form(snapshot: &Snapshot) -> Result<Option<InterfaceForm>, String> {
    Ok(journal(snapshot)?.interface)
}
pub fn forms(snapshot: &Snapshot) -> Result<BTreeMap<Id, PortForm>, String> {
    Ok(journal(snapshot)?.ports)
}
fn reference(text: &str) -> Result<Id, String> {
    let text = text.trim();
    let id = text.parse::<u64>().map_err(|_| "PORT_REFERENCE_ID")?;
    if id == 0 || id.to_string() != text {
        return Err("PORT_REFERENCE_ID".into());
    }
    Ok(Id(id))
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    if j.ports.len() > 512
        || j.interface.as_ref().is_some_and(|f| {
            f.system.0 == 0 || [&f.name, &f.payload].iter().any(|s| s.len() > 4096)
        })
        || j.ports.values().any(|f| {
            f.owner.0 == 0
                || [&f.name, &f.contract, &f.assignment]
                    .iter()
                    .any(|s| s.len() > 4096)
        })
    {
        return Err("PORT_DRAFT_SHAPE_OR_LIMIT".into());
    }
    Ok(())
}
pub(super) fn validate_store(store: &Store, ids: &mut BTreeSet<Id>) -> Result<(), String> {
    for (id, i) in &store.interfaces {
        if *id != i.id || id.0 == 0 || !ids.insert(*id) {
            return Err("PORT_INTERFACE_ID".into());
        }
        name(&i.name)?;
        let payload = store
            .definitions
            .get(&i.payload)
            .ok_or("PORT_PAYLOAD_MISSING_OR_WRONG_KIND")?;
        if !matches!(payload.kind, Kind::Value(_)) {
            return Err("PORT_PAYLOAD_WRONG_KIND".into());
        }
        if i.system != payload.system {
            return Err("PORT_CROSS_SYSTEM_PAYLOAD_UNSUPPORTED".into());
        }
    }
    for (id, p) in &store.ports {
        if *id != p.id || id.0 == 0 || !ids.insert(*id) {
            return Err("PORT_ID".into());
        }
        name(&p.name)?;
        let owner = store
            .definitions
            .get(&p.owner)
            .ok_or("PORT_OWNER_MISSING_OR_WRONG_KIND")?;
        if owner.kind != Kind::Component {
            return Err("PORT_OWNER_WRONG_KIND".into());
        }
        let declared = store
            .interfaces
            .get(&p.contract)
            .ok_or("PORT_CONTRACT_MISSING_OR_WRONG_KIND")?;
        if declared.system != owner.system {
            return Err("PORT_CROSS_SYSTEM_CONTRACT_UNSUPPORTED".into());
        }
        if let Some(id) = p.assignment {
            let assigned = store
                .interfaces
                .get(&id)
                .ok_or("PORT_ASSIGNMENT_MISSING_OR_WRONG_KIND")?;
            if (assigned.system, assigned.payload, assigned.direction)
                != (declared.system, declared.payload, declared.direction)
            {
                return Err(
                    "PORT_ASSIGNMENT_INCOMPATIBLE: exact payload, direction and System must agree"
                        .into(),
                );
            }
        }
    }
    Ok(())
}
pub(super) fn apply_inner(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut store = load(snapshot)?;
    let mut j = journal(snapshot)?;
    match action {
        Action::SetInterfaceForm(form) => {
            j.interface = Some(form);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::AddInterface => {
            let mut f = j
                .interface
                .clone()
                .ok_or("PORT_INTERFACE_DRAFT_UNAVAILABLE")?;
            name(&f.name)?;
            let payload = reference(&f.payload)?;
            let id = allocate(snapshot)?;
            store.interfaces.insert(
                id,
                Interface {
                    id,
                    system: f.system,
                    name: f.name.clone(),
                    payload,
                    direction: f.direction,
                },
            );
            store.validate()?;
            f.name.clear();
            j.interface = Some(f);
            put(snapshot, NAMESPACE, &store)?;
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Model)
        }
        Action::BeginPort { owner } => {
            if store
                .definitions
                .get(&owner)
                .is_none_or(|d| d.kind != Kind::Component)
            {
                return Err("PORT_OWNER_WRONG_KIND".into());
            }
            let id = allocate(snapshot)?;
            j.ports.insert(
                id,
                PortForm {
                    owner,
                    name: String::new(),
                    contract: String::new(),
                    assignment: String::new(),
                },
            );
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::SetPortForm { id, form } => {
            if !j.ports.contains_key(&id) && !store.ports.contains_key(&id) {
                return Err("PORT_DRAFT_UNAVAILABLE".into());
            }
            j.ports.insert(id, form);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::ApplyPort { id } => {
            let f = j.ports.get(&id).ok_or("PORT_DRAFT_UNAVAILABLE")?;
            name(&f.name)?;
            let contract = reference(&f.contract)?;
            let assignment = if f.assignment.trim().is_empty() {
                None
            } else {
                Some(reference(&f.assignment)?)
            };
            store.ports.insert(
                id,
                Port {
                    id,
                    owner: f.owner,
                    name: f.name.clone(),
                    contract,
                    assignment,
                },
            );
            store.validate()?;
            put(snapshot, NAMESPACE, &store)?;
            Ok(Change::Model)
        }
    }
}
pub fn preview(snapshot: &Snapshot, id: Id) -> Result<Port, String> {
    let mut s = snapshot.clone();
    apply(&mut s, super::Action::Port(Action::ApplyPort { id }))?;
    load(&s)?
        .ports
        .get(&id)
        .cloned()
        .ok_or("PORT_UNAVAILABLE".into())
}
pub fn preview_interface(snapshot: &Snapshot) -> Result<Interface, String> {
    let mut s = snapshot.clone();
    let id = Id(s.next_id);
    apply(&mut s, super::Action::Port(Action::AddInterface))?;
    load(&s)?
        .interfaces
        .get(&id)
        .cloned()
        .ok_or("PORT_INTERFACE_UNAVAILABLE".into())
}
