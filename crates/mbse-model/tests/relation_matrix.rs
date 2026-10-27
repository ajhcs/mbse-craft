use mbse_model::{
    relation_matrix::*,
    stage1::*,
    stage1_graph::{self, RelationRole},
};
use serde_json::{Value, json};
fn editor() -> Editor {
    let v: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p28/fixtures/native.kernspan"
    ))
    .unwrap();
    Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "matrix:test".into(),
    )
    .unwrap()
}
fn report(e: &Editor, kind: Kind) -> Report {
    query(
        e,
        Query {
            kind,
            ..Default::default()
        },
    )
    .unwrap()
}
fn apply(e: &mut Editor, r: &Report, a: u64, b: u64) {
    let proof = r.edit(e, RecordId(a), RecordId(b)).unwrap();
    let cmd = proof.resolve(e).unwrap();
    e.command(&e.freshness(), cmd).unwrap();
}
#[test]
fn matrix_all_cells_equal_authoritative_graph_projection_and_read_only_roles_preserve_source() {
    let e = editor();
    let before = e.snapshot.clone();
    let graph = stage1_graph::derive_graph(&before).unwrap();
    for (kind, role) in [
        (Kind::ContributesTo, RelationRole::ContributesTo),
        (Kind::Subject, RelationRole::Subject),
        (Kind::ForObjective, RelationRole::ForObjective),
        (Kind::Scope, RelationRole::Scope),
        (Kind::CalculationInput, RelationRole::CalculationInput),
    ] {
        let r = report(&e, kind);
        assert_eq!(r.total(), 5);
        assert_eq!(r.cells().len(), 25);
        for c in r.cells() {
            assert_eq!(
                c.present,
                graph.edges.iter().any(|x| x.id.role == role
                    && x.id.source == c.source
                    && x.id.target == c.target)
            );
            if matches!(kind, Kind::Scope | Kind::CalculationInput) {
                assert!(!c.editable);
                assert!(r.edit(&e, c.source, c.target).is_err());
            }
        }
        assert_eq!(e.snapshot, before);
    }
    assert!(
        report(&e, Kind::ContributesTo)
            .edit(&e, RecordId(1000), RecordId(1001))
            .is_err()
    );
}
#[test]
fn three_matrix_edit_families_use_native_history_and_singular_replacement_without_losing_raw() {
    let mut e = editor();
    let raw = e.snapshot.drafts.clone();
    let ext = e.snapshot.extensions.clone();
    let records = e.snapshot.records.clone();
    let r = report(&e, Kind::ContributesTo);
    apply(&mut e, &r, 1001, 1000);
    assert!(e.snapshot.records[&RecordId(1001)].links().is_empty());
    assert!(e.undo_model(&e.freshness()).unwrap());
    assert_eq!(e.snapshot.records, records);
    assert!(e.redo_model(&e.freshness()).unwrap());
    assert!(e.snapshot.records[&RecordId(1001)].links().is_empty());
    let r = report(&e, Kind::ContributesTo);
    apply(&mut e, &r, 1001, 1000);
    let r = report(&e, Kind::Subject);
    apply(&mut e, &r, 1002, 1);
    assert_eq!(
        e.snapshot.records[&RecordId(1002)].subject(),
        Some(SubjectRef::System(RecordId(1)))
    );
    let r = report(&e, Kind::Subject);
    apply(&mut e, &r, 1002, 1);
    assert_eq!(e.snapshot.records[&RecordId(1002)].subject(), None);
    let added = e
        .command(
            &e.freshness(),
            Command::Add {
                kind: RecordKind::Objective,
                owner: RecordId(1),
                name: "Alternate Objective".into(),
            },
        )
        .unwrap()
        .created
        .unwrap();
    let r = report(&e, Kind::ForObjective);
    assert!(r.edit(&e, RecordId(1003), RecordId(1002)).is_err());
    apply(&mut e, &r, 1003, added.0);
    assert_eq!(
        e.snapshot.records[&RecordId(1003)].fields["forObjective"],
        json!(added.0)
    );
    assert_eq!(
        stage1_graph::derive_graph(&e.snapshot)
            .unwrap()
            .edges
            .iter()
            .filter(|x| x.id.role == RelationRole::ForObjective && x.id.source == RecordId(1003))
            .map(|x| x.id.target)
            .collect::<Vec<_>>(),
        vec![added]
    );
    assert_eq!(e.snapshot.drafts, raw);
    assert_eq!(e.snapshot.extensions, ext);
}
#[test]
fn wrong_kind_cross_system_and_required_current_cell_cannot_mutate_native_state() {
    let mut e = Editor::new(
        Snapshot::new(DocumentId("matrix:cross".into())),
        "matrix:cross-runtime".into(),
    )
    .unwrap();
    let mut secondary = e.snapshot.records[&RecordId(1)].clone();
    secondary.id = RecordId(2);
    secondary.scope = RecordId(2);
    e.snapshot.records.insert(secondary.id, secondary);
    let mission = e
        .command(
            &e.freshness(),
            Command::Add {
                kind: RecordKind::Mission,
                owner: RecordId(1),
                name: "Visible duplicate".into(),
            },
        )
        .unwrap()
        .created
        .unwrap();
    let cap = e
        .command(
            &e.freshness(),
            Command::Add {
                kind: RecordKind::Capability,
                owner: RecordId(2),
                name: "Visible duplicate".into(),
            },
        )
        .unwrap()
        .created
        .unwrap();
    let old = e.snapshot.clone();
    let r = report(&e, Kind::ContributesTo);
    let c = r
        .cells()
        .iter()
        .find(|c| c.source == cap && c.target == mission)
        .unwrap();
    assert!(!c.editable && c.reason.as_ref().unwrap().contains("Cross-System"));
    assert!(r.edit(&e, cap, mission).is_err());
    assert!(r.edit(&e, mission, cap).is_err());
    assert_eq!(e.snapshot, old);
}
#[test]
fn queued_matrix_edit_and_axis_refuse_same_counter_payload_raw_and_runtime_changes() {
    let mut e = editor();
    let r = report(&e, Kind::ContributesTo);
    let proof = r.edit(&e, RecordId(1001), RecordId(1000)).unwrap();
    e.select(RecordId(1003)).unwrap();
    proof.resolve(&e).unwrap();
    r.axis(&e, RecordId(1001)).unwrap();
    let cold = Editor::new(e.snapshot.clone(), "matrix:cold".into()).unwrap();
    assert!(proof.resolve(&cold).is_err());
    e.snapshot.records.get_mut(&RecordId(1001)).unwrap().name = "New bytes same counters".into();
    let old = e.snapshot.clone();
    assert!(proof.resolve(&e).is_err());
    assert!(r.axis(&e, RecordId(1001)).is_err());
    assert_eq!(e.snapshot, old);
    let mut e = editor();
    e.snapshot.drafts[0].text.push('?');
    assert!(proof.resolve(&e).is_err());
    assert!(r.check(&e).is_err());
}
#[test]
fn matrix_paging_has_stable_id_axes_complete_totals_and_no_off_page_edit_authority() {
    let mut e = Editor::new(
        Snapshot::new(DocumentId("matrix:pages".into())),
        "matrix:pages-runtime".into(),
    )
    .unwrap();
    for _ in 0..40 {
        e.command(
            &e.freshness(),
            Command::Add {
                kind: RecordKind::Mission,
                owner: RecordId(1),
                name: "Same label".into(),
            },
        )
        .unwrap();
    }
    let cap = e
        .command(
            &e.freshness(),
            Command::Add {
                kind: RecordKind::Capability,
                owner: RecordId(1),
                name: "Source".into(),
            },
        )
        .unwrap()
        .created
        .unwrap();
    let r = report(&e, Kind::ContributesTo);
    assert_eq!(r.total(), 42);
    assert_eq!(
        (r.rows().len(), r.columns().len(), r.cells().len()),
        (32, 32, 1024)
    );
    assert!(r.edit(&e, cap, RecordId(1000)).is_err());
    let p = query(
        &e,
        Query {
            row_offset: 32,
            column_offset: 32,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(p.rows().len(), 10);
    assert!(p.rows().windows(2).all(|a| a[0].id < a[1].id));
    assert!(
        !r.rows()
            .iter()
            .any(|a| p.rows().iter().any(|b| a.id == b.id))
    );
    let end = query(
        &e,
        Query {
            row_offset: usize::MAX,
            column_offset: usize::MAX,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        (end.query().row_offset, end.query().column_offset),
        (42, 42)
    );
    assert!(end.cells().is_empty());
}
