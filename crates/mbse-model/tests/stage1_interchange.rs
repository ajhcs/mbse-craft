//! Independent source/state/roundtrip acceptance for the closed SysML profile.
use serde_json::{json, Value};
use std::collections::BTreeMap;
use mbse_model::stage1::{DocumentId, RecordId, RecordKind, Snapshot};
use mbse_model::stage1_calc::{
    self as calc, CalculationId, Definition, Expr, Function, Input, Literal, QuantityKind,
    QuantityType, UnitId,
};
use mbse_model::stage1_sysml::{
    self as sysml, ExchangeCode, ExchangeModel, ExcludedState, SemanticId,
};

fn fixture() -> Snapshot {
    let value: Value =
        serde_json::from_str(include_str!("../fixtures/stage1/inspection.json")).unwrap();
    Snapshot::from_fixture(DocumentId("synthetic-exchange".into()), &value).unwrap()
}
fn baseline() -> ExchangeModel {
    sysml::from_records(&fixture().records).unwrap()
}
fn literal(decimal: &str, kind: QuantityKind, unit: UnitId) -> Input {
    Input::Literal(Literal {
        decimal: decimal.into(),
        quantity: QuantityType { kind, unit },
    })
}
fn attach(
    model: &mut ExchangeModel,
    id: u64,
    bindings: BTreeMap<String, Input>,
    expression: Expr,
    kind: QuantityKind,
    unit: UnitId,
) {
    if id != 301 {
        let mut record = model.records[&RecordId(301)].clone();
        record.id = RecordId(id);
        record.name = format!("Independent calculation{id}");
        record.fields.remove("calculation");
        model.records.insert(record.id, record);
    }
    let definition = Definition {
        id: CalculationId(id),
        owner: RecordId(id),
        bindings,
        expression,
        output: QuantityType { kind, unit },
    };
    model.records.get_mut(&RecordId(id)).unwrap().fields.insert(
        "calculation".into(),
        serde_json::to_value(&definition).unwrap(),
    );
    model.calculations.insert(definition.id, definition);
}
fn program() -> ExchangeModel {
    let mut model = baseline();
    attach(
        &mut model,
        301,
        BTreeMap::from([
            (
                "distance".into(),
                literal("120", QuantityKind::Length, UnitId::Metre),
            ),
            (
                "duration".into(),
                literal("8", QuantityKind::Duration, UnitId::Second),
            ),
        ]),
        Expr::Div(
            Box::new(Expr::Binding("distance".into())),
            Box::new(Expr::Binding("duration".into())),
        ),
        QuantityKind::Speed,
        UnitId::MetrePerSecond,
    );
    attach(
        &mut model,
        302,
        BTreeMap::from([
            ("speed".into(), Input::Output(CalculationId(301))),
            (
                "time".into(),
                literal("2", QuantityKind::Duration, UnitId::Second),
            ),
        ]),
        Expr::Mul(
            Box::new(Expr::Binding("speed".into())),
            Box::new(Expr::Binding("time".into())),
        ),
        QuantityKind::Length,
        UnitId::Metre,
    );
    attach(
        &mut model,
        303,
        BTreeMap::from([
            (
                "first".into(),
                literal("1", QuantityKind::Length, UnitId::Kilometre),
            ),
            (
                "second".into(),
                literal("250", QuantityKind::Length, UnitId::Metre),
            ),
        ]),
        Expr::Add(
            Box::new(Expr::Binding("first".into())),
            Box::new(Expr::Binding("second".into())),
        ),
        QuantityKind::Length,
        UnitId::Kilometre,
    );
    attach(
        &mut model,
        304,
        BTreeMap::from([(
            "ratio".into(),
            literal("0.95", QuantityKind::Ratio, UnitId::RatioOne),
        )]),
        Expr::Binding("ratio".into()),
        QuantityKind::Ratio,
        UnitId::Percent,
    );
    attach(
        &mut model,
        305,
        BTreeMap::from([
            (
                "recorded".into(),
                literal("100", QuantityKind::Count, UnitId::Count),
            ),
            (
                "total".into(),
                literal("200", QuantityKind::Count, UnitId::Count),
            ),
        ]),
        Expr::Div(
            Box::new(Expr::Binding("recorded".into())),
            Box::new(Expr::Binding("total".into())),
        ),
        QuantityKind::Ratio,
        UnitId::RatioOne,
    );
    attach(
        &mut model,
        306,
        BTreeMap::from([(
            "value".into(),
            literal("9", QuantityKind::Scalar, UnitId::ScalarOne),
        )]),
        Expr::Call(Function::Sqrt, vec![Expr::Binding("value".into())]),
        QuantityKind::Scalar,
        UnitId::ScalarOne,
    );
    model
}
fn text(model: &ExchangeModel) -> String {
    sysml::export(model)
        .unwrap_or_else(|e| panic!("Export: {e:#?}"))
        .utf8
}
fn refused(source: &str, code: ExchangeCode) {
    let old = source.as_bytes().to_vec();
    let failure = sysml::import("unsupported.sysml", source.as_bytes()).unwrap_err();
    assert_eq!(
        failure.source_asset.original_bytes.as_deref(),
        Some(old.as_slice())
    );
    assert_eq!(source.as_bytes(), old);
    assert!(
        failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code),
        "{:#?}",
        failure.diagnostics
    );
    assert!(failure.diagnostics.iter().all(|diagnostic| diagnostic
        .span
        .as_ref()
        .is_none_or(|span| span.byte_start <= span.byte_end && span.byte_end <= source.len())));
}

