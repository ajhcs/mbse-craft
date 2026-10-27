//! Independent sequence presentations; accepted participants/messages/order/guards remain native typed data.
use super::*;
use crate::types::{self as t, scenarios as a};
pub const PROFILE: &str = "mbse-craft.native-views/8";
pub const RECOVERY_PROFILE: &str = "mbse-craft.native-views-recovery/8";
pub fn profile(s: &str) -> bool {
    matches!(s, PROFILE | RECOVERY_PROFILE)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Lifeline,
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
    pub scenario: t::Id,
    pub occurrences: BTreeMap<OccurrenceId, Occurrence>,
}
#[derive(Clone, Debug)]
pub enum Layout {
    Create {
        scenario: t::Id,
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
pub fn elements(scenario: &a::Scenario) -> Vec<(t::Id, Kind)> {
    scenario
        .lifelines
        .keys()
        .map(|id| (*id, Kind::Lifeline))
        .collect()
}
pub fn kind(scenario: &a::Scenario, id: t::Id) -> Option<Kind> {
    scenario
        .lifelines
        .contains_key(&id)
        .then_some(Kind::Lifeline)
}
fn resolves(d: &Diagram, o: &Occurrence, snapshot: &Snapshot, s: &t::Store) -> bool {
    d.source_document == snapshot.document_id
        && s.scenarios
            .get(&d.scenario)
            .and_then(|a| kind(a, o.element))
            == Some(o.expected)
}
pub(super) fn count(doc: &Document) -> usize {
    doc.scenario_diagrams
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
    if doc.scenario_diagrams.is_empty() {
        return Ok(());
    }
    if !doc.scenario_diagrams.is_empty() && !profile(&doc.profile) {
        return Err("VIEWS_SCENARIO_REQUIRES_V8".into());
    }
    let store = if let Some(snapshot) = snapshot {
        snapshot.validate()?;
        if doc
            .scenario_diagrams
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
    for (view, d) in &doc.scenario_diagrams {
        if doc.activity_diagrams.contains_key(view)
            || doc.state_machine_diagrams.contains_key(view)
            || doc.occurrences.values().any(|o| o.view == *view)
            || doc.typed_ports.values().any(|o| o.view == *view)
            || doc.edges.values().any(|o| o.view == *view)
            || doc.connector_edges.values().any(|o| o.view == *view)
        {
            return Err("VIEWS_SCENARIO_VIEW_MUST_CONTAIN_ONLY_LIFELINES".into());
        }
        if !doc.views.contains_key(view)
            || d.scenario.0 == 0
            || d.source_document.0.is_empty()
            || d.source_document.0.len() > 4096
        {
            return Err("VIEWS_SCENARIO_BINDING".into());
        }
        for (key, o) in &d.occurrences {
            if *key != o.id || !ids.insert(o.id.0) || o.element.0 == 0 || !o.geometry.valid() {
                return Err("VIEWS_SCENARIO_OCCURRENCE_INVALID".into());
            }
            check_presence(&o.presence)?;
            if o.presence.active()
                && snapshot
                    .zip(store.as_ref())
                    .is_some_and(|(snapshot, s)| !resolves(d, o, snapshot, s))
            {
                return Err("VIEWS_SCENARIO_ACTIVE_TARGET_MISSING_OR_KIND".into());
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
    snapshot.validate()?;
    match action {
        Layout::Create { scenario, title } => {
            let store = t::load(snapshot)?;
            let scenario = store
                .scenarios
                .get(&scenario)
                .ok_or("VIEWS_SCENARIO_MISSING")?;
            if title.trim().is_empty() || title.len() > 1024 {
                return Err("VIEWS_SCENARIO_TITLE".into());
            }
            let elements = elements(scenario);
            if doc.views.len() >= MAX_VIEWS
                || elements.len() > MAX_PER_VIEW
                || doc.occurrences.len()
                    + doc.typed_ports.len()
                    + activities::count(doc)
                    + state_machines::count(doc)
                    + count(doc)
                    + elements.len()
                    > MAX_OCCURRENCES
            {
                return Err("VIEWS_SCENARIO_CAPACITY".into());
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
                        geometry: Geometry::at(25. + i as f64 * 294., 24.),
                        visible: true,
                        pinned: false,
                        presence: Presence::Active,
                    },
                );
            }
            doc.scenario_diagrams.insert(
                view,
                Diagram {
                    source_document: snapshot.document_id.clone(),
                    scenario: scenario.id,
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
                .scenario_diagrams
                .get(&view)
                .ok_or("VIEWS_NO_SCENARIO_DIAGRAM")?;
            let s = t::load(snapshot)?;
            if d.source_document != snapshot.document_id {
                return Err("VIEWS_SCENARIO_FOREIGN_SOURCE".into());
            }
            let expected = s
                .scenarios
                .get(&d.scenario)
                .and_then(|a| {
                    elements(a)
                        .into_iter()
                        .find(|(id, _)| *id == element)
                        .map(|(_, k)| k)
                })
                .ok_or("VIEWS_SCENARIO_ELEMENT_MISSING_OR_FOREIGN")?;
            let native = doc.occurrences.values().filter(|o| o.view == view).count()
                + doc.typed_ports.values().filter(|o| o.view == view).count();
            if doc.occurrences.len()
                + doc.typed_ports.len()
                + activities::count(doc)
                + state_machines::count(doc)
                + count(doc)
                >= MAX_OCCURRENCES
                || d.occurrences.len() + native >= MAX_PER_VIEW
            {
                return Err("VIEWS_SCENARIO_CAPACITY".into());
            }
            let id = OccurrenceId(doc.allocate()?);
            doc.scenario_diagrams
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
                .scenario_diagrams
                .get_mut(&view)
                .ok_or("VIEWS_NO_SCENARIO_DIAGRAM")?
                .occurrences
                .remove(&occurrence)
                .is_none()
            {
                return Err("VIEWS_NO_SCENARIO_OCCURRENCE".into());
            }
        }
        Layout::Clear(view) => {
            if doc.scenario_diagrams.remove(&view).is_none() {
                return Err("VIEWS_NO_SCENARIO_DIAGRAM".into());
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
    doc.scenario_diagrams
        .get_mut(&view)
        .ok_or("VIEWS_NO_SCENARIO_DIAGRAM")?
        .occurrences
        .get_mut(&id)
        .ok_or_else(|| "VIEWS_NO_SCENARIO_OCCURRENCE".into())
}
pub(super) fn reconcile(
    doc: &mut Document,
    snapshot: &Snapshot,
    undo: bool,
    redo: bool,
    proof: &HistoryProof,
    proofs: &mut BTreeMap<u64, HistoryProof>,
) -> Result<(), String> {
    if doc.scenario_diagrams.is_empty() {
        return Ok(());
    }
    let store = t::load(snapshot)?;
    for d in doc.scenario_diagrams.values_mut() {
        let binding = Diagram {
            source_document: d.source_document.clone(),
            scenario: d.scenario,
            occurrences: BTreeMap::new(),
        };
        for o in d.occurrences.values_mut() {
            let current = resolves(&binding, o, snapshot, &store);
            if o.presence.active() && !current {
                o.presence = Presence::Detached {
                    reason: "Scenario lifeline missing; original identity retained".into(),
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
pub const ROW_HEIGHT: f64 = 80.;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Row {
    Message { message: t::Id },
    AlternativeStart { alternative: t::Id },
    OperandStart { alternative: t::Id, operand: t::Id },
    AlternativeEnd { alternative: t::Id },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Route {
    pub message: t::Id,
    pub source: OccurrenceId,
    pub target: OccurrenceId,
    pub row: usize,
    pub kind: a::MessageKind,
    pub operand: Option<t::Id>,
    pub reply_to: Option<t::Id>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Projection {
    pub scenario_resolved: bool,
    pub rows: Vec<Row>,
    pub routes: Vec<Route>,
    pub unresolved_lifelines: Vec<t::Id>,
    pub unshown_messages: usize,
    pub limitations: Vec<String>,
}
pub fn project(d: &Diagram, snapshot: &Snapshot) -> Result<Projection, String> {
    let mut p=Projection{scenario_resolved:false,rows:Vec::new(),routes:Vec::new(),unresolved_lifelines:Vec::new(),unshown_messages:0,limitations:vec!["Documentary authored order and guarded alternatives; no execution, causality, timing or guard truth. Arrows require unique visible active lifelines with exact resolved participants.".into()]};
    if d.source_document != snapshot.document_id {
        p.limitations
            .push("Foreign scenario source; no semantic ID is rebound.".into());
        return Ok(p);
    }
    snapshot.validate()?;
    let s = t::load(snapshot)?;
    let Some(a) = s.scenarios.get(&d.scenario) else {
        p.limitations
            .push("Scenario missing; original references and layout retained.".into());
        return Ok(p);
    };
    p.scenario_resolved = true;
    for fragment in a::main_sequence(a) {
        match fragment {
            a::Fragment::Message(id) => p.rows.push(Row::Message { message: id }),
            a::Fragment::Alternative(id) => {
                p.rows.push(Row::AlternativeStart { alternative: id });
                let alt = &a.alternatives[&id];
                for o in a::ordered_operands(alt) {
                    p.rows.push(Row::OperandStart {
                        alternative: id,
                        operand: o.id,
                    });
                    for message in a::operand_sequence(a, o.id)? {
                        p.rows.push(Row::Message { message });
                    }
                }
                p.rows.push(Row::AlternativeEnd { alternative: id });
            }
        }
    }
    let mut by_element = BTreeMap::<t::Id, Vec<OccurrenceId>>::new();
    for l in a.lifelines.values() {
        if a::participant_status(snapshot, &s, a, l).is_err() {
            p.unresolved_lifelines.push(l.id);
        }
    }
    for o in d.occurrences.values().filter(|o| {
        o.visible
            && o.presence.active()
            && resolves(d, o, snapshot, &s)
            && !p.unresolved_lifelines.contains(&o.element)
    }) {
        by_element.entry(o.element).or_default().push(o.id);
    }
    for (row, item) in p.rows.iter().enumerate() {
        if let Row::Message { message } = item {
            let m = &a.messages[message];
            if let (Some(src), Some(dst)) = (by_element.get(&m.source), by_element.get(&m.target)) {
                if src.len() == 1 && dst.len() == 1 {
                    p.routes.push(Route {
                        message: m.id,
                        source: src[0],
                        target: dst[0],
                        row,
                        kind: m.kind,
                        operand: m.operand,
                        reply_to: m.reply_to,
                    });
                    continue;
                }
            }
            p.unshown_messages += 1;
        }
    }
    Ok(p)
}
/// Presentation extent, including hidden-arrow authored rows; never a semantic coordinate.
pub fn bounds(d: &Diagram, snapshot: &Snapshot) -> Result<Option<[f64; 4]>, String> {
    let p = project(d, snapshot)?;
    if !p.scenario_resolved {
        return Ok(None);
    }
    let s = t::load(snapshot)?;
    let visible: Vec<_> = d
        .occurrences
        .values()
        .filter(|o| o.visible && o.presence.active() && resolves(d, o, snapshot, &s))
        .collect();
    if visible.is_empty() {
        return Ok(None);
    }
    let left = visible
        .iter()
        .map(|o| o.geometry.x)
        .fold(f64::INFINITY, f64::min);
    let top = visible
        .iter()
        .map(|o| o.geometry.y)
        .fold(f64::INFINITY, f64::min);
    let right = visible
        .iter()
        .map(|o| o.geometry.x + o.geometry.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = visible
        .iter()
        .map(|o| o.geometry.y + o.geometry.height)
        .fold(f64::NEG_INFINITY, f64::max)
        + 48.
        + (p.rows.len() + 1) as f64 * ROW_HEIGHT;
    Ok(Some([left, top, right, bottom]))
}
pub(super) fn check_wire(
    root: &serde_json::Map<String, serde_json::Value>,
    profile_name: &str,
) -> Result<(), String> {
    if root.contains_key("scenario_diagrams") && !profile(profile_name) {
        return Err("VIEWS_SCENARIO_REQUIRES_V8".into());
    }
    Ok(())
}
