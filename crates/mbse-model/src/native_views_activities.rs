//! Independent activity diagram presentations; semantic flows remain native typed data.
use super::*;
use crate::types::{self as t, activities as a};
pub const PROFILE: &str = "mbse-craft.native-views/6";
pub const RECOVERY_PROFILE: &str = "mbse-craft.native-views-recovery/6";
pub fn profile(s: &str) -> bool {
    matches!(s, PROFILE | RECOVERY_PROFILE)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Partition,
    Node(a::NodeKind),
    Pin(a::Direction),
}
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
    pub activity: t::Id,
    pub occurrences: BTreeMap<OccurrenceId, Occurrence>,
}
#[derive(Clone, Debug)]
pub enum Layout {
    Create {
        activity: t::Id,
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
pub fn elements(activity: &a::Activity) -> Vec<(t::Id, Kind)> {
    activity
        .partitions
        .keys()
        .map(|id| (*id, Kind::Partition))
        .chain(activity.nodes.values().map(|n| (n.id, Kind::Node(n.kind))))
        .chain(
            activity
                .pins
                .values()
                .map(|p| (p.id, Kind::Pin(p.direction))),
        )
        .collect()
}
pub fn kind(activity: &a::Activity, id: t::Id) -> Option<Kind> {
    if activity.partitions.contains_key(&id) {
        Some(Kind::Partition)
    } else if let Some(n) = activity.nodes.get(&id) {
        Some(Kind::Node(n.kind))
    } else {
        activity.pins.get(&id).map(|p| Kind::Pin(p.direction))
    }
}
fn resolves(d: &Diagram, o: &Occurrence, snapshot: &Snapshot, s: &t::Store) -> bool {
    d.source_document == snapshot.document_id
        && s.activities
            .get(&d.activity)
            .and_then(|a| kind(a, o.element))
            == Some(o.expected)
}
pub(super) fn count(doc: &Document) -> usize {
    doc.activity_diagrams
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
    if !doc.activity_diagrams.is_empty() && !profile(&doc.profile) {
        return Err("VIEWS_ACTIVITY_REQUIRES_V6".into());
    }
    let store = if let Some(snapshot) = snapshot {
        if doc
            .activity_diagrams
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
    for (view, d) in &doc.activity_diagrams {
        if !doc.views.contains_key(view)
            || d.activity.0 == 0
            || d.source_document.0.is_empty()
            || d.source_document.0.len() > 4096
        {
            return Err("VIEWS_ACTIVITY_BINDING".into());
        }
        for (key, o) in &d.occurrences {
            if *key != o.id || !ids.insert(o.id.0) || o.element.0 == 0 || !o.geometry.valid() {
                return Err("VIEWS_ACTIVITY_OCCURRENCE_INVALID".into());
            }
            check_presence(&o.presence)?;
            if o.presence.active()
                && snapshot
                    .zip(store.as_ref())
                    .is_some_and(|(snapshot, s)| !resolves(d, o, snapshot, s))
            {
                return Err("VIEWS_ACTIVITY_ACTIVE_TARGET_MISSING_OR_KIND".into());
            }
            *counts.entry(*view).or_default() += 1;
        }
    }
    Ok(())
}
fn upgrade(doc: &mut Document) {
    doc.profile = if bound_profile(&doc.profile) {
        PROFILE
    } else {
        RECOVERY_PROFILE
    }
    .into();
}
pub(super) fn apply(doc: &mut Document, snapshot: &Snapshot, action: Layout) -> Result<(), String> {
    match action {
        Layout::Create { activity, title } => {
            let store = t::load(snapshot)?;
            let activity = store
                .activities
                .get(&activity)
                .ok_or("VIEWS_ACTIVITY_MISSING")?;
            if title.trim().is_empty() || title.len() > 1024 {
                return Err("VIEWS_ACTIVITY_TITLE".into());
            }
            let elements = elements(activity);
            if doc.views.len() >= MAX_VIEWS
                || elements.len() > MAX_PER_VIEW
                || doc.occurrences.len() + doc.typed_ports.len() + count(doc) + elements.len()
                    > MAX_OCCURRENCES
            {
                return Err("VIEWS_ACTIVITY_CAPACITY".into());
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
            doc.activity_diagrams.insert(
                view,
                Diagram {
                    source_document: snapshot.document_id.clone(),
                    activity: activity.id,
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
                .activity_diagrams
                .get(&view)
                .ok_or("VIEWS_NO_ACTIVITY_DIAGRAM")?;
            let s = t::load(snapshot)?;
            if d.source_document != snapshot.document_id {
                return Err("VIEWS_ACTIVITY_FOREIGN_SOURCE".into());
            }
            let expected = s
                .activities
                .get(&d.activity)
                .and_then(|a| {
                    elements(a)
                        .into_iter()
                        .find(|(id, _)| *id == element)
                        .map(|(_, k)| k)
                })
                .ok_or("VIEWS_ACTIVITY_ELEMENT_MISSING_OR_FOREIGN")?;
            let native = doc.occurrences.values().filter(|o| o.view == view).count()
                + doc.typed_ports.values().filter(|o| o.view == view).count();
            if doc.occurrences.len() + doc.typed_ports.len() + count(doc) >= MAX_OCCURRENCES
                || d.occurrences.len() + native >= MAX_PER_VIEW
            {
                return Err("VIEWS_ACTIVITY_CAPACITY".into());
            }
            let id = OccurrenceId(doc.allocate()?);
            doc.activity_diagrams
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
                .activity_diagrams
                .get_mut(&view)
                .ok_or("VIEWS_NO_ACTIVITY_DIAGRAM")?
                .occurrences
                .remove(&occurrence)
                .is_none()
            {
                return Err("VIEWS_NO_ACTIVITY_OCCURRENCE".into());
            }
        }
        Layout::Clear(view) => {
            if doc.activity_diagrams.remove(&view).is_none() {
                return Err("VIEWS_NO_ACTIVITY_DIAGRAM".into());
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
    doc.activity_diagrams
        .get_mut(&view)
        .ok_or("VIEWS_NO_ACTIVITY_DIAGRAM")?
        .occurrences
        .get_mut(&id)
        .ok_or_else(|| "VIEWS_NO_ACTIVITY_OCCURRENCE".into())
}
pub(super) fn reconcile(
    doc: &mut Document,
    snapshot: &Snapshot,
    undo: bool,
    redo: bool,
    proof: &HistoryProof,
    proofs: &mut BTreeMap<u64, HistoryProof>,
) -> Result<(), String> {
    if doc.activity_diagrams.is_empty() {
        return Ok(());
    }
    let store = t::load(snapshot)?;
    for d in doc.activity_diagrams.values_mut() {
        let binding = Diagram {
            source_document: d.source_document.clone(),
            activity: d.activity,
            occurrences: BTreeMap::new(),
        };
        for o in d.occurrences.values_mut() {
            let current = resolves(&binding, o, snapshot, &store);
            if o.presence.active() && !current {
                o.presence = Presence::Detached {
                    reason: "Activity element missing or kind changed; original identity retained"
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
    pub flow: t::Id,
    pub source: OccurrenceId,
    pub target: OccurrenceId,
    pub kind: a::FlowKind,
    pub guard: Option<a::Guard>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Projection {
    pub activity_resolved: bool,
    pub routes: Vec<Route>,
    pub unshown_flows: usize,
    pub limitations: Vec<String>,
}
pub fn project(d: &Diagram, snapshot: &Snapshot) -> Result<Projection, String> {
    let mut p=Projection{activity_resolved:false,routes:Vec::new(),unshown_flows:0,limitations:vec!["Diagram presentation is documentary; no activity execution, reachability or liveness is inferred. Flows render only with unique visible active endpoints.".into()]};
    if d.source_document != snapshot.document_id {
        p.limitations
            .push("Foreign activity source; no semantic ID is rebound.".into());
        return Ok(p);
    }
    let s = t::load(snapshot)?;
    let Some(a) = s.activities.get(&d.activity) else {
        p.limitations
            .push("Activity missing; original references/layout retained.".into());
        return Ok(p);
    };
    p.activity_resolved = true;
    let mut by_element = BTreeMap::<t::Id, Vec<OccurrenceId>>::new();
    for o in d
        .occurrences
        .values()
        .filter(|o| o.visible && o.presence.active() && resolves(d, o, snapshot, &s))
    {
        by_element.entry(o.element).or_default().push(o.id);
    }
    for f in a.flows.values() {
        if let (Some(src), Some(dst)) = (by_element.get(&f.source), by_element.get(&f.target)) {
            if src.len() == 1 && dst.len() == 1 {
                p.routes.push(Route {
                    flow: f.id,
                    source: src[0],
                    target: dst[0],
                    kind: f.kind,
                    guard: f.guard,
                });
                continue;
            }
        }
        p.unshown_flows += 1;
    }
    Ok(p)
}
pub(super) fn check_wire(
    root: &serde_json::Map<String, serde_json::Value>,
    profile_name: &str,
) -> Result<(), String> {
    if root.contains_key("activity_diagrams") && !profile(profile_name) {
        return Err("VIEWS_ACTIVITY_REQUIRES_V6".into());
    }
    Ok(())
}