#[test]
fn complete_baseline_accepted_meaning_roundtrips_without_native_raw_promotion() {
    let snapshot = fixture();
    let original = snapshot.clone();
    let model = baseline();
    let exported = sysml::export(&model).unwrap();
    assert!(exported
        .utf8
        .contains("95[KernspanStage1DraftOntology::percent]"));
    assert!(!exported.utf8.contains("9e"));
    let imported = sysml::import("complete.sysml", exported.utf8.as_bytes()).unwrap();
    assert_eq!(imported.semantics.records.len(), 10);
    assert_eq!(
        imported.semantics.records[&RecordId(301)].number("criterion"),
        Some(95.0)
    );
    assert_eq!(
        imported.semantics.records[&RecordId(301)].text("unit"),
        "percent"
    );
    assert_eq!(
        imported.semantics.records[&RecordId(200)].links(),
        vec![RecordId(100)]
    );
    assert_eq!(
        imported.semantics.records[&RecordId(401)].fields["affiliations"],
        json!({"900":"Internal"})
    );
    assert_eq!(
        imported.semantics.records[&RecordId(600)].fields["context"][0]["id"],
        601
    );
    assert_eq!(
        imported.semantics.records[&RecordId(600)].fields["context"][1]["id"],
        602
    );
    assert_eq!(
        sysml::semantic_digest(&model).unwrap(),
        sysml::semantic_digest(&imported.semantics).unwrap()
    );
    assert!(exported
        .excluded_session_state
        .contains(&ExcludedState::RawDrafts));
    assert_eq!(snapshot, original);
}

#[test]
fn genuine_calc_quantities_dependencies_and_scalar_function_preserve_numeric_oracles() {
    let model = program();
    let before = model.clone();
    let source = text(&model);
    assert!(source.contains("calc def <ks_c_301>"));
    assert!(source.contains("120[SI::m]"));
    assert!(source.contains("run_301.result"));
    assert!(source.contains("RealFunctions::sqrt"));
    assert!(!source.contains("\"calculation\""));
    let imported = sysml::import("calculations.sysml", source.as_bytes()).unwrap();
    assert_eq!(
        sysml::semantic_digest(&model).unwrap(),
        sysml::semantic_digest(&imported.semantics).unwrap()
    );
    let stamp = calc::DocumentStamp {
        document_id: "synthetic".into(),
        instance_id: "independent".into(),
        model_revision: 0,
        raw_version: 0,
        snapshot_hash: "test-only".into(),
    };
    let receipt = calc::evaluate(
        &imported.semantics.calculations,
        &BTreeMap::new(),
        &stamp,
        &calc::Limits::default(),
    );
    assert!(receipt.successful, "{:#?}", receipt.diagnostics);
    for (id, number) in [
        (301, 15.0),
        (302, 30.0),
        (303, 1.25),
        (304, 95.0),
        (305, 0.5),
        (306, 3.0),
    ] {
        assert!(
            (receipt.outcomes[&CalculationId(id)].value().unwrap().value - number).abs() < 1e-12
        );
    }
    assert_eq!(
        imported.semantics.calculations[&CalculationId(304)]
            .output
            .unit,
        UnitId::Percent
    );
    assert_eq!(
        imported.semantics.records[&RecordId(301)].number("criterion"),
        Some(95.0)
    );
    assert_eq!(model, before);
}

