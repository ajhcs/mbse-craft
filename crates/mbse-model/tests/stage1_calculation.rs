//! Independent numeric, failure, preservation and freshness oracles for the
//! closed Stage1 evaluator. Full Editor/history/disk/UI qualification is separate.
use std::collections::BTreeMap;
use mbse_model::stage1::{RawDraft, RawKey, RecordId, Validity};
use mbse_model::stage1_calc::*;

fn quantity(unit: UnitId) -> QuantityType {
    QuantityType {
        kind: unit_kind(unit),
        unit,
    }
}
fn literal(text: &str, unit: UnitId) -> Literal {
    Literal {
        decimal: text.into(),
        quantity: quantity(unit),
    }
}
fn input(text: &str, unit: UnitId) -> Input {
    Input::Literal(literal(text, unit))
}
fn definition(
    id: u64,
    expression: &str,
    bindings: Vec<(&str, Input)>,
    output: UnitId,
) -> Definition {
    Definition {
        id: CalculationId(id),
        owner: RecordId(id),
        expression: parse(expression, &Limits::default()).unwrap(),
        bindings: bindings
            .into_iter()
            .map(|(name, input)| (name.into(), input))
            .collect(),
        output: quantity(output),
    }
}
fn stamp() -> DocumentStamp {
    DocumentStamp {
        document_id: "synthetic-calculation-oracle".into(),
        instance_id: "test-instance".into(),
        model_revision: 1,
        raw_version: 0,
        snapshot_hash: "a".repeat(64),
    }
}
fn definitions(values: Vec<Definition>) -> BTreeMap<CalculationId, Definition> {
    values.into_iter().map(|value| (value.id, value)).collect()
}
fn run(values: Vec<Definition>) -> EvaluationReceipt {
    evaluate(
        &definitions(values),
        &BTreeMap::new(),
        &stamp(),
        &Limits::default(),
    )
}
fn value(receipt: &EvaluationReceipt, id: u64, expected: f64, unit: UnitId) {
    let actual = receipt.outcomes[&CalculationId(id)]
        .value()
        .unwrap_or_else(|| {
            panic!(
                "expected Value, got {:?}",
                receipt.outcomes[&CalculationId(id)]
            )
        });
    assert_eq!(actual.quantity, quantity(unit));
    assert!(
        (actual.value - expected).abs() <= 1e-12 * expected.abs().max(1.0),
        "actual={}, expected={expected}",
        actual.value
    );
}
fn failure(receipt: &EvaluationReceipt, id: u64, expected: DiagnosticCode) {
    assert_eq!(
        receipt.outcomes[&CalculationId(id)]
            .diagnostic()
            .expect("expected failure")
            .code,
        expected
    );
    assert!(!receipt.successful);
}
fn draft(
    owner: u64,
    field: &str,
    text: &str,
    validity: Validity,
    unit: Option<UnitId>,
    generation: u64,
) -> RawDraft {
    RawDraft {
        key: RawKey {
            record: RecordId(owner),
            field: field.into(),
            item: None,
        },
        text: text.into(),
        version: generation,
        validity,
        unit_context: unit.map(|unit| unit_key(unit).into()),
        owner_generation: Some(generation),
    }
}
fn ratio_definition(id: u64) -> Definition {
    definition(
        id,
        "recorded/inspected",
        vec![
            ("recorded", input("76", UnitId::Count)),
            ("inspected", input("80", UnitId::Count)),
        ],
        UnitId::RatioOne,
    )
}

