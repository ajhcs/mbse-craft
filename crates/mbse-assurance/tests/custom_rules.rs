//! Bounded declarative rules and exact-source quick-fix contracts.
use mbse_assurance::{validation::*, *};
use mbse_model::stage1::{Editor, RecordId};
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn setup() -> (Board, Editor) {
    let v: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p27a/fixtures/native.kernspan"
    ))
    .unwrap();
    let e = Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "custom:runtime".into(),
    )
    .unwrap();
    let mut b = Board::new();
    b.add_identified_requirement("Original requirement".into(), "R-1".into(), None)
        .unwrap();
    b.add_identified_requirement("Other requirement".into(), "R-2".into(), None)
        .unwrap();
    (b, e)
}
fn rule(field: TextField) -> CustomRule {
    CustomRule {
        id: "naming-1".into(),
        requirement: RequirementId(1),
        field,
        predicate: TextPredicate::StartsWith {
            prefix: "REQ-".into(),
        },
        replacement: Some("REQ-1".into()),
    }
}
fn suite(r: CustomRule) -> Suite {
    Suite {
        rules: BTreeSet::new(),
        custom: vec![r],
    }
}
fn admit(b: &mut Board, s: Suite) {
    b.set_validation_form("Reviewed local".into(), serde_json::to_string(&s).unwrap())
        .unwrap();
    let review = b.preview_validation_suite().unwrap();
    assert_eq!(review.suite(), &s);
    b.admit_validation_review(&review).unwrap();
}
fn first(b: &Board, e: &Editor) -> (Report, String) {
    let r = b.run_validation("Reviewed local", e).unwrap();
    let id = r.issues()[0].id.clone();
    (r, id)
}
#[test]
fn custom_suite_requires_explicit_exact_proposal_review_and_refuses_changed_source() {
    let (mut b, _) = setup();
    let s = suite(rule(TextField::Identifier));
    let old = b.register.clone();
    assert_eq!(
        b.set_validation_suite("Reviewed local".into(), s.clone())
            .unwrap_err(),
        "CUSTOM_RULE_REVIEW_REQUIRED"
    );
    assert_eq!(b.register, old);
    b.set_validation_form("Reviewed local".into(), serde_json::to_string(&s).unwrap())
        .unwrap();
    assert_eq!(
        b.apply_validation_form().unwrap_err(),
        "CUSTOM_RULE_REVIEW_REQUIRED"
    );
    let review = b.preview_validation_suite().unwrap();
    b.edit_field(RequirementId(2), "rationale", "New review source".into())
        .unwrap();
    let before = b.register.clone();
    assert_eq!(
        b.admit_validation_review(&review).unwrap_err(),
        "CUSTOM_RULE_REVIEW_STALE"
    );
    assert_eq!(b.register, before);
    let fresh = b.preview_validation_suite().unwrap();
    b.admit_validation_review(&fresh).unwrap();
    assert_eq!(b.register.validation_suites["Reviewed local"], s);
}
#[test]
fn custom_run_stable_exact_requirement_location_is_read_only_and_missing_scope_disclosed() {
    let (mut b, e) = setup();
    let mut s = suite(rule(TextField::Identifier));
    let mut missing = rule(TextField::Statement);
    missing.id = "missing".into();
    missing.requirement = RequirementId(999);
    s.custom.push(missing);
    admit(&mut b, s);
    let old = b.register.clone();
    let native = e.snapshot.clone();
    let (r, id) = first(&b, &e);
    assert_eq!(id, "MBSE.CUSTOM.naming-1:requirement1:Identifier");
    assert_eq!(r.issue_total(), 1);
    assert!(r.issues()[0].quick_fix);
    assert!(r.limitations().iter().any(|l| l.contains("#999 missing")));
    assert_eq!(
        r.navigate(&b, &e, &id).unwrap().resolve(&b, &e).unwrap(),
        Target::Requirement {
            requirement: RequirementId(1),
            field: RequirementField::Identifier
        }
    );
    assert_eq!(b.register, old);
    assert_eq!(e.snapshot, native);
    assert!(b.register.decisions.is_empty());
}
#[test]
fn literal_fix_preview_apply_and_single_undo_redo_preserve_native_opaque_and_lifecycle() {
    let (mut b, e) = setup();
    b.register
        .unknown
        .insert("future/1".into(), json!({"script":"remains inert data"}));
    admit(&mut b, suite(rule(TextField::Identifier)));
    let old = b.register.clone();
    let native = e.snapshot.clone();
    let (r, id) = first(&b, &e);
    let p = r.preview_quick_fix(&b, &e, &id).unwrap();
    assert_eq!(b.register, old);
    assert_eq!(p.changes().len(), 2);
    assert!(p.changes().iter().any(|c| c.field == "identifier"
        && c.before.as_deref() == Some("R-1")
        && c.after.as_deref() == Some("REQ-1")));
    b.apply_quick_fix(&e, &p).unwrap();
    let after = b.register.clone();
    assert_eq!(after.revision, old.revision + 1);
    assert_eq!(
        after.requirements[&RequirementId(1)].maturity,
        old.requirements[&RequirementId(1)].maturity
    );
    assert_eq!(after.unknown, old.unknown);
    assert_eq!(e.snapshot, native);
    assert_eq!(
        b.run_validation("Reviewed local", &e)
            .unwrap()
            .issue_total(),
        0
    );
    assert!(b.undo());
    assert_eq!(b.register, old);
    assert!(b.redo());
    assert_eq!(b.register, after);
    assert!(b.apply_quick_fix(&e, &p).is_err());
}
#[test]
fn duplicate_identifier_copy_slave_and_invalid_statement_fixes_refuse_without_raw_or_history_change()
 {
    for field in [TextField::Identifier, TextField::Statement] {
        let (mut b, e) = setup();
        let mut r = rule(field);
        r.predicate = TextPredicate::Equals {
            value: if field == TextField::Identifier {
                "R-2".into()
            } else {
                " ".into()
            },
        };
        r.replacement = Some(if field == TextField::Identifier {
            "R-2".into()
        } else {
            " ".into()
        });
        admit(&mut b, suite(r));
        let before = b.register.clone();
        let (report, id) = first(&b, &e);
        assert_eq!(
            report.preview_quick_fix(&b, &e, &id).unwrap_err(),
            "CUSTOM_FIX_REFUSED_BY_EXISTING_AUTHORITY"
        );
        assert_eq!(b.register, before);
        assert!(b.undo());
        assert!(!b.register.validation_suites.contains_key("Reviewed local"));
    }
    let (mut b, e) = setup();
    b.edit_field(RequirementId(2), "statement", "Original requirement".into())
        .unwrap();
    b.add_trace(
        TraceKind::Copy,
        TraceEndpoint::Requirement(RequirementId(2)),
        RequirementId(1),
        "Copy".into(),
        &e.snapshot,
    )
    .unwrap();
    let mut r = rule(TextField::Statement);
    r.requirement = RequirementId(2);
    admit(&mut b, suite(r));
    let old = b.register.clone();
    let (report, id) = first(&b, &e);
    assert!(report.preview_quick_fix(&b, &e, &id).is_err());
    assert_eq!(b.register, old);
}
#[test]
fn pending_target_draft_refuses_fix_and_master_copy_propagation_is_fully_previewed() {
    let (mut b, e) = setup();
    b.edit_field(RequirementId(2), "statement", "Original requirement".into())
        .unwrap();
    b.add_trace(
        TraceKind::Copy,
        TraceEndpoint::Requirement(RequirementId(2)),
        RequirementId(1),
        "Copy".into(),
        &e.snapshot,
    )
    .unwrap();
    b.edit_field(
        RequirementId(2),
        "statement",
        "Retained slave proposal".into(),
    )
    .unwrap();
    admit(&mut b, suite(rule(TextField::Statement)));
    let old = b.register.clone();
    let (report, id) = first(&b, &e);
    let p = report.preview_quick_fix(&b, &e, &id).unwrap();
    assert_eq!(
        p.changes()
            .iter()
            .filter(|c| c.field == "statement")
            .count(),
        2
    );
    b.apply_quick_fix(&e, &p).unwrap();
    assert_eq!(b.register.drafts["2:statement"], old.drafts["2:statement"]);
    assert_eq!(
        b.register.requirements[&RequirementId(2)].statement,
        "REQ-1"
    );
    assert!(b.undo());
    assert_eq!(b.register, old);
    b.edit_field(RequirementId(1), "statement", "  ".into())
        .unwrap();
    let before = b.register.clone();
    let (r, id) = first(&b, &e);
    assert_eq!(
        r.preview_quick_fix(&b, &e, &id).unwrap_err(),
        "CUSTOM_FIX_PENDING_DRAFT_PRESERVED"
    );
    assert_eq!(b.register, before);
}
#[test]
fn fix_proof_rejects_new_raw_same_counters_new_runtime_and_allows_selection_only() {
    let (mut b, mut e) = setup();
    admit(&mut b, suite(rule(TextField::Identifier)));
    let (r, id) = first(&b, &e);
    let p = r.preview_quick_fix(&b, &e, &id).unwrap();
    e.select(RecordId(1003)).unwrap();
    p.check(&b, &e).unwrap();
    let other = Editor::new(e.snapshot.clone(), "cold:new-instance".into()).unwrap();
    let old = b.register.clone();
    assert!(b.apply_quick_fix(&other, &p).is_err());
    assert_eq!(b.register, old);
    e.snapshot.records.get_mut(&RecordId(1003)).unwrap().name =
        "Different bytes same counters".into();
    assert!(b.apply_quick_fix(&e, &p).is_err());
    assert_eq!(b.register, old);
    let (_, e) = setup();
    b.register
        .requirements
        .get_mut(&RequirementId(2))
        .unwrap()
        .rationale = "Different register same counters".into();
    let before = b.register.clone();
    assert!(b.apply_quick_fix(&e, &p).is_err());
    assert_eq!(b.register, before);
}
#[test]
fn custom_rule_grammar_rejects_unsupported_code_fields_duplicate_ids_and_limits() {
    let (mut b, _) = setup();
    for raw in [
        json!({"rules":[],"custom":[{"id":"x","requirement":1,"field":"Identifier","predicate":{"ExecuteShell":{"command":"never"}}}]}),
        json!({"rules":[],"custom":[{"id":"x","requirement":1,"field":"Identifier","predicate":"NonBlank","script":"never"}]}),
        json!({"rules":[],"custom":[{"id":"x","requirement":1,"field":"Parent","predicate":"NonBlank"}]}),
    ] {
        b.set_validation_form("Rejected".into(), raw.to_string())
            .unwrap();
        let old = b.register.clone();
        assert!(b.preview_validation_suite().is_err());
        assert_eq!(b.register, old);
    }
    let mut bads = vec![];
    let mut r = rule(TextField::Identifier);
    r.replacement = Some("WRONG".into());
    bads.push(suite(r));
    let mut s = suite(rule(TextField::Identifier));
    s.custom.push(s.custom[0].clone());
    bads.push(s);
    let mut s = suite(rule(TextField::Identifier));
    s.custom = vec![s.custom[0].clone(); 33];
    bads.push(s);
    let mut r = rule(TextField::Identifier);
    r.id = "unsupported.id".into();
    bads.push(suite(r));
    for s in bads {
        b.set_validation_form("Rejected".into(), serde_json::to_string(&s).unwrap())
            .unwrap();
        let old = b.register.clone();
        assert!(b.preview_validation_suite().is_err());
        assert_eq!(b.register, old);
    }
}
#[test]
fn saved_custom_data_stays_inert_cold_and_cannot_restore_review_or_fix_authority() {
    let (mut b, e) = setup();
    admit(&mut b, suite(rule(TextField::Identifier)));
    let (report, id) = first(&b, &e);
    let preview = report.preview_quick_fix(&b, &e, &id).unwrap();
    let dir = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let path = dir.path().join("register.json");
    b.save_new(&path).unwrap();
    let (cold, _) = Board::open(&path).unwrap();
    assert_eq!(cold.register, b.register);
    assert_eq!(
        cold.register.requirements[&RequirementId(1)].identifier,
        "R-1"
    );
    assert!(cold.register.decisions.is_empty());
    let native = Editor::new(e.snapshot.clone(), "cold".into()).unwrap();
    assert!(preview.check(&cold, &native).is_err());
    assert!(
        !serde_json::to_string(&preview)
            .unwrap()
            .contains("after\":{\"")
    );
    let fresh = cold.run_validation("Reviewed local", &native).unwrap();
    assert_eq!(fresh.issues()[0].id, id);
    assert!(fresh.preview_quick_fix(&cold, &native, &id).is_ok());
}