#[test]
fn exact_source_unicode_duplicate_labels_and_qualified_references_keep_identities_and_spans() {
    let mut model = baseline();
    model.records.get_mut(&RecordId(100)).unwrap().name = "same / 'name' / Δ\nline".into();
    model.records.get_mut(&RecordId(200)).unwrap().name =
        model.records[&RecordId(100)].name.clone();
    let source = text(&model).replace(
        "contributesTo = (r_100)",
        "contributesTo = (KernspanStage1Exchange::r_100)",
    );
    let imported = sysml::import("unicode.sysml", source.as_bytes()).unwrap();
    assert_eq!(imported.source.source_utf8, source);
    assert_eq!(
        imported.semantics.records[&RecordId(100)].name,
        model.records[&RecordId(100)].name
    );
    assert_eq!(
        imported.semantics.records[&RecordId(200)].links(),
        vec![RecordId(100)]
    );
    for symbol in &imported.source.symbols {
        assert!(source.is_char_boundary(symbol.declaration.byte_start));
        assert!(source.is_char_boundary(symbol.declaration.byte_end));
        assert!(symbol.declaration.byte_start < symbol.declaration.byte_end);
    }
    assert!(imported
        .source
        .symbols
        .iter()
        .any(|symbol| symbol.stable_id == SemanticId::Context(RecordId(601))));
}

#[test]
fn original_positive_fixture_and_standard_construct_refusals_are_distinct() {
    let supported = include_str!("../fixtures/stage1-interchange/supported-minimal.sysml");
    let imported = sysml::import("minimal.sysml", supported.as_bytes()).unwrap();
    assert_eq!(imported.semantics.records.len(), 1);
    assert_eq!(
        imported.semantics.records[&RecordId(1)].kind,
        RecordKind::System
    );
    refused(
        include_str!("../fixtures/stage1-interchange/unsupported-behavior.sysml"),
        ExchangeCode::UnsupportedConstruct,
    );
    refused(
        include_str!("../fixtures/stage1-interchange/unsupported-standard-requirement.sysml"),
        ExchangeCode::UnsupportedConstruct,
    );
}

#[test]
fn parser_body_superset_does_not_admit_unsupported_native_record_behaviors() {
    let original = text(&baseline());
    let source = original.replacen(
        "attribute :>> 'purpose' =",
        "action illegalNestedAction; attribute :>> 'purpose' =",
        1,
    );
    assert_ne!(
        source, original,
        "behavior mutation must alter actual generated source"
    );
    assert!(sysmlv2_syntax::parser::parse_source(&source)
        .diagnostics
        .is_empty());
    let failure = sysml::import("nested.sysml", source.as_bytes()).unwrap_err();
    let diagnostic = &failure.diagnostics[0];
    assert_eq!(diagnostic.code, ExchangeCode::UnsupportedConstruct);
    let span = diagnostic.span.as_ref().unwrap();
    assert!(source[span.byte_start..span.byte_end].contains("illegalNestedAction"));
}

#[test]
fn wrong_kind_missing_and_cross_scope_references_refuse_without_label_guessing() {
    let source = text(&baseline());
    refused(
        &source.replace("contributesTo = (r_100)", "contributesTo = (r_300)"),
        ExchangeCode::WrongType,
    );
    refused(
        &source.replace("contributesTo = (r_100)", "contributesTo = (missing)"),
        ExchangeCode::MissingSymbol,
    );
    let mut model = baseline();
    let mut second = model.records[&RecordId(1)].clone();
    second.id = RecordId(2);
    second.scope = RecordId(2);
    model.records.insert(second.id, second);
    model.records.get_mut(&RecordId(100)).unwrap().scope = RecordId(2);
    assert!(sysml::export(&model).is_err());
}

#[test]
fn duplicate_ids_symbols_and_numeric_tag_mismatch_are_atomic_refusals() {
    let source = text(&baseline());
    refused(
        &source.replacen("attribute :>> id = 1;", "attribute :>> id = 2;", 1),
        ExchangeCode::DuplicateIdentity,
    );
    refused(
        &source.replace("<ks_r_200> r_200", "<ks_r_200> r_100"),
        ExchangeCode::AmbiguousSymbol,
    );
    refused(
        &source.replace("contributesTo = (r_100)", "contributesTo = (r_100,r_100)"),
        ExchangeCode::DuplicateIdentity,
    );
}

