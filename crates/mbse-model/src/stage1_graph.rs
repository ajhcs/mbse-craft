//! Deterministic projections of authored Stage1 relations. Geometry is not semantics.
use crate::stage1::{RecordId, RecordKind, Snapshot, SubjectRef};
use crate::stage1_calc::Input;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationRole {
    Scope,
    ContributesTo,
    Subject,
    ForObjective,
    CalculationInput,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeId {
    pub role: RelationRole,
    pub source: RecordId,
    pub target: RecordId,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEdge {
    pub id: EdgeId,
    pub label: &'static str,
    pub binding_names: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graph {
    pub nodes: Vec<RecordId>,
    pub edges: Vec<GraphEdge>,
}

pub fn derive_graph(snapshot: &Snapshot) -> Result<Graph, String> {
    snapshot.validate()?;
    let mut edges = BTreeSet::new();
    for record in snapshot.records.values() {
        if record.id != record.scope {
            edges.insert(EdgeId {
                role: RelationRole::Scope,
                source: record.scope,
                target: record.id,
            });
        }
        match record.kind {
            RecordKind::Capability => {
                for target in record.links() {
                    edges.insert(EdgeId {
                        role: RelationRole::ContributesTo,
                        source: record.id,
                        target,
                    });
                }
            }
            RecordKind::Objective => {
                if let Some(subject) = record.subject() {
                    let target = match subject {
                        SubjectRef::System(id)
                        | SubjectRef::Mission(id)
                        | SubjectRef::Capability(id) => id,
                    };
                    edges.insert(EdgeId {
                        role: RelationRole::Subject,
                        source: record.id,
                        target,
                    });
                }
            }
            RecordKind::Measure => {
                let target = RecordId(
                    record
                        .fields
                        .get("forObjective")
                        .and_then(serde_json::Value::as_u64)
                        .ok_or("Measure has no Objective")?,
                );
                edges.insert(EdgeId {
                    role: RelationRole::ForObjective,
                    source: record.id,
                    target,
                });
            }
            _ => {}
        }
    }
    let definitions = snapshot.calculations()?;
    let mut bindings: BTreeMap<EdgeId, BTreeSet<String>> = BTreeMap::new();
    for definition in definitions.values() {
        for (name, input) in &definition.bindings {
            if let Input::Output(dependency) = input {
                // Authored unresolved dependencies remain diagnostics in the evaluator;
                // the view never fabricates a missing producer node.
                if let Some(producer) = definitions.get(dependency) {
                    let id = EdgeId {
                        role: RelationRole::CalculationInput,
                        source: producer.owner,
                        target: definition.owner,
                    };
                    edges.insert(id);
                    bindings.entry(id).or_default().insert(name.clone());
                }
            }
        }
    }
    Ok(Graph {
        nodes: snapshot.records.keys().copied().collect(),
        edges: edges
            .into_iter()
            .map(|id| GraphEdge {
                label: match id.role {
                    RelationRole::Scope => "scope",
                    RelationRole::ContributesTo => "contributes to",
                    RelationRole::Subject => "subject",
                    RelationRole::ForObjective => "for objective",
                    RelationRole::CalculationInput => "calculation input",
                },
                binding_names: bindings
                    .remove(&id)
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
                id,
            })
            .collect(),
    })
}
pub fn related_edges(graph: &Graph, record: RecordId) -> Vec<EdgeId> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.id.source == record || edge.id.target == record)
        .map(|edge| edge.id)
        .collect()
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubjectFilter {
    #[default]
    All,
    System,
    Mission,
    Capability,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectCandidate {
    pub id: RecordId,
    pub kind: RecordKind,
    pub name: String,
}
pub fn subject_candidates(
    snapshot: &Snapshot,
    objective: RecordId,
    filter: SubjectFilter,
) -> Result<Vec<SubjectCandidate>, String> {
    let record = snapshot
        .records
        .get(&objective)
        .filter(|record| record.kind == RecordKind::Objective)
        .ok_or("Choose an existing Objective")?;
    Ok(snapshot
        .records
        .values()
        .filter(|item| {
            item.scope == record.scope
                && matches!(
                    item.kind,
                    RecordKind::System | RecordKind::Mission | RecordKind::Capability
                )
        })
        .filter(|item| match filter {
            SubjectFilter::All => true,
            SubjectFilter::System => item.kind == RecordKind::System,
            SubjectFilter::Mission => item.kind == RecordKind::Mission,
            SubjectFilter::Capability => item.kind == RecordKind::Capability,
        })
        .map(|item| SubjectCandidate {
            id: item.id,
            kind: item.kind,
            name: item.name.clone(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage1::DocumentId;
    fn fixture() -> Snapshot {
        Snapshot::from_fixture(
            DocumentId("graph-test".into()),
            &serde_json::from_str(include_str!("../fixtures/stage1/inspection.json")).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn authored_edges_are_exact_and_do_not_invent_invocation_ownership() {
        let mut snapshot = fixture();
        let graph = derive_graph(&snapshot).unwrap();
        assert_eq!(graph.nodes.len(), 10);
        let authored: Vec<_> = graph
            .edges
            .iter()
            .filter(|e| e.id.role != RelationRole::Scope)
            .map(|e| e.id)
            .collect();
        assert_eq!(
            authored,
            vec![
                EdgeId {
                    role: RelationRole::ContributesTo,
                    source: RecordId(200),
                    target: RecordId(100)
                },
                EdgeId {
                    role: RelationRole::Subject,
                    source: RecordId(300),
                    target: RecordId(200)
                },
                EdgeId {
                    role: RelationRole::ForObjective,
                    source: RecordId(301),
                    target: RecordId(300)
                },
            ]
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.id.role == RelationRole::Scope)
                .count(),
            9
        );
        snapshot.records.get_mut(&RecordId(200)).unwrap().name = "Same name".into();
        assert_eq!(derive_graph(&snapshot).unwrap(), graph);
        assert_eq!(related_edges(&graph, RecordId(301)).len(), 2);
    }
    #[test]
    fn chooser_filters_are_views_and_cross_scope_or_wrong_typed_edges_refuse() {
        let mut snapshot = fixture();
        let before = snapshot.clone();
        assert_eq!(
            subject_candidates(&snapshot, RecordId(300), SubjectFilter::All)
                .unwrap()
                .len(),
            3
        );
        for (filter, id) in [
            (SubjectFilter::System, 1),
            (SubjectFilter::Mission, 100),
            (SubjectFilter::Capability, 200),
        ] {
            assert_eq!(
                subject_candidates(&snapshot, RecordId(300), filter).unwrap()[0].id,
                RecordId(id)
            );
        }
        assert_eq!(snapshot, before);
        snapshot
            .records
            .get_mut(&RecordId(300))
            .unwrap()
            .fields
            .insert(
                "subject".into(),
                serde_json::json!({"kind":"Mission","id":200}),
            );
        assert!(derive_graph(&snapshot).is_err());
        assert!(subject_candidates(&snapshot, RecordId(301), SubjectFilter::All).is_err());
    }
    #[test]
    fn calculation_dependencies_coalesce_named_uses_without_criterion_edges() {
        use crate::stage1::{Command, Editor};
        use crate::stage1_calc::{
            CalculationId, Definition, Expr, Literal, QuantityKind, QuantityType, UnitId,
        };
        let mut editor = Editor::new(fixture(), "dependency-graph".into()).unwrap();
        let consumer = editor
            .command(
                &editor.freshness(),
                Command::Add {
                    kind: RecordKind::Measure,
                    owner: RecordId(300),
                    name: "Consumer".into(),
                },
            )
            .unwrap()
            .created
            .unwrap();
        let quantity = QuantityType {
            kind: QuantityKind::Scalar,
            unit: UnitId::ScalarOne,
        };
        let producer = Definition {
            id: CalculationId(301),
            owner: RecordId(301),
            bindings: BTreeMap::new(),
            expression: Expr::Literal(Literal {
                decimal: "2".into(),
                quantity,
            }),
            output: quantity,
        };
        editor
            .command(
                &editor.freshness(),
                Command::PutCalculation {
                    measure: RecordId(301),
                    definition: Some(producer),
                },
            )
            .unwrap();
        let definition = Definition {
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
            output: quantity,
        };
        editor
            .command(
                &editor.freshness(),
                Command::PutCalculation {
                    measure: consumer,
                    definition: Some(definition),
                },
            )
            .unwrap();
        let graph = derive_graph(&editor.snapshot).unwrap();
        let edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.id.role == RelationRole::CalculationInput)
            .collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].id.source, RecordId(301));
        assert_eq!(edges[0].id.target, consumer);
        assert_eq!(edges[0].binding_names, vec!["a", "z"]);
        assert!(graph
            .edges
            .iter()
            .filter(|edge| edge.id.role != RelationRole::CalculationInput)
            .all(|edge| edge.binding_names.is_empty()));
    }
}
