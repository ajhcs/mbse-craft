//! Original synthetic assertions for local typed dependencies, not normative engine conformance.
use mbse_assurance::{traces::*, *};
use mbse_model::stage1::Snapshot;
use serde_json::{Value, json};
fn snapshot() -> Snapshot {
    let v: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p23/fixtures/profile-tags.kernspan"
    ))
    .unwrap();
    serde_json::from_value(v["snapshot"].clone()).unwrap()
}
fn setup() -> (Board, RequirementId, RequirementId, Snapshot) {
    let mut b = Board::new();
    let a = b
        .add_identified_requirement("Master shall retain source".into(), "R-1".into(), None)
        .unwrap();
    let c = b
        .add_identified_requirement("Master shall retain source".into(), "R-2".into(), Some(a))
        .unwrap();
    (b, a, c, snapshot())
}
fn area() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap()
}
fn req(id: RequirementId) -> TraceEndpoint {
    TraceEndpoint::Requirement(id)
}
#[test]
fn five_distinct_kinds_enforce_declared_endpoint_types_and_do_not_change_status() {
    let (mut b, a, c, s) = setup();
    let case = b
        .add_test_case(
            "TC-1".into(),
            "Inspect capture".into(),
            "Source stays exact".into(),
        )
        .unwrap();
    let status = b.register.requirements.clone();
    let endpoints = [
        (TraceKind::Derive, req(c)),
        (TraceKind::Refine, native_endpoint(&s, 1002).unwrap()),
        (TraceKind::Satisfy, native_endpoint(&s, 1021).unwrap()),
        (TraceKind::Verify, TraceEndpoint::TestCase(case)),
        (TraceKind::Copy, req(c)),
    ];
    for (kind, source) in endpoints {
        b.add_trace(kind, source, a, "same misleading display text".into(), &s)
            .unwrap();
    }
    assert_eq!(b.register.requirements, status);
    assert_eq!(b.register.traces.len(), 5);
    assert!(b.trace_projection(&s).iter().all(|p| p.active));
    assert!(b.register.decisions.is_empty());
    assert!(b.register.evidence.is_empty());
    for (kind, source) in [
        (TraceKind::Derive, native_endpoint(&s, 1021).unwrap()),
        (TraceKind::Refine, TraceEndpoint::TestCase(case)),
        (TraceKind::Satisfy, req(c)),
        (TraceKind::Satisfy, native_endpoint(&s, 1003).unwrap()),
        (TraceKind::Verify, req(c)),
        (TraceKind::Verify, native_endpoint(&s, 1021).unwrap()),
        (TraceKind::Copy, TraceEndpoint::TestCase(case)),
    ] {
        let before = b.register.clone();
        assert!(b.add_trace(kind, source, a, "".into(), &s).is_err());
        assert_eq!(b.register, before);
    }
}
#[test]
fn display_rename_cannot_retype_retarget_or_claim_passed_and_duplicates_are_atomic() {
    let (mut b, a, c, s) = setup();
    let id = b
        .add_trace(TraceKind::Derive, req(c), a, "satisfy".into(), &s)
        .unwrap();
    let old = b.register.traces[&id].clone();
    b.rename_trace(id, "verify CompletedPassed".into()).unwrap();
    let current = &b.register.traces[&id];
    assert_eq!(
        (current.kind, &current.source, current.target),
        (old.kind, &old.source, old.target)
    );
    assert_eq!(
        b.register.requirements[&a].verification,
        VerificationStatus::NotStarted
    );
    let before = b.register.clone();
    assert!(
        b.add_trace(TraceKind::Derive, req(c), a, "different label".into(), &s)
            .unwrap_err()
            .contains("DUPLICATE")
    );
    assert_eq!(b.register, before);
    assert!(b.rename_trace(id, "x".repeat(513)).is_err());
    assert_eq!(b.register, before);
}
#[test]
fn copy_statement_is_read_only_and_master_edit_propagates_transitively_preserving_invalid_raw() {
    let (mut b, a, c, s) = setup();
    let g = b
        .add_identified_requirement("Master shall retain source".into(), "R-3".into(), None)
        .unwrap();
    b.add_trace(TraceKind::Copy, req(c), a, "copy".into(), &s)
        .unwrap();
    b.add_trace(TraceKind::Copy, req(g), c, "nested".into(), &s)
        .unwrap();
    let accepted = b.register.requirements.clone();
    assert!(
        !b.edit_field(c, "statement", "Local override attempt".into())
            .unwrap()
    );
    assert_eq!(b.register.requirements, accepted);
    assert_eq!(
        b.register.drafts["2:statement"],
        RawDraft {
            text: "Local override attempt".into(),
            valid: false
        }
    );
    assert!(
        b.edit_field(a, "statement", "Master changed exact statement".into())
            .unwrap()
    );
    for id in [a, c, g] {
        assert_eq!(
            b.register.requirements[&id].statement,
            "Master changed exact statement"
        );
        assert_eq!(b.register.requirements[&id].revision, 1);
        assert_eq!(
            b.register.requirements[&id].verification,
            VerificationStatus::NotStarted
        );
    }
    assert_eq!(
        b.register.drafts["2:statement"].text,
        "Local override attempt"
    );
    assert!(!b.register.drafts["2:statement"].valid);
    let after = b.register.clone();
    assert!(b.undo());
    assert_eq!(b.register.requirements, accepted);
    assert!(b.redo());
    assert_eq!(b.register, after);
}
#[test]
fn copy_multiple_masters_cycles_and_unequal_text_refuse_then_removal_releases_slave() {
    let (mut b, a, c, s) = setup();
    let g = b.add_requirement("Other text".into()).unwrap();
    let before = b.register.clone();
    assert!(
        b.add_trace(TraceKind::Copy, req(g), a, "".into(), &s)
            .is_err()
    );
    assert_eq!(b.register, before);
    let id = b
        .add_trace(TraceKind::Copy, req(c), a, "".into(), &s)
        .unwrap();
    let before = b.register.clone();
    assert!(
        b.add_trace(TraceKind::Copy, req(a), c, "".into(), &s)
            .is_err()
    );
    assert_eq!(b.register, before);
    let d = b
        .add_requirement("Master shall retain source".into())
        .unwrap();
    let before = b.register.clone();
    assert!(
        b.add_trace(TraceKind::Copy, req(c), d, "".into(), &s)
            .is_err()
    );
    assert_eq!(b.register, before);
    b.remove_trace(id).unwrap();
    assert!(
        b.edit_field(c, "statement", "Released independent statement".into())
            .unwrap()
    );
    assert_eq!(
        b.register.requirements[&a].statement,
        "Master shall retain source"
    );
    assert!(b.undo());
    assert!(b.undo());
    assert_eq!(b.copy_master(c), Some(a));
}
#[test]
fn native_identity_document_and_kind_mismatch_detach_without_retargeting() {
    let (mut b, a, _, s) = setup();
    let endpoint = native_endpoint(&s, 1021).unwrap();
    let id = b
        .add_trace(TraceKind::Satisfy, endpoint.clone(), a, "".into(), &s)
        .unwrap();
    let before = b.register.clone();
    let mut other = s.clone();
    other.document_id.0 = "other-document".into();
    assert!(!b.trace_projection(&other)[0].active);
    assert!(
        b.add_trace(TraceKind::Satisfy, endpoint, a, "".into(), &other)
            .is_err()
    );
    assert_eq!(b.register, before);
    let mut missing = s.clone();
    missing
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .get_mut(mbse_model::types::NAMESPACE)
        .unwrap()["definitions"]
        .as_object_mut()
        .unwrap()
        .remove("1021");
    assert!(!b.trace_projection(&missing)[0].active);
    assert!(b.register.traces.contains_key(&id));
    let mut fake = native_endpoint(&s, 1021).unwrap();
    if let TraceEndpoint::Native(n) = &mut fake {
        n.expected = mbse_model::types::tags::TargetKind::Part;
    }
    assert!(
        b.add_trace(TraceKind::Satisfy, fake, a, "".into(), &s)
            .is_err()
    );
    assert_eq!(b.register, before);
}
#[test]
fn dependency_history_save_and_cold_reopen_keep_exact_types_endpoints_drafts_and_unknown() {
    let (mut b, a, c, s) = setup();
    b.set_trace_draft("source", "native:1021?".into()).unwrap();
    b.set_trace_draft("label", "verify".into()).unwrap();
    let id = b
        .add_trace(TraceKind::Copy, req(c), a, "satisfy".into(), &s)
        .unwrap();
    b.register
        .unknown
        .insert("future".into(), json!({"opaque":[null,1]}));
    b.edit_field(c, "statement", "invalid slave proposal".into())
        .unwrap();
    let before = b.register.clone();
    b.remove_trace(id).unwrap();
    assert!(b.undo());
    assert_eq!(b.register, before);
    let dir = area();
    let path = dir.path().join("traces.json");
    b.save_new(&path).unwrap();
    let (mut cold, _) = Board::open(&path).unwrap();
    assert_eq!(cold.register, b.register);
    assert!(!cold.undo());
    assert_eq!(cold.copy_master(c), Some(a));
    assert!(cold.trace_projection(&s).iter().all(|p| p.active));
    assert_eq!(cold.register.drafts["trace:source"].text, "native:1021?");
}
#[test]
fn malformed_typed_trace_maps_fields_copy_text_and_shared_id_collisions_refuse() {
    let (mut b, a, c, s) = setup();
    let id = b
        .add_trace(TraceKind::Copy, req(c), a, "".into(), &s)
        .unwrap();
    let dir = area();
    let good = json!({"profile":PROFILE,"register":b.register});
    for mode in 0..7 {
        let mut v = good.clone();
        match mode {
            0 => {
                v["register"]["traces"][format!("0{id}")] =
                    v["register"]["traces"][id.to_string()].clone();
            }
            1 => v["register"]["traces"][id.to_string()]["kind"] = json!("CustomVerify"),
            2 => v["register"]["traces"][id.to_string()]["future"] = json!(true),
            3 => v["register"]["traces"][id.to_string()]["target"] = json!(999),
            4 => {
                v["register"]["requirements"][c.0.to_string()]["statement"] =
                    json!("Forged mismatched Copy")
            }
            5 => {
                let trace = v["register"]["traces"][id.to_string()].clone();
                v["register"]["traces"].as_object_mut().unwrap().clear();
                v["register"]["traces"][a.0.to_string()] = trace;
                v["register"]["traces"][a.0.to_string()]["id"] = json!(a.0);
            }
            _ => v["register"]["traces"][id.to_string()]["source"] = json!({"TestCase":999}),
        }
        let path = dir.path().join(format!("bad-{mode}.json"));
        let bytes = serde_json::to_vec(&v).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(Board::open(&path).is_err(), "mode {mode}");
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
#[test]
fn documentary_case_is_separate_from_evidence_proof_and_ids_never_recycle() {
    let (mut b, a, _, s) = setup();
    let before = b.register.clone();
    assert!(
        b.add_test_case("TC".into(), " ".into(), "Result".into())
            .is_err()
    );
    assert_eq!(b.register, before);
    let id = b
        .add_test_case("TC-1".into(), "Inspect bytes".into(), "Exact source".into())
        .unwrap();
    let link = b
        .add_trace(
            TraceKind::Verify,
            TraceEndpoint::TestCase(id),
            a,
            "".into(),
            &s,
        )
        .unwrap();
    assert!(link > id);
    assert!(
        b.register.evidence.is_empty()
            && b.register.decisions.is_empty()
            && b.register.decision_history.is_empty()
    );
    assert!(b.undo());
    assert!(b.undo());
    assert!(!b.register.test_cases.contains_key(&id));
    let next = b
        .add_test_case("TC-2".into(), "Inspect bytes".into(), "Exact source".into())
        .unwrap();
    assert!(next > link);
}
#[test]
fn copy_master_propagation_stales_existing_slave_evidence_without_fabricating_verification() {
    use mbse_model::{stage1::Editor, stage1_calc::CalculationId};
    let (mut b, a, c, s) = setup();
    b.add_trace(TraceKind::Copy, req(c), a, "".into(), &s)
        .unwrap();
    let e = Editor::new(s, "p24:evidence".into()).unwrap();
    let config = json!({"purpose":"synthetic Copy propagation"});
    let evidence = b
        .attach_calculation(c, &e, &config, CalculationId(1003))
        .unwrap();
    let ctx = EvidenceContext::from_editor(&e, &config).unwrap();
    assert!(
        !b.evidence_staleness(evidence, &ctx)
            .unwrap()
            .contains(&"requirement changed".to_string())
    );
    b.edit_field(a, "statement", "New master text".into())
        .unwrap();
    assert!(
        b.evidence_staleness(evidence, &ctx)
            .unwrap()
            .contains(&"requirement changed".to_string())
    );
    assert_eq!(
        b.register.requirements[&c].verification,
        VerificationStatus::NotStarted
    );
}
#[test]
fn endpoint_proposals_require_explicit_canonical_prefix_and_wrong_fields_preserve_work() {
    let (mut b, _, _, s) = setup();
    for raw in ["2", "req:02", "test:0", "native:1021?", "label:verify"] {
        assert!(parse_endpoint(raw, &s).is_err());
    }
    assert!(matches!(
        parse_endpoint("req:2", &s).unwrap(),
        TraceEndpoint::Requirement(RequirementId(2))
    ));
    let before = b.register.clone();
    assert!(b.set_trace_draft("kind", "Verify".into()).is_err());
    assert!(b.set_trace_draft("source", "x".repeat(16385)).is_err());
    assert_eq!(b.register, before);
}
