//! Explicit mapping review and sealed, point-in-time presentation-copy Apply.
use crate::{
    native_views::{
        self as v, Geometry, OccurrenceId, Origin, SemanticKey, Stamp, ViewId, Views, VisualEdgeId,
    },
    native_views_inspect::Inspection,
    stage1::{Editor, RecordId},
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    Unselected,
    Target(RecordId),
    Skip(String),
}

pub struct Plan {
    plan_origin: Origin,
    generation: u64,
    origin: Origin,
    source_sha256: String,
    view: ViewId,
    target: Stamp,
    choices: BTreeMap<OccurrenceId, Choice>,
}
#[derive(Debug)]
pub struct ProposedOccurrence {
    pub source: OccurrenceId,
    pub target: Option<RecordId>,
    pub reason: Option<String>,
    pub geometry: Geometry,
    pub visible: bool,
    pub pinned: bool,
}
#[derive(Debug)]
pub struct ProposedEdge {
    pub source: VisualEdgeId,
    pub source_occurrence: OccurrenceId,
    pub target_occurrence: OccurrenceId,
    pub key: Option<SemanticKey>,
    pub reason: Option<String>,
}
#[derive(Debug)]
pub struct Preview {
    pub occurrences: Vec<ProposedOccurrence>,
    pub edges: Vec<ProposedEdge>,
}
/// Cannot be forged, deserialized or edited. Only a checked Plan can mint it.
pub struct ReviewedCopy {
    plan_origin: Origin,
    generation: u64,
    title: String,
    preview: Preview,
}
impl ReviewedCopy {
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn preview(&self) -> &Preview {
        &self.preview
    }
}
// Existing inspection retains at most8MiB raw plus8MiB encoded payload.
// Rehearsal adds no source/target/history clone; charge all rows/edges and
// reasons against this separate4MiB logical envelope, not an RSS claim.
const MAX_PLAN_BYTES: usize = 4 * 1024 * 1024;
impl Plan {
    fn charged_bytes(&self, source: &Inspection) -> usize {
        self.choices
            .values()
            .map(|c| {
                256 + match c {
                    Choice::Skip(s) => 2 * s.len(),
                    _ => 0,
                }
            })
            .sum::<usize>()
            + source
                .declared()
                .edges
                .values()
                .filter(|e| e.view == self.view)
                .count()
                * 384
    }
    fn budget(&self, source: &Inspection) -> Result<(), String> {
        if self.choices.len() > v::MAX_PER_VIEW || self.charged_bytes(source) > MAX_PLAN_BYTES {
            return Err("VIEWS_PLAN_BUDGET".into());
        }
        Ok(())
    }

