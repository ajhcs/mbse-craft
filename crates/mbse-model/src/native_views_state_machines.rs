//! Independent machine diagram presentations; semantic transitions remain native typed data.
use super::*;
use crate::types::{self as t, state_machines as a};
pub const PROFILE: &str = "mbse-craft.native-views/7";
pub const RECOVERY_PROFILE: &str = "mbse-craft.native-views-recovery/7";
pub fn profile(s: &str) -> bool {
    matches!(s, PROFILE | RECOVERY_PROFILE)||super::scenarios::profile(s)
}
pub use a::StateKind as Kind;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Occurrence {
    pub id: OccurrenceId,
    #[serde(with = "ports::port_id")]
    pub element: t::Id,
    pub expected: Kind,
    pub geometry: Geometry,
    pub visible: bool,
    pub pinned: bool,
    pub presence: Presence,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagram {
    pub source_document: DocumentId,
    #[serde(with = "ports::port_id")]
    pub machine: t::Id,
    pub occurrences: BTreeMap<OccurrenceId, Occurrence>,
}
#[derive(Clone, Debug)]
pub enum Layout {
    Create {
        machine: t::Id,
        title: String,
    },
    Reuse {
        view: ViewId,
        element: t::Id,
        geometry: Geometry,
    },
    Move {
        view: ViewId,
        occurrence: OccurrenceId,
        geometry: Geometry,
    },
    Visible {
        view: ViewId,
        occurrence: OccurrenceId,
        visible: bool,
    },
    Pin {
        view: ViewId,
        occurrence: OccurrenceId,
        pinned: bool,
    },
    Remove {
        view: ViewId,
        occurrence: OccurrenceId,
    },
    Clear(ViewId),
}
pub fn elements(machine: &a::Machine) -> Vec<(t::Id, Kind)> {
    machine.states.values().map(|n| (n.id, n.kind)).collect()
}
pub fn kind(machine: &a::Machine, id: t::Id) -> Option<Kind> {
    machine.states.get(&id).map(|n| n.kind)
}
fn resolves(d: &Diagram, o: &Occurrence, snapshot: &Snapshot, s: &t::Store) -> bool {
    d.source_document == snapshot.document_id
        && s.state_machines
            .get(&d.machine)
            .and_then(|a| kind(a, o.element))
            == Some(o.expected)
}
pub(super) fn count(doc: &Document) -> usize {
    doc.state_machine_diagrams
        .values()
        .map(|d| d.occurrences.len())
        .sum()
}
pub(super) fn validate(
    doc: &Document,
    ids: &mut BTreeSet<u64>,
    counts: &mut BTreeMap<ViewId, usize>,
    snapshot: Option<&Snapshot>,
) -> Result<(), String> {
    if !doc.state_machine_diagrams.is_empty() && !profile(&doc.profile) {
        return Err("VIEWS_STATE_MACHINE_REQUIRES_V7".into());
    }
    let store = if let Some(snapshot) = snapshot {
        if doc
            .state_machine_diagrams
            .values()
            .any(|d| d.occurrences.values().any(|o| o.presence.active()))
        {
            Some(t::load(snapshot)?)
        } else {
            None
        }
    } else {
        None
    };
    for (view, d) in &doc.state_machine_diagrams {
        if doc.activity_diagrams.contains_key(view)
            || doc.scenario_diagrams.contains_key(view)
            || doc.occurrences.values().any(|o| o.view == *view)
            || doc.typed_ports.values().any(|o| o.view == *view)
            || doc.edges.values().any(|o| o.view == *view)
            || doc.connector_edges.values().any(|o| o.view == *view)
        {
            return Err("VIEWS_STATE_MACHINE_VIEW_MUST_CONTAIN_ONLY_STATES".into());
        }
        if !doc.views.contains_key(view)
            || d.machine.0 == 0
            || d.source_document.0.is_empty()
            || d.source_document.0.len() > 4096
        {
            return Err("VIEWS_STATE_MACHINE_BINDING".into());
        }
        for (key, o) in &d.occurrences {
            if *key != o.id || !ids.insert(o.id.0) || o.element.0 == 0 || !o.geometry.valid() {
                return Err("VIEWS_STATE_MACHINE_OCCURRENCE_INVALID".into());
            }
            check_presence(&o.presence)?;
            if o.presence.active()
                && snapshot
                    .zip(store.as_ref())
                    .is_some_and(|(snapshot, s)| !resolves(d, o, snapshot, s))
            {
                return Err("VIEWS_STATE_MACHINE_ACTIVE_TARGET_MISSING_OR_KIND".into());
            }
            *counts.entry(*view).or_default() += 1;
        }
    }
    Ok(())
}
fn upgrade(doc: &mut Document) {
    if super::scenarios::profile(&doc.profile){return;}
    doc.profile = if bound_profile(&doc.profile) {
        PROFILE
    } else {
        RECOVERY_PROFILE
    }
    .into();
}
pub(super) fn apply(doc: &mut Document, snapshot: &Snapshot, action: Layout) -> Result<(), String> {
    match action {
        Layout::Create { machine, title } => {
            let store = t::load(snapshot)?;
            let machine = store
                .state_machines
                .get(&machine)
                .ok_or("VIEWS_STATE_MACHINE_MISSING")?;
            if title.trim().is_empty() || title.len() > 1024 {
                return Err("VIEWS_STATE_MACHINE_TITLE".into());
            }
            let elements = elements(machine);
            if doc.views.len() >= MAX_VIEWS
                || elements.len() > MAX_PER_VIEW
                || doc.occurrences.len()
                    + doc.typed_ports.len()
                    + activities::count(doc)
                    + super::scenarios::count(doc)
                    + count(doc)
                    + elements.len()
                    > MAX_OCCURRENCES
            {
                return Err("VIEWS_STATE_MACHINE_CAPACITY".into());
            }
            let view = ViewId(doc.allocate()?);
            doc.views.insert(
                view,
                View {
                    id: view,
                    title,
                    camera: Camera::default(),
                    copy_origin: None,
                    viewpoint: None,
                    viewpoint_form: None,
                },
            );
            let mut occurrences = BTreeMap::new();
            for (i, (element, kind)) in elements.into_iter().enumerate() {
                let id = OccurrenceId(doc.allocate()?);
                occurrences.insert(
                    id,
                    Occurrence {
                        id,
                        element,
                        expected: kind,
                        geometry: Geometry::at(
                            25. + (i % 4) as f64 * 294.,
                            24. + (i / 4) as f64 * 155.,
                        ),
                        visible: true,
                        pinned: false,
                        presence: Presence::Active,
                    },
                );
            }
            doc.state_machine_diagrams.insert(
                view,
                Diagram {
                    source_document: snapshot.document_id.clone(),
                    machine: machine.id,
                    occurrences,
                },
            );
            upgrade(doc);
        }
        Layout::Reuse {
            view,
            element,
            geometry,
        } => {
            let d = doc
                .state_machine_diagrams
                .get(&view)
                .ok_or("VIEWS_NO_STATE_MACHINE_DIAGRAM")?;
            let s = t::load(snapshot)?;
            if d.source_document != snapshot.document_id {
                return Err("VIEWS_STATE_MACHINE_FOREIGN_SOURCE".into());
            }
            let expected = s
                .state_machines
                .get(&d.machine)
                .and_then(|a| {
                    elements(a)
                        .into_iter()
                        .find(|(id, _)| *id == element)
                        .map(|(_, k)| k)
                })
                .ok_or("VIEWS_STATE_MACHINE_ELEMENT_MISSING_OR_FOREIGN")?;
            let native = doc.occurrences.values().filter(|o| o.view == view).count()
                + doc.typed_ports.values().filter(|o| o.view == view).count();
            if doc.occurrences.len() + doc.typed_ports.len() + activities::count(doc) + super::scenarios::count(doc) + count(doc)
                >= MAX_OCCURRENCES
                || d.occurrences.len() + native >= MAX_PER_VIEW
            {
                return Err("VIEWS_STATE_MACHINE_CAPACITY".into());
            }
            let id = OccurrenceId(doc.allocate()?);
            doc.state_machine_diagrams
                .get_mut(&view)
                .unwrap()
                .occurrences
                .insert(
                    id,
                    Occurrence {
                        id,
                        element,
                        expected,
                        geometry,
                        visible: true,
                        pinned: false,
                        presence: Presence::Active,
                    },
                );
        }
        Layout::Move {
            view,
            occurrence,
            geometry,
        } => {
            let o = occurrence_mut(doc, view, occurrence)?;
            if o.pinned {
                return Err("VIEWS_PINNED".into());
            }
            o.geometry = geometry;
        }
        Layout::Visible {
            view,
            occurrence,
            visible,
        } => occurrence_mut(doc, view, occurrence)?.visible = visible,
        Layout::Pin {
            view,
            occurrence,
            pinned,
        } => occurrence_mut(doc, view, occurrence)?.pinned = pinned,
        Layout::Remove { view, occurrence } => {
            if doc
                .state_machine_diagrams
                .get_mut(&view)
                .ok_or("VIEWS_NO_STATE_MACHINE_DIAGRAM")?
                .occurrences
                .remove(&occurrence)
                .is_none()
            {
                return Err("VIEWS_NO_STATE_MACHINE_OCCURRENCE".into());
            }
        }
        Layout::Clear(view) => {
            if doc.state_machine_diagrams.remove(&view).is_none() {
                return Err("VIEWS_NO_STATE_MACHINE_DIAGRAM".into());
            }
        }
    }
    Ok(())
}
fn occurrence_mut(
    doc: &mut Document,
    view: ViewId,
    id: OccurrenceId,
) -> Result<&mut Occurrence, String> {
    doc.state_machine_diagrams
        .get_mut(&view)
        .ok_or("VIEWS_NO_STATE_MACHINE_DIAGRAM")?
        .occurrences
        .get_mut(&id)
        .ok_or_else(|| "VIEWS_NO_STATE_MACHINE_OCCURRENCE".into())
}
pub(super) fn reconcile(
    doc: &mut Document,
    snapshot: &Snapshot,
    undo: bool,
    redo: bool,
    proof: &HistoryProof,
    proofs: &mut BTreeMap<u64, HistoryProof>,
) -> Result<(), String> {
    if doc.state_machine_diagrams.is_empty() {
        return Ok(());
    }
    let store = t::load(snapshot)?;
    for d in doc.state_machine_diagrams.values_mut() {
        let binding = Diagram {
            source_document: d.source_document.clone(),
            machine: d.machine,
            occurrences: BTreeMap::new(),
        };
        for o in d.occurrences.values_mut() {
            let current = resolves(&binding, o, snapshot, &store);
            if o.presence.active() && !current {
                o.presence = Presence::Detached {
                    reason:
                        "State machine element missing or kind changed; original identity retained"
                            .into(),
                };
                if proof.before != proof.after {
                    proofs.insert(o.id.0, proof.clone());
                }
            } else if !o.presence.active()
                && current
                && (undo || redo)
                && matches_inverse(proofs.get(&o.id.0), proof)
            {
                o.presence = Presence::Active;
                proofs.remove(&o.id.0);
            }
        }
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Route {
    pub transition: t::Id,
    pub source: OccurrenceId,
    pub target: OccurrenceId,
    pub event: Option<t::Id>,
    pub guard: Option<a::Guard>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Projection {
    pub machine_resolved: bool,
    pub routes: Vec<Route>,
    pub unshown_transitions: usize,
    pub limitations: Vec<String>,
}
pub fn project(d: &Diagram, snapshot: &Snapshot) -> Result<Projection, String> {
    let mut p=Projection{machine_resolved:false,routes:Vec::new(),unshown_transitions:0,limitations:vec!["Diagram presentation is documentary; no machine execution, reachability or liveness is inferred. Transitions render only with unique visible active endpoints.".into()]};
    if d.source_document != snapshot.document_id {
        p.limitations
            .push("Foreign machine source; no semantic ID is rebound.".into());
        return Ok(p);
    }
    let s = t::load(snapshot)?;
    let Some(a) = s.state_machines.get(&d.machine) else {
        p.limitations
            .push("State machine missing; original references/layout retained.".into());
        return Ok(p);
    };
    p.machine_resolved = true;
    let mut by_element = BTreeMap::<t::Id, Vec<OccurrenceId>>::new();
    for o in d
        .occurrences
        .values()
        .filter(|o| o.visible && o.presence.active() && resolves(d, o, snapshot, &s))
    {
        by_element.entry(o.element).or_default().push(o.id);
    }
    for f in a.transitions.values() {
        if let (Some(src), Some(dst)) = (by_element.get(&f.source), by_element.get(&f.target)) {
            if src.len() == 1 && dst.len() == 1 {
                p.routes.push(Route {
                    transition: f.id,
                    source: src[0],
                    target: dst[0],
                    event: f.event,
                    guard: f.guard,
                });
                continue;
            }
        }
        p.unshown_transitions += 1;
    }
    Ok(p)
}
pub(super) fn check_wire(
    root: &serde_json::Map<String, serde_json::Value>,
    profile_name: &str,
) -> Result<(), String> {
    if root.contains_key("state_machine_diagrams") && !profile(profile_name) {
        return Err("VIEWS_STATE_MACHINE_REQUIRES_V7".into());
    }
    Ok(())
}
