use mbse_model::{
    stage1::*,
    stage1_calc as c,
    types::{
        self as t, Id,
        parametrics::{
            self as p, Action, Form, SourceForm,
            analysis::{self as a, Assessment},
        },
    },
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[path = "support/parametric_source.rs"]
mod support;
use support::*;
fn check(name: &str) {
    let case = data(name);
    let (e, id, _) = fixture(name);
    let before = e.snapshot.clone();
    let program = a::prepare(&e, id).unwrap();
    assert_eq!(e.snapshot, before);
    let result = a::evaluate(&e, &program).unwrap();
    assert!(
        result.is_current(&e)
            && result.parametric_execution_performed
            && !result.verification_written
    );
    assert_eq!(e.snapshot, before);
    if let Some(code) = case.get("expected_failure") {
        assert_eq!(
            format!("{:?}", result.outcome.diagnostic().unwrap().code),
            code.as_str().unwrap()
        );
        assert!(matches!(result.assessment, Assessment::Unavailable(_)));
    } else {
        let value = result
            .outcome
            .value()
            .unwrap_or_else(|| panic!("{name}: {:?}", result.outcome));
        let expected = case["expected_value"].as_f64().unwrap();
        assert!(
            (value.value - expected).abs() <= 1e-12 + expected.abs() * 1e-12,
            "{name}: {} != {expected}",
            value.value
        );
        assert_eq!(value.quantity, q(unit(case["output"].as_str().unwrap())));
        if let Some(expected) = case.get("expected_assessment") {
            assert_eq!(
                matches!(result.assessment, Assessment::Met),
                expected.as_bool().unwrap()
            );
            assert!(!matches!(result.assessment, Assessment::Unavailable(_)));
        } else if let Some(code) = case.get("expected_assessment_failure") {
            assert!(
                matches!(result.assessment,Assessment::Unavailable(ref d)if format!("{:?}",d.code)==code.as_str().unwrap())
            );
        } else {
            assert!(
                matches!(result.assessment,Assessment::Unavailable(ref d)if d.code==c::DiagnosticCode::MissingReference)
            );
        }
    }
    assert_eq!(
        result.native_receipt.algorithm_revision,
        c::ALGORITHM_REVISION
    );
    assert_eq!(
        result.native_receipt.unit_table_revision,
        c::UNIT_TABLE_REVISION
    );
    assert_eq!(result.stamp.snapshot_hash, e.snapshot.digest());
}
macro_rules! fixed{($($test:ident=>$case:literal),*$(,)?)=>{$(#[test]fn $test(){check($case);})*}}
fixed! {
 parametric_fixed_length_conversion_and_lower_threshold=>"length_conversion",
 parametric_fixed_duration_conversion_without_invented_criterion=>"duration_conversion",
 parametric_fixed_speed_uses_existing_quantity_algebra=>"speed",
 parametric_fixed_ratio_percent_comparison_uses_native_units=>"ratio_percent",
 parametric_fixed_scalar_multiplication_and_equal_threshold=>"scalar_equal",
 parametric_fixed_count_literal_stays_exact_and_unassessed=>"count_literal",
 parametric_fixed_length_threshold_equality_converts_native_metres=>"length_equal",
 parametric_fixed_length_upper_threshold_computes_readonly_met=>"length_upper",
 parametric_fixed_zero_denominator_preserves_explicit_failure=>"zero_denominator",
 parametric_fixed_quantity_mismatch_never_becomes_false_assessment=>"quantity_mismatch",
 parametric_fixed_sqrt_domain_failure_stays_explicit=>"sqrt_domain",
 parametric_fixed_overflow_stays_explicit=>"overflow",
 parametric_fixed_underflow_stays_explicit=>"underflow",
 parametric_fixed_threshold_kind_mismatch_stays_unavailable=>"criterion_quantity_mismatch",
}
fn rebind(e: &mut Editor, specification: Id, parameter: Id, binding: Id, source: SourceForm) {
    accept(
        e,
        Form::Binding {
            specification,
            id: Some(binding),
            parameter: parameter.0.to_string(),
            source,
        },
    );
}
fn typed(e: &mut Editor, kind: t::Scalar, text: &str) -> (Id, Id) {
    let mut s = t::load(&e.snapshot).unwrap();
    let base = e.snapshot.next_id;
    let container = Id(base);
    let component = Id(base + 1);
    let value = Id(base + 2);
    let property = Id(base + 3);
    let part = Id(base + 4);
    for (id, name, kind) in [
        (container, "Container", t::Kind::Component),
        (component, "Target", t::Kind::Component),
        (value, "Numeric source", t::Kind::Value(kind)),
    ] {
        s.definitions.insert(
            id,
            t::Definition {
                id,
                system: RecordId(1),
                name: name.into(),
                kind,
            },
        );
    }
    s.properties.insert(
        property,
        t::Property {
            id: property,
            owner: component,
            name: "Explicit source value".into(),
            value_type: value,
            multiplicity: t::Multiplicity {
                lower: 1,
                upper: Some(1),
            },
        },
    );
    s.usages.insert(
        part,
        t::Usage {
            id: part,
            name: "Exact Part context".into(),
            owner: container,
            target: component,
            kind: t::UsageKind::Part,
        },
    );
    s.defaults.insert(
        property,
        t::values::Literal {
            kind,
            text: text.into(),
        },
    );
    e.snapshot.next_id = base + 5;
    e.snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert(t::NAMESPACE.into(), serde_json::to_value(s).unwrap());
    e.snapshot.validate().unwrap();
    (property, part)
}
#[test]
fn parametric_typed_override_changes_only_ephemeral_inputs_and_records_inherited_and_explicit_origins()
 {
    let (mut e, id, keys) = fixture("length_conversion");
    let (property, part) = typed(&mut e, t::Scalar::Real, "17");
    let (parameter, binding) = keys["d"];
    rebind(
        &mut e,
        id,
        parameter,
        binding,
        SourceForm::TypedValue {
            property: property.0.to_string(),
            usage: part.0.to_string(),
            unit: c::UnitId::Metre,
        },
    );
    let before = e.snapshot.clone();
    let result = a::evaluate(&e, &a::prepare(&e, id).unwrap()).unwrap();
    assert_eq!(result.outcome.value().unwrap().value, 0.017);
    assert_eq!(
        result.bindings[0].value_origin,
        Some(t::values::Origin::Default)
    );
    assert_eq!(e.snapshot, before);
    assert_eq!(
        e.snapshot.calculations().unwrap()[&c::CalculationId(100)].bindings["d"],
        c::Input::Literal(c::Literal {
            decimal: "2500".into(),
            quantity: q(c::UnitId::Metre)
        })
    );
    assert!(matches!(
        result.bindings[0].source,
        p::Source::TypedValue { .. }
    ));
    let target = t::values::Target::Override {
        usage: part,
        property,
    };
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::Value(t::values::Action::Begin(target))),
    )
    .unwrap();
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::Value(t::values::Action::Set(t::values::Form {
            target,
            text: "19".into(),
            expression: "not executed scratch".into(),
        }))),
    )
    .unwrap();
    assert!(a::prepare(&e, id).is_err());
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::Value(t::values::Action::Apply(target))),
    )
    .unwrap();
    let before = e.snapshot.clone();
    let r = a::evaluate(&e, &a::prepare(&e, id).unwrap()).unwrap();
    assert_eq!(r.outcome.value().unwrap().value, 0.019);
    assert_eq!(
        r.bindings[0].value_origin,
        Some(t::values::Origin::Override)
    );
    assert_eq!(e.snapshot, before);
    assert!(!result.is_current(&e));
}
#[test]
fn parametric_pending_typed_literal_and_integer_precision_cannot_fall_back_to_accepted_values() {
    let (mut e, id, keys) = fixture("length_conversion");
    let (property, _) = typed(&mut e, t::Scalar::Real, "17");
    let (parameter, binding) = keys["d"];
    rebind(
        &mut e,
        id,
        parameter,
        binding,
        SourceForm::TypedValue {
            property: property.0.to_string(),
            usage: String::new(),
            unit: c::UnitId::Metre,
        },
    );
    let program = a::prepare(&e, id).unwrap();
    let target = t::values::Target::Default { property };
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::Value(t::values::Action::Begin(target))),
    )
    .unwrap();
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::Value(t::values::Action::Set(t::values::Form {
            target,
            text: "1e".into(),
            expression: String::new(),
        }))),
    )
    .unwrap();
    let before = e.snapshot.clone();
    assert!(
        a::prepare(&e, id)
            .unwrap_err()
            .message
            .contains("TYPED_VALUE_DRAFT_PENDING")
    );
    assert!(
        a::evaluate(&e, &program)
            .unwrap_err()
            .message
            .contains("STALE")
    );
    assert_eq!(e.snapshot, before);
    let (mut e, id, keys) = fixture("length_conversion");
    let (property, _) = typed(&mut e, t::Scalar::Integer, "9007199254740993");
    let (parameter, binding) = keys["d"];
    rebind(
        &mut e,
        id,
        parameter,
        binding,
        SourceForm::TypedValue {
            property: property.0.to_string(),
            usage: String::new(),
            unit: c::UnitId::Metre,
        },
    );
    assert!(
        a::prepare(&e, id)
            .unwrap_err()
            .message
            .contains("INTEGER_PRECISION")
    );
}
#[test]
fn parametric_self_output_cycle_is_reported_by_native_engine_without_source_mutation() {
    let (mut e, id, keys) = fixture("scalar_equal");
    let (parameter, binding) = keys["a"];
    rebind(
        &mut e,
        id,
        parameter,
        binding,
        SourceForm::NativeOutput {
            calculation: "100".into(),
        },
    );
    let before = e.snapshot.clone();
    let prepared = a::prepare(&e, id).unwrap();
    let result = a::evaluate(&e, &prepared).unwrap();
    assert_eq!(
        result.outcome.diagnostic().unwrap().code,
        c::DiagnosticCode::Cycle
    );
    assert!(
        matches!(result.assessment,Assessment::Unavailable(ref d)if d.cause==Some(c::DiagnosticCode::Cycle))
    );
    assert_eq!(e.snapshot, before);
}
#[test]
fn parametric_dependency_outputs_require_current_formula_but_direct_literal_sources_have_precise_scope()
 {
    let (mut e, id, keys) = fixture("length_conversion");
    let mut second = data("length_conversion");
    second["inputs"]["d"]["decimal"] = json!("3000");
    second["output"] = json!("Metre");
    e.snapshot
        .records
        .insert(RecordId(200), native(200, 2, &second));
    e.snapshot.next_id = 201;
    e.snapshot.validate().unwrap();
    let (parameter, binding) = keys["d"];
    rebind(
        &mut e,
        id,
        parameter,
        binding,
        SourceForm::NativeOutput {
            calculation: "200".into(),
        },
    );
    let before = e.snapshot.clone();
    let run = a::evaluate(&e, &a::prepare(&e, id).unwrap()).unwrap();
    assert_eq!(run.outcome.value().unwrap().value, 3.);
    assert!(matches!(run.assessment, Assessment::Met));
    assert_eq!(run.native_receipt.outcomes.len(), 2);
    assert_eq!(e.snapshot, before);
    e.command(
        &e.freshness(),
        Command::EditText {
            key: RawKey {
                record: RecordId(200),
                field: "calc.expression".into(),
                item: None,
            },
            text: "d +".into(),
        },
    )
    .unwrap();
    assert!(
        a::prepare(&e, id)
            .unwrap_err()
            .message
            .contains("NATIVE_RAW_PENDING")
    );
    rebind(
        &mut e,
        id,
        parameter,
        binding,
        SourceForm::NativeInput {
            calculation: "200".into(),
            name: "d".into(),
        },
    );
    let r = a::evaluate(&e, &a::prepare(&e, id).unwrap()).unwrap();
    assert_eq!(r.outcome.value().unwrap().value, 3.);
    assert_eq!(r.native_receipt.outcomes.len(), 1);
}
#[test]
fn parametric_native_input_raw_and_current_configuration_proposals_block_preparation() {
    let (mut e, id, _) = fixture("length_conversion");
    e.command(
        &e.freshness(),
        Command::EditText {
            key: RawKey {
                record: RecordId(100),
                field: "calc.input.d".into(),
                item: None,
            },
            text: "1e".into(),
        },
    )
    .unwrap();
    let before = e.snapshot.clone();
    assert!(
        a::prepare(&e, id)
            .unwrap_err()
            .message
            .contains("NATIVE_RAW_PENDING")
    );
    assert_eq!(e.snapshot, before);
    let (mut e, id, _) = fixture("length_conversion");
    let key = e.snapshot.calculation_config_id(RecordId(100)).unwrap();
    e.command(
        &e.freshness(),
        Command::PutCreationDraft {
            id: key,
            draft: CreationDraft {
                owner: RecordId(100),
                kind: RecordKind::Measure,
                fields: BTreeMap::from([
                    ("profile".into(), "kernspan.stage1.calc-config/1".into()),
                    ("name".into(), "new_input".into()),
                    ("decimal".into(), "1e".into()),
                ]),
            },
        },
    )
    .unwrap();
    assert!(
        a::prepare(&e, id)
            .unwrap_err()
            .message
            .contains("NATIVE_CONFIG_PENDING")
    );
}
#[test]
fn parametric_selected_scope_proposals_block_but_unrelated_incomplete_and_retired_scopes_are_retained()
 {
    let (mut e, id, keys) = fixture("length_conversion");
    let other = accept(
        &mut e,
        Form::Specification {
            id: None,
            system: "1".into(),
            name: "Incomplete other scope".into(),
            calculation: String::new(),
        },
    );
    run(
        &mut e,
        Action::Set(Form::Binding {
            specification: other,
            id: None,
            parameter: "?".into(),
            source: SourceForm::NativeOutput {
                calculation: String::new(),
            },
        }),
    )
    .unwrap();
    e.snapshot.creation_drafts.insert(
        "calc-config:100:99".into(),
        CreationDraft {
            owner: RecordId(100),
            kind: RecordKind::Measure,
            fields: BTreeMap::from([
                ("profile".into(), "kernspan.stage1.calc-config/1".into()),
                ("name".into(), "retired pending".into()),
            ]),
        },
    );
    e.snapshot.validate().unwrap();
    assert!(a::prepare(&e, id).is_ok());
    let (parameter, _) = keys["d"];
    let mut f =
        p::accepted_form(&t::load(&e.snapshot).unwrap().parametrics[&id], parameter).unwrap();
    if let Form::Parameter { name, .. } = &mut f {
        *name = "d +".into();
    }
    run(&mut e, Action::Set(f)).unwrap();
    let before = e.snapshot.clone();
    assert!(
        a::prepare(&e, id)
            .unwrap_err()
            .message
            .contains("PROPOSAL_PENDING")
    );
    assert_eq!(e.snapshot, before);
}
#[test]
fn parametric_stale_same_counter_and_foreign_runtime_programs_never_compute_and_historic_receipts_remain_exact()
 {
    let (mut e, id, _) = fixture("length_conversion");
    let p = a::prepare(&e, id).unwrap();
    let r = a::evaluate(&e, &p).unwrap();
    let old = serde_json::to_vec(&r).unwrap();
    let foreign = Editor::new(e.snapshot.clone(), "different-runtime".into()).unwrap();
    assert!(!p.is_current(&foreign) && !r.is_current(&foreign));
    assert!(a::evaluate(&foreign, &p).is_err());
    e.snapshot.records.get_mut(&RecordId(100)).unwrap().name =
        "Different source at same counters".into();
    assert!(!p.is_current(&e) && !r.is_current(&e));
    assert!(a::evaluate(&e, &p).unwrap_err().message.contains("STALE"));
    assert_eq!(serde_json::to_vec(&r).unwrap(), old);
}
#[test]
fn parametric_criterion_raw_failure_does_not_replace_numeric_result_or_write_verification() {
    let (mut e, id, _) = fixture("length_conversion");
    e.command(
        &e.freshness(),
        Command::EditText {
            key: RawKey {
                record: RecordId(100),
                field: "criterion".into(),
                item: None,
            },
            text: "9e".into(),
        },
    )
    .unwrap();
    let before = e.snapshot.clone();
    let r = a::evaluate(&e, &a::prepare(&e, id).unwrap()).unwrap();
    assert_eq!(r.outcome.value().unwrap().value, 2.5);
    assert!(
        matches!(r.assessment,Assessment::Unavailable(ref d)if d.code==c::DiagnosticCode::IncompleteInput)
    );
    assert!(!r.verification_written);
    assert_eq!(e.snapshot, before);
    assert_eq!(
        e.snapshot.records[&RecordId(100)].number("criterion"),
        Some(3000.)
    );
}
#[test]
fn parametric_analysis_preserves_entire_prior_native_raw_opaque_and_eight_maps_across_bound_cold_recovery()
 {
    use mbse_model::{
        native_views::{self as v, Views},
        native_views_io as io,
        stage1_io::{Session, execute_write},
    };
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    let root: Value = serde_json::from_slice(include_bytes!(
        "../../../docs/parity/p35b/verification/native-02/run/parametric-final-cold-copy.kernspan"
    ))
    .unwrap();
    session.editor = Editor::new(
        serde_json::from_value(root["snapshot"].clone()).unwrap(),
        session.editor.freshness().instance_id,
    )
    .unwrap();
    let before = session.editor.snapshot.clone();
    let old = t::load(&before).unwrap();
    let (id, _) = selected(&mut session.editor, 1003);
    let source = session.editor.snapshot.clone();
    let program = a::prepare(&session.editor, id).unwrap();
    let result = a::evaluate(&session.editor, &program).unwrap();
    assert_eq!(result.outcome.value().unwrap().value, 2.5);
    assert!(
        matches!(result.assessment,Assessment::Unavailable(ref d)if d.code==c::DiagnosticCode::IncompleteInput)
    );
    assert_eq!(session.editor.snapshot, source);
    let mut types = t::load(&source).unwrap();
    types.parametrics.remove(&id);
    assert_eq!(types, old);
    assert_eq!(source.records, before.records);
    assert_eq!(source.drafts, before.drafts);
    assert_eq!(source.creation_drafts, before.creation_drafts);
    let mut doc:v::Document=serde_json::from_slice(include_bytes!("../../../docs/parity/p35b/verification/native-02/run/parametric-final-cold-copy.mbse-views.json")).unwrap();
    doc.profile = v::scenarios::RECOVERY_PROFILE.into();
    doc.binding = None;
    let views = Views::restore(&doc.encoded().unwrap(), &session.editor).unwrap();
    let maps = [
        "views",
        "occurrences",
        "edges",
        "typed_ports",
        "connector_edges",
        "activity_diagrams",
        "state_machine_diagrams",
        "scenario_diagrams",
    ];
    let expected = serde_json::to_value(views.document()).unwrap();
    let file = area.path().join("analysis.kernspan");
    let job = session.begin_save(&file).unwrap();
    assert!(session.complete(execute_write(job)));
    let side = area.path().join("analysis.mbse-views.json");
    let mut writes = io::Io::default();
    let job = writes
        .begin(&session, &views, &side, io::Kind::Bound)
        .unwrap();
    assert!(writes.complete(&session, &views, io::execute(job)));
    let cold = Session::open(area.path(), &file).unwrap();
    assert_eq!(cold.editor.snapshot, source);
    assert!(!program.is_current(&cold.editor) && !result.is_current(&cold.editor));
    assert!(a::evaluate(&cold.editor, &program).is_err());
    let fresh = a::prepare(&cold.editor, id).unwrap();
    assert_eq!(
        a::evaluate(&cold.editor, &fresh).unwrap().outcome,
        result.outcome
    );
    let decoded = Views::decode(
        &std::fs::read(side).unwrap(),
        &cold.editor,
        &cold.native_checkpoint().unwrap().unwrap(),
    )
    .unwrap();
    let actual = serde_json::to_value(decoded.document()).unwrap();
    for k in maps {
        assert_eq!(actual.get(k), expected.get(k));
    }
    let job = session.begin_recovery().unwrap();
    assert!(session.complete(execute_write(job)));
    let restored = Session::restore(area.path(), session.last_recovery().unwrap()).unwrap();
    let mut recovered = restored.editor.snapshot;
    recovered.document_id = source.document_id.clone();
    assert_eq!(recovered, source);
    let recovery = area.path().join("analysis-recovery.mbse-views.json");
    let destination = writes
        .freeze_destination(&session, &views, &recovery, io::Kind::Recovery)
        .unwrap();
    let job = writes
        .begin_captured(
            &session,
            &views,
            views.capture(&session.editor),
            destination,
        )
        .unwrap();
    assert!(writes.complete(&session, &views, io::execute(job)));
    let recovered = Views::restore(&std::fs::read(recovery).unwrap(), &session.editor).unwrap();
    let actual = serde_json::to_value(recovered.document()).unwrap();
    for k in maps {
        assert_eq!(actual.get(k), expected.get(k));
    }
}
