//! Typed port occurrences share presentation identity/history, not core record kinds.
use super::*;
pub const PROFILE: &str = "mbse-craft.native-views/3";
pub const RECOVERY_PROFILE: &str = "mbse-craft.native-views-recovery/3";
pub fn typed_profile(profile: &str) -> bool {
    matches!(profile, PROFILE | RECOVERY_PROFILE)
}
mod port_id {
    use super::*;
    pub fn serialize<S: Serializer>(id: &crate::types::Id, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&id.0.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<crate::types::Id, D::Error> {
        decimal(d).map(crate::types::Id)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortOccurrence {
    pub id: OccurrenceId,
    pub view: ViewId,
    #[serde(with = "port_id")]
    pub port: crate::types::Id,
    pub geometry: Geometry,
    pub visible: bool,
    pub pinned: bool,
    pub presence: Presence,
}
#[derive(Clone, Debug)]
pub enum Layout {
    Reuse {
        view: ViewId,
        port: crate::types::Id,
        geometry: Geometry,
    },
    Move(OccurrenceId, Geometry),
    Visible(OccurrenceId, bool),
    Pin(OccurrenceId, bool),
    Remove(OccurrenceId),
}
pub(super) fn validate(
    doc: &Document,
    ids: &mut BTreeSet<u64>,
    counts: &mut BTreeMap<ViewId, usize>,
    snapshot: Option<&Snapshot>,
) -> Result<(), String> {
    if !doc.typed_ports.is_empty() && !typed_profile(&doc.profile) {
        return Err("VIEWS_TYPED_PORTS_REQUIRE_V3".into());
    }
    let store = if let Some(s) = snapshot {
        if doc.typed_ports.values().any(|o| o.presence.active()) {
            Some(crate::types::load(s)?)
        } else {
            None
        }
    } else {
        None
    };
    for (id, o) in &doc.typed_ports {
        if *id != o.id
            || !ids.insert(id.0)
            || !doc.views.contains_key(&o.view)
            || o.port.0 == 0
            || !o.geometry.valid()
        {
            return Err("VIEWS_TYPED_PORT_OCCURRENCE_INVALID".into());
        }
        if o.presence.active()
            && store
                .as_ref()
                .is_some_and(|s| !s.ports.contains_key(&o.port))
        {
            return Err("VIEWS_TYPED_PORT_TARGET_MISSING_OR_KIND".into());
        }
        check_presence(&o.presence)?;
        *counts.entry(o.view).or_default() += 1;
    }
    Ok(())
}
pub(super) fn apply(doc: &mut Document, snapshot: &Snapshot, action: Layout) -> Result<(), String> {
    match action {
        Layout::Reuse {
            view,
            port,
            geometry,
        } => {
            if !crate::types::load(snapshot)?.ports.contains_key(&port) {
                return Err("VIEWS_TYPED_PORT_TARGET_MISSING_OR_KIND".into());
            }
            if doc.occurrences.len() + doc.typed_ports.len() >= MAX_OCCURRENCES
                || doc.occurrences.values().filter(|o| o.view == view).count()
                    + doc.typed_ports.values().filter(|o| o.view == view).count()
                    >= MAX_PER_VIEW
            {
                return Err("VIEWS_OCCURRENCE_BUDGET".into());
            }
            doc.profile = if bound_profile(&doc.profile) {
                PROFILE
            } else {
                RECOVERY_PROFILE
            }
            .into();
            let id = OccurrenceId(doc.allocate()?);
            doc.typed_ports.insert(
                id,
                PortOccurrence {
                    id,
                    view,
                    port,
                    geometry,
                    visible: true,
                    pinned: false,
                    presence: Presence::Active,
                },
            );
        }
        Layout::Move(id, geometry) => {
            let o = doc
                .typed_ports
                .get_mut(&id)
                .ok_or("VIEWS_NO_PORT_OCCURRENCE")?;
            if o.pinned {
                return Err("VIEWS_PINNED".into());
            }
            o.geometry = geometry;
        }
        Layout::Visible(id, visible) => {
            doc.typed_ports
                .get_mut(&id)
                .ok_or("VIEWS_NO_PORT_OCCURRENCE")?
                .visible = visible
        }
        Layout::Pin(id, pinned) => {
            doc.typed_ports
                .get_mut(&id)
                .ok_or("VIEWS_NO_PORT_OCCURRENCE")?
                .pinned = pinned
        }
        Layout::Remove(id) => {
            doc.typed_ports
                .remove(&id)
                .ok_or("VIEWS_NO_PORT_OCCURRENCE")?;
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
    if doc.typed_ports.is_empty() {
        return Ok(());
    }
    let store = match crate::types::load(snapshot) {
        Ok(store) => store,
        Err(_) if doc.typed_ports.values().all(|o| !o.presence.active()) => {
            crate::types::Store::default()
        }
        Err(error) => return Err(error),
    };
    for o in doc.typed_ports.values_mut() {
        let resolves = store.ports.contains_key(&o.port);
        if o.presence.active() && !resolves {
            o.presence = Presence::Detached {
                reason: "Semantic port removed by model history".into(),
            };
            if undo {
                proofs.insert(o.id.0, proof.clone());
            }
        } else if !o.presence.active()
            && resolves
            && redo
            && matches_redo(proofs.get(&o.id.0), proof)
        {
            o.presence = Presence::Active;
            proofs.remove(&o.id.0);
        }
    }
    Ok(())
}
