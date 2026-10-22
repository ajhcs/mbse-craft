//! Original MIT synthetic requirements and calculations; no external execution.
use mbse_assurance::*;
use mbse_model::{
    stage1::{Command, DocumentId, Editor, RecordId, Snapshot},
    stage1_calc::{
        self as calc, CalculationId, Definition, Input, Literal, QuantityKind, QuantityType, UnitId,
    },
};
use serde_json::json;
use std::collections::BTreeMap;
fn model() -> Editor {
    let mut e=Editor::new(Snapshot::from_fixture(DocumentId("synthetic".into()),&json!({"records":[{"id":1,"type":"System","name":"Synthetic","scope":1},{"id":10,"type":"Objective","name":"Motion","scope":1},{"id":11,"type":"Measure","name":"Speed","scope":1,"forObjective":10}]})).unwrap(),"instance".into()).unwrap();
    let literal = |decimal: &str, kind, unit| {
        Input::Literal(Literal {
            decimal: decimal.into(),
            quantity: QuantityType { kind, unit },
        })
    };
    let definition = Definition {
        id: CalculationId(11),
        owner: RecordId(11),
        bindings: BTreeMap::from([
            (
                "distance".into(),
                literal("120", QuantityKind::Length, UnitId::Metre),
            ),
            (
                "elapsed".into(),
                literal("8", QuantityKind::Duration, UnitId::Second),
            ),
        ]),
        expression: calc::parse("distance / elapsed", &calc::Limits::default()).unwrap(),
        output: QuantityType {
            kind: QuantityKind::Speed,
            unit: UnitId::MetrePerSecond,
        },
    };
    e.command(
        &e.freshness(),
        Command::PutCalculation {
            measure: RecordId(11),
            definition: Some(definition),
        },
    )
    .unwrap();
    e
}
fn area() -> tempfile::TempDir {
    let root = std::env::var_os("TMPDIR").expect("workspace TMPDIR");
    assert!(std::path::Path::new(&root).starts_with("/workspace"));
    tempfile::tempdir_in(root).unwrap()
}
#[test]
fn lifecycle_dimensions_independent_and_calculation_result_is_not_approval() {
    let e = model();
    let mut b = Board::new();
    let req = b
        .add_requirement("Synthetic speed evidence shall be reviewable".into())
        .unwrap();
    b.set_maturity(req, Maturity::Approved).unwrap();
    assert_eq!(
        b.register.requirements[&req].implementation,
        ImplementationStatus::UnderReview
    );
    assert_eq!(
        b.register.requirements[&req].verification,
        VerificationStatus::NotStarted
    );
    b.set_implementation(req, ImplementationStatus::Implemented)
        .unwrap();
    assert_eq!(
        b.register.requirements[&req].verification,
        VerificationStatus::NotStarted
    );
    let config = json!({"purpose":"synthetic"});
    let id = b
        .attach_calculation(req, &e, &config, CalculationId(11))
        .unwrap();
    assert_eq!(
        b.register.evidence[&id]
            .selected_quantity
            .as_ref()
            .unwrap()
            .value,
        15.0
    );
    assert_eq!(
        b.register.evidence[&id]
            .selected_quantity
            .as_ref()
            .unwrap()
            .quantity
            .unit,
        UnitId::MetrePerSecond
    );
    assert_eq!(b.register.evidence[&id].adequacy, Adequacy::NotReviewed);
    let context = EvidenceContext::from_editor(&e, &config).unwrap();
    let before = b.register.clone();
    assert!(b
        .record_verification(
            req,
            id,
            VerificationStatus::CompletedPassed,
            &context,
            "Reviewer".into(),
            "Calculation only".into()
        )
        .is_err());
    assert_eq!(b.register, before);
    assert!(b
        .review_evidence(id, Adequacy::Adequate, "".into(), "".into())
        .is_err());
    b.review_evidence(
        id,
        Adequacy::Adequate,
        "Reviewer".into(),
        "Synthetic reviewed evidence supports this declared criterion".into(),
    )
    .unwrap();
    b.record_verification(
        req,
        id,
        VerificationStatus::CompletedPassed,
        &context,
        "Reviewer".into(),
        "Explicit application decision, no engineering authority inferred".into(),
    )
    .unwrap();
    assert!(b.verification_current(req, &context));
}
#[test]
fn every_freshness_dimension_and_incomplete_raw_refuses_pass() {
    let e = model();
    let mut b = Board::new();
    let req = b
        .add_requirement("Review synthetic evidence".into())
        .unwrap();
    let config = json!({"adapter":"bounded"});
    let id = b
        .attach_calculation(req, &e, &config, CalculationId(11))
        .unwrap();
    let context = EvidenceContext::from_editor(&e, &config).unwrap();
    b.review_evidence(
        id,
        Adequacy::Adequate,
        "Reviewer".into(),
        "Reviewed mapping".into(),
    )
    .unwrap();
    let mut contexts = vec![];
    let mut c = context.clone();
    c.model.model_revision += 1;
    contexts.push(c);
    let mut c = context.clone();
    c.model.raw_version += 1;
    contexts.push(c);
    let mut c = context.clone();
    c.model.snapshot_hash.push('x');
    contexts.push(c);
    let mut c = context.clone();
    c.config_hash.push('x');
    contexts.push(c);
    let mut c = context.clone();
    c.input_hash.push('x');
    contexts.push(c);
    let mut c = context.clone();
    c.tool_version.push('x');
    contexts.push(c);
    for c in contexts {
        assert!(!b.evidence_staleness(id, &c).unwrap().is_empty());
        assert!(b
            .record_verification(
                req,
                id,
                VerificationStatus::CompletedPassed,
                &c,
                "R".into(),
                "Review".into()
            )
            .is_err());
    }
    let saved = b.clone();
    b.edit_field(req, "statement", "".into()).unwrap();
    assert!(!b.register.drafts[&format!("{}:statement", req.0)].valid);
    assert!(!b.evidence_staleness(id, &context).unwrap().is_empty());
    assert!(b
        .record_verification(
            req,
            id,
            VerificationStatus::CompletedPassed,
            &context,
            "R".into(),
            "Review".into()
        )
        .is_err());
    assert!(b.undo());
    assert_eq!(b.register, saved.register);
    b.edit_field(req, "statement", "Changed requirement".into())
        .unwrap();
    assert!(b
        .record_verification(
            req,
            id,
            VerificationStatus::CompletedPassed,
            &context,
            "R".into(),
            "Review".into()
        )
        .is_err());
    let mut b = saved;
    b.register
        .evidence
        .get_mut(&id)
        .unwrap()
        .artifact_bytes
        .push(1);
    assert!(b
        .record_verification(
            req,
            id,
            VerificationStatus::CompletedPassed,
            &context,
            "R".into(),
            "Review".into()
        )
        .is_err());
}
#[test]
fn unknown_metadata_invalid_raw_and_source_units_survive_native_sidecar_reopen() {
    let area = area();
    let path = area.path().join("requirements.json");
    let mut b = Board::new();
    let req = b
        .add_requirement("Original accepted statement".into())
        .unwrap();
    b.edit_field(req, "statement", "".into()).unwrap();
    b.register
        .unknown
        .insert("externalFuture".into(), json!({"opaque":[1,"unknown"]}));
    b.register
        .requirements
        .get_mut(&req)
        .unwrap()
        .unknown
        .insert("priorityFromSource".into(), json!("CustomPriority"));
    b.set_anchor(
        req,
        Some(SourceAnchor {
            format_contract: "mbse-craft.external.uml-xmi/1".into(),
            artifact_sha256: "synthetic source hash".into(),
            metamodel: "UML20131001".into(),
            source_element_id: "original-id".into(),
            native_document: Some("synthetic".into()),
            native_record: Some(11),
            units: BTreeMap::from([("speed".into(), "m/s source string".into())]),
            references: vec!["external#unit".into()],
            package_integrity: ProjectionStatus::Supported,
            parsing: ProjectionStatus::Supported,
            reference_resolution: ProjectionStatus::Partial,
            semantic_projection: ProjectionStatus::Partial,
            editable_admission: ProjectionStatus::Unsupported,
            execution: ProjectionStatus::Unknown,
            unknown: BTreeMap::new(),
        }),
    )
    .unwrap();
    let hash = b.save_new(&path).unwrap();
    assert!(b.save_new(&path).is_err());
    let (mut opened, read_hash) = Board::open(&path).unwrap();
    assert_eq!(hash, read_hash);
    assert_eq!(opened.register, b.register);
    opened.edit_field(req, "owner", "Reviewer".into()).unwrap();
    let prior = std::fs::read(&path).unwrap();
    assert!(opened.replace(&path, "wrong expected hash").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), prior);
    opened.replace(&path, &hash).unwrap();
    let (reopened, _) = Board::open(&path).unwrap();
    assert_eq!(reopened.register, opened.register);
    let bad = area.path().join("duplicate.json");
    std::fs::write(&bad, b"{\"profile\":\"a\",\"profile\":\"b\"}").unwrap();
    assert!(Board::open(&bad).is_err());
}
#[test]
fn failed_unsupported_and_changed_local_artifacts_cannot_pass() {
    let area = area();
    let artifact = area.path().join("evidence.txt");
    std::fs::write(&artifact, b"synthetic evidence").unwrap();
    let e = model();
    let context = EvidenceContext::from_editor(&e, &json!({})).unwrap();
    let mut b = Board::new();
    let req = b.add_requirement("Review source evidence".into()).unwrap();
    for outcome in [
        EvidenceOutcome::NotRun,
        EvidenceOutcome::Failed,
        EvidenceOutcome::Unsupported,
        EvidenceOutcome::Cancelled,
    ] {
        let id = b
            .attach_local_evidence(req, context.clone(), &artifact, outcome)
            .unwrap();
        b.review_evidence(id, Adequacy::Adequate, "R".into(), "Reviewed".into())
            .unwrap();
        assert!(b
            .record_verification(
                req,
                id,
                VerificationStatus::CompletedPassed,
                &context,
                "R".into(),
                "Review".into()
            )
            .is_err());
    }
    let id = b
        .attach_local_evidence(req, context.clone(), &artifact, EvidenceOutcome::Completed)
        .unwrap();
    b.review_evidence(id, Adequacy::Adequate, "R".into(), "Reviewed".into())
        .unwrap();
    std::fs::write(&artifact, b"changed artifact").unwrap();
    assert!(!b.evidence_staleness(id, &context).unwrap().is_empty());
    assert!(b
        .record_verification(
            req,
            id,
            VerificationStatus::CompletedPassed,
            &context,
            "R".into(),
            "Review".into()
        )
        .is_err());
    assert!(b
        .set_verification(req, VerificationStatus::CompletedPassed)
        .is_err());
}

