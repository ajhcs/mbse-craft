//! Exact Part-qualified port contexts and connector presentation routes.
use super::*;
use crate::types::{self, Id, UsageKind, connectors::Endpoint};
pub const PROFILE: &str = "mbse-craft.native-views/4";
pub const RECOVERY_PROFILE: &str = "mbse-craft.native-views-recovery/4";
pub fn qualified_profile(p: &str) -> bool {
    matches!(p, PROFILE | RECOVERY_PROFILE) || viewpoints::profile(p)
}
pub(super) fn upgrade(doc: &mut Document) {
    if viewpoints::profile(&doc.profile){return;}
    doc.profile = if bound_profile(&doc.profile) {
        PROFILE
    } else {
        RECOVERY_PROFILE
    }
    .into();
}
pub fn port_context(store: &types::Store, port: Id, usage: Id) -> bool {
    store.usages.get(&usage).is_some_and(|u| {
        u.kind == UsageKind::Part && store.ports.get(&port).is_some_and(|p| p.owner == u.target)
    })
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorEdge {
    pub id: VisualEdgeId,
    pub view: ViewId,
    #[serde(with = "ports::port_id")]
    pub connector: Id,
    pub source: OccurrenceId,
    pub target: OccurrenceId,
    pub visible: bool,
    pub presence: Presence,
}
#[derive(Clone, Debug)]
pub enum Layout {
    Route {
        view: ViewId,
        connector: Id,
        source: OccurrenceId,
        target: OccurrenceId,
    },
    Visible(VisualEdgeId, bool),
    Remove(VisualEdgeId),
}
fn endpoints<'a>(
    doc: &'a Document,
    e: &ConnectorEdge,
) -> Result<(&'a ports::PortOccurrence, &'a ports::PortOccurrence), String> {
    let a = doc
        .typed_ports
        .get(&e.source)
        .ok_or("VIEWS_CONNECTOR_ENDPOINT_MISSING")?;
    let b = doc
        .typed_ports
        .get(&e.target)
        .ok_or("VIEWS_CONNECTOR_ENDPOINT_MISSING")?;
    if a.view != e.view || b.view != e.view || a.usage.is_none() || b.usage.is_none() {
        return Err("VIEWS_CONNECTOR_ENDPOINT_CONTEXT".into());
    }
    Ok((a, b))
}
pub fn resolves(doc: &Document, e: &ConnectorEdge, store: &types::Store) -> bool {
    let Ok((a, b)) = endpoints(doc, e) else {
        return false;
    };
    let Some(c) = store.connectors.get(&e.connector) else {
        return false;
    };
    a.presence.active()
        && b.presence.active()
        && port_context(store, a.port, a.usage.unwrap())
        && port_context(store, b.port, b.usage.unwrap())
        && c.source
            == Endpoint {
                usage: a.usage.unwrap(),
                port: a.port,
            }
        && c.target
            == Endpoint {
                usage: b.usage.unwrap(),
                port: b.port,
            }
}
pub(super) fn validate(
    doc: &Document,
    ids: &mut BTreeSet<u64>,
    snapshot: Option<&Snapshot>,
) -> Result<(), String> {
    if (!doc.connector_edges.is_empty() || doc.typed_ports.values().any(|o| o.usage.is_some()))
        && !qualified_profile(&doc.profile)
    {
        return Err("VIEWS_QUALIFIED_CONTEXT_REQUIRES_V4".into());
    }
    if doc.connector_edges.is_empty() {
        return Ok(());
    }
    let store = if doc.connector_edges.values().any(|e| e.presence.active()) {
        snapshot.map(types::load).transpose()?
    } else {
        None
    };
    for (id, e) in &doc.connector_edges {
        if *id != e.id
            || !ids.insert(id.0)
            || e.connector.0 == 0
            || !doc.views.contains_key(&e.view)
        {
            return Err("VIEWS_CONNECTOR_EDGE_ID".into());
        }
        let (a, b) = endpoints(doc, e)?;
        check_presence(&e.presence)?;
        if e.presence.active()
            && (!a.presence.active()
                || !b.presence.active()
                || store.as_ref().is_some_and(|s| !resolves(doc, e, s)))
        {
            return Err("VIEWS_CONNECTOR_SEMANTIC_OR_CONTEXT_MISMATCH".into());
        }
    }
    Ok(())
}
pub(super) fn apply(doc: &mut Document, snapshot: &Snapshot, action: Layout) -> Result<(), String> {
    match action {
        Layout::Route {
            view,
            connector,
            source,
            target,
        } => {
            if doc.edges.len() + doc.connector_edges.len() >= MAX_EDGES {
                return Err("VIEWS_EDGE_BUDGET".into());
            }
            if doc.connector_edges.values().any(|e| {
                e.view == view
                    && e.connector == connector
                    && e.source == source
                    && e.target == target
            }) {
                return Err("VIEWS_CONNECTOR_DUPLICATE_ROUTE".into());
            }
            let e = ConnectorEdge {
                id: VisualEdgeId(doc.allocate()?),
                view,
                connector,
                source,
                target,
                visible: true,
                presence: Presence::Active,
            };
            if !resolves(doc, &e, &types::load(snapshot)?) {
                return Err("VIEWS_CONNECTOR_SEMANTIC_OR_CONTEXT_MISMATCH".into());
            }
            upgrade(doc);
            doc.connector_edges.insert(e.id, e);
        }
        Layout::Visible(id, visible) => {
            doc.connector_edges
                .get_mut(&id)
                .ok_or("VIEWS_NO_CONNECTOR_EDGE")?
                .visible = visible
        }
        Layout::Remove(id) => {
            doc.connector_edges
                .remove(&id)
                .ok_or("VIEWS_NO_CONNECTOR_EDGE")?;
        }
    }
    Ok(())
}
pub(super) fn reconcile(
    doc: &mut Document,
    snapshot: &Snapshot,
    undo: bool,
    redo: bool,
    proof: &HistoryProof,
    proofs: &mut BTreeMap<u64, HistoryProof>,
) -> Result<(), String> {
    if doc.connector_edges.is_empty() {
        return Ok(());
    }
    let store = if snapshot.extensions.is_none() {
        types::Store::default()
    } else {
        types::load(snapshot)?
    };
    let states: Vec<_> = doc
        .connector_edges
        .values()
        .map(|e| (e.id, resolves(doc, e, &store)))
        .collect();
    for (id, resolves) in states {
        let e = doc.connector_edges.get_mut(&id).unwrap();
        if e.presence.active() && !resolves {
            e.presence = Presence::Detached {
                reason: "Semantic connector or qualified endpoint removed or changed".into(),
            };
            if proof.before!=proof.after {
                proofs.insert(id.0, proof.clone());
            }
        } else if !e.presence.active() && resolves && (undo||redo) && matches_inverse(proofs.get(&id.0), proof)
        {
            e.presence = Presence::Active;
            proofs.remove(&id.0);
        }
    }
    Ok(())
}