#[test]
fn c01_count_ratio_is_real_output_not_a_criterion() {
    let accepted = ratio_definition(301);
    let preserved = accepted.clone();
    let receipt = run(vec![accepted.clone()]);
    value(&receipt, 301, 0.95, UnitId::RatioOne);
    assert!(receipt.successful);
    assert_eq!(accepted, preserved);
    assert_eq!(receipt.dependencies[&CalculationId(301)].len(), 0);
}
#[test]
fn c02_output_reference_converts_ratio_scale_once() {
    let receipt = run(vec![
        ratio_definition(301),
        definition(
            302,
            "result",
            vec![("result", Input::Output(CalculationId(301)))],
            UnitId::Percent,
        ),
    ]);
    value(&receipt, 302, 95.0, UnitId::Percent);
    assert_eq!(
        receipt.dependencies[&CalculationId(302)]
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![CalculationId(301)]
    );
}
#[test]
fn c03_explicit_times_100_is_not_heuristically_removed() {
    let receipt = run(vec![
        ratio_definition(301),
        definition(
            302,
            "result*100",
            vec![("result", Input::Output(CalculationId(301)))],
            UnitId::Percent,
        ),
    ]);
    value(&receipt, 302, 9500.0, UnitId::Percent);
}
#[test]
fn c04_mixed_length_scales_have_known_reference() {
    let receipt = run(vec![definition(
        1,
        "a+b",
        vec![
            ("a", input("2", UnitId::Kilometre)),
            ("b", input("500", UnitId::Metre)),
        ],
        UnitId::Metre,
    )]);
    value(&receipt, 1, 2500.0, UnitId::Metre);
    let receipt = run(vec![definition(
        1,
        "a+b",
        vec![
            ("a", input("1", UnitId::Kilometre)),
            ("b", input("250", UnitId::Metre)),
        ],
        UnitId::Kilometre,
    )]);
    value(&receipt, 1, 1.25, UnitId::Kilometre);
}
fn travel_definitions() -> Vec<Definition> {
    vec![
        definition(
            4,
            "a+b",
            vec![
                ("a", input("2", UnitId::Kilometre)),
                ("b", input("500", UnitId::Metre)),
            ],
            UnitId::Metre,
        ),
        definition(
            5,
            "route/time",
            vec![
                ("route", Input::Output(CalculationId(4))),
                ("time", input("2", UnitId::Minute)),
            ],
            UnitId::MetrePerSecond,
        ),
        definition(
            6,
            "speed*time",
            vec![
                ("speed", Input::Output(CalculationId(5))),
                ("time", input("3", UnitId::Minute)),
            ],
            UnitId::Metre,
        ),
        definition(
            7,
            "speed",
            vec![("speed", Input::Output(CalculationId(5)))],
            UnitId::KilometrePerHour,
        ),
    ]
}
#[test]
fn c05_length_duration_dependency_yields_speed() {
    value(
        &run(travel_definitions()),
        5,
        20.833333333333332,
        UnitId::MetrePerSecond,
    );
}
#[test]
fn c06_speed_duration_dependency_yields_distance() {
    value(&run(travel_definitions()), 6, 3750.0, UnitId::Metre);
}
#[test]
fn c07_speed_output_scale_and_astra_reference() {
    value(
        &run(travel_definitions()),
        7,
        75.0,
        UnitId::KilometrePerHour,
    );
    let receipt = run(vec![
        definition(
            1,
            "distance/time",
            vec![
                ("distance", input("120", UnitId::Metre)),
                ("time", input("8", UnitId::Second)),
            ],
            UnitId::MetrePerSecond,
        ),
        definition(
            2,
            "speed*time",
            vec![
                ("speed", Input::Output(CalculationId(1))),
                ("time", input("2", UnitId::Second)),
            ],
            UnitId::Metre,
        ),
    ]);
    value(&receipt, 1, 15.0, UnitId::MetrePerSecond);
    value(&receipt, 2, 30.0, UnitId::Metre);
}
#[test]
fn c08_nominal_kind_distinguishes_equal_dimensions() {
    for unit in [UnitId::ScalarOne, UnitId::Count] {
        let receipt = run(vec![definition(
            1,
            "a+b",
            vec![("a", input("1", unit)), ("b", input("1", UnitId::RatioOne))],
            unit,
        )]);
        failure(&receipt, 1, DiagnosticCode::QuantityKindMismatch);
    }
    let mut declaration = literal("1", UnitId::RatioOne);
    declaration.quantity.kind = QuantityKind::Scalar;
    assert_eq!(
        validate_literal(&declaration).unwrap_err().code,
        DiagnosticCode::QuantityKindMismatch
    );
}
#[test]
fn c09_wrong_physical_dimensions_are_unit_errors() {
    let receipt = run(vec![definition(
        1,
        "a+b",
        vec![
            ("a", input("1", UnitId::Metre)),
            ("b", input("1", UnitId::Second)),
        ],
        UnitId::Metre,
    )]);
    failure(&receipt, 1, DiagnosticCode::UnitMismatch);
    let receipt = run(vec![definition(
        1,
        "a",
        vec![("a", input("1", UnitId::Second))],
        UnitId::Metre,
    )]);
    failure(&receipt, 1, DiagnosticCode::UnitMismatch);
}
#[test]
fn c10_both_signs_of_zero_refuse_division() {
    for expression in ["1/0", "1/-0", "1/(2-2)"] {
        failure(
            &run(vec![definition(1, expression, vec![], UnitId::ScalarOne)]),
            1,
            DiagnosticCode::DivisionByZero,
        );
    }
    failure(
        &run(vec![definition(
            1,
            "a/b",
            vec![
                ("a", input("76", UnitId::Count)),
                ("b", input("0", UnitId::Count)),
            ],
            UnitId::RatioOne,
        )]),
        1,
        DiagnosticCode::DivisionByZero,
    );
}
#[test]
fn c11_domain_and_count_exactness_are_enforced_before_rounding() {
    failure(
        &run(vec![definition(1, "sqrt(-1)", vec![], UnitId::ScalarOne)]),
        1,
        DiagnosticCode::DomainError,
    );
    for text in [
        "-1",
        "1.5",
        "9007199254740992",
        "9007199254740991.1",
        "90071992547409910e-1",
    ] {
        if text == "90071992547409910e-1" {
            assert!(validate_literal(&literal(text, UnitId::Count)).is_ok());
        } else {
            assert_eq!(
                validate_literal(&literal(text, UnitId::Count))
                    .unwrap_err()
                    .code,
                DiagnosticCode::DomainError,
                "{text}"
            );
        }
    }
    assert!(validate_literal(&literal("9007199254740991", UnitId::Count)).is_ok());
    assert!(validate_literal(&literal("-0.000", UnitId::Count)).is_ok());
    assert!(validate_literal(&literal("1.00", UnitId::Count)).is_ok());
    failure(
        &run(vec![definition(
            1,
            "a*0.5",
            vec![("a", input("3", UnitId::Count))],
            UnitId::Count,
        )]),
        1,
        DiagnosticCode::DomainError,
    );
}
#[test]
fn c12_overflow_underflow_and_authored_zero_are_distinct() {
    failure(
        &run(vec![definition(
            1,
            "1e308*1e308",
            vec![],
            UnitId::ScalarOne,
        )]),
        1,
        DiagnosticCode::Overflow,
    );
    failure(
        &run(vec![definition(
            1,
            "1e-308/1e308",
            vec![],
            UnitId::ScalarOne,
        )]),
        1,
        DiagnosticCode::Underflow,
    );
    assert_eq!(
        decimal("1e-999").unwrap_err().code,
        DiagnosticCode::Underflow
    );
    assert_eq!(decimal("1e999").unwrap_err().code, DiagnosticCode::Overflow);
    value(
        &run(vec![definition(1, "1-1", vec![], UnitId::ScalarOne)]),
        1,
        0.0,
        UnitId::ScalarOne,
    );
    let zero = run(vec![definition(1, "-0", vec![], UnitId::ScalarOne)]);
    assert_eq!(
        zero.outcomes[&CalculationId(1)]
            .value()
            .unwrap()
            .value
            .to_bits(),
        0.0f64.to_bits()
    );
    let authored = literal("-0", UnitId::ScalarOne);
    assert_eq!(authored.decimal, "-0");
    assert_eq!(
        convert_literal(&literal("1e-323", UnitId::RatioOne), UnitId::Percent)
            .unwrap()
            .quantity
            .unit,
        UnitId::Percent
    );
    assert!(validate_literal(
        &convert_literal(&literal("1e-323", UnitId::RatioOne), UnitId::Percent).unwrap()
    )
    .is_ok());
    assert!(validate_literal(
        &convert_literal(&literal("1e305", UnitId::RatioOne), UnitId::Percent).unwrap()
    )
    .is_ok());
    assert_eq!(
        validate_literal(&literal("1e-323", UnitId::Percent))
            .unwrap_err()
            .code,
        DiagnosticCode::Underflow
    );
}
#[test]
fn c13_missing_output_and_unbound_name_are_not_zero() {
    let receipt = run(vec![definition(
        1,
        "source",
        vec![("source", Input::Output(CalculationId(9999)))],
        UnitId::ScalarOne,
    )]);
    failure(&receipt, 1, DiagnosticCode::MissingReference);
    assert_eq!(
        receipt.outcomes[&CalculationId(1)]
            .diagnostic()
            .unwrap()
            .related,
        vec![CalculationId(9999)]
    );
    failure(
        &run(vec![definition(1, "missing", vec![], UnitId::ScalarOne)]),
        1,
        DiagnosticCode::InvalidInput,
    );
}
#[test]
fn c14_cycle_members_sorted_downstream_blocked_independent_value_survives() {
    let receipt = run(vec![
        definition(
            20,
            "b",
            vec![("b", Input::Output(CalculationId(10)))],
            UnitId::ScalarOne,
        ),
        definition(
            10,
            "a",
            vec![("a", Input::Output(CalculationId(20)))],
            UnitId::ScalarOne,
        ),
        definition(
            30,
            "cycle",
            vec![("cycle", Input::Output(CalculationId(20)))],
            UnitId::ScalarOne,
        ),
        definition(40, "2+3", vec![], UnitId::ScalarOne),
    ]);
    for id in [10, 20] {
        failure(&receipt, id, DiagnosticCode::Cycle);
        assert_eq!(
            receipt.outcomes[&CalculationId(id)]
                .diagnostic()
                .unwrap()
                .related,
            vec![CalculationId(10), CalculationId(20)]
        );
    }
    failure(&receipt, 30, DiagnosticCode::BlockedBy);
    value(&receipt, 40, 5.0, UnitId::ScalarOne);
    assert_eq!(
        receipt.outcomes[&CalculationId(30)]
            .diagnostic()
            .unwrap()
            .cause,
        Some(DiagnosticCode::Cycle)
    );
    failure(
        &run(vec![definition(
            1,
            "self_ref",
            vec![("self_ref", Input::Output(CalculationId(1)))],
            UnitId::ScalarOne,
        )]),
        1,
        DiagnosticCode::Cycle,
    );
}
#[test]
fn c15_incomplete_formula_blocks_old_accepted_program() {
    let definitions = definitions(vec![ratio_definition(301)]);
    let before = definitions.clone();
    let old = evaluate(&definitions, &BTreeMap::new(), &stamp(), &Limits::default());
    let raw = BTreeMap::from([(
        CalculationId(301),
        vec![draft(
            301,
            "calc.expression",
            "recorded/",
            Validity::Incomplete,
            None,
            1,
        )],
    )]);
    let mut current = stamp();
    current.raw_version += 1;
    let receipt = evaluate(&definitions, &raw, &current, &Limits::default());
    failure(&receipt, 301, DiagnosticCode::IncompleteInput);
    assert!(!old.is_current(&definitions, &raw, &current));
    assert_eq!(definitions, before);
}
#[test]
fn c16_invalid_or_incomplete_literal_never_falls_back_to_accepted_input() {
    let definitions = definitions(vec![ratio_definition(301)]);
    for (text, valid, expected) in [
        ("7e", Validity::Incomplete, DiagnosticCode::IncompleteInput),
        ("banana", Validity::Invalid, DiagnosticCode::InvalidInput),
    ] {
        let raw = BTreeMap::from([(
            CalculationId(301),
            vec![draft(
                301,
                "calc.input.recorded",
                text,
                valid,
                Some(UnitId::Count),
                1,
            )],
        )]);
        failure(
            &evaluate(&definitions, &raw, &stamp(), &Limits::default()),
            301,
            expected,
        );
    }
}
#[test]
fn c17_closed_functions_and_algebra_refuse_unsupported_forms() {
    for expression in ["sin(1)", "2^3", "x=2", "2[Metre]", "print(1)"] {
        assert_eq!(
            parse(expression, &Limits::default()).unwrap_err().code,
            DiagnosticCode::Unsupported,
            "{expression}"
        );
    }
    for (a, b) in [
        (UnitId::Metre, UnitId::Metre),
        (UnitId::RatioOne, UnitId::Metre),
        (UnitId::Count, UnitId::Count),
    ] {
        failure(
            &run(vec![definition(
                1,
                "a*b",
                vec![("a", input("1", a)), ("b", input("1", b))],
                a,
            )]),
            1,
            DiagnosticCode::UnsupportedAlgebra,
        );
    }
    failure(
        &run(vec![definition(
            1,
            "sqrt(a)",
            vec![("a", input("4", UnitId::RatioOne))],
            UnitId::ScalarOne,
        )]),
        1,
        DiagnosticCode::QuantityKindMismatch,
    );
}
#[test]
fn c18_unrelated_snapshot_edit_invalidates_receipt_but_not_formula_number() {
    let definitions = definitions(vec![ratio_definition(301)]);
    let raw = BTreeMap::new();
    let old = evaluate(&definitions, &raw, &stamp(), &Limits::default());
    let mut changed = stamp();
    changed.model_revision += 1;
    changed.snapshot_hash = "criterion95-to90-in-owner-snapshot".into();
    assert!(!old.is_current(&definitions, &raw, &changed));
    let recomputed = evaluate(&definitions, &raw, &changed, &Limits::default());
    value(&recomputed, 301, 0.95, UnitId::RatioOne);
    assert!(recomputed.is_current(&definitions, &raw, &changed));
}
#[test]
fn c19_native_definition_raw_serialization_preserves_text_and_does_not_persist_current_receipt() {
    let definitions = definitions(vec![ratio_definition(301)]);
    let raw = BTreeMap::from([(
        CalculationId(301),
        vec![draft(
            301,
            "calc.input.recorded",
            "+0076",
            Validity::Valid,
            Some(UnitId::Count),
            1,
        )],
    )]);
    let restored_definitions: BTreeMap<CalculationId, Definition> =
        serde_json::from_slice(&serde_json::to_vec(&definitions).unwrap()).unwrap();
    let restored_raw: CalculationRawState =
        serde_json::from_slice(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(restored_definitions, definitions);
    assert_eq!(restored_raw, raw);
    failure(
        &evaluate(&definitions, &raw, &stamp(), &Limits::default()),
        301,
        DiagnosticCode::InvalidInput,
    );
    assert_eq!(restored_raw[&CalculationId(301)][0].text, "+0076");
    let mut accepted = definitions.clone();
    if let Input::Literal(value) = accepted
        .get_mut(&CalculationId(301))
        .unwrap()
        .bindings
        .get_mut("recorded")
        .unwrap()
    {
        value.decimal = "+0076".into();
    }
    value(
        &evaluate(&accepted, &raw, &stamp(), &Limits::default()),
        301,
        0.95,
        UnitId::RatioOne,
    );
}
#[test]
fn c20_late_receipt_and_changed_definitions_or_raw_cannot_be_current() {
    let definitions = definitions(vec![ratio_definition(301)]);
    let raw = BTreeMap::new();
    let original_stamp = stamp();
    let receipt = evaluate(&definitions, &raw, &original_stamp, &Limits::default());
    assert!(receipt.is_current(&definitions, &raw, &original_stamp));
    for field in 0..5 {
        let mut next = original_stamp.clone();
        match field {
            0 => next.document_id.push('x'),
            1 => next.instance_id.push('x'),
            2 => next.model_revision += 1,
            3 => next.raw_version += 1,
            _ => next.snapshot_hash.push('x'),
        }
        assert!(!receipt.is_current(&definitions, &raw, &next));
    }
    let mut changed = definitions.clone();
    changed.get_mut(&CalculationId(301)).unwrap().output = quantity(UnitId::Percent);
    assert!(!receipt.is_current(&changed, &raw, &original_stamp));
    let raw = BTreeMap::from([(
        CalculationId(301),
        vec![draft(
            301,
            "calc.expression",
            "recorded / inspected",
            Validity::Valid,
            None,
            1,
        )],
    )]);
    assert!(!receipt.is_current(&definitions, &raw, &original_stamp));
}
#[test]
fn c21_resources_refuse_without_success_and_report_truncation() {
    assert_eq!(
        parse(
            &format!("{}1{}", "(".repeat(65), ")".repeat(65)),
            &Limits::default()
        )
        .unwrap_err()
        .code,
        DiagnosticCode::ResourceLimit
    );
    let too_many = definitions(
        (1..=1025)
            .map(|id| definition(id, "1", vec![], UnitId::ScalarOne))
            .collect(),
    );
    let receipt = evaluate(&too_many, &BTreeMap::new(), &stamp(), &Limits::default());
    assert!(!receipt.successful);
    assert!(receipt
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit));
    assert!(!receipt.is_current(&too_many, &BTreeMap::new(), &stamp()));
    let low_budget = Limits {
        node_visits: 1,
        ..Limits::default()
    };
    failure(
        &evaluate(
            &definitions(vec![definition(1, "1+2", vec![], UnitId::ScalarOne)]),
            &BTreeMap::new(),
            &stamp(),
            &low_budget,
        ),
        1,
        DiagnosticCode::ResourceLimit,
    );
    let many_failures = definitions(
        (1..=260)
            .map(|id| definition(id, "missing", vec![], UnitId::ScalarOne))
            .collect(),
    );
    let receipt = evaluate(
        &many_failures,
        &BTreeMap::new(),
        &stamp(),
        &Limits::default(),
    );
    assert_eq!(receipt.diagnostics.len(), 256);
    assert_eq!(
        receipt.diagnostics.last().unwrap().code,
        DiagnosticCode::ResourceLimit
    );
    assert!(!receipt.successful);
}
#[test]
fn active_generation_filter_preserves_removed_and_reused_binding_drafts() {
    let current = definition(
        1,
        "x",
        vec![("x", input("2", UnitId::Metre))],
        UnitId::Metre,
    );
    let historic = draft(
        1,
        "calc.input.x",
        "1e",
        Validity::Incomplete,
        Some(UnitId::Metre),
        1,
    );
    let active = draft(
        1,
        "calc.input.x",
        "2",
        Validity::Valid,
        Some(UnitId::Metre),
        2,
    );
    let journals = vec![historic.clone(), active.clone()];
    // Owner is responsible for generation selection; evaluator takes only ACTIVE.
    let raw = BTreeMap::from([(
        CalculationId(1),
        journals
            .iter()
            .filter(|draft| draft.owner_generation == Some(2))
            .cloned()
            .collect(),
    )]);
    let receipt = evaluate(
        &definitions(vec![current]),
        &raw,
        &stamp(),
        &Limits::default(),
    );
    value(&receipt, 1, 2.0, UnitId::Metre);
    assert_eq!(journals, vec![historic, active]);
    assert_eq!(raw[&CalculationId(1)][0].owner_generation, Some(2));
}
#[test]
fn historic_unit_context_classifies_itself_but_blocks_execution_after_undo() {
    let current = definition(
        1,
        "x",
        vec![("x", input("1", UnitId::RatioOne))],
        UnitId::RatioOne,
    );
    let old = draft(
        1,
        "calc.input.x",
        "95",
        Validity::Valid,
        Some(UnitId::Percent),
        1,
    );
    assert_eq!(
        raw_validity(&current, &old, &Limits::default()),
        Some(Validity::Valid)
    );
    assert_eq!(
        classify_raw(&current, &old, &Limits::default())
            .unwrap_err()
            .code,
        DiagnosticCode::InvalidInput
    );
    let mut unknown = old.clone();
    unknown.unit_context = Some("invented-unit".into());
    assert_eq!(
        raw_validity(&current, &unknown, &Limits::default()),
        Some(Validity::Invalid)
    );
    let current = definition(
        1,
        "x",
        vec![("x", input("1", UnitId::Count))],
        UnitId::Count,
    );
    let historic_fraction = draft(
        1,
        "calc.input.x",
        "0.5",
        Validity::Valid,
        Some(UnitId::RatioOne),
        1,
    );
    assert_eq!(
        raw_validity(&current, &historic_fraction, &Limits::default()),
        Some(Validity::Valid)
    );
    assert!(classify_raw(&current, &historic_fraction, &Limits::default()).is_err());
}
#[test]
fn exact_raw_validity_claims_and_detached_fields_are_not_trusted() {
    let current = ratio_definition(301);
    let forged = draft(
        301,
        "calc.expression",
        "recorded/",
        Validity::Valid,
        None,
        1,
    );
    assert_eq!(
        raw_validity(&current, &forged, &Limits::default()),
        Some(Validity::Incomplete)
    );
    assert_eq!(
        classify_raw(&current, &forged, &Limits::default())
            .unwrap_err()
            .code,
        DiagnosticCode::InvalidInput
    );
    let detached = draft(
        301,
        "calc.input.deleted",
        "banana",
        Validity::Invalid,
        Some(UnitId::Metre),
        1,
    );
    assert_eq!(raw_validity(&current, &detached, &Limits::default()), None);
    assert!(classify_raw(&current, &detached, &Limits::default()).is_ok());
    let unknown = draft(
        301,
        "valueExpression",
        "recorded/inspected*100",
        Validity::Unsupported,
        None,
        1,
    );
    assert_eq!(raw_validity(&current, &unknown, &Limits::default()), None);
}
#[test]
fn parser_precedence_functions_spans_and_formatter_roundtrip() {
    let text = "max(abs(-3),sqrt(16))+min(1,2)*+2/4";
    let parsed = parse(text, &Limits::default()).unwrap();
    assert_eq!(
        parse(&format_expression(&parsed), &Limits::default()).unwrap(),
        parsed
    );
    value(
        &run(vec![definition(1, text, vec![], UnitId::ScalarOne)]),
        1,
        4.5,
        UnitId::ScalarOne,
    );
    value(
        &run(vec![definition(1, "12/3/2-1", vec![], UnitId::ScalarOne)]),
        1,
        1.0,
        UnitId::ScalarOne,
    );
    for text in ["a+", "sqrt(", "1e", "1e+", "("] {
        let error = parse(text, &Limits::default()).unwrap_err();
        assert_eq!(error.code, DiagnosticCode::IncompleteInput, "{text}");
        assert!(error.span.is_some());
    }
    for text in ["a)", "min(1)", "max(1,2,3)", "2 3", "1efoo"] {
        assert_eq!(
            parse(text, &Limits::default()).unwrap_err().code,
            DiagnosticCode::InvalidInput,
            "{text}"
        );
    }
}
#[test]
fn definitions_reject_unknown_structural_keys_and_bad_attachment_identity() {
    let original = ratio_definition(301);
    let mut json = serde_json::to_value(&original).unwrap();
    json["extra"] = true.into();
    assert!(serde_json::from_value::<Definition>(json).is_err());
    let mut json = serde_json::to_value(literal("1", UnitId::Metre)).unwrap();
    json["extra"] = true.into();
    assert!(serde_json::from_value::<Literal>(json).is_err());
    let mut bad = original.clone();
    bad.owner = RecordId(302);
    assert_eq!(
        validate(&bad, &Limits::default()).unwrap_err().code,
        DiagnosticCode::InvalidInput
    );
}
#[test]
fn thousand_output_chain_is_deterministic_and_bounded() {
    let mut definitions = BTreeMap::new();
    definitions.insert(
        CalculationId(1000),
        definition(1000, "1", vec![], UnitId::ScalarOne),
    );
    // Descending identities catch implementations that evaluate only insertion order.
    for id in (1..1000).rev() {
        definitions.insert(
            CalculationId(id),
            definition(
                id,
                "prior+1",
                vec![("prior", Input::Output(CalculationId(id + 1)))],
                UnitId::ScalarOne,
            ),
        );
    }
    let raw = BTreeMap::new();
    let current = stamp();
    let receipt = evaluate(&definitions, &raw, &current, &Limits::default());
    assert!(receipt.successful);
    assert_eq!(receipt.outcomes.len(), 1000);
    value(&receipt, 1, 1000.0, UnitId::ScalarOne);
    assert_eq!(
        receipt,
        evaluate(&definitions, &raw, &current, &Limits::default())
    );
}