#[test]
fn serialized_stamps_and_coordinated_output_tampering_never_mint_native_authenticity() {
    use sha2::{Digest, Sha256};
    let editor = model();
    let config = json!({});
    let context = EvidenceContext::from_editor(&editor, &config).unwrap();
    let mut b = Board::new();
    let req = b
        .add_requirement("Review selected speed result".into())
        .unwrap();
    let id = b
        .attach_calculation(req, &editor, &config, CalculationId(11))
        .unwrap();
    b.review_evidence(
        id,
        Adequacy::Adequate,
        "Reviewer".into(),
        "Original result reviewed".into(),
    )
    .unwrap();
    b.record_verification(
        req,
        id,
        VerificationStatus::CompletedPassed,
        &context,
        "Reviewer".into(),
        "Original decision".into(),
    )
    .unwrap();
    assert!(b.verification_current(req, &context));
    let area = area();
    let path = area.path().join("native-receipt.json");
    b.save_new(&path).unwrap();
    let (mut reopened, _) = Board::open(&path).unwrap();
    assert!(!reopened.verification_current(req, &context));
    assert!(reopened
        .evidence_staleness(id, &context)
        .unwrap()
        .iter()
        .any(|v| v.contains("runtime proof")));
    reopened
        .review_evidence(
            id,
            Adequacy::Adequate,
            "Reviewer".into(),
            "Re-review does not authenticate imported native bytes".into(),
        )
        .unwrap();
    assert!(reopened
        .record_verification(
            req,
            id,
            VerificationStatus::CompletedPassed,
            &context,
            "Reviewer".into(),
            "Refuse historical stamp only".into()
        )
        .is_err());
    let e = b.register.evidence.get_mut(&id).unwrap();
    let mut receipt: serde_json::Value = serde_json::from_slice(&e.artifact_bytes).unwrap();
    receipt["outcomes"]["11"]["Value"]["value"] = json!(999.0);
    e.artifact_bytes = serde_json::to_vec(&receipt).unwrap();
    e.artifact_sha256 = format!("{:x}", Sha256::digest(&e.artifact_bytes));
    e.selected_quantity.as_mut().unwrap().value = 999.0;
    b.review_evidence(
        id,
        Adequacy::Adequate,
        "Reviewer".into(),
        "Tampered values cannot mint a native receipt".into(),
    )
    .unwrap();
    assert!(b
        .record_verification(
            req,
            id,
            VerificationStatus::CompletedPassed,
            &context,
            "Reviewer".into(),
            "Reject forged result".into()
        )
        .is_err());
    assert!(!b.verification_current(req, &context));
}
#[test]
fn raw_9e_invalidates_delayed_evidence_and_audit_records_survive_later_reviews_and_undo() {
    let mut editor = model();
    let config = json!({});
    let context = EvidenceContext::from_editor(&editor, &config).unwrap();
    let mut b = Board::new();
    let req = b.add_requirement("Retain speed evidence".into()).unwrap();
    let id = b
        .attach_calculation(req, &editor, &config, CalculationId(11))
        .unwrap();
    b.review_evidence(
        id,
        Adequacy::Adequate,
        "Reviewer one".into(),
        "Initial adequacy".into(),
    )
    .unwrap();
    b.record_verification(
        req,
        id,
        VerificationStatus::CompletedPassed,
        &context,
        "Reviewer one".into(),
        "Initial verdict".into(),
    )
    .unwrap();
    let first_review = b.register.review_history[0].clone();
    let first_decision = b.register.decision_history[0].clone();
    b.review_evidence(
        id,
        Adequacy::Inadequate,
        "Reviewer two".into(),
        "Later contrary review".into(),
    )
    .unwrap();
    assert_eq!(b.register.review_history[0], first_review);
    assert_eq!(b.register.decision_history[0], first_decision);
    assert_eq!(b.register.review_history.len(), 2);
    assert!(b.undo());
    assert_eq!(b.register.review_history.len(), 2);
    assert_eq!(b.register.decision_history[0], first_decision);
    let key = mbse_model::stage1::RawKey {
        record: RecordId(11),
        field: "calc.input.elapsed".into(),
        item: None,
    };
    editor
        .command(
            &editor.freshness(),
            Command::EditText {
                key,
                text: "9e".into(),
            },
        )
        .unwrap();
    let current = EvidenceContext::from_editor(&editor, &config).unwrap();
    assert_ne!(context, current);
    b.review_evidence(
        id,
        Adequacy::Adequate,
        "Reviewer".into(),
        "Old result cannot acknowledge later raw input".into(),
    )
    .unwrap();
    assert!(b
        .record_verification(
            req,
            id,
            VerificationStatus::CompletedPassed,
            &current,
            "Reviewer".into(),
            "Delayed completion refused".into()
        )
        .is_err());
}
#[test]
fn unrelated_global_failure_is_retained_while_selected_quantity_remains_successful() {
    let mut editor = model();
    editor
        .command(
            &editor.freshness(),
            Command::Add {
                kind: mbse_model::stage1::RecordKind::Measure,
                owner: RecordId(10),
                name: "Unrelated failed calculation".into(),
            },
        )
        .unwrap();
    let other = *editor.snapshot.records.keys().max().unwrap();
    editor
        .command(
            &editor.freshness(),
            Command::PutCalculation {
                measure: other,
                definition: Some(Definition {
                    id: CalculationId(other.0),
                    owner: other,
                    bindings: BTreeMap::new(),
                    expression: calc::parse("1 / 0", &calc::Limits::default()).unwrap(),
                    output: QuantityType {
                        kind: QuantityKind::Scalar,
                        unit: UnitId::ScalarOne,
                    },
                }),
            },
        )
        .unwrap();
    let mut b = Board::new();
    let req = b
        .add_requirement("Review selected evidence only".into())
        .unwrap();
    let id = b
        .attach_calculation(req, &editor, &json!({}), CalculationId(11))
        .unwrap();
    let evidence = &b.register.evidence[&id];
    assert_eq!(evidence.outcome, EvidenceOutcome::Completed);
    assert_eq!(evidence.global_calculation_success, Some(false));
    assert_eq!(evidence.selected_quantity.as_ref().unwrap().value, 15.0);
    let receipt: calc::EvaluationReceipt =
        serde_json::from_slice(&evidence.artifact_bytes).unwrap();
    assert!(!receipt.successful);
    assert!(receipt.outcomes[&CalculationId(other.0)]
        .diagnostic()
        .is_some());
}

