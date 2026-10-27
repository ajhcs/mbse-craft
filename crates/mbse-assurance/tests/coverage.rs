//! Synthetic, source-grounded local coverage oracles; no external engineering verdict.
use mbse_assurance::{coverage::*, *};
use mbse_model::{stage1::Editor, stage1_calc::CalculationId};
use serde_json::{Value, json};
fn setup() -> (Board, Editor, RequirementId, RequirementId) {
    let v: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p24/fixtures/native.kernspan"
    ))
    .unwrap();
    let e = Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "p25:test".into(),
    )
    .unwrap();
    let mut b = Board::new();
    let a = b
        .add_identified_requirement("Source shall retain exact links".into(), "R-1".into(), None)
        .unwrap();
    let c = b
        .add_identified_requirement(
            "Consumer shall retain exact evidence".into(),
            "R-2".into(),
            None,
        )
        .unwrap();
    (b, e, a, c)
}
fn req(id: RequirementId) -> TraceEndpoint {
    TraceEndpoint::Requirement(id)
}
fn config() -> Value {
    json!({"purpose":"P25 synthetic coverage"})
}
fn area() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap()
}
fn row(report: &CoverageReport, id: RequirementId) -> &CoverageRow {
    report
        .requirements
        .iter()
        .find(|r| r.requirement == id)
        .unwrap()
}
#[test]
fn exact_missing_link_and_isolated_requirement_ids_do_not_confuse_dependency_kinds() {
    let (mut b, e, a, c) = setup();
    let orphan = b.add_requirement("Isolated root".into()).unwrap();
    b.add_trace(TraceKind::Derive, req(c), a, "satisfy".into(), &e.snapshot)
        .unwrap();
    b.add_trace(
        TraceKind::Satisfy,
        traces::native_endpoint(&e.snapshot, 1021).unwrap(),
        a,
        "verify".into(),
        &e.snapshot,
    )
    .unwrap();
    let case = b
        .add_test_case("TC".into(), "Inspect".into(), "Exact".into())
        .unwrap();
    b.add_trace(
        TraceKind::Verify,
        TraceEndpoint::TestCase(case),
        c,
        "satisfy".into(),
        &e.snapshot,
    )
    .unwrap();
    let report = b.coverage(&e, &config(), Some(a)).unwrap();
    assert!(!row(&report, a).missing_satisfy && row(&report, a).missing_verify);
    assert!(row(&report, c).missing_satisfy && !row(&report, c).missing_verify);
    assert!(row(&report, orphan).orphan);
    assert!(!row(&report, c).orphan);
    assert!(b.register.evidence.is_empty() && b.register.decisions.is_empty());
}
#[test]
fn detached_native_link_never_counts_as_satisfaction_or_retargets_by_id() {
    let (mut b, mut e, a, _) = setup();
    b.add_trace(
        TraceKind::Satisfy,
        traces::native_endpoint(&e.snapshot, 1021).unwrap(),
        a,
        "".into(),
        &e.snapshot,
    )
    .unwrap();
    let before = b.register.clone();
    e.snapshot.document_id.0 = "different-document".into();
    let report = b.coverage(&e, &config(), None).unwrap();
    assert!(row(&report, a).missing_satisfy);
    assert_eq!(report.detached_traces.len(), 1);
    assert!(report.detached_traces[0].reason.contains("DOCUMENT"));
    assert_eq!(b.register, before);
}
#[test]
fn supplier_change_stales_consumer_evidence_without_rewriting_consumer_or_old_receipt() {
    let (mut b, e, a, c) = setup();
    let link = b
        .add_trace(TraceKind::Derive, req(c), a, "".into(), &e.snapshot)
        .unwrap();
    let evidence = b
        .attach_calculation(c, &e, &config(), CalculationId(1003))
        .unwrap();
    let context = EvidenceContext::from_editor(&e, &config()).unwrap();
    assert!(b.evidence_staleness(evidence, &context).unwrap().is_empty());
    let old = b.register.evidence[&evidence].clone();
    let consumer = b.register.requirements[&c].clone();
    b.rename_trace(link, "Copy verify display only".into())
        .unwrap();
    assert!(b.evidence_staleness(evidence, &context).unwrap().is_empty());
    b.edit_field(a, "statement", "Source accepted statement changed".into())
        .unwrap();
    assert_eq!(b.register.requirements[&c], consumer);
    assert_eq!(b.register.evidence[&evidence], old);
    let report = b.coverage(&e, &config(), Some(a)).unwrap();
    assert!(
        report.evidence[0]
            .reasons
            .contains(&"requirement dependency context changed".to_string())
    );
    assert_eq!(report.potential_requirements, vec![a, c]);
    assert_eq!(report.potential_evidence, vec![evidence]);
    assert_eq!(
        b.register.requirements[&c].verification,
        VerificationStatus::NotStarted
    );
}
#[test]
fn transitive_supplier_parent_and_cycle_context_are_iterative_and_conservative() {
    let (mut b, e, a, c) = setup();
    let g = b
        .add_requirement_under("Grandchild".into(), Some(c))
        .unwrap();
    b.add_trace(TraceKind::Derive, req(c), a, "".into(), &e.snapshot)
        .unwrap();
    b.add_trace(
        TraceKind::Refine,
        req(a),
        c,
        "explicit cyclic declaration".into(),
        &e.snapshot,
    )
    .unwrap();
    let evidence = b
        .attach_calculation(g, &e, &config(), CalculationId(1003))
        .unwrap();
    let old = b.register.requirements[&g].clone();
    b.edit_field(a, "rationale", "Changed upstream context".into())
        .unwrap();
    let report = b.coverage(&e, &config(), Some(a)).unwrap();
    assert_eq!(report.potential_requirements, vec![a, c, g]);
    assert!(
        report.evidence[0]
            .reasons
            .iter()
            .any(|r| r.contains("dependency context changed"))
    );
    assert_eq!(report.potential_evidence, vec![evidence]);
    assert_eq!(b.register.requirements[&g], old);
}
#[test]
fn unknown_register_native_and_anchor_locations_are_disclosed_and_retained() {
    let (mut b, mut e, a, _) = setup();
    b.register.unknown.insert(
        "future.trace/1".into(),
        json!({"uninterpretedRelation":"allocate?"}),
    );
    b.register
        .requirements
        .get_mut(&a)
        .unwrap()
        .unknown
        .insert("futureRequirementLink".into(), json!([999]));
    e.snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert("future.native-relations/1".into(), json!({"opaque":[1,2]}));
    let before = b.register.clone();
    let native = e.snapshot.clone();
    let report = b.coverage(&e, &config(), Some(a)).unwrap();
    assert_eq!(report.unsupported_count, 3);
    assert!(
        report
            .unsupported_locations
            .iter()
            .any(|p| p.contains("future.trace/1"))
    );
    assert!(
        report
            .unsupported_locations
            .iter()
            .any(|p| p.contains("future.native-relations/1"))
    );
    assert_eq!(b.register, before);
    assert_eq!(e.snapshot, native);
}
#[test]
fn legacy_evidence_without_context_is_not_silently_qualified_after_dependencies_are_added() {
    let (mut b, e, a, c) = setup();
    let evidence = b
        .attach_calculation(c, &e, &config(), CalculationId(1003))
        .unwrap();
    assert!(b.register.evidence[&evidence].dependency_hash.is_none());
    let wire = serde_json::to_value(&b.register.evidence[&evidence]).unwrap();
    assert!(wire.get("dependency_hash").is_none());
    b.add_trace(TraceKind::Derive, req(c), a, "".into(), &e.snapshot)
        .unwrap();
    let report = b.coverage(&e, &config(), Some(a)).unwrap();
    assert!(
        report.evidence[0]
            .reasons
            .contains(&"requirement dependency context was not captured".to_string())
    );
    assert!(b.register.evidence[&evidence].dependency_hash.is_none());
}
#[test]
fn dependency_context_and_stale_reasons_survive_save_cold_open_without_runtime_authentication() {
    let (mut b, e, a, c) = setup();
    b.add_trace(TraceKind::Derive, req(c), a, "".into(), &e.snapshot)
        .unwrap();
    let evidence = b
        .attach_calculation(c, &e, &config(), CalculationId(1003))
        .unwrap();
    b.edit_field(a, "statement", "Changed source".into())
        .unwrap();
    let dir = area();
    let path = dir.path().join("coverage.json");
    b.save_new(&path).unwrap();
    let (cold, _) = Board::open(&path).unwrap();
    assert_eq!(cold.register, b.register);
    let report = cold.coverage(&e, &config(), Some(a)).unwrap();
    let row = report
        .evidence
        .iter()
        .find(|r| r.evidence == evidence)
        .unwrap();
    assert!(
        row.reasons
            .contains(&"requirement dependency context changed".to_string())
    );
    assert!(row.reasons.iter().any(|r| r.contains("runtime proof")));
    assert!(!cold.verification_current(c, &EvidenceContext::from_editor(&e, &config()).unwrap()));
}
#[test]
fn dependency_hash_format_refuses_without_changing_original_import_bytes() {
    let (mut b, e, a, c) = setup();
    b.add_trace(TraceKind::Derive, req(c), a, "".into(), &e.snapshot)
        .unwrap();
    let evidence = b
        .attach_calculation(c, &e, &config(), CalculationId(1003))
        .unwrap();
    let mut wire = json!({"profile":PROFILE,"register":b.register});
    wire["register"]["evidence"][evidence.0.to_string()]["dependency_hash"] =
        json!("unsupported stamp");
    let dir = area();
    let path = dir.path().join("bad-context.json");
    let bytes = serde_json::to_vec(&wire).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert!(Board::open(&path).is_err());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
#[test]
fn coverage_query_has_explicit_evidence_and_unknown_detail_budgets() {
    let (mut b, e, a, _) = setup();
    let first = b
        .attach_calculation(a, &e, &config(), CalculationId(1003))
        .unwrap();
    let template = b.register.evidence[&first].clone();
    let mut next = b.register.next_id;
    for _ in 0..256 {
        let mut copy = template.clone();
        copy.id = EvidenceId(next);
        b.register.evidence.insert(copy.id, copy);
        next += 1;
    }
    b.register.next_id = next;
    for i in 0..200 {
        b.register
            .unknown
            .insert(format!("future-{i}"), json!({"opaque":true}));
    }
    let before = b.register.clone();
    let report = b.coverage(&e, &config(), None).unwrap();
    assert_eq!(
        (
            report.evidence_total,
            report.evidence.len(),
            report.evidence_unexamined
        ),
        (257, 256, 1)
    );
    assert_eq!(report.unsupported_count, 200);
    assert_eq!(report.unsupported_locations.len(), 128);
    assert_eq!(b.register, before);
}
#[test]
fn query_is_read_only_and_exact_stamp_detects_same_revision_different_register() {
    let (mut b, e, a, c) = setup();
    b.add_trace(TraceKind::Derive, req(c), a, "".into(), &e.snapshot)
        .unwrap();
    b.set_trace_draft("source", "native:1021?".into()).unwrap();
    let before = b.register.clone();
    let native = e.snapshot.clone();
    let first = b.coverage(&e, &config(), Some(a)).unwrap();
    assert_eq!(b.register, before);
    assert_eq!(e.snapshot, native);
    assert!(b.coverage(&e, &config(), Some(RequirementId(999))).is_err());
    assert_eq!(b.register, before);
    b.register.requirements.get_mut(&a).unwrap().rationale =
        "Different accepted bytes without revision bump".into();
    let second = b.coverage(&e, &config(), Some(a)).unwrap();
    assert_ne!(first.register_sha256, second.register_sha256);
    assert_eq!(first.native_sha256, second.native_sha256);
}
