//! Experimental presentation state over Stage1. There is no semantic record store here.
//! Wire IDs are canonical decimal strings. Runtime history proof is never serialized.
use crate::{
    stage1::{
        Command, DocumentId, EditOutcome, Editor, Freshness, RawKey, RecordId, RecordKind, Snapshot,
    },
    stage1_graph::{self, RelationRole},
    stage1_io::NativeCheckpoint,
};
pub(crate) type EdgeIndex = BTreeSet<(RecordId, RelationRole, RecordId)>;
pub(crate) fn edge_index(snapshot: &Snapshot) -> Result<EdgeIndex, String> {
    Ok(stage1_graph::derive_graph(snapshot)?
        .edges
        .into_iter()
        .map(|e| (e.id.source, e.id.role, e.id.target))
        .collect())
}
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Origin(u64);
static NEXT_ORIGIN: AtomicU64 = AtomicU64::new(1);
pub(crate) fn new_origin() -> Result<Origin, String> {
    NEXT_ORIGIN
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map(Origin)
        .map_err(|_| "VIEWS_RUNTIME_ORIGIN_EXHAUSTED".into())
}

pub const PROFILE: &str = "mbse-craft.native-views/1";
pub const RECOVERY_PROFILE: &str = "mbse-craft.native-views-recovery/1";
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_VIEWS: usize = 16;
pub const MAX_OCCURRENCES: usize = 4096;
pub const MAX_EDGES: usize = 8192;
pub const MAX_PER_VIEW: usize = 1024;
pub const HISTORY_ENTRIES: usize = 64;
pub const HISTORY_BYTES: usize = 8 * 1024 * 1024;
const LIMIT: f64 = 1_000_000.;

