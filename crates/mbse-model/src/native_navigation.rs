//! Read-only native reference navigation over the existing authored graph.
//! Graph drawing direction is normalized to referencing-record → referenced-record.
use crate::{
    native_views::{Stamp, Views},
    stage1::{Editor, RecordId},
    stage1_graph::{self, EdgeId, RelationRole},
};

#[derive(Debug, PartialEq, Eq)]
pub struct Usage {
    pub referencing_record: RecordId,
    pub role: RelationRole,
    pub binding_names: Vec<String>,
    pub graph_edge: EdgeId,
}
pub struct UsageReport {
    target: RecordId,
    stamp: Stamp,
    usages: Vec<Usage>,
}
impl UsageReport {
    pub fn target(&self) -> RecordId {
        self.target
    }
    pub fn usages(&self) -> &[Usage] {
        &self.usages
    }
    /// The full presentation stamp supplies sealed runtime identity as well as
    /// native freshness/hash. Layout edits conservatively require refresh too;
    /// geometry never participates in deriving reference meaning.
    pub fn check(&self, editor: &Editor, views: &Views) -> Result<(), String> {
        if views.stamp(editor) != self.stamp {
            return Err("NATIVE_USAGES_STALE: refresh before navigating".into());
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
        let id = self
            .usages
            .get(index)
            .ok_or("NATIVE_USAGES_INDEX")?
            .referencing_record;
        if !editor.snapshot.records.contains_key(&id) {
            return Err("NATIVE_USAGES_RECORD_MISSING".into());
        }
        Ok(id)
    }
}
pub fn find_usages(
    editor: &Editor,
    views: &Views,
    target: RecordId,
) -> Result<UsageReport, String> {
    if !editor.snapshot.records.contains_key(&target) {
        return Err("NATIVE_USAGES_TARGET_MISSING".into());
    }
    let graph = stage1_graph::derive_graph(&editor.snapshot)?;
    let usages = graph
        .edges
        .into_iter()
        .filter_map(|edge| {
            let (referencing, referenced) = match edge.id.role {
                // Native child.scope references parent; graph draws parent→child.
                // Calculation consumer references producer; graph draws producer→consumer.
                RelationRole::Scope | RelationRole::CalculationInput => {
                    (edge.id.target, edge.id.source)
                }
                RelationRole::ContributesTo
                | RelationRole::Subject
                | RelationRole::ForObjective => (edge.id.source, edge.id.target),
            };
            (referenced == target).then_some(Usage {
                referencing_record: referencing,
                role: edge.id.role,
                binding_names: edge.binding_names,
                graph_edge: edge.id,
            })
        })
        .collect();
    Ok(UsageReport {
        target,
        stamp: views.stamp(editor),
        usages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage1::{Command, DocumentId, RecordKind, Snapshot};
    fn fixture() -> (Editor, Views) {
        let snapshot = Snapshot::from_fixture(
            DocumentId("native-usages".into()),
            &serde_json::from_str(include_str!("../fixtures/stage1/inspection.json")).unwrap(),
        )
        .unwrap();
        let editor = Editor::new(snapshot, "native-usages-runtime".into()).unwrap();
        let views = Views::new(&editor).unwrap();
        (editor, views)
    }
    #[test]
    fn exact_authored_reference_roles_preserve_owner_direction_and_distinct_ids() {
        let (e, v) = fixture();
        let before = (
            e.snapshot.digest(),
            v.document().digest().unwrap(),
            v.history_usage(),
            v.document().next_id(),
        );
        for (target, owner, role) in [
            (100, 200, RelationRole::ContributesTo),
            (200, 300, RelationRole::Subject),
            (300, 301, RelationRole::ForObjective),
            (1, 100, RelationRole::Scope),
        ] {
            let r = find_usages(&e, &v, RecordId(target)).unwrap();
            assert!(r
                .usages()
                .iter()
                .any(|u| u.referencing_record == RecordId(owner) && u.role == role));
        }
        let mission = find_usages(&e, &v, RecordId(100)).unwrap();
        assert!(!mission
            .usages()
            .iter()
            .any(|u| u.referencing_record == RecordId(1))); // scope parent is not a user of its child
        assert_eq!(
            (
                e.snapshot.digest(),
                v.document().digest().unwrap(),
                v.history_usage(),
                v.document().next_id()
            ),
            before
        );
        assert!(find_usages(&e, &v, RecordId(999999)).is_err());
    }
    #[test]
    fn stale_runtime_raw_and_layout_refuse_navigation_without_mutation() {
        let (mut e, mut v) = fixture();
        let report = find_usages(&e, &v, RecordId(100)).unwrap();
        let other = Views::new(&e).unwrap();
        assert!(report
            .referencing_record(&e, &other, 0)
            .unwrap_err()
            .contains("STALE"));
        assert!(report.referencing_record(&e, &v, 9999).is_err());
        e.snapshot.raw_version += 1;
        assert!(report.referencing_record(&e, &v, 0).is_err());
        let report = find_usages(&e, &v, RecordId(100)).unwrap();
        let stamp = v.stamp(&e);
        v.apply(
            &e,
            &stamp,
            crate::native_views::Layout::AddView("new view".into()),
        )
        .unwrap();
        assert!(report.referencing_record(&e, &v, 0).is_err());
    }
    #[test]
    fn selection_is_navigation_and_does_not_change_report_freshness_or_histories() {
        let (mut e, v) = fixture();
        let report = find_usages(&e, &v, RecordId(100)).unwrap();
        let before = e.snapshot.digest();
        let index = report
            .usages()
            .iter()
            .position(|u| u.referencing_record == RecordId(200))
            .unwrap();
        let owner = report.referencing_record(&e, &v, index).unwrap();
        e.select(owner).unwrap();
        assert_eq!(e.selected, Some(RecordId(200)));
        assert_eq!(e.snapshot.digest(), before);
        assert!(report.check(&e, &v).is_ok());
    }
    #[test]
    fn calculation_consumer_uses_producer_under_named_bindings_not_the_reverse() {
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
        let report = find_usages(&e, &v, RecordId(301)).unwrap();
        let usage = report
            .usages()
            .iter()
            .find(|u| u.role == RelationRole::CalculationInput)
            .unwrap();
        assert_eq!(usage.referencing_record, consumer);
        assert_eq!(usage.binding_names, vec!["a", "z"]);
        assert!(!find_usages(&e, &v, consumer)
            .unwrap()
            .usages()
            .iter()
            .any(|u| u.role == RelationRole::CalculationInput));
    }
}
