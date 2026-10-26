//! Directed exchange between exact Part-usage/Port endpoint contexts.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub usage: Id,
    pub port: Id,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connector {
    pub id: Id,
    pub owner: Id,
    pub name: String,
    pub source: Endpoint,
    pub target: Endpoint,
    pub conveyed: Id,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub owner: Id,
    pub name: String,
    pub source_usage: String,
    pub source_port: String,
    pub target_usage: String,
    pub target_port: String,
    pub conveyed: String,
}
#[derive(Clone, Debug)]
pub enum Action {
    Begin { owner: Id },
    Set { id: Id, form: Form },
    Apply { id: Id },
}
pub fn forms(snapshot: &Snapshot) -> Result<BTreeMap<Id, Form>, String> {
    Ok(journal(snapshot)?.connectors)
}
fn reference(text: &str) -> Result<Id, String> {
    let text = text.trim();
    let id = text.parse::<u64>().map_err(|_| "CONNECTOR_REFERENCE_ID")?;
    if id == 0 || id.to_string() != text {
        return Err("CONNECTOR_REFERENCE_ID".into());
    }
    Ok(Id(id))
}
pub fn endpoint_interface<'a>(
    store: &'a Store,
    owner: Id,
    endpoint: Endpoint,
) -> Result<&'a ports::Interface, String> {
    let usage = store
        .usages
        .get(&endpoint.usage)
        .ok_or("CONNECTOR_PART_MISSING_OR_WRONG_KIND")?;
    if usage.kind != UsageKind::Part || usage.owner != owner {
        return Err("CONNECTOR_PART_OWNERSHIP_OR_KIND".into());
    }
    let port = store
        .ports
        .get(&endpoint.port)
        .ok_or("CONNECTOR_PORT_MISSING_OR_WRONG_KIND")?;
    if usage.target != port.owner {
        return Err("CONNECTOR_PORT_NOT_ON_PART_TYPE".into());
    }
    store
        .interfaces
        .get(&port.assignment.unwrap_or(port.contract))
        .ok_or("CONNECTOR_INTERFACE_MISSING".into())
}
pub(super) fn validate_store(store: &Store, ids: &mut BTreeSet<Id>) -> Result<(), String> {
    for (id, c) in &store.connectors {
        if *id != c.id || id.0 == 0 || !ids.insert(*id) {
            return Err("CONNECTOR_ID".into());
        }
        name(&c.name)?;
        if store
            .definitions
            .get(&c.owner)
            .is_none_or(|d| d.kind != Kind::Component)
        {
            return Err("CONNECTOR_OWNER_WRONG_KIND".into());
        }
        if c.source == c.target {
            return Err("CONNECTOR_IDENTICAL_ENDPOINTS".into());
        }
        let source = endpoint_interface(store, c.owner, c.source)?;
        let target = endpoint_interface(store, c.owner, c.target)?;
        if !matches!(
            source.direction,
            ports::Direction::Out | ports::Direction::InOut
        ) || !matches!(
            target.direction,
            ports::Direction::In | ports::Direction::InOut
        ) {
            return Err("CONNECTOR_DIRECTION_INCOMPATIBLE".into());
        }
        if source.payload != target.payload || source.payload != c.conveyed {
            return Err("CONNECTOR_CONVEYED_TYPE_INCOMPATIBLE".into());
        }
        if store
            .definitions
            .get(&c.conveyed)
            .is_none_or(|d| !matches!(d.kind, Kind::Value(_)))
        {
            return Err("CONNECTOR_CONVEYED_WRONG_KIND".into());
        }
    }
    Ok(())
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    if j.connectors.len() > 512
        || j.connectors.values().any(|f| {
            f.owner.0 == 0
                || [
                    &f.name,
                    &f.source_usage,
                    &f.source_port,
                    &f.target_usage,
                    &f.target_port,
                    &f.conveyed,
                ]
                .iter()
                .any(|s| s.len() > 4096)
        })
    {
        return Err("CONNECTOR_DRAFT_SHAPE_OR_LIMIT".into());
    }
    Ok(())
}
pub(super) fn apply_inner(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut store = load(snapshot)?;
    let mut j = journal(snapshot)?;
    match action {
        Action::Begin { owner } => {
            if store
                .definitions
                .get(&owner)
                .is_none_or(|d| d.kind != Kind::Component)
            {
                return Err("CONNECTOR_OWNER_WRONG_KIND".into());
            }
            let id = allocate(snapshot)?;
            j.connectors.insert(
                id,
                Form {
                    owner,
                    name: String::new(),
                    source_usage: String::new(),
                    source_port: String::new(),
                    target_usage: String::new(),
                    target_port: String::new(),
                    conveyed: String::new(),
                },
            );
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Set { id, form } => {
            if !j.connectors.contains_key(&id) && !store.connectors.contains_key(&id) {
                return Err("CONNECTOR_DRAFT_UNAVAILABLE".into());
            }
            j.connectors.insert(id, form);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Apply { id } => {
            let f = j.connectors.get(&id).ok_or("CONNECTOR_DRAFT_UNAVAILABLE")?;
            name(&f.name)?;
            let source = Endpoint {
                usage: reference(&f.source_usage)?,
                port: reference(&f.source_port)?,
            };
            let target = Endpoint {
                usage: reference(&f.target_usage)?,
                port: reference(&f.target_port)?,
            };
            store.connectors.insert(
                id,
                Connector {
                    id,
                    owner: f.owner,
                    name: f.name.clone(),
                    source,
                    target,
                    conveyed: reference(&f.conveyed)?,
                },
            );
            store.validate()?;
            put(snapshot, NAMESPACE, &store)?;
            Ok(Change::Model)
        }
    }
}
pub fn preview(snapshot: &Snapshot, id: Id) -> Result<Connector, String> {
    let mut s = snapshot.clone();
    apply(&mut s, super::Action::Connector(Action::Apply { id }))?;
    load(&s)?
        .connectors
        .get(&id)
        .cloned()
        .ok_or("CONNECTOR_UNAVAILABLE".into())
}