    pub fn new(
        source: &Inspection,
        view: ViewId,
        editor: &Editor,
        target: &Views,
    ) -> Result<Self, String> {
        source.verify_unchanged()?;
        if !source.declared().views.contains_key(&view) {
            return Err("VIEWS_PLAN_SOURCE_VIEW".into());
        }
        let plan = Self {
            plan_origin: v::new_origin()?,
            generation: 0,
            origin: source.origin(),
            source_sha256: source.source_sha256().into(),
            view,
            target: target.stamp(editor),
            choices: source
                .declared()
                .occurrences
                .values()
                .filter(|o| o.view == view)
                .map(|o| (o.id, Choice::Unselected))
                .collect(),
        };
        plan.budget(source)?;
        Ok(plan)
    }
    pub fn source_view(&self) -> ViewId {
        self.view
    }
    pub fn choices(&self) -> &BTreeMap<OccurrenceId, Choice> {
        &self.choices
    }
    pub fn check(
        &self,
        source: &Inspection,
        editor: &Editor,
        target: &Views,
    ) -> Result<(), String> {
        if self.origin != source.origin() || self.source_sha256 != source.source_sha256() {
            return Err("VIEWS_PLAN_SOURCE_ORIGIN_CHANGED: start a new rehearsal".into());
        }
        if self.target != target.stamp(editor) {
            return Err("VIEWS_PLAN_TARGET_STALE: start a new rehearsal".into());
        }
        Ok(())
    }
    pub fn choose(
        &mut self,
        source: &Inspection,
        editor: &Editor,
        target: &Views,
        id: OccurrenceId,
        choice: Choice,
    ) -> Result<(), String> {
        self.check(source, editor, target)?;
        if !self.choices.contains_key(&id) {
            return Err("VIEWS_PLAN_OCCURRENCE_OUTSIDE_VIEW".into());
        }
        match &choice {
            Choice::Target(record) => {
                if editor.snapshot.records.get(record).map(|r| r.kind)
                    != Some(source.declared().occurrences[&id].expected_kind)
                {
                    return Err("VIEWS_PLAN_TARGET_MISSING_OR_KIND".into());
                }
            }
            Choice::Skip(reason) if reason.trim().is_empty() || reason.len() > 1024 => {
                return Err("VIEWS_PLAN_SKIP_REASON".into())
            }
            _ => {}
        }
        // Compute the proposed charge before changing choices (refusal atomic).
        let old = self.choices.get(&id).unwrap();
        let size = |c: &Choice| match c {
            Choice::Skip(s) => 2 * s.len(),
            _ => 0,
        };
        if self.charged_bytes(source) - size(old) + size(&choice) > MAX_PLAN_BYTES {
            return Err("VIEWS_PLAN_BUDGET".into());
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or("VIEWS_PLAN_GENERATION_EXHAUSTED")?;
        self.choices.insert(id, choice);
        self.generation = generation;
        Ok(())
    }
    pub fn review(
        &self,
        source: &Inspection,
        editor: &Editor,
        target: &Views,
        title: &str,
    ) -> Result<ReviewedCopy, String> {
        if title.trim().is_empty() || title.len() > 1024 {
            return Err("VIEWS_COPY_TITLE".into());
        }
        Ok(ReviewedCopy {
            plan_origin: self.plan_origin,
            generation: self.generation,
            title: title.into(),
            preview: self.preview(source, editor, target)?,
        })
    }
    pub fn check_review(
        &self,
        source: &Inspection,
        editor: &Editor,
        target: &Views,
        reviewed: &ReviewedCopy,
    ) -> Result<(), String> {
        self.check(source, editor, target)?;
        if reviewed.plan_origin != self.plan_origin || reviewed.generation != self.generation {
            return Err("VIEWS_COPY_REVIEW_STALE: check the changed mapping again".into());
        }
        Ok(())
    }
    pub fn apply_copy(
        &self,
        source: &Inspection,
        editor: &Editor,
        target: &mut Views,
        reviewed: &ReviewedCopy,
    ) -> Result<ViewId, String> {
        self.check_review(source, editor, target, reviewed)?;
        source.verify_unchanged()?;
        target.commit_copy(
            editor,
            &self.target,
            source,
            self.view,
            reviewed.title(),
            reviewed.preview(),
            &self.choices,
        )
    }
    pub fn preview(
        &self,
        source: &Inspection,
        editor: &Editor,
        target: &Views,
    ) -> Result<Preview, String> {
        self.check(source, editor, target)?;
        source.verify_unchanged()?;
        self.budget(source)?;
        // Derive the authoritative graph once for all proposed source edges.
        let index = v::edge_index(&editor.snapshot)?;
        let mut occurrences = Vec::new();
        for (id, choice) in &self.choices {
            let o = &source.declared().occurrences[id];
            let (record, reason) = match choice {
                Choice::Unselected => (
                    None,
                    Some("Unselected: no target identity was chosen".into()),
                ),
                Choice::Skip(reason) => (None, Some(format!("Skipped: {reason}"))),
                Choice::Target(record) => {
                    if editor.snapshot.records.get(record).map(|r| r.kind) != Some(o.expected_kind)
                    {
                        return Err("VIEWS_PLAN_TARGET_MISSING_OR_KIND".into());
                    }
                    (Some(*record), None)
                }
            };
            occurrences.push(ProposedOccurrence {
                source: *id,
                target: record,
                reason,
                geometry: o.geometry.clone(),
                visible: o.visible,
                pinned: o.pinned,
            });
        }
        let mut edges = Vec::new();
        for e in source
            .declared()
            .edges
            .values()
            .filter(|e| e.view == self.view)
        {
            let (key, reason) = match (self.choices.get(&e.source), self.choices.get(&e.target)) {
                (Some(Choice::Target(a)), Some(Choice::Target(b))) => {
                    let key = SemanticKey {
                        owner: *a,
                        role: e.key.role,
                        target: *b,
                        target_kind: editor.snapshot.records[b].kind,
                    };
                    if key.exists_in(&editor.snapshot, &index)? {
                        (Some(key), None)
                    } else {
                        (
                            None,
                            Some(
                                "Target relationship is not authored under this stable key".into(),
                            ),
                        )
                    }
                }
                _ => (
                    None,
                    Some("Both exact source occurrence endpoints must be explicitly mapped".into()),
                ),
            };
            edges.push(ProposedEdge {
                source: e.id,
                source_occurrence: e.source,
                target_occurrence: e.target,
                key,
                reason,
            });
        }
        Ok(Preview { occurrences, edges })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        native_views::{Layout, NativeAction, NativeOutcome, SemanticRole},
        stage1::{Command, DocumentId, RecordKind, Snapshot, SubjectRef},
    };
    fn fixture() -> (tempfile::TempDir, Inspection, Editor, Views) {
        let area = tempfile::Builder::new()
            .prefix("mapping-")
            .tempdir_in(std::env::var_os("TMPDIR").expect("workspace TMPDIR"))
            .unwrap();
        assert!(area.path().starts_with("/workspace") || cfg!(windows));
        let editor = Editor::new(
            Snapshot::new(DocumentId("target".into())),
            "target-runtime".into(),
        )
        .unwrap();
        let target = Views::new(&editor).unwrap();
        let mut source = target.recovery(&editor).unwrap();
        source.document_id.0 = "foreign-source".into();
        let path = area.path().join("source.json");
        std::fs::write(&path, source.encoded().unwrap()).unwrap();
        (area, Inspection::open(&path).unwrap(), editor, target)
    }
    fn add(e: &mut Editor, v: &mut Views, kind: RecordKind) -> RecordId {
        let stamp = v.stamp(e);
        match v
            .dispatch(
                e,
                &stamp,
                NativeAction::Command(Command::Add {
                    kind,
                    owner: RecordId(1),
                    name: "Same display name".into(),
                }),
            )
            .unwrap()
        {
            NativeOutcome::Edit(outcome) => outcome.created.unwrap(),
            _ => panic!("create"),
        }
    }
    #[test]
    fn identical_numbers_never_autofill_and_rehearsal_leaves_all_target_state_untouched() {
        let (_area, s, e, v) = fixture();
        let before = (
            e.snapshot.digest(),
            v.document().digest().unwrap(),
            v.history_usage(),
            v.document().next_id(),
            v.stamp(&e),
        );
        let mut p = Plan::new(&s, s.declared().primary, &e, &v).unwrap();
        assert!(p.choices().values().all(|c| *c == Choice::Unselected));
        let id = *p.choices().keys().next().unwrap();
        assert!(p.preview(&s, &e, &v).unwrap().occurrences[0]
            .reason
            .as_ref()
            .unwrap()
            .contains("Unselected"));
        p.choose(&s, &e, &v, id, Choice::Target(RecordId(1)))
            .unwrap();
        assert_eq!(
            p.preview(&s, &e, &v).unwrap().occurrences[0].target,
            Some(RecordId(1))
        );
        assert_eq!(
            (
                e.snapshot.digest(),
                v.document().digest().unwrap(),
                v.history_usage(),
                v.document().next_id(),
                v.stamp(&e)
            ),
            before
        );
    }
    #[test]
    fn wrong_kind_missing_target_bad_skip_and_foreign_occurrence_refuse_atomically() {
        let (_area, s, mut e, mut v) = fixture();
        let mission = add(&mut e, &mut v, RecordKind::Mission);
        let mut p = Plan::new(&s, s.declared().primary, &e, &v).unwrap();
        let id = *p.choices().keys().next().unwrap();
        let before = p.choices.clone();
        for choice in [
            Choice::Target(mission),
            Choice::Target(RecordId(999999)),
            Choice::Skip(" ".into()),
            Choice::Skip("x".repeat(1025)),
        ] {
            assert!(p.choose(&s, &e, &v, id, choice).is_err());
            assert_eq!(p.choices, before);
        }
        let other = s
            .declared()
            .occurrences
            .values()
            .find(|o| o.view != s.declared().primary)
            .unwrap()
            .id;
        assert!(p
            .choose(&s, &e, &v, other, Choice::Target(RecordId(1)))
            .is_err());
        p.choose(
            &s,
            &e,
            &v,
            id,
            Choice::Skip("Outside this target view".into()),
        )
        .unwrap();
        assert!(p.preview(&s, &e, &v).unwrap().occurrences[0]
            .reason
            .as_ref()
            .unwrap()
            .contains("Outside this target view"));
    }
    #[test]
    fn same_byte_reopen_and_same_id_revision_target_replacement_refuse() {
        let (_area, s, e, v) = fixture();
        let p = Plan::new(&s, s.declared().primary, &e, &v).unwrap();
        let reopened = Inspection::open(s.path()).unwrap();
        assert!(p
            .preview(&reopened, &e, &v)
            .unwrap_err()
            .contains("SOURCE_ORIGIN"));
        let other = Views::new(&e).unwrap();
        assert_eq!(v.document().revision(), other.document().revision());
        assert!(p
            .preview(&s, &e, &other)
            .unwrap_err()
            .contains("TARGET_STALE"));
    }
    #[test]
    fn whitespace_change_missing_source_and_native_or_layout_change_refuse_without_mutation() {
        let (_area, s, mut e, mut v) = fixture();
        let p = Plan::new(&s, s.declared().primary, &e, &v).unwrap();
        let original = std::fs::read(s.path()).unwrap();
        let mut changed = original.clone();
        changed.push(b'\n');
        std::fs::write(s.path(), changed).unwrap();
        assert!(p
            .preview(&s, &e, &v)
            .unwrap_err()
            .contains("SOURCE_CHANGED"));
        std::fs::write(s.path(), original).unwrap();
        let stamp = v.stamp(&e);
        v.apply(&e, &stamp, Layout::AddView("new target view".into()))
            .unwrap();
        assert!(p.preview(&s, &e, &v).unwrap_err().contains("TARGET_STALE"));
        let p = Plan::new(&s, s.declared().primary, &e, &v).unwrap();
        add(&mut e, &mut v, RecordKind::Mission);
        assert!(p.preview(&s, &e, &v).unwrap_err().contains("TARGET_STALE"));
        let p = Plan::new(&s, s.declared().primary, &e, &v).unwrap();
        std::fs::remove_file(s.path()).unwrap();
        assert!(p.preview(&s, &e, &v).is_err());
    }
    #[test]
    fn raw_draft_freshness_alone_stales_rehearsal() {
        let (_area, s, mut e, v) = fixture();
        let p = Plan::new(&s, s.declared().primary, &e, &v).unwrap();
        e.snapshot.raw_version += 1;
        assert!(p.preview(&s, &e, &v).unwrap_err().contains("TARGET_STALE"));
    }
    #[test]
    fn exact_duplicate_endpoints_and_authored_target_relationship_control_edge_proposals() {
        let (area, _, mut e, mut v) = fixture();
        let mission = add(&mut e, &mut v, RecordKind::Mission);
        let objective = add(&mut e, &mut v, RecordKind::Objective);
        let stamp = v.stamp(&e);
        v.dispatch(
            &mut e,
            &stamp,
            NativeAction::Command(Command::SetSubject {
                objective,
                subject: Some(SubjectRef::Mission(mission)),
            }),
        )
        .unwrap();
        let view = v.document().primary;
        for record in [objective, mission, mission] {
            let stamp = v.stamp(&e);
            v.apply(
                &e,
                &stamp,
                Layout::Reuse {
                    view,
                    record,
                    geometry: Geometry::at(100., 200.),
                },
            )
            .unwrap();
        }
        let a = v
            .document()
            .occurrences
            .values()
            .find(|o| o.record == objective)
            .unwrap()
            .id;
        let bs: Vec<_> = v
            .document()
            .occurrences
            .values()
            .filter(|o| o.record == mission)
            .map(|o| o.id)
            .collect();
        let stamp = v.stamp(&e);
        v.apply(
            &e,
            &stamp,
            Layout::Route {
                view,
                key: SemanticKey::checked(
                    &e.snapshot,
                    objective,
                    SemanticRole::ObjectiveSubject,
                    mission,
                )
                .unwrap(),
                source: a,
                target: bs[0],
            },
        )
        .unwrap();
        let mut d = v.recovery(&e).unwrap();
        d.document_id.0 = "source".into();
        // Detached source data has no native proof, but explicit mappings can
        // propose a fresh target relationship without reactivating old history.
        d.occurrences.get_mut(&a).unwrap().presence = v::Presence::Detached {
            reason: "old source".into(),
        };
        for edge in d.edges.values_mut() {
            edge.presence = v::Presence::Detached {
                reason: "old source".into(),
            };
        }
        let path = area.path().join("edge-source.json");
        std::fs::write(&path, d.encoded().unwrap()).unwrap();
        let s = Inspection::open(&path).unwrap();
        let mut p = Plan::new(&s, view, &e, &v).unwrap();
        p.choose(&s, &e, &v, a, Choice::Target(objective)).unwrap();
        p.choose(&s, &e, &v, bs[1], Choice::Target(mission))
            .unwrap();
        assert!(p.preview(&s, &e, &v).unwrap().edges[0].key.is_none()); // the other occurrence cannot stand in for the exact endpoint
        p.choose(&s, &e, &v, bs[0], Choice::Target(mission))
            .unwrap();
        let edge_preview = p.preview(&s, &e, &v).unwrap();
        assert!(edge_preview.edges[0].key.is_some());
        assert_eq!(edge_preview.edges[0].source_occurrence, a);
        assert_eq!(edge_preview.edges[0].target_occurrence, bs[0]);
        let other = add(&mut e, &mut v, RecordKind::Mission);
        let mut p = Plan::new(&s, view, &e, &v).unwrap();
        p.choose(&s, &e, &v, a, Choice::Target(objective)).unwrap();
        p.choose(&s, &e, &v, bs[0], Choice::Target(other)).unwrap();
        assert!(p.preview(&s, &e, &v).unwrap().edges[0]
            .reason
            .as_ref()
            .unwrap()
            .contains("not authored"));
        assert!(matches!(
            s.declared().occurrences[&a].presence,
            v::Presence::Detached { .. }
        ));
    }
    #[test]
    fn combined_choices_and_report_reason_copies_are_bounded_before_choice_mutation() {
        let (area, source, e, v) = fixture();
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(source.path()).unwrap()).unwrap();
        let template = value["occurrences"]["3"].clone();
        let occurrences = value["occurrences"].as_object_mut().unwrap();
        occurrences.clear();
        for n in 10000..11024 {
            let mut o = template.clone();
            o["id"] = serde_json::json!(n.to_string());
            occurrences.insert(n.to_string(), o);
        }
        value["next_id"] = serde_json::json!("30000");
        let edges = value["edges"].as_object_mut().unwrap();
        for n in 20000..28192 {
            edges.insert(n.to_string(),serde_json::json!({"id":n.to_string(),"view":"1","source":"10000","target":"10000","key":{"owner":"1","role":"ObjectiveSubject","target":"1","target_kind":"System"},"presence":{"state":"Detached","reason":"source claim"}}));
        }
        let path = area.path().join("budget-source.json");
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let source = Inspection::open(&path).unwrap();
        let mut p = Plan::new(&source, source.declared().primary, &e, &v).unwrap();
        let ids: Vec<_> = p.choices.keys().copied().collect();
        let mut refused = false;
        for id in ids {
            let before = p.choices.clone();
            match p.choose(&source, &e, &v, id, Choice::Skip("r".repeat(1024))) {
                Ok(()) => assert!(p.charged_bytes(&source) <= MAX_PLAN_BYTES),
                Err(error) => {
                    assert!(error.contains("BUDGET"));
                    assert_eq!(before, p.choices);
                    refused = true;
                    break;
                }
            }
        }
        assert!(
            refused,
            "combined retained choice and formatted preview reasons must be charged"
        );
    }
}