#[test]
fn unknown_units_types_and_unreviewed_functions_are_not_strings_of_success() {
    let source = text(&baseline());
    refused(
        &source.replace("95[KernspanStage1DraftOntology::percent]", "95[SI::m]"),
        ExchangeCode::Unit,
    );
    refused(
        &source.replace(
            "criterion : MeasurementReferences::DimensionOneValue",
            "criterion : ISQ::LengthValue",
        ),
        ExchangeCode::QuantityKind,
    );
    let program = text(&program());
    refused(
        &program.replace("RealFunctions::sqrt", "UntrustedFunctions::sqrt"),
        ExchangeCode::UnsupportedConstruct,
    );
    refused(
        &program.replace("120[SI::m]", "120[SI::unknown]"),
        ExchangeCode::Unit,
    );
}

#[test]
fn every_modified_prefix_default_ontology_and_import_is_explicitly_refused() {
    let source = text(&baseline());
    for bad in [
        source.replacen("part <ks_r_1>", "abstract part <ks_r_1>", 1),
        source.replacen(
            "attribute :>> 'purpose' =",
            "attribute :>> 'purpose' default =",
            1,
        ),
        source.replacen(
            "r_1 : KernspanStage1DraftOntology::SystemDraft",
            "r_1 : KernspanStage1DraftOntology::SystemDraft[0..1]",
            1,
        ),
        source.replacen(
            "part def SystemDraft :> RecordDraft",
            "part def SystemDraft :> RecordDraft, MissionDraft",
            1,
        ),
        source.replacen(
            "package KernspanStage1Exchange {",
            "package KernspanStage1Exchange { private import External::*;",
            1,
        ),
    ] {
        assert_ne!(
            bad, source,
            "every negative mutation must alter actual generated source"
        );
        assert!(
            sysml::import("unsupported.sysml", bad.as_bytes()).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn opaque_values_nonempty_unqualified_observations_and_physical_ast_literals_refuse() {
    let mut model = baseline();
    let original = model.clone();
    model
        .records
        .get_mut(&RecordId(301))
        .unwrap()
        .fields
        .insert("unreviewedMeaning".into(), json!({"x":1}));
    let before = model.clone();
    assert_eq!(
        sysml::export(&model).unwrap_err()[0].code,
        ExchangeCode::UnsupportedField
    );
    assert_eq!(model, before);
    model = original;
    model
        .records
        .get_mut(&RecordId(301))
        .unwrap()
        .fields
        .insert(
            "observations".into(),
            json!([{"id":901,"scope":1,"value":7}]),
        );
    assert!(sysml::export(&model).is_err());
    let mut model = baseline();
    attach(
        &mut model,
        301,
        BTreeMap::new(),
        Expr::Literal(Literal {
            decimal: "120".into(),
            quantity: QuantityType {
                kind: QuantityKind::Length,
                unit: UnitId::Metre,
            },
        }),
        QuantityKind::Length,
        UnitId::Metre,
    );
    assert_eq!(
        sysml::export(&model).unwrap_err()[0].code,
        ExchangeCode::UnsupportedConstruct
    );
}

#[test]
fn calculation_attachment_map_cycle_and_missing_dependency_cannot_disagree() {
    let mut model = program();
    model.calculations.remove(&CalculationId(301));
    assert_eq!(
        sysml::export(&model).unwrap_err()[0].code,
        ExchangeCode::Calculation
    );
    let mut model = program();
    let mut definition = model.calculations[&CalculationId(301)].clone();
    definition
        .bindings
        .insert("distance".into(), Input::Output(CalculationId(9999)));
    model
        .records
        .get_mut(&RecordId(301))
        .unwrap()
        .fields
        .insert(
            "calculation".into(),
            serde_json::to_value(&definition).unwrap(),
        );
    model.calculations.insert(definition.id, definition);
    assert_eq!(
        sysml::export(&model).unwrap_err()[0].code,
        ExchangeCode::MissingSymbol
    );
    let mut model = baseline();
    attach(
        &mut model,
        301,
        BTreeMap::from([("prior".into(), Input::Output(CalculationId(302)))]),
        Expr::Binding("prior".into()),
        QuantityKind::Scalar,
        UnitId::ScalarOne,
    );
    attach(
        &mut model,
        302,
        BTreeMap::from([("prior".into(), Input::Output(CalculationId(301)))]),
        Expr::Binding("prior".into()),
        QuantityKind::Scalar,
        UnitId::ScalarOne,
    );
    assert_eq!(
        sysml::export(&model).unwrap_err()[0].code,
        ExchangeCode::Calculation
    );
}

#[test]
fn native_generation_metadata_and_original_archive_are_excluded_without_mutation() {
    let mut model = program();
    let original_digest = sysml::semantic_digest(&model).unwrap();
    model
        .records
        .get_mut(&RecordId(301))
        .unwrap()
        .fields
        .insert(
            "_kernspanCalculationRawOwners".into(),
            json!({"expression":500,"inputs":{"distance":501,"duration":502}}),
        );
    model.records.get_mut(&RecordId(1)).unwrap().fields.insert(
        "_kernspanSysmlSource".into(),
        json!({"nativeOnly":"immutable original source"}),
    );
    let original = model.clone();
    let exported = sysml::export(&model).unwrap();
    assert!(!exported.utf8.contains("_kernspan"));
    assert_eq!(original_digest, exported.semantic_digest);
    assert!(exported
        .excluded_session_state
        .contains(&ExcludedState::NativeJournalOwnershipMetadata));
    assert_eq!(model, original);
}

#[test]
fn invalid_utf8_and_resource_failures_retain_or_explicitly_leave_caller_originals() {
    let bytes = [0xff, 0xfe, 0x00];
    let failed = sysml::import("bad.sysml", &bytes).unwrap_err();
    assert_eq!(failed.source_asset.original_bytes, Some(bytes.to_vec()));
    let oversized = vec![b' '; 1024 * 1024 + 1];
    let before = oversized.clone();
    let failed = sysml::import("large.sysml", &oversized).unwrap_err();
    assert_eq!(failed.diagnostics[0].code, ExchangeCode::Limit);
    assert_eq!(failed.source_asset.original_bytes, None);
    assert_eq!(failed.source_asset.byte_len, oversized.len());
    assert_eq!(oversized, before);
    let depth = format!("{}{}", "package A {".repeat(65), "}".repeat(65));
    assert_eq!(
        sysml::import("deep.sysml", depth.as_bytes())
            .unwrap_err()
            .diagnostics[0]
            .code,
        ExchangeCode::Limit
    );
}

#[test]
fn signed_scalar_literals_and_optional_absence_have_declared_semantic_normalization() {
    let mut model = baseline();
    attach(
        &mut model,
        301,
        BTreeMap::new(),
        Expr::Literal(Literal {
            decimal: "-1.25".into(),
            quantity: QuantityType {
                kind: QuantityKind::Scalar,
                unit: UnitId::ScalarOne,
            },
        }),
        QuantityKind::Scalar,
        UnitId::ScalarOne,
    );
    let exported = sysml::export(&model).unwrap();
    let imported = sysml::import("signed.sysml", exported.utf8.as_bytes()).unwrap();
    assert_eq!(
        exported.semantic_digest,
        sysml::semantic_digest(&imported.semantics).unwrap()
    );
    assert_ne!(
        baseline().records[&RecordId(301)].fields.get("criterion"),
        Some(&Value::Null)
    );
}

#[test]
fn imported_members_namespace_roots_and_path_segments_cannot_be_shadowed() {
    let source = text(&baseline());
    let colliding = source
        .replacen(
            "package KernspanStage1Exchange {",
            "package KernspanStage1Exchange { private import ScalarValues::Real;",
            1,
        )
        .replacen("<ks_r_200> r_200", "<ks_r_200> Real", 1);
    assert_ne!(colliding, source);
    refused(&colliding, ExchangeCode::AmbiguousSymbol);
    let namespace = source.replacen("<ks_r_200> r_200", "<ks_r_200> SI", 1);
    refused(&namespace, ExchangeCode::AmbiguousSymbol);
    let quoted_fake_path = source.replace(
        "95[KernspanStage1DraftOntology::percent]",
        "95['KernspanStage1DraftOntology::percent']",
    );
    refused(&quoted_fake_path, ExchangeCode::UnsupportedConstruct);
    let mut model = baseline();
    attach(
        &mut model,
        301,
        BTreeMap::from([(
            "SI".into(),
            literal("1", QuantityKind::Scalar, UnitId::ScalarOne),
        )]),
        Expr::Binding("SI".into()),
        QuantityKind::Scalar,
        UnitId::ScalarOne,
    );
    let before = model.clone();
    assert_eq!(
        sysml::export(&model).unwrap_err()[0].code,
        ExchangeCode::AmbiguousSymbol
    );
    assert_eq!(model, before);
}

#[test]
fn scientific_scalar_extremes_preserve_unary_minus_and_bounded_semantic_roundtrip() {
    for decimal in ["-1e308", "-1e-308", "1e308", "1e-308"] {
        let mut model = baseline();
        attach(
            &mut model,
            301,
            BTreeMap::new(),
            calc::parse(decimal, &calc::Limits::default()).unwrap(),
            QuantityKind::Scalar,
            UnitId::ScalarOne,
        );
        let output = sysml::export(&model).unwrap();
        let imported = sysml::import("scientific.sysml", output.utf8.as_bytes()).unwrap();
        assert_eq!(
            output.semantic_digest,
            sysml::semantic_digest(&imported.semantics).unwrap()
        );
        let stamp = calc::DocumentStamp {
            document_id: "scientific".into(),
            instance_id: "test".into(),
            model_revision: 0,
            raw_version: 0,
            snapshot_hash: "x".into(),
        };
        let receipt = calc::evaluate(
            &imported.semantics.calculations,
            &BTreeMap::new(),
            &stamp,
            &calc::Limits::default(),
        );
        let actual = receipt.outcomes[&CalculationId(301)].value().unwrap().value;
        assert_eq!(actual, decimal.parse::<f64>().unwrap());
        assert!(actual.is_finite());
        assert_ne!(actual, 0.0);
    }
}

#[test]
fn aggregate_calculation_ast_limit_is_an_exchange_admission_boundary() {
    fn expression(depth: usize) -> Expr {
        if depth == 0 {
            return Expr::Binding("x".into());
        }
        Expr::Add(
            Box::new(expression(depth - 1)),
            Box::new(expression(depth - 1)),
        )
    }
    let mut model = baseline();
    for id in 301..365 {
        attach(
            &mut model,
            id,
            BTreeMap::from([(
                "x".into(),
                literal("1", QuantityKind::Scalar, UnitId::ScalarOne),
            )]),
            expression(9),
            QuantityKind::Scalar,
            UnitId::ScalarOne,
        );
    }
    assert!(
        calc::validate_document_resources(&model.calculations, &calc::Limits::default()).is_ok()
    );
    attach(
        &mut model,
        365,
        BTreeMap::from([(
            "x".into(),
            literal("1", QuantityKind::Scalar, UnitId::ScalarOne),
        )]),
        expression(9),
        QuantityKind::Scalar,
        UnitId::ScalarOne,
    );
    let before = model.clone();
    let errors = sysml::export(&model).unwrap_err();
    assert!(errors.iter().any(|error| error.code == ExchangeCode::Limit));
    assert_eq!(model, before);
}

#[test]
fn native_valid_result_binding_refuses_exchange_return_feature_collision_atomically() {
    let mut model = baseline();
    attach(
        &mut model,
        301,
        BTreeMap::from([(
            "result".into(),
            literal("2", QuantityKind::Scalar, UnitId::ScalarOne),
        )]),
        Expr::Binding("result".into()),
        QuantityKind::Scalar,
        UnitId::ScalarOne,
    );
    calc::validate(
        &model.calculations[&CalculationId(301)],
        &calc::Limits::default(),
    )
    .unwrap();
    let before = model.clone();
    assert!(sysml::export(&model)
        .unwrap_err()
        .iter()
        .any(|error| error.code == ExchangeCode::AmbiguousSymbol));
    assert_eq!(model, before);
    let mut ordinary = baseline();
    attach(
        &mut ordinary,
        301,
        BTreeMap::from([(
            "input".into(),
            literal("2", QuantityKind::Scalar, UnitId::ScalarOne),
        )]),
        Expr::Binding("input".into()),
        QuantityKind::Scalar,
        UnitId::ScalarOne,
    );
    let original = text(&ordinary);
    let collision = original.replace("'input'", "'result'");
    assert_ne!(collision, original);
    refused(&collision, ExchangeCode::AmbiguousSymbol);
}
