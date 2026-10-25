//! Inspection of direct known native usages and current presentation impact.
//! No deletion command, transitive closure, or safe-to-delete verdict.
use crate::{
    native_navigation,
    native_views::{OccurrenceId, Presence, Stamp, ViewId, Views, VisualEdgeId},
    stage1::{Editor, RecordId, RecordKind},
    stage1_graph::RelationRole,
};
use std::collections::BTreeSet;
pub const ROW_LIMIT: usize = 128;
pub const LABEL_SCALARS: usize = 64;
pub const DISPLAY_BYTES: usize = 4 * 1024 * 1024;
fn label(text: &str) -> String {
    let mut result: String = text.chars().take(LABEL_SCALARS).collect();
    if text.chars().nth(LABEL_SCALARS).is_some() {
        result.push_str(" [shortened]");
    }
    result
}
fn presence(p: &Presence) -> String {
    match p {
        Presence::Active => "Active".into(),
        Presence::Detached { reason } => format!("Detached: {}", label(reason)),
    }
}
#[derive(Debug)]
pub struct UsageRow {
    pub record: RecordId,
    pub name: String,
    pub role: RelationRole,
    pub binding_names: Vec<String>,
}
#[derive(Debug)]
pub struct OccurrenceRow {
    pub id: OccurrenceId,
    pub view: ViewId,
    pub title: String,
    pub visible: bool,
    pub presence: String,
    pub expected_kind: RecordKind,
}
#[derive(Debug)]
pub struct EdgeRow {
    pub id: VisualEdgeId,
    pub view: ViewId,
    pub source: OccurrenceId,
    pub target: OccurrenceId,
    pub presence: String,
}
#[derive(Debug)]
pub struct Report {
    target: RecordId,
    kind: RecordKind,
    name: String,
    stamp: Stamp,
    views_digest: String,
    usages: Vec<UsageRow>,
    occurrences: Vec<OccurrenceRow>,
    edges: Vec<EdgeRow>,
    usage_total: usize,
    occurrence_total: usize,
    edge_total: usize,
    drafts: usize,
    creation_drafts: usize,
}
impl Report {
    pub fn target(&self) -> RecordId {
        self.target
    }
    pub fn kind(&self) -> RecordKind {
        self.kind
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn usages(&self) -> &[UsageRow] {
        &self.usages
    }
    pub fn occurrences(&self) -> &[OccurrenceRow] {
        &self.occurrences
    }
    pub fn edges(&self) -> &[EdgeRow] {
        &self.edges
    }
    pub fn totals(&self) -> (usize, usize, usize) {
        (self.usage_total, self.occurrence_total, self.edge_total)
    }
    pub fn drafts(&self) -> (usize, usize) {
        (self.drafts, self.creation_drafts)
    }
    pub fn check(&self, editor: &Editor, views: &Views) -> Result<(), String> {
        if views.stamp(editor) != self.stamp || views.document().digest()? != self.views_digest {
            return Err(
                "NATIVE_IMPACT_STALE: Refresh the original target before navigation".into(),
            );
        }
        Ok(())
    }
    pub fn referencing_record(
        &self,
        editor: &Editor,
        views: &Views,
        index: usize,
    ) -> Result<RecordId, String> {
        self.check(editor, views)?;
        self.usages
            .get(index)
            .map(|row| row.record)
            .ok_or_else(|| "NATIVE_IMPACT_INDEX".into())
    }
}
pub fn inspect(editor: &Editor, views: &Views, target: RecordId) -> Result<Report, String> {
    if editor
        .snapshot
        .records
        .iter()
        .any(|(key, record)| *key != record.id)
    {
        return Err("NATIVE_IMPACT_RECORD_KEY_MISMATCH".into());
    }
    views.document().validate(&editor.snapshot)?;
    let record = editor
        .snapshot
        .records
        .get(&target)
        .ok_or("NATIVE_IMPACT_TARGET_MISSING")?;
    let references = native_navigation::find_usages(editor, views, target)?;
    let usage_total = references.usages().len();
    let usages: Vec<_> = references
        .usages()
        .iter()
        .take(ROW_LIMIT)
        .map(|usage| UsageRow {
            record: usage.referencing_record,
            name: label(&editor.snapshot.records[&usage.referencing_record].name),
            role: usage.role,
            binding_names: usage.binding_names.iter().map(|s| label(s)).collect(),
        })
        .collect();
    let document = views.document();
    let occurrence_ids: BTreeSet<_> = document
        .occurrences
        .values()
        .filter(|o| o.record == target)
        .map(|o| o.id)
        .collect();
    let occurrence_total = occurrence_ids.len();
    let occurrences: Vec<_> = document
        .occurrences
        .values()
        .filter(|o| occurrence_ids.contains(&o.id))
        .take(ROW_LIMIT)
        .map(|o| OccurrenceRow {
            id: o.id,
            view: o.view,
            title: label(&document.views[&o.view].title),
            visible: o.visible,
            presence: presence(&o.presence),
            expected_kind: o.expected_kind,
        })
        .collect();
    let touching = || {
        document
            .edges
            .values()
            .filter(|e| occurrence_ids.contains(&e.source) || occurrence_ids.contains(&e.target))
    };
    let edge_total = touching().count();
    let edges: Vec<_> = touching()
        .take(ROW_LIMIT)
        .map(|e| EdgeRow {
            id: e.id,
            view: e.view,
            source: e.source,
            target: e.target,
            presence: presence(&e.presence),
        })
        .collect();
    let name = label(&record.name);
    let bytes = name.len()
        + usages
            .iter()
            .map(|u| u.name.len() + u.binding_names.iter().map(String::len).sum::<usize>())
            .sum::<usize>()
        + occurrences
            .iter()
            .map(|o| o.title.len() + o.presence.len())
            .sum::<usize>()
        + edges.iter().map(|e| e.presence.len()).sum::<usize>();
    if bytes > DISPLAY_BYTES {
        return Err("NATIVE_IMPACT_DISPLAY_LIMIT".into());
    }
    Ok(Report {
        target,
        kind: record.kind,
        name,
        stamp: views.stamp(editor),
        views_digest: document.digest()?,
        usages,
        occurrences,
        edges,
        usage_total,
        occurrence_total,
        edge_total,
        drafts: editor
            .snapshot
            .drafts
            .iter()
            .filter(|d| d.key.record == target)
            .count(),
        creation_drafts: editor
            .snapshot
            .creation_drafts
            .values()
            .filter(|d| d.owner == target)
            .count(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        native_views::{Geometry, Layout, SemanticKey, SemanticRole},
        stage1::{Command, CreationDraft, DocumentId, RawKey, Snapshot},
    };
    fn fixture() -> (Editor, Views) {
        let s = Snapshot::from_fixture(
            DocumentId("record-impact".into()),
            &serde_json::from_str(include_str!("../fixtures/stage1/inspection.json")).unwrap(),
        )
        .unwrap();
        let e = Editor::new(s, "impact-runtime".into()).unwrap();
        let v = Views::new(&e).unwrap();
        (e, v)
    }
    fn apply(e: &Editor, v: &mut Views, command: Layout) {
        v.apply(e, &v.stamp(e), command).unwrap();
    }
    #[test]
    fn direct_roles_remain_normalized_and_inspection_does_not_mutate_state() {
        let (e, v) = fixture();
        let before = (
            e.snapshot.digest(),
            v.document().digest().unwrap(),
            v.history_usage(),
            v.document().next_id(),
            e.selected,
            e.calculation_stamp(),
        );
        for (target, owner, role) in [
            (100, 200, RelationRole::ContributesTo),
            (200, 300, RelationRole::Subject),
            (300, 301, RelationRole::ForObjective),
            (1, 100, RelationRole::Scope),
        ] {
            let r = inspect(&e, &v, RecordId(target)).unwrap();
            assert!(r
                .usages()
                .iter()
                .any(|u| u.record == RecordId(owner) && u.role == role));
        }
        assert!(!inspect(&e, &v, RecordId(100))
            .unwrap()
            .usages()
            .iter()
            .any(|u| u.record == RecordId(1)));
        assert_eq!(
            (
                e.snapshot.digest(),
                v.document().digest().unwrap(),
                v.history_usage(),
                v.document().next_id(),
                e.selected,
                e.calculation_stamp()
            ),
            before
        );
        assert!(inspect(&e, &v, RecordId(999999)).is_err());
    }
    #[test]
    fn all_views_hidden_and_detached_occurrences_and_exact_touching_edges() {
        let (e, mut v) = fixture();
        let primary = v.document().primary;
        let second = *v
            .document()
            .views
            .keys()
            .find(|id| **id != primary)
            .unwrap();
        apply(
            &e,
            &mut v,
            Layout::Reuse {
                view: second,
                record: RecordId(100),
                geometry: Geometry::at(10., 10.),
            },
        );
        let hidden = v
            .document()
            .occurrences
            .values()
            .find(|o| o.view == second && o.record == RecordId(100))
            .unwrap()
            .id;
        apply(&e, &mut v, Layout::Visible(hidden, false));
        let mission = v
            .document()
            .occurrences
            .values()
            .find(|o| o.view == primary && o.record == RecordId(100))
            .unwrap()
            .id;
        let capability = v
            .document()
            .occurrences
            .values()
            .find(|o| o.view == primary && o.record == RecordId(200))
            .unwrap()
            .id;
        apply(
            &e,
            &mut v,
            Layout::Route {
                view: primary,
                key: SemanticKey::checked(
                    &e.snapshot,
                    RecordId(200),
                    SemanticRole::CapabilityContributesToMission,
                    RecordId(100),
                )
                .unwrap(),
                source: capability,
                target: mission,
            },
        );
        let mut d = v.recovery(&e).unwrap();
        d.occurrences.get_mut(&hidden).unwrap().presence = Presence::Detached {
            reason: "Retained presentation diagnostic".into(),
        };
        d.occurrences.get_mut(&capability).unwrap().record = RecordId(100);
        d.occurrences.get_mut(&capability).unwrap().presence = Presence::Detached {
            reason: "Retained endpoint kind mismatch".into(),
        };
        for edge in d.edges.values_mut() {
            edge.key.owner = RecordId(100); // exact binding retained even while detached
            edge.presence = Presence::Detached {
                reason: "Retained old routing".into(),
            };
        }
        let v = Views::restore(&d.encoded().unwrap(), &e).unwrap();
        let r = inspect(&e, &v, RecordId(100)).unwrap();
        assert_eq!(r.totals().1, 3);
        let row = r.occurrences().iter().find(|o| o.id == hidden).unwrap();
        assert!(!row.visible);
        assert!(row.presence.starts_with("Detached:"));
        assert_eq!(row.view, second);
        assert_eq!(r.totals().2, 1); // both exact endpoints match target; count edge once
        assert_eq!(
            (r.edges()[0].source, r.edges()[0].target),
            (capability, mission)
        );
    }
    #[test]
    fn stale_native_raw_layout_and_runtime_refuse_but_selection_is_navigation() {
        let (mut e, mut v) = fixture();
        let r = inspect(&e, &v, RecordId(100)).unwrap();
        let index = r
            .usages()
            .iter()
            .position(|u| u.record == RecordId(200))
            .unwrap();
        e.select(r.referencing_record(&e, &v, index).unwrap())
            .unwrap();
        assert!(r.check(&e, &v).is_ok());
        assert!(r.referencing_record(&e, &v, usize::MAX).is_err());
        let other = Views::new(&e).unwrap();
        assert!(r.check(&e, &other).is_err());
        e.snapshot.raw_version += 1;
        assert!(r.check(&e, &v).is_err());
        let r = inspect(&e, &v, RecordId(100)).unwrap();
        e.snapshot
            .records
            .get_mut(&RecordId(100))
            .unwrap()
            .name
            .push('x');
        assert!(r.check(&e, &v).is_err());
        let r = inspect(&e, &v, RecordId(100)).unwrap();
        apply(&e, &mut v, Layout::AddView("Third".into()));
        assert!(r.check(&e, &v).is_err());
        let fresh = inspect(&e, &v, RecordId(100)).unwrap();
        assert_eq!(fresh.target(), RecordId(100));
    }
    #[test]
    fn calculation_output_usage_retains_consumer_direction_and_named_bindings() {
        use crate::stage1_calc::{
            CalculationId, Definition, Expr, Input, Literal, QuantityKind, QuantityType, UnitId,
        };
        use std::collections::BTreeMap;
        let (mut e, v) = fixture();
        let consumer = e
            .command(
                &e.freshness(),
                Command::Add {
                    kind: RecordKind::Measure,
                    owner: RecordId(300),
                    name: "Consumer".into(),
                },
            )
            .unwrap()
            .created
            .unwrap();
        let q = QuantityType {
            kind: QuantityKind::Scalar,
            unit: UnitId::ScalarOne,
        };
        e.command(
            &e.freshness(),
            Command::PutCalculation {
                measure: RecordId(301),
                definition: Some(Definition {
                    id: CalculationId(301),
                    owner: RecordId(301),
                    bindings: BTreeMap::new(),
                    expression: Expr::Literal(Literal {
                        decimal: "2".into(),
                        quantity: q,
                    }),
                    output: q,
                }),
            },
        )
        .unwrap();
        e.command(
            &e.freshness(),
            Command::PutCalculation {
                measure: consumer,
                definition: Some(Definition {
                    id: CalculationId(consumer.0),
                    owner: consumer,
                    bindings: BTreeMap::from([
                        ("z".into(), Input::Output(CalculationId(301))),
                        ("a".into(), Input::Output(CalculationId(301))),
                    ]),
                    expression: Expr::Add(
                        Box::new(Expr::Binding("a".into())),
                        Box::new(Expr::Binding("z".into())),
                    ),
                    output: q,
                }),
            },
        )
        .unwrap();
        let r = inspect(&e, &v, RecordId(301)).unwrap();
        let row = r
            .usages()
            .iter()
            .find(|u| u.role == RelationRole::CalculationInput)
            .unwrap();
        assert_eq!(row.record, consumer);
        assert_eq!(row.binding_names, vec!["a", "z"]);
        assert!(!inspect(&e, &v, consumer)
            .unwrap()
            .usages()
            .iter()
            .any(|u| u.role == RelationRole::CalculationInput));
    }
    #[test]
    fn counts_all_stored_owner_drafts_and_pending_creation_drafts_without_text_copy() {
        let (mut e, v) = fixture();
        e.command(
            &e.freshness(),
            Command::EditText {
                key: RawKey {
                    record: RecordId(301),
                    field: "criterion".into(),
                    item: None,
                },
                text: "12e-".into(),
            },
        )
        .unwrap();
        let mut duplicate = e
            .snapshot
            .drafts
            .iter()
            .find(|d| d.key.record == RecordId(301))
            .unwrap()
            .clone();
        duplicate.key.field = "calc.expression".into();
        duplicate.owner_generation = Some(0);
        duplicate.version = 0;
        duplicate.validity = crate::stage1::Validity::Unsupported;
        e.snapshot.drafts.push(duplicate); // every stored entry, not active-generation lookup
        e.snapshot.creation_drafts.insert(
            "pending-child".into(),
            CreationDraft {
                owner: RecordId(300),
                kind: RecordKind::Measure,
                fields: Default::default(),
            },
        );
        let measure = inspect(&e, &v, RecordId(301)).unwrap();
        assert_eq!(
            measure.drafts().0,
            e.snapshot
                .drafts
                .iter()
                .filter(|d| d.key.record == RecordId(301))
                .count()
        );
        assert_eq!(inspect(&e, &v, RecordId(300)).unwrap().drafts().1, 1);
    }
    #[test]
    fn label_limits_high_ids_and_truncated_rows_keep_exact_totals() {
        let (mut e, mut v) = fixture();
        let high = RecordId(9007199254741001);
        let mut record = e.snapshot.records[&RecordId(100)].clone();
        record.id = high;
        record.name = "界".repeat(100);
        e.snapshot.records.insert(high, record);
        e.snapshot.next_id = high.0 + 1;
        for _ in 0..ROW_LIMIT + 1 {
            let view = v.document().primary;
            apply(
                &e,
                &mut v,
                Layout::Reuse {
                    view,
                    record: high,
                    geometry: Geometry::at(1., 1.),
                },
            );
        }
        let r = inspect(&e, &v, high).unwrap();
        assert_eq!(r.target(), high);
        assert!(r.name().ends_with(" [shortened]"));
        assert_eq!(r.occurrences().len(), ROW_LIMIT);
        assert_eq!(r.totals().1, ROW_LIMIT + 1);
        assert!(r.name().len() <= 4 * LABEL_SCALARS + " [shortened]".len());
    }
    #[test]
    fn mismatched_document_and_record_key_fail_explicitly() {
        let (mut e, v) = fixture();
        e.snapshot.document_id = DocumentId("wrong".into());
        assert!(inspect(&e, &v, RecordId(100)).is_err());
        let (mut e, v) = fixture();
        let r = e.snapshot.records.remove(&RecordId(100)).unwrap();
        e.snapshot.records.insert(RecordId(999), r);
        assert!(inspect(&e, &v, RecordId(999))
            .unwrap_err()
            .contains("KEY_MISMATCH"));
    }
}