#[test]
fn resource_refusal_is_current_for_exact_document_but_never_a_current_computation() {
    use mbse_model::stage1::{Command, DocumentId, Editor, Record, RecordKind, Snapshot};
    let mut snapshot = Snapshot::new(DocumentId("retained-overlimit-calculations".into()));
    for id in [2, 3] {
        snapshot.records.insert(
            RecordId(id),
            Record {
                id: RecordId(id),
                kind: RecordKind::Objective,
                name: format!("Synthetic objective{id}"),
                scope: RecordId(1),
                fields: BTreeMap::new(),
            },
        );
    }
    // Native retention is intentionally broader than executable graph resources.
    for id in 1000..2025 {
        let definition = definition(id, "1", vec![], UnitId::ScalarOne);
        snapshot.records.insert(
            RecordId(id),
            Record {
                id: RecordId(id),
                kind: RecordKind::Measure,
                name: format!("Retained calculation{id}"),
                scope: RecordId(1),
                fields: BTreeMap::from([
                    ("forObjective".into(), serde_json::json!(2)),
                    (
                        "calculation".into(),
                        serde_json::to_value(definition).unwrap(),
                    ),
                ]),
            },
        );
    }
    snapshot.next_id = 2025;
    snapshot.validate().unwrap();
    let mut editor = Editor::new(snapshot, "refusal-instance".into()).unwrap();
    let definitions = editor.snapshot.calculations().unwrap();
    assert_eq!(definitions.len(), 1025);
    let raw = editor.snapshot.calculation_raw_state();
    let stamp = editor.calculation_stamp();
    let receipt = editor.evaluate_calculations().unwrap();
    assert!(!receipt.successful);
    assert_eq!(receipt.definitions_digest, "resource-refused");
    assert_eq!(receipt.raw_digest, "resource-refused");
    assert_eq!(receipt.input_closure_digest, "resource-refused");
    assert!(receipt
        .outcomes
        .values()
        .all(|outcome| outcome.value().is_none()));
    assert!(receipt.refusal_is_current(&stamp));
    assert!(!receipt.is_current(&definitions, &raw, &stamp));

    // An actual retained raw edit invalidates the refusal even though accepted
    // records and the overlimit definition graph did not change.
    editor
        .command(
            &editor.freshness(),
            Command::EditText {
                key: RawKey {
                    record: RecordId(1000),
                    field: "valueExpression".into(),
                    item: None,
                },
                text: "Retained unsupported source".into(),
            },
        )
        .unwrap();
    let raw_stamp = editor.calculation_stamp();
    assert_eq!(raw_stamp.model_revision, stamp.model_revision);
    assert!(raw_stamp.raw_version > stamp.raw_version);
    assert_ne!(raw_stamp.snapshot_hash, stamp.snapshot_hash);
    assert!(!receipt.refusal_is_current(&raw_stamp));
    let after_raw = editor.evaluate_calculations().unwrap();
    assert!(after_raw.refusal_is_current(&raw_stamp));

    // A real model-only edit also invalidates it without changing raw_version.
    editor
        .command(
            &editor.freshness(),
            Command::SetForObjective {
                measure: RecordId(1000),
                objective: RecordId(3),
            },
        )
        .unwrap();
    let model_stamp = editor.calculation_stamp();
    assert!(model_stamp.model_revision > raw_stamp.model_revision);
    assert_eq!(model_stamp.raw_version, raw_stamp.raw_version);
    assert_ne!(model_stamp.snapshot_hash, raw_stamp.snapshot_hash);
    assert!(!after_raw.refusal_is_current(&model_stamp));
    assert!(editor
        .evaluate_calculations()
        .unwrap()
        .refusal_is_current(&model_stamp));

    // Every stamp component and the algorithm/unit revisions matter. A digest
    // mismatch with unchanged counters still makes the old refusal stale.
    for field in 0..5 {
        let mut changed = stamp.clone();
        match field {
            0 => changed.document_id.push('x'),
            1 => changed.instance_id.push('x'),
            2 => changed.model_revision += 1,
            3 => changed.raw_version += 1,
            _ => changed.snapshot_hash.push('x'),
        }
        assert!(!receipt.refusal_is_current(&changed));
    }
    let mut altered = receipt.clone();
    altered.algorithm_revision.push('x');
    assert!(!altered.refusal_is_current(&stamp));
    let mut altered = receipt.clone();
    altered.unit_table_revision.push('x');
    assert!(!altered.refusal_is_current(&stamp));
    let mut altered = receipt.clone();
    altered.successful = true;
    assert!(!altered.refusal_is_current(&stamp));
    let mut altered = receipt.clone();
    altered.definitions_digest = "ordinary-digest".into();
    assert!(!altered.refusal_is_current(&stamp));
    let mut altered = receipt.clone();
    altered.outcomes.insert(
        CalculationId(1000),
        Outcome::Value(QuantityValue {
            value: 1.0,
            quantity: quantity(UnitId::ScalarOne),
        }),
    );
    assert!(!altered.refusal_is_current(&stamp));
    let ordinary_value = run(vec![definition(1, "1", vec![], UnitId::ScalarOne)]);
    assert!(!ordinary_value.refusal_is_current(&ordinary_value.stamp));
    let ordinary_failure = run(vec![definition(1, "1/0", vec![], UnitId::ScalarOne)]);
    assert!(!ordinary_failure.refusal_is_current(&ordinary_failure.stamp));
}