fn decimal<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let s = String::deserialize(d)?;
    if s.is_empty() || s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(de::Error::custom(
            "expected canonical positive decimal u64 string",
        ));
    }
    s.parse().map_err(de::Error::custom)
}
macro_rules! id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(u64);
        impl $name {
            pub fn get(self) -> u64 {
                self.0
            }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.0.to_string())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                decimal(d).map(Self)
            }
        }
    };
}
id!(ViewId);
id!(OccurrenceId);
id!(VisualEdgeId);
id!(Serial);
mod native_id {
    use super::*;
    pub fn serialize<S: Serializer>(id: &RecordId, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&id.0.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<RecordId, D::Error> {
        decimal(d).map(RecordId)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeBinding {
    pub document_id: DocumentId,
    pub artifact_sha256: String,
    pub snapshot_sha256: String,
}
impl NativeBinding {
    pub fn from_checkpoint(c: &NativeCheckpoint) -> Self {
        Self {
            document_id: c.document_id().clone(),
            artifact_sha256: c.artifact_sha256().into(),
            snapshot_sha256: c.snapshot_sha256().into(),
        }
    }
    pub fn matches(&self, c: &NativeCheckpoint) -> bool {
        *self == Self::from_checkpoint(c)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}
impl Camera {
    pub const MIN_ZOOM: f64 = 0.05;
    pub const MAX_ZOOM: f64 = 20.;
    pub fn checked(x: f64, y: f64, zoom: f64) -> Result<Self, String> {
        let camera = Self { x, y, zoom };
        if !camera.valid() {
            return Err("VIEWS_CAMERA_INVALID".into());
        }
        Ok(camera)
    }
    fn valid(&self) -> bool {
        [self.x, self.y]
            .iter()
            .all(|n| n.is_finite() && n.abs() <= LIMIT)
            && self.zoom.is_finite()
            && (Self::MIN_ZOOM..=Self::MAX_ZOOM).contains(&self.zoom)
    }
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            zoom: 1.,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Geometry {
    pub fn at(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            width: 270.,
            height: 112.,
        }
    }
    fn valid(&self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= LIMIT)
            && self.width > 0.
            && self.height > 0.
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum Presence {
    Active,
    Detached { reason: String },
}
impl Presence {
    pub fn active(&self) -> bool {
        matches!(self, Self::Active)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub id: ViewId,
    pub title: String,
    pub camera: Camera,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Occurrence {
    pub id: OccurrenceId,
    pub view: ViewId,
    #[serde(with = "native_id")]
    pub record: RecordId,
    pub expected_kind: RecordKind,
    pub geometry: Geometry,
    pub visible: bool,
    pub pinned: bool,
    pub presence: Presence,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticRole {
    CapabilityContributesToMission,
    ObjectiveSubject,
    MeasureForObjective,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticKey {
    #[serde(with = "native_id")]
    pub owner: RecordId,
    pub role: SemanticRole,
    #[serde(with = "native_id")]
    pub target: RecordId,
    pub target_kind: RecordKind,
}
impl SemanticKey {
    pub fn checked(
        snapshot: &Snapshot,
        owner: RecordId,
        role: SemanticRole,
        target: RecordId,
    ) -> Result<Self, String> {
        let kind = snapshot
            .records
            .get(&target)
            .ok_or("VIEWS_EDGE_TARGET_MISSING")?
            .kind;
        let key = Self {
            owner,
            role,
            target,
            target_kind: kind,
        };
        if !key.exists(snapshot)? {
            return Err("VIEWS_SEMANTIC_EDGE_NOT_AUTHORED".into());
        }
        Ok(key)
    }
    fn exists(&self, snapshot: &Snapshot) -> Result<bool, String> {
        self.exists_in(snapshot, &edge_index(snapshot)?)
    }
    pub(crate) fn exists_in(&self, snapshot: &Snapshot, index: &EdgeIndex) -> Result<bool, String> {
        let Some(owner) = snapshot.records.get(&self.owner) else {
            return Ok(false);
        };
        if snapshot.records.get(&self.target).map(|r| r.kind) != Some(self.target_kind) {
            return Ok(false);
        }
        let (kind, role) = match self.role {
            SemanticRole::CapabilityContributesToMission => {
                (RecordKind::Capability, RelationRole::ContributesTo)
            }
            SemanticRole::ObjectiveSubject => (RecordKind::Objective, RelationRole::Subject),
            SemanticRole::MeasureForObjective => (RecordKind::Measure, RelationRole::ForObjective),
        };
        Ok(owner.kind == kind && index.contains(&(self.owner, role, self.target)))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualEdge {
    pub id: VisualEdgeId,
    pub view: ViewId,
    pub key: SemanticKey,
    pub source: OccurrenceId,
    pub target: OccurrenceId,
    pub presence: Presence,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub profile: String,
    pub sidecar_id: String,
    pub document_id: DocumentId,
    pub binding: Option<NativeBinding>,
    pub revision: Serial,
    pub next_id: Serial,
    pub primary: ViewId,
    pub views: BTreeMap<ViewId, View>,
    pub occurrences: BTreeMap<OccurrenceId, Occurrence>,
    pub edges: BTreeMap<VisualEdgeId, VisualEdge>,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn records_hash(snapshot: &Snapshot) -> Result<String, String> {
    serde_json::to_vec(&snapshot.records)
        .map(|b| hash(&b))
        .map_err(|e| e.to_string())
}
impl Document {
    fn allocate(&mut self) -> Result<u64, String> {
        let id = self.next_id.0;
        self.next_id.0 = id.checked_add(1).ok_or("VIEWS_ID_EXHAUSTED")?;
        Ok(id)
    }
    fn bump(&mut self) -> Result<(), String> {
        self.revision.0 = self
            .revision
            .0
            .checked_add(1)
            .ok_or("VIEWS_REVISION_EXHAUSTED")?;
        Ok(())
    }
    pub fn revision(&self) -> u64 {
        self.revision.0
    }
    pub fn next_id(&self) -> u64 {
        self.next_id.0
    }
    pub fn encoded(&self) -> Result<Vec<u8>, String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_BYTES {
            return Err("VIEWS_ENCODED_BUDGET".into());
        }
        Ok(bytes)
    }
    pub fn digest(&self) -> Result<String, String> {
        self.encoded().map(|b| hash(&b))
    }
    pub(crate) fn validate(&self, snapshot: &Snapshot) -> Result<(), String> {
        let index = edge_index(snapshot)?;
        self.validate_with_index(snapshot, &index)
    }
    fn validate_with_index(&self, snapshot: &Snapshot, index: &EdgeIndex) -> Result<(), String> {
        self.validate_contents(Some((snapshot, index)))
    }
    fn validate_contents(&self, native: Option<(&Snapshot, &EdgeIndex)>) -> Result<(), String> {
        if self.profile != PROFILE && self.profile != RECOVERY_PROFILE {
            return Err("VIEWS_FUTURE_PROFILE".into());
        }
        if !self.sidecar_id.starts_with("views:") || self.sidecar_id.len() > 1024 {
            return Err("VIEWS_SIDECAR_ID".into());
        }
        if native.is_some_and(|(snapshot, _)| self.document_id != snapshot.document_id) {
            return Err("VIEWS_DOCUMENT_MISMATCH".into());
        }
        if native.is_none() && (self.document_id.0.is_empty() || self.document_id.0.len() > 1024) {
            return Err("VIEWS_SOURCE_DOCUMENT_ID".into());
        }
        if self.binding.as_ref().is_some_and(|b| {
            b.document_id != self.document_id
                || !sha(&b.artifact_sha256)
                || !sha(&b.snapshot_sha256)
        }) {
            return Err("VIEWS_BINDING".into());
        }
        if self.views.is_empty()
            || self.views.len() > MAX_VIEWS
            || self.occurrences.len() > MAX_OCCURRENCES
            || self.edges.len() > MAX_EDGES
            || !self.views.contains_key(&self.primary)
        {
            return Err("VIEWS_COUNT_OR_PRIMARY".into());
        }
        let mut ids = BTreeSet::new();
        let mut counts = BTreeMap::<ViewId, usize>::new();
        for (id, v) in &self.views {
            if *id != v.id
                || !ids.insert(id.0)
                || v.title.is_empty()
                || v.title.len() > 1024
                || !v.camera.valid()
            {
                return Err("VIEWS_VIEW_INVALID".into());
            }
        }
        for (id, o) in &self.occurrences {
            if *id != o.id
                || !ids.insert(id.0)
                || !self.views.contains_key(&o.view)
                || !o.geometry.valid()
                || o.record.0 == 0
            {
                return Err("VIEWS_OCCURRENCE_INVALID".into());
            }
            *counts.entry(o.view).or_default() += 1;
            if o.presence.active()
                && native.is_some_and(|(snapshot, _)| {
                    snapshot.records.get(&o.record).map(|r| r.kind) != Some(o.expected_kind)
                })
            {
                return Err("VIEWS_ACTIVE_TARGET_MISSING_OR_KIND".into());
            }
            check_presence(&o.presence)?;
        }
        if counts.values().any(|n| *n > MAX_PER_VIEW) {
            return Err("VIEWS_PER_VIEW_BUDGET".into());
        }
        for (id, edge) in &self.edges {
            if *id != edge.id || !ids.insert(id.0) || !self.views.contains_key(&edge.view) {
                return Err("VIEWS_EDGE_ID".into());
            }
            let a = self
                .occurrences
                .get(&edge.source)
                .ok_or("VIEWS_EDGE_OCCURRENCE_MISSING")?;
            let b = self
                .occurrences
                .get(&edge.target)
                .ok_or("VIEWS_EDGE_OCCURRENCE_MISSING")?;
            if a.view != edge.view
                || b.view != edge.view
                || a.record != edge.key.owner
                || b.record != edge.key.target
                || b.expected_kind != edge.key.target_kind
            {
                return Err("VIEWS_EDGE_ENDPOINT_BINDING".into());
            }
            if edge.presence.active() {
                if !a.presence.active() || !b.presence.active() {
                    return Err("VIEWS_ACTIVE_EDGE_UNRESOLVED".into());
                }
                if let Some((snapshot, index)) = native {
                    if !edge.key.exists_in(snapshot, index)? {
                        return Err("VIEWS_ACTIVE_EDGE_UNRESOLVED".into());
                    }
                }
            }
            check_presence(&edge.presence)?;
        }
        if self.next_id.0 == 0
            || ids.last().is_some_and(|n| *n >= self.next_id.0)
            || self.revision.0 == 0
        {
            return Err("VIEWS_ALLOCATOR".into());
        }
        self.encoded()?;
        Ok(())
    }
}
fn sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn check_presence(p: &Presence) -> Result<(), String> {
    if let Presence::Detached { reason } = p {
        if reason.is_empty() || reason.len() > 1024 {
            return Err("VIEWS_DETACHED_REASON".into());
        }
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    origin: Origin,
    native: Freshness,
    snapshot_hash: String,
    revision: u64,
}
#[derive(Clone, Debug)]
pub enum Layout {
    AddView(String),
    RenameView(ViewId, String),
    CloseView(ViewId),
    Camera(ViewId, Camera),
    Reuse {
        view: ViewId,
        record: RecordId,
        geometry: Geometry,
    },
    Move(OccurrenceId, Geometry),
    Visible(OccurrenceId, bool),
    Pin(OccurrenceId, bool),
    Remove(OccurrenceId),
    Route {
        view: ViewId,
        key: SemanticKey,
        source: OccurrenceId,
        target: OccurrenceId,
    },
}
pub enum NativeAction {
    Command(Command),
    ModelUndo,
    ModelRedo,
    TextUndo(RawKey),
    TextRedo(RawKey),
}
pub enum NativeOutcome {
    Edit(EditOutcome),
    History(bool),
}
struct Entry {
    document: Document,
    bytes: usize,
}
#[derive(Clone)]
struct HistoryProof {
    instance: String,
    before: String,
    after: String,
}
/// Bounded immutable layout frozen alongside a native request. It carries no
/// I/O authority; a sealed accepted native checkpoint is still required.
pub struct Capture {
    origin: Origin,
    document: Document,
    native: Freshness,
    snapshot_hash: String,
}
impl Capture {
    pub(crate) fn origin(&self) -> Origin {
        self.origin
    }
    pub(crate) fn document(
        self,
        editor: &Editor,
        checkpoint: Option<&NativeCheckpoint>,
    ) -> Result<Document, String> {
        if self.native != editor.freshness() || self.snapshot_hash != editor.snapshot.digest() {
            return Err("VIEWS_CAPTURE_NATIVE_CHANGED".into());
        }
        let mut document = self.document;
        if let Some(c) = checkpoint {
            if c.document_id() != &document.document_id
                || c.snapshot_sha256() != self.snapshot_hash
                || c.instance_id().0 != self.native.instance_id
            {
                return Err("VIEWS_CAPTURE_CHECKPOINT_MISMATCH".into());
            }
            document.profile = PROFILE.into();
            document.binding = Some(NativeBinding::from_checkpoint(c));
        } else {
            document.profile = RECOVERY_PROFILE.into();
            document.binding = None;
        }
        document.validate(&editor.snapshot)?;
        Ok(document)
    }
}
/// Presentation transaction owner. Session and native I/O ownership are not cloned.
pub struct Views {
    origin: Origin,
    document: Document,
    past: VecDeque<Entry>,
    future: VecDeque<Entry>,
    proofs: BTreeMap<u64, HistoryProof>,
    pristine_default: Option<Stamp>,
}
impl Views {
    pub fn new(editor: &Editor) -> Result<Self, String> {
        editor.snapshot.validate()?;
        let mut document = Document {
            profile: PROFILE.into(),
            sidecar_id: format!("views:{}", editor.freshness().instance_id),
            document_id: editor.snapshot.document_id.clone(),
            binding: None,
            revision: Serial(1),
            next_id: Serial(3),
            primary: ViewId(1),
            views: [
                (
                    ViewId(1),
                    View {
                        id: ViewId(1),
                        title: "Primary canvas".into(),
                        camera: Camera::default(),
                    },
                ),
                (
                    ViewId(2),
                    View {
                        id: ViewId(2),
                        title: "Review view".into(),
                        camera: Camera::default(),
                    },
                ),
            ]
            .into(),
            occurrences: BTreeMap::new(),
            edges: BTreeMap::new(),
        };
        for (i, record) in editor.snapshot.records.values().enumerate() {
            if i >= MAX_PER_VIEW {
                break;
            }
            let id = OccurrenceId(document.allocate()?);
            document.occurrences.insert(
                id,
                Occurrence {
                    id,
                    view: document.primary,
                    record: record.id,
                    expected_kind: record.kind,
                    geometry: Geometry::at(
                        25. + (i % 3) as f64 * 294.,
                        24. + (i / 3) as f64 * 155.,
                    ),
                    visible: true,
                    pinned: false,
                    presence: Presence::Active,
                },
            );
        }
        if let Some(record) = editor.snapshot.records.values().next() {
            let id = OccurrenceId(document.allocate()?);
            document.occurrences.insert(
                id,
                Occurrence {
                    id,
                    view: ViewId(2),
                    record: record.id,
                    expected_kind: record.kind,
                    geometry: Geometry::at(25., 24.),
                    visible: true,
                    pinned: false,
                    presence: Presence::Active,
                },
            );
        }
        document.validate(&editor.snapshot)?;
        let origin = new_origin()?;
        Ok(Self {
            origin,
            pristine_default: Some(Stamp {
                origin,
                native: editor.freshness(),
                snapshot_hash: editor.snapshot.digest(),
                revision: document.revision(),
            }),
            document,
            past: VecDeque::new(),
            future: VecDeque::new(),
            proofs: BTreeMap::new(),
        })
    }
    pub fn capture(&self, editor: &Editor) -> Capture {
        Capture {
            origin: self.origin,
            document: self.document.clone(),
            native: editor.freshness(),
            snapshot_hash: editor.snapshot.digest(),
        }
    }
    /// Runtime-only capability created solely by new(), never by wire revision.
    /// Monotonic native/view stamps prevent Undo from recreating this capability.
    pub(crate) fn is_pristine_default(&self, editor: &Editor) -> bool {
        self.pristine_default
            .as_ref()
            .is_some_and(|initial| initial == &self.stamp(editor))
    }
    pub(crate) fn origin(&self) -> Origin {
        self.origin
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn stamp(&self, editor: &Editor) -> Stamp {
        Stamp {
            origin: self.origin,
            native: editor.freshness(),
            snapshot_hash: editor.snapshot.digest(),
            revision: self.document.revision(),
        }
    }
    fn check(&self, editor: &Editor, stamp: &Stamp) -> Result<(), String> {
        if &self.stamp(editor) != stamp {
            return Err("VIEWS_STALE_COMMAND".into());
        }
        Ok(())
    }
    pub fn history_usage(&self) -> (usize, usize) {
        (
            self.past.len() + self.future.len(),
            self.past.iter().chain(&self.future).map(|e| e.bytes).sum(),
        )
    }
    fn retain(&mut self, old: Document) -> Result<(), String> {
        let bytes = old.encoded()?.len();
        if bytes > HISTORY_BYTES {
            return Err("VIEWS_HISTORY_PAYLOAD_BUDGET".into());
        }
        self.future.clear();
        self.past.push_back(Entry {
            document: old,
            bytes,
        });
        while self.history_usage().0 > HISTORY_ENTRIES || self.history_usage().1 > HISTORY_BYTES {
            self.past.pop_front();
        }
        self.prune_proofs();
        Ok(())
    }
    fn prune_proofs(&mut self) {
        let mut reachable = BTreeSet::new();
        for doc in std::iter::once(&self.document)
            .chain(self.past.iter().chain(&self.future).map(|e| &e.document))
        {
            reachable.extend(doc.occurrences.keys().map(|id| id.0));
            reachable.extend(doc.edges.keys().map(|id| id.0));
        }
        self.proofs.retain(|id, _| reachable.contains(id));
    }
    pub fn apply(&mut self, editor: &Editor, stamp: &Stamp, action: Layout) -> Result<(), String> {
        self.check(editor, stamp)?;
        // Validate bounds before cloning any presentation payload.
        self.document.validate(&editor.snapshot)?;
        let mut next = self.document.clone();
        match action {
            Layout::AddView(title) => {
                if next.views.len() >= MAX_VIEWS || title.is_empty() || title.len() > 1024 {
                    return Err("VIEWS_VIEW_BUDGET_OR_TITLE".into());
                }
                let id = ViewId(next.allocate()?);
                next.views.insert(
                    id,
                    View {
                        id,
                        title,
                        camera: Camera::default(),
                    },
                );
            }
            Layout::RenameView(id, title) => {
                next.views.get_mut(&id).ok_or("VIEWS_NO_VIEW")?.title = title;
            }
            Layout::CloseView(id) => {
                if id == next.primary {
                    return Err("VIEWS_PRIMARY_CANNOT_CLOSE".into());
                }
                next.views.remove(&id).ok_or("VIEWS_NO_VIEW")?;
                next.occurrences.retain(|_, o| o.view != id);
                next.edges.retain(|_, e| e.view != id);
            }
            Layout::Camera(id, camera) => {
                next.views.get_mut(&id).ok_or("VIEWS_NO_VIEW")?.camera = camera;
            }
            Layout::Reuse {
                view,
                record,
                geometry,
            } => {
                if next.occurrences.len() >= MAX_OCCURRENCES
                    || next.occurrences.values().filter(|o| o.view == view).count() >= MAX_PER_VIEW
                {
                    return Err("VIEWS_OCCURRENCE_BUDGET".into());
                }
                let record = editor
                    .snapshot
                    .records
                    .get(&record)
                    .ok_or("VIEWS_NO_RECORD")?;
                let id = OccurrenceId(next.allocate()?);
                next.occurrences.insert(
                    id,
                    Occurrence {
                        id,
                        view,
                        record: record.id,
                        expected_kind: record.kind,
                        geometry,
                        visible: true,
                        pinned: false,
                        presence: Presence::Active,
                    },
                );
            }
            Layout::Move(id, geometry) => {
                let o = next.occurrences.get_mut(&id).ok_or("VIEWS_NO_OCCURRENCE")?;
                if o.pinned {
                    return Err("VIEWS_PINNED".into());
                }
                o.geometry = geometry;
            }
            Layout::Visible(id, visible) => {
                next.occurrences
                    .get_mut(&id)
                    .ok_or("VIEWS_NO_OCCURRENCE")?
                    .visible = visible;
            }
            Layout::Pin(id, pinned) => {
                next.occurrences
                    .get_mut(&id)
                    .ok_or("VIEWS_NO_OCCURRENCE")?
                    .pinned = pinned;
            }
            Layout::Remove(id) => {
                next.occurrences.remove(&id).ok_or("VIEWS_NO_OCCURRENCE")?;
                next.edges.retain(|_, e| e.source != id && e.target != id);
            }
            Layout::Route {
                view,
                key,
                source,
                target,
            } => {
                if next.edges.len() >= MAX_EDGES {
                    return Err("VIEWS_EDGE_BUDGET".into());
                }
                if !key.exists(&editor.snapshot)? {
                    return Err("VIEWS_EDGE_NOT_AUTHORED".into());
                }
                let id = VisualEdgeId(next.allocate()?);
                next.edges.insert(
                    id,
                    VisualEdge {
                        id,
                        view,
                        key,
                        source,
                        target,
                        presence: Presence::Active,
                    },
                );
            }
        }
        next.bump()?;
        next.validate(&editor.snapshot)?;
        self.check(editor, stamp)?;
        self.retain(self.document.clone())?;
        self.document = next;
        self.prune_proofs();
        Ok(())
    }
    pub fn undo(&mut self, editor: &Editor, stamp: &Stamp, redo: bool) -> Result<bool, String> {
        self.check(editor, stamp)?;
        self.document.validate(&editor.snapshot)?;
        let queue = if redo { &self.future } else { &self.past };
        let Some(entry) = queue.back() else {
            return Ok(false);
        };
        let mut next = entry.document.clone();
        next.next_id = self.document.next_id;
        next.revision = self.document.revision;
        next.binding = self.document.binding.clone();
        next.bump()?;
        // Refuse the entire operation before a history pop if a restored active
        // occurrence/key no longer resolves. No semantic resurrection occurs.
        next.validate(&editor.snapshot)?;
        let old = self.document.clone();
        let bytes = old.encoded()?.len();
        let total = self.history_usage().1 - entry.bytes + bytes;
        if total > HISTORY_BYTES {
            return Err("VIEWS_HISTORY_PAYLOAD_BUDGET".into());
        }
        self.check(editor, stamp)?;
        if redo {
            self.future.pop_back();
            self.past.push_back(Entry {
                document: old,
                bytes,
            });
        } else {
            self.past.pop_back();
            self.future.push_back(Entry {
                document: old,
                bytes,
            });
        }
        self.document = next;
        self.prune_proofs();
        Ok(true)
    }
    /// Single staging boundary for semantic commands and both native histories.
    pub fn dispatch(
        &mut self,
        editor: &mut Editor,
        stamp: &Stamp,
        action: NativeAction,
    ) -> Result<NativeOutcome, String> {
        self.check(editor, stamp)?;
        self.document.validate(&editor.snapshot)?;
        let mut candidate = editor.clone();
        let undo = matches!(&action, NativeAction::ModelUndo);
        let redo = matches!(&action, NativeAction::ModelRedo);
        let outcome = match action {
            NativeAction::Command(command) => {
                NativeOutcome::Edit(candidate.command(&stamp.native, command)?)
            }
            NativeAction::ModelUndo => NativeOutcome::History(candidate.undo_model(&stamp.native)?),
            NativeAction::ModelRedo => NativeOutcome::History(candidate.redo_model(&stamp.native)?),
            NativeAction::TextUndo(key) => {
                NativeOutcome::History(candidate.undo_text(&stamp.native, &key)?)
            }
            NativeAction::TextRedo(key) => {
                NativeOutcome::History(candidate.redo_text(&stamp.native, &key)?)
            }
        };
        let candidate_index = edge_index(&candidate.snapshot)?;
        let before = records_hash(&editor.snapshot)?;
        let after = records_hash(&candidate.snapshot)?;
        let proof = HistoryProof {
            instance: stamp.native.instance_id.clone(),
            before: before.clone(),
            after: after.clone(),
        };
        let mut next = self.document.clone();
        let mut proofs = self.proofs.clone();
        for o in next.occurrences.values_mut() {
            let resolves =
                candidate.snapshot.records.get(&o.record).map(|r| r.kind) == Some(o.expected_kind);
            if o.presence.active() && !resolves {
                o.presence = Presence::Detached {
                    reason: "Native target removed by model history".into(),
                };
                if undo {
                    proofs.insert(o.id.0, proof.clone());
                }
            } else if !o.presence.active()
                && resolves
                && redo
                && matches_redo(proofs.get(&o.id.0), &proof)
            {
                o.presence = Presence::Active;
                proofs.remove(&o.id.0);
            }
        }
        for edge in next.edges.values_mut() {
            let resolves = next
                .occurrences
                .get(&edge.source)
                .is_some_and(|o| o.presence.active())
                && next
                    .occurrences
                    .get(&edge.target)
                    .is_some_and(|o| o.presence.active())
                && edge.key.exists_in(&candidate.snapshot, &candidate_index)?;
            if edge.presence.active() && !resolves {
                edge.presence = Presence::Detached {
                    reason: "Native relation or endpoint no longer present".into(),
                };
                if undo {
                    proofs.insert(edge.id.0, proof.clone());
                }
            } else if !edge.presence.active()
                && resolves
                && redo
                && matches_redo(proofs.get(&edge.id.0), &proof)
            {
                edge.presence = Presence::Active;
                proofs.remove(&edge.id.0);
            }
        }
        if next != self.document {
            next.bump()?;
        }
        next.validate_with_index(&candidate.snapshot, &candidate_index)?;
        self.check(editor, stamp)?;
        // All fallible work completed; publish both candidates together.
        *editor = candidate;
        self.document = next;
        self.proofs = proofs;
        self.prune_proofs();
        Ok(outcome)
    }
    pub fn freeze(
        &self,
        editor: &Editor,
        checkpoint: &NativeCheckpoint,
    ) -> Result<Document, String> {
        if checkpoint.document_id() != &self.document.document_id
            || checkpoint.snapshot_sha256() != editor.snapshot.digest()
            || checkpoint.instance_id().0 != editor.freshness().instance_id
        {
            return Err("VIEWS_NATIVE_CHECKPOINT_NOT_CURRENT".into());
        }
        let mut frozen = self.document.clone();
        frozen.profile = PROFILE.into();
        frozen.binding = Some(NativeBinding::from_checkpoint(checkpoint));
        frozen.validate(&editor.snapshot)?;
        Ok(frozen)
    }
    pub fn recovery(&self, editor: &Editor) -> Result<Document, String> {
        self.document.validate(&editor.snapshot)?;
        let mut frozen = self.document.clone();
        frozen.profile = RECOVERY_PROFILE.into();
        frozen.binding = None;
        Ok(frozen)
    }
    pub fn decode(
        bytes: &[u8],
        editor: &Editor,
        checkpoint: &NativeCheckpoint,
    ) -> Result<Self, String> {
        let document = decode_document(bytes)?;
        if document.profile != PROFILE
            || !document
                .binding
                .as_ref()
                .is_some_and(|b| b.matches(checkpoint))
            || checkpoint.snapshot_sha256() != editor.snapshot.digest()
            || checkpoint.document_id() != &editor.snapshot.document_id
        {
            return Err("VIEWS_EXACT_NATIVE_BINDING_REQUIRED".into());
        }
        document.validate(&editor.snapshot)?;
        Ok(Self {
            origin: new_origin()?,
            document,
            past: VecDeque::new(),
            future: VecDeque::new(),
            proofs: BTreeMap::new(),
            pristine_default: None,
        })
    }
    /// Explicit recovery import carries no native artifact authority and retains
    /// detached state. A fresh native Save is still required before a bound Save.
    pub fn restore(bytes: &[u8], editor: &Editor) -> Result<Self, String> {
        let document = decode_document(bytes)?;
        if document.profile != RECOVERY_PROFILE || document.binding.is_some() {
            return Err("VIEWS_NOT_UNBOUND_RECOVERY".into());
        }
        document.validate(&editor.snapshot)?;
        Ok(Self {
            origin: new_origin()?,
            document,
            past: VecDeque::new(),
            future: VecDeque::new(),
            proofs: BTreeMap::new(),
            pristine_default: None,
        })
    }
}
fn matches_redo(prior: Option<&HistoryProof>, current: &HistoryProof) -> bool {
    prior.is_some_and(|p| {
        p.instance == current.instance && p.before == current.after && p.after == current.before
    })
}
fn decode_document(bytes: &[u8]) -> Result<Document, String> {
    if bytes.len() > MAX_BYTES {
        return Err("VIEWS_ENCODED_BUDGET".into());
    }
    // Reuse native duplicate-key rejection, then check collection counts before
    // allocating typed maps. This is an encoded/payload budget, not an RSS cap.
    let unique: crate::stage1_io::UniqueValue =
        serde_json::from_slice(bytes).map_err(|e| format!("VIEWS_JSON: {e}"))?;
    let root = unique.0.as_object().ok_or("VIEWS_OBJECT_REQUIRED")?;
    for (key, max) in [
        ("views", MAX_VIEWS),
        ("occurrences", MAX_OCCURRENCES),
        ("edges", MAX_EDGES),
    ] {
        if root
            .get(key)
            .and_then(serde_json::Value::as_object)
            .ok_or("VIEWS_COLLECTION_REQUIRED")?
            .len()
            > max
        {
            return Err("VIEWS_COUNT_BUDGET".into());
        }
    }
    serde_json::from_value(unique.0).map_err(|e| format!("VIEWS_SCHEMA: {e}"))
}

/// Structural source inspection only: returns a declared DTO, never a Views
/// runtime, native checkpoint, replacement authority or semantic admission.
pub(crate) fn inspect_document(bytes: &[u8]) -> Result<Document, String> {
    let document = decode_document(bytes)?;
    document.validate_contents(None)?;
    if (document.profile == PROFILE && document.binding.is_none())
        || (document.profile == RECOVERY_PROFILE && document.binding.is_some())
    {
        return Err("VIEWS_SOURCE_PROFILE_BINDING".into());
    }
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn editor() -> Editor {
        Editor::new(
            Snapshot::new(DocumentId("views-test".into())),
            "runtime-a".into(),
        )
        .unwrap()
    }
    fn layout(v: &mut Views, e: &Editor, a: Layout) {
        let stamp = v.stamp(e);
        v.apply(e, &stamp, a).unwrap();
    }
    fn native(v: &mut Views, e: &mut Editor, a: NativeAction) -> NativeOutcome {
        let stamp = v.stamp(e);
        v.dispatch(e, &stamp, a).unwrap()
    }
    fn add(
        v: &mut Views,
        e: &mut Editor,
        kind: RecordKind,
        owner: RecordId,
        name: &str,
    ) -> RecordId {
        match native(
            v,
            e,
            NativeAction::Command(Command::Add {
                kind,
                owner,
                name: name.into(),
            }),
        ) {
            NativeOutcome::Edit(outcome) => outcome.created.expect("accepted typed creation"),
            _ => panic!("wrong outcome"),
        }
    }
    fn reuse(v: &mut Views, e: &Editor, record: RecordId) -> OccurrenceId {
        let view = v.document.primary;
        layout(
            v,
            e,
            Layout::Reuse {
                view,
                record,
                geometry: Geometry::at(10., 20.),
            },
        );
        *v.document.occurrences.keys().last().unwrap()
    }
    #[test]
    fn checked_camera_uses_the_existing_finite_wire_limits() {
        for (x, y, z) in [
            (0., 0., Camera::MIN_ZOOM),
            (LIMIT, -LIMIT, Camera::MAX_ZOOM),
        ] {
            assert!(Camera::checked(x, y, z).is_ok());
        }
        for (x, y, z) in [
            (f64::NAN, 0., 1.),
            (0., f64::INFINITY, 1.),
            (LIMIT + 1., 0., 1.),
            (0., 0., 0.),
            (0., 0., Camera::MAX_ZOOM + 0.01),
            (0., 0., f64::NAN),
        ] {
            assert!(Camera::checked(x, y, z).is_err());
        }
    }
    #[test]
    fn invalid_camera_refuses_layout_and_source_restore_without_mutation() {
        let e = editor();
        let mut views = Views::new(&e).unwrap();
        let before = (
            views.document().digest().unwrap(),
            views.history_usage(),
            views.document().next_id(),
        );
        let bad = Camera {
            x: 0.,
            y: 0.,
            zoom: 21.,
        };
        let stamp = views.stamp(&e);
        assert!(views
            .apply(
                &e,
                &stamp,
                Layout::Camera(views.document().primary, bad.clone())
            )
            .is_err());
        assert_eq!(
            (
                views.document().digest().unwrap(),
                views.history_usage(),
                views.document().next_id()
            ),
            before
        );
        let mut d = views.recovery(&e).unwrap();
        d.views.get_mut(&d.primary).unwrap().camera = bad;
        let bytes = d.encoded().unwrap();
        assert!(Views::restore(&bytes, &e).is_err());
        assert!(inspect_document(&bytes).is_err());
    }
    #[test]
    fn same_element_two_views_move_hide_and_non_reused_ids() {
        let e = editor();
        let mut v = Views::new(&e).unwrap();
        let original = e.snapshot.digest();
        let calc = e.calculation_stamp();
        let ids: Vec<_> = v
            .document
            .occurrences
            .values()
            .filter(|o| o.record == RecordId(1))
            .map(|o| o.id)
            .collect();
        assert_eq!(ids.len(), 2);
        layout(&mut v, &e, Layout::Move(ids[1], Geometry::at(75., 88.)));
        assert_ne!(
            v.document.occurrences[&ids[0]].geometry,
            v.document.occurrences[&ids[1]].geometry
        );
        layout(&mut v, &e, Layout::Visible(ids[1], false));
        let stamp = v.stamp(&e);
        assert!(v.undo(&e, &stamp, false).unwrap());
        assert!(v.document.occurrences[&ids[1]].visible);
        assert_eq!(e.snapshot.digest(), original);
        assert_eq!(e.calculation_stamp(), calc);
        let next = v.document.next_id();
        let id = reuse(&mut v, &e, RecordId(1));
        assert_eq!(id.get(), next);
        let stamp = v.stamp(&e);
        v.undo(&e, &stamp, false).unwrap();
        let next = v.document.next_id();
        let second = reuse(&mut v, &e, RecordId(1));
        assert_eq!(second.get(), next);
        assert_ne!(second, id);
    }
    #[test]
    fn stale_command_and_nonfinite_refuse_without_history_or_allocator_change() {
        let e = editor();
        let mut v = Views::new(&e).unwrap();
        let stamp = v.stamp(&e);
        layout(&mut v, &e, Layout::AddView("Third".into()));
        let digest = v.document.digest().unwrap();
        let usage = v.history_usage();
        assert!(v
            .apply(&e, &stamp, Layout::AddView("Stale".into()))
            .is_err());
        let stamp = v.stamp(&e);
        let id = *v.document.occurrences.keys().next().unwrap();
        assert!(v
            .apply(&e, &stamp, Layout::Move(id, Geometry::at(f64::NAN, 0.)))
            .is_err());
        assert_eq!(v.document.digest().unwrap(), digest);
        assert_eq!(v.history_usage(), usage);
        assert!(v
            .apply(&e, &stamp, Layout::CloseView(v.document.primary))
            .is_err());
    }
    #[test]
    fn native_undo_detaches_and_only_observed_redo_reactivates() {
        let mut e = editor();
        let mut v = Views::new(&e).unwrap();
        let mission = add(&mut v, &mut e, RecordKind::Mission, RecordId(1), "Mission");
        let id = reuse(&mut v, &e, mission);
        let geometry = v.document.occurrences[&id].geometry.clone();
        native(&mut v, &mut e, NativeAction::ModelUndo);
        assert!(!v.document.occurrences[&id].presence.active());
        native(&mut v, &mut e, NativeAction::ModelRedo);
        assert!(v.document.occurrences[&id].presence.active());
        assert_eq!(v.document.occurrences[&id].geometry, geometry);
    }
    #[test]
    fn layout_undo_missing_target_refuses_atomically_and_does_not_discard_native_redo() {
        let mut e = editor();
        let mut v = Views::new(&e).unwrap();
        let mission = add(&mut v, &mut e, RecordKind::Mission, RecordId(1), "Mission");
        let id = reuse(&mut v, &e, mission);
        layout(&mut v, &e, Layout::Remove(id));
        native(&mut v, &mut e, NativeAction::ModelUndo);
        let digest = v.document.digest().unwrap();
        let usage = v.history_usage();
        let stamp = v.stamp(&e);
        assert!(v.undo(&e, &stamp, false).is_err());
        assert_eq!(v.document.digest().unwrap(), digest);
        assert_eq!(v.history_usage(), usage);
        native(&mut v, &mut e, NativeAction::ModelRedo);
        let stamp = v.stamp(&e);
        assert!(v.undo(&e, &stamp, false).unwrap());
        assert_eq!(v.document.occurrences[&id].record, mission);
    }
    #[test]
    fn reopened_detached_has_no_runtime_redo_authority() {
        let mut e = editor();
        let mut v = Views::new(&e).unwrap();
        let m = add(&mut v, &mut e, RecordKind::Mission, RecordId(1), "Mission");
        let id = reuse(&mut v, &e, m);
        native(&mut v, &mut e, NativeAction::ModelUndo);
        let bytes = v.recovery(&e).unwrap().encoded().unwrap();
        let mut reopened = Views::restore(&bytes, &e).unwrap();
        assert!(reopened.proofs.is_empty());
        native(&mut reopened, &mut e, NativeAction::ModelRedo);
        assert!(!reopened.document.occurrences[&id].presence.active());
    }
    #[test]
    fn duplicate_occurrence_edge_restores_exact_pair_and_three_keys_are_checked() {
        let mut e = editor();
        let mut v = Views::new(&e).unwrap();
        let mission = add(&mut v, &mut e, RecordKind::Mission, RecordId(1), "Mission");
        let capability = add(
            &mut v,
            &mut e,
            RecordKind::Capability,
            RecordId(1),
            "Capability",
        );
        native(
            &mut v,
            &mut e,
            NativeAction::Command(Command::ToggleMission {
                capability,
                mission,
            }),
        );
        let objective = add(
            &mut v,
            &mut e,
            RecordKind::Objective,
            RecordId(1),
            "Objective",
        );
        native(
            &mut v,
            &mut e,
            NativeAction::Command(Command::SetSubject {
                objective,
                subject: Some(crate::stage1::SubjectRef::Mission(mission)),
            }),
        );
        let measure = add(&mut v, &mut e, RecordKind::Measure, objective, "Measure");
        for (owner, role, target) in [
            (
                capability,
                SemanticRole::CapabilityContributesToMission,
                mission,
            ),
            (objective, SemanticRole::ObjectiveSubject, mission),
            (measure, SemanticRole::MeasureForObjective, objective),
        ] {
            SemanticKey::checked(&e.snapshot, owner, role, target).unwrap();
        }
        assert!(SemanticKey::checked(
            &e.snapshot,
            mission,
            SemanticRole::CapabilityContributesToMission,
            capability
        )
        .is_err());
        let a = reuse(&mut v, &e, capability);
        let b = reuse(&mut v, &e, mission);
        let duplicate = reuse(&mut v, &e, mission);
        let key = SemanticKey::checked(
            &e.snapshot,
            capability,
            SemanticRole::CapabilityContributesToMission,
            mission,
        )
        .unwrap();
        let primary = v.document.primary;
        layout(
            &mut v,
            &e,
            Layout::Route {
                view: primary,
                key,
                source: a,
                target: b,
            },
        );
        let edge = v.document.edges.values().next().unwrap().clone();
        layout(&mut v, &e, Layout::Remove(b));
        assert!(v.document.edges.is_empty());
        assert!(v.document.occurrences.contains_key(&duplicate));
        let stamp = v.stamp(&e);
        v.undo(&e, &stamp, false).unwrap();
        assert_eq!(v.document.edges[&edge.id], edge);
    }
    #[test]
    fn model_text_layout_histories_interleave_without_semantic_delete() {
        let mut e = editor();
        let mut v = Views::new(&e).unwrap();
        let key = RawKey {
            record: RecordId(1),
            field: "name".into(),
            item: None,
        };
        native(
            &mut v,
            &mut e,
            NativeAction::Command(Command::EditText {
                key: key.clone(),
                text: "Alpha".into(),
            }),
        );
        let native_digest = e.snapshot.digest();
        let id = *v.document.occurrences.keys().next().unwrap();
        layout(&mut v, &e, Layout::Visible(id, false));
        assert_eq!(e.snapshot.digest(), native_digest);
        native(&mut v, &mut e, NativeAction::TextUndo(key.clone()));
        native(&mut v, &mut e, NativeAction::TextRedo(key));
        native(&mut v, &mut e, NativeAction::ModelUndo);
        let stamp = v.stamp(&e);
        v.undo(&e, &stamp, false).unwrap();
        native(&mut v, &mut e, NativeAction::ModelRedo);
        assert!(e.snapshot.records.contains_key(&RecordId(1)));
        assert!(v.document.occurrences[&id].visible);
        // The exposed action enum contains no semantic deletion operation.
    }
    #[test]
    fn canonical_u64_wire_future_unknown_duplicate_and_corrupt_refused() {
        let e = editor();
        let mut v = Views::new(&e).unwrap();
        v.document.next_id = Serial(9_007_199_254_740_993);
        let id = reuse(&mut v, &e, RecordId(1));
        assert_eq!(id.get(), 9_007_199_254_740_993);
        let bytes = v.recovery(&e).unwrap().encoded().unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("\"9007199254740993\""));
        let decoded = Views::restore(&bytes, &e).unwrap();
        assert_eq!(decoded.document.next_id(), 9_007_199_254_740_994);
        for bad in [
            text.replace(RECOVERY_PROFILE, "future/99"),
            text.replacen("{", "{\"unknown\":true,", 1),
            text.replacen("{", "{\"profile\":\"duplicate\",", 1),
            "{".into(),
            text.replace("\"9007199254740994\"", "9007199254740994"),
            text.replace("\"9007199254740994\"", "\"09007199254740994\""),
        ] {
            assert!(Views::restore(bad.as_bytes(), &e).is_err());
        }
    }
    #[test]
    fn history_budget_evicts_and_proof_churn_prunes_orphans() {
        let mut e = editor();
        let mut v = Views::new(&e).unwrap();
        for n in 0..(HISTORY_ENTRIES + 2) {
            let mission = add(
                &mut v,
                &mut e,
                RecordKind::Mission,
                RecordId(1),
                &format!("M{n}"),
            );
            let id = reuse(&mut v, &e, mission);
            native(&mut v, &mut e, NativeAction::ModelUndo);
            layout(&mut v, &e, Layout::Remove(id));
        }
        for n in 0..(HISTORY_ENTRIES + 2) {
            layout(
                &mut v,
                &e,
                Layout::RenameView(ViewId(2), format!("Review {n}")),
            );
        }
        assert_eq!(v.history_usage().0, HISTORY_ENTRIES);
        assert!(v.history_usage().1 <= HISTORY_BYTES);
        assert!(v.proofs.is_empty());
        let mission = add(&mut v, &mut e, RecordKind::Mission, RecordId(1), "New");
        let id = reuse(&mut v, &e, mission);
        assert!(!v.proofs.contains_key(&id.0));
    }
    #[test]
    fn budgets_refuse_without_mutation_and_detached_counts() {
        let e = editor();
        let mut v = Views::new(&e).unwrap();
        while v.document.views.len() < MAX_VIEWS {
            let title = format!("V{}", v.document.views.len());
            layout(&mut v, &e, Layout::AddView(title));
        }
        let stamp = v.stamp(&e);
        let digest = v.document.digest().unwrap();
        let usage = v.history_usage();
        assert!(v
            .apply(&e, &stamp, Layout::AddView("Overflow".into()))
            .is_err());
        assert!(v
            .apply(&e, &stamp, Layout::RenameView(ViewId(2), "x".repeat(1025)))
            .is_err());
        assert_eq!(v.document.digest().unwrap(), digest);
        assert_eq!(v.history_usage(), usage);
        let mut detached = v.document.clone();
        for o in detached.occurrences.values_mut() {
            o.presence = Presence::Detached {
                reason: "Retained".into(),
            };
        }
        assert_eq!(detached.occurrences.len(), v.document.occurrences.len());
        detached.validate(&e.snapshot).unwrap();
    }
    #[test]
    fn detached_occurrences_share_per_view_budget_and_oversized_input_refuses() {
        let e = editor();
        let mut v = Views::new(&e).unwrap();
        let template = v.document.occurrences.values().next().unwrap().clone();
        v.document.occurrences.clear();
        for _ in 0..MAX_PER_VIEW {
            let mut o = template.clone();
            o.id = OccurrenceId(v.document.allocate().unwrap());
            o.presence = Presence::Detached {
                reason: "Retained unavailable target".into(),
            };
            v.document.occurrences.insert(o.id, o);
        }
        v.document.validate(&e.snapshot).unwrap();
        let digest = v.document.digest().unwrap();
        let usage = v.history_usage();
        let stamp = v.stamp(&e);
        assert!(v
            .apply(
                &e,
                &stamp,
                Layout::Reuse {
                    view: template.view,
                    record: RecordId(1),
                    geometry: Geometry::at(0., 0.)
                }
            )
            .is_err());
        assert_eq!(v.document.digest().unwrap(), digest);
        assert_eq!(v.history_usage(), usage);
        assert!(Views::restore(&vec![b' '; MAX_BYTES + 1], &e).is_err());
    }
    #[test]
    fn native_ids_beyond_js_precision_round_trip_without_expanding_core_range() {
        for native in [9_007_199_254_740_993_u64, u64::MAX - 1] {
            let fixture = serde_json::json!({"records":[{"id":native,"type":"System","name":"Large identity","scope":native,"purpose":""}]});
            let snapshot =
                Snapshot::from_fixture(DocumentId("large-native".into()), &fixture).unwrap();
            let e = Editor::new(snapshot, "large-runtime".into()).unwrap();
            let v = Views::new(&e).unwrap();
            let bytes = v.recovery(&e).unwrap().encoded().unwrap();
            let reopened = Views::restore(&bytes, &e).unwrap();
            assert!(reopened
                .document
                .occurrences
                .values()
                .all(|o| o.record.0 == native));
            let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let expected = native.to_string();
            assert!(wire["occurrences"]
                .as_object()
                .unwrap()
                .values()
                .all(|o| o["record"].as_str() == Some(expected.as_str())));
        }
        let maximum = serde_json::json!({"records":[{"id":u64::MAX,"type":"System","name":"Exhausted identity","scope":u64::MAX,"purpose":""}]});
        assert!(Snapshot::from_fixture(DocumentId("max-native".into()), &maximum).is_err());
    }
    #[test]
    fn same_wire_sidecar_revision_geometry_or_record_cannot_reuse_runtime_stamp() {
        let mut e = editor();
        let mut setup = Views::new(&e).unwrap();
        let one = add(&mut setup, &mut e, RecordKind::Mission, RecordId(1), "One");
        let two = add(&mut setup, &mut e, RecordKind::Mission, RecordId(1), "Two");
        let a = Views::new(&e).unwrap();
        let stamp = a.stamp(&e);
        let occurrence = a
            .document
            .occurrences
            .values()
            .find(|o| o.record == one)
            .unwrap()
            .id;
        for change_target in [false, true] {
            let mut wire = serde_json::to_value(a.recovery(&e).unwrap()).unwrap();
            if change_target {
                wire["occurrences"][occurrence.get().to_string()]["record"] =
                    serde_json::json!(two.0.to_string());
            } else {
                wire["occurrences"][occurrence.get().to_string()]["geometry"]["x"] =
                    serde_json::json!(987.);
            }
            let mut b = Views::restore(&serde_json::to_vec(&wire).unwrap(), &e).unwrap();
            assert_eq!(a.document.sidecar_id, b.document.sidecar_id);
            assert_eq!(a.document.revision(), b.document.revision());
            assert_ne!(a.origin, b.origin);
            let digest = b.document.digest().unwrap();
            let history = b.history_usage();
            let next = b.document.next_id();
            let native = e.snapshot.digest();
            assert!(b
                .apply(&e, &stamp, Layout::Move(occurrence, Geometry::at(5., 6.)))
                .is_err());
            assert!(b.apply(&e, &stamp, Layout::Remove(occurrence)).is_err());
            assert!(b.undo(&e, &stamp, false).is_err());
            assert!(b.dispatch(&mut e, &stamp, NativeAction::ModelUndo).is_err());
            assert_eq!(b.document.digest().unwrap(), digest);
            assert_eq!(b.history_usage(), history);
            assert_eq!(b.document.next_id(), next);
            assert_eq!(e.snapshot.digest(), native);
        }
    }
}