#[test]
fn saved_history_identity_admission_and_high_water_survive_undone_objects() {
    let editor = model();
    let mut board = Board::new();
    let req = board
        .add_requirement("Retain audit identity".into())
        .unwrap();
    let evidence = board
        .attach_calculation(req, &editor, &json!({}), CalculationId(11))
        .unwrap();
    board
        .review_evidence(
            evidence,
            Adequacy::Adequate,
            "Reviewer".into(),
            "Explicit review".into(),
        )
        .unwrap();
    let review_id = board.register.review_history[0].review_id;
    assert!(board.undo());
    assert!(board.undo());
    assert!(!board.register.evidence.contains_key(&evidence));
    assert_eq!(board.register.review_history[0].review_id, review_id);
    let area = area();
    let path = area.path().join("history.json");
    board.save_new(&path).unwrap();
    let (mut reopened, _) = Board::open(&path).unwrap();
    assert!(reopened.register.next_id > review_id);
    reopened.register.next_id = review_id;
    assert!(reopened
        .save_new(&area.path().join("bad-highwater.json"))
        .unwrap_err()
        .contains("HIGH_WATER"));
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let duplicate = value["register"]["review_history"][0].clone();
    value["register"]["review_history"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    let malformed = area.path().join("duplicate-audit.json");
    std::fs::write(&malformed, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(Board::open(&malformed).unwrap_err().contains("REVIEW_ID"));
}

#[test]
fn exact_native_source_anchor_requires_relink_after_native_raw_or_model_changes() {
    let mut editor = model();
    let mut board = Board::new();
    let req = board
        .add_requirement("Review exact linked native revision".into())
        .unwrap();
    let anchor = SourceAnchor {
        format_contract: mbse_model::stage1_io::PROFILE.into(),
        artifact_sha256: editor.snapshot.digest(),
        metamodel: "app-owned records".into(),
        source_element_id: "record:11".into(),
        native_document: Some(editor.snapshot.document_id.0.clone()),
        native_record: Some(11),
        units: BTreeMap::new(),
        references: vec![],
        package_integrity: ProjectionStatus::Supported,
        parsing: ProjectionStatus::Supported,
        reference_resolution: ProjectionStatus::Partial,
        semantic_projection: ProjectionStatus::Partial,
        editable_admission: ProjectionStatus::Supported,
        execution: ProjectionStatus::Unknown,
        unknown: BTreeMap::new(),
    };
    board.set_anchor(req, Some(anchor)).unwrap();
    board
        .attach_calculation(req, &editor, &json!({}), CalculationId(11))
        .unwrap();
    let key = mbse_model::stage1::RawKey {
        record: RecordId(1),
        field: "purpose".into(),
        item: None,
    };
    editor
        .command(
            &editor.freshness(),
            Command::EditText {
                key,
                text: "Changed source revision".into(),
            },
        )
        .unwrap();
    assert!(board
        .attach_calculation(req, &editor, &json!({}), CalculationId(11))
        .unwrap_err()
        .contains("ANCHOR_SOURCE_STALE"));
    let mut anchor = board.register.requirements[&req].anchor.clone().unwrap();
    anchor.artifact_sha256 = editor.snapshot.digest();
    board.set_anchor(req, Some(anchor)).unwrap();
    assert!(board
        .attach_calculation(req, &editor, &json!({}), CalculationId(11))
        .is_ok());
}

#[test]
fn canonical_context_metadata_and_fixed_engine_limits_survive_save_reopen() {
    let editor = model();
    let metadata = json!({"purpose":"Review speed", "external_engine":false});
    let mut board = Board::new();
    let req = board
        .add_requirement("Retain evidence context".into())
        .unwrap();
    let id = board
        .attach_calculation(req, &editor, &metadata, CalculationId(11))
        .unwrap();
    let area = area();
    let path = area.path().join("context.json");
    board.save_new(&path).unwrap();
    let (reopened, _) = Board::open(&path).unwrap();
    let evidence = &reopened.register.evidence[&id];
    assert_eq!(evidence.context_metadata, Some(metadata.clone()));
    assert_eq!(evidence.evaluator_limits, Some(calc::Limits::default()));
    let mut malformed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    malformed["register"]["evidence"][id.0.to_string()]["context_metadata"]["purpose"] =
        json!("Tampered context");
    let tampered = area.path().join("tampered-context.json");
    std::fs::write(&tampered, serde_json::to_vec(&malformed).unwrap()).unwrap();
    assert!(Board::open(&tampered)
        .unwrap_err()
        .contains("EVIDENCE_INTEGRITY"));
}

#[test]
fn unrecognized_typed_sibling_fields_refuse_instead_of_disappearing_on_reopen() {
    let editor = model();
    let mut board = Board::new();
    let req = board
        .add_requirement("Strict typed boundary".into())
        .unwrap();
    let evidence = board
        .attach_calculation(req, &editor, &json!({}), CalculationId(11))
        .unwrap();
    let area = area();
    let original = area.path().join("strict.json");
    board.save_new(&original).unwrap();
    let bytes = std::fs::read(&original).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for location in [0, 1, 2, 3] {
        let mut malformed = value.clone();
        let target = match location {
            0 => &mut malformed["register"],
            1 => &mut malformed["register"]["requirements"][req.0.to_string()],
            2 => &mut malformed["register"]["evidence"][evidence.0.to_string()],
            _ => &mut malformed["register"]["evidence"][evidence.0.to_string()]["context"],
        };
        target["futureSibling"] = json!({"must_not_disappear":true});
        let path = area.path().join(format!("unknown-{location}.json"));
        std::fs::write(&path, serde_json::to_vec(&malformed).unwrap()).unwrap();
        assert!(Board::open(&path).unwrap_err().contains("unknown field"));
    }
    assert_eq!(std::fs::read(&original).unwrap(), bytes);
    let (opened, _) = Board::open(&original).unwrap();
    let copy = area.path().join("strict-copy.json");
    opened.save_new(&copy).unwrap();
    assert_eq!(Board::open(&copy).unwrap().0.register, board.register);
}

#[test]
fn aggregate_and_encoded_evidence_budgets_refuse_atomically_and_leave_register_usable() {
    let editor = model();
    let metadata = json!({});
    let context = EvidenceContext::from_editor(&editor, &metadata).unwrap();
    let area = area();
    let artifact = area.path().join("two-megabytes.bin");
    std::fs::write(&artifact, vec![0u8; 2 * 1024 * 1024]).unwrap();
    let mut board = Board::new();
    let req = board
        .add_requirement("Bound accumulated evidence".into())
        .unwrap();
    for _ in 0..4 {
        board
            .attach_local_evidence_with_metadata(
                req,
                context.clone(),
                &metadata,
                &artifact,
                EvidenceOutcome::NotRun,
            )
            .unwrap();
    }
    let before = board.register.clone();
    assert!(board
        .attach_local_evidence_with_metadata(
            req,
            context.clone(),
            &metadata,
            &artifact,
            EvidenceOutcome::NotRun
        )
        .unwrap_err()
        .contains("ARTIFACT_BUDGET"));
    assert_eq!(board.register, before);
    board
        .edit_field(req, "owner", "Still usable".into())
        .unwrap();
    board.save_new(&area.path().join("bounded.json")).unwrap();
    let mut separate = Board::new();
    let req = separate
        .add_requirement("Count encoded Vec expansion".into())
        .unwrap();
    std::fs::write(&artifact, vec![255u8; MAX_ARTIFACT_BYTES]).unwrap();
    let before = separate.register.clone();
    assert!(separate
        .attach_local_evidence_with_metadata(
            req,
            context,
            &metadata,
            &artifact,
            EvidenceOutcome::NotRun
        )
        .unwrap_err()
        .contains("DOCUMENT_LIMIT"));
    assert_eq!(separate.register, before);
    separate
        .save_new(&area.path().join("encoded-refusal-retained.json"))
        .unwrap();
}
