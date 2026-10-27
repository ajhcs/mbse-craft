use mbse_model::{
    extensions::Extensions,
    stage1::*,
    stage1_calc as c,
    stage1_io::{Session, execute_write},
    types::{
        self as t, Id,
        parametrics::{self as p, Action, Form, Source, SourceForm},
    },
};
use serde_json::{Value, json};
fn editor() -> Editor {
    let root: Value = serde_json::from_slice(include_bytes!(
        "../../../docs/parity/p34c/verification/native-02/run/scenario-final-cold-copy.kernspan"
    ))
    .unwrap();
    Editor::new(
        serde_json::from_value(root["snapshot"].clone()).unwrap(),
        "parametric:runtime".into(),
    )
    .unwrap()
}
fn run(e: &mut Editor, a: Action) -> Result<(), String> {
    e.command(&e.freshness(), Command::Typed(t::Action::Parametric(a)))
        .map(|_| ())
}
fn set(e: &mut Editor, f: Form) -> String {
    let key = f.key();
    run(e, Action::Set(f)).unwrap();
    key
}
fn accept(e: &mut Editor, f: Form) -> Id {
    let id = f.id().unwrap_or(Id(e.snapshot.next_id));
    let key = set(e, f);
    run(e, Action::Apply(key)).unwrap();
    id
}
fn q(unit: c::UnitId) -> c::QuantityType {
    c::QuantityType {
        kind: c::unit_kind(unit),
        unit,
    }
}
fn spec(e: &mut Editor, calc: bool) -> Id {
    accept(
        e,
        Form::Specification {
            id: None,
            system: "1".into(),
            name: "Documentary distance relation".into(),
            calculation: if calc { "1003".into() } else { String::new() },
        },
    )
}
fn parameter(e: &mut Editor, specification: Id, name: &str, unit: c::UnitId) -> Id {
    accept(
        e,
        Form::Parameter {
            specification,
            id: None,
            name: name.into(),
            quantity: q(unit),
        },
    )
}
fn binding(e: &mut Editor, specification: Id, parameter: Id, source: SourceForm) -> Id {
    accept(
        e,
        Form::Binding {
            specification,
            id: None,
            parameter: parameter.0.to_string(),
            source,
        },
    )
}
fn input(name: &str) -> SourceForm {
    SourceForm::NativeInput {
        calculation: "1003".into(),
        name: name.into(),
    }
}
fn graph() -> (Editor, Id, Id, Id, Id, Id) {
    let mut e = editor();
    let a = spec(&mut e, true);
    let x = parameter(&mut e, a, "x", c::UnitId::Kilometre);
    let duration = parameter(&mut e, a, "x1", c::UnitId::Second);
    let b = binding(&mut e, a, x, input("x"));
    let b1 = binding(&mut e, a, duration, input("x1"));
    (e, a, x, duration, b, b1)
}
fn put(e: &mut Editor, s: &t::Store) {
    e.snapshot
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert(t::NAMESPACE.into(), serde_json::to_value(s).unwrap());
}
#[test]
fn parametric_specs_and_unbound_parameters_persist_without_calculation_or_execution() {
    let mut e = editor();
    let native = e.snapshot.records.clone();
    let before = e.snapshot.calculations().unwrap();
    let a = spec(&mut e, false);
    let x = parameter(&mut e, a, "distance", c::UnitId::Metre);
    let report = p::report(&e.snapshot, a).unwrap();
    assert_eq!(report.unbound, vec![x]);
    assert!(!report.execution_performed && !report.accepted_bindings_complete);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "PARAMETRIC_CALCULATION_UNASSIGNED")
    );
    let cold: Snapshot = serde_json::from_slice(&serde_json::to_vec(&e.snapshot).unwrap()).unwrap();
    cold.validate().unwrap();
    assert_eq!(p::report(&cold, a).unwrap(), report);
    assert_eq!(e.snapshot.records, native);
    assert_eq!(e.snapshot.calculations().unwrap(), before);
}
#[test]
fn parametric_native_input_output_bindings_keep_exact_ids_units_and_source_authority() {
    let (e, a, x, _, b, _) = graph();
    let report = p::report(&e.snapshot, a).unwrap();
    assert!(report.accepted_bindings_complete && !report.execution_performed);
    assert_eq!(report.snapshot_hash, e.snapshot.digest());
    assert_eq!(report.unit_table_revision, c::UNIT_TABLE_REVISION);
    let original = e.snapshot.records.clone();
    let mut e = e;
    let key = set(
        &mut e,
        Form::Binding {
            specification: a,
            id: Some(b),
            parameter: x.0.to_string(),
            source: SourceForm::NativeOutput {
                calculation: "1003".into(),
            },
        },
    );
    run(&mut e, Action::Apply(key)).unwrap();
    assert!(
        p::report(&e.snapshot, a)
            .unwrap()
            .accepted_bindings_complete
    );
    assert_eq!(
        t::load(&e.snapshot).unwrap().parametrics[&a].bindings[&b].source,
        Source::NativeOutput {
            calculation: c::CalculationId(1003)
        }
    );
    assert_eq!(e.snapshot.records, original);
}
#[test]
fn parametric_unbound_and_undeclared_native_inputs_are_visible_without_native_default_fallback() {
    let mut e = editor();
    let a = spec(&mut e, true);
    let x = parameter(&mut e, a, "x", c::UnitId::Metre);
    let report = p::report(&e.snapshot, a).unwrap();
    assert_eq!(report.unbound, vec![x]);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "PARAMETRIC_INPUT_UNDECLARED" && d.message.contains("x1"))
    );
    let before = e.snapshot.clone();
    let _ = p::report(&e.snapshot, a).unwrap();
    assert_eq!(e.snapshot, before);
    let key = set(
        &mut e,
        Form::Parameter {
            specification: a,
            id: None,
            name: "wrong_input".into(),
            quantity: q(c::UnitId::Metre),
        },
    );
    let before = e.snapshot.clone();
    assert!(
        run(&mut e, Action::Apply(key))
            .unwrap_err()
            .contains("DECLARED_INPUT_MISSING")
    );
    assert_eq!(e.snapshot, before);
}
#[test]
fn parametric_wrong_dimension_noncanonical_reference_and_duplicate_binding_refuse_atomically() {
    let (mut e, a, x, _, b, _) = graph();
    let original = t::load(&e.snapshot).unwrap();
    for source in [
        input("x1"),
        SourceForm::NativeOutput {
            calculation: "01003".into(),
        },
        SourceForm::NativeInput {
            calculation: "1003".into(),
            name: "X".into(),
        },
    ] {
        let key = set(
            &mut e,
            Form::Binding {
                specification: a,
                id: Some(b),
                parameter: x.0.to_string(),
                source,
            },
        );
        let before = e.snapshot.clone();
        assert!(run(&mut e, Action::Apply(key)).is_err());
        assert_eq!(e.snapshot, before);
        assert_eq!(t::load(&e.snapshot).unwrap(), original);
    }
    let key = set(
        &mut e,
        Form::Binding {
            specification: a,
            id: None,
            parameter: x.0.to_string(),
            source: input("x"),
        },
    );
    let before = e.snapshot.clone();
    assert!(
        run(&mut e, Action::Apply(key))
            .unwrap_err()
            .contains("DUPLICATE_BINDING")
    );
    assert_eq!(e.snapshot, before);
    let key = set(
        &mut e,
        Form::Parameter {
            specification: a,
            id: Some(x),
            name: "x".into(),
            quantity: q(c::UnitId::Second),
        },
    );
    assert!(
        run(&mut e, Action::Apply(key))
            .unwrap_err()
            .contains("QUANTITY_KIND_MISMATCH")
    );
}
#[test]
fn parametric_typed_value_default_and_part_override_bindings_use_authored_units_and_keep_unset_visible()
 {
    let (mut e, a, x, _, b, _) = graph();
    let records = e.snapshot.records.clone();
    for (property, usage, unset) in [
        ("1006", "", true),
        ("1020", "", false),
        ("1020", "1009", false),
    ] {
        let key = set(
            &mut e,
            Form::Binding {
                specification: a,
                id: Some(b),
                parameter: x.0.to_string(),
                source: SourceForm::TypedValue {
                    property: property.into(),
                    usage: usage.into(),
                    unit: c::UnitId::Metre,
                },
            },
        );
        run(&mut e, Action::Apply(key)).unwrap();
        let r = p::report(&e.snapshot, a).unwrap();
        assert_eq!(r.accepted_bindings_complete, !unset);
        assert_eq!(
            r.diagnostics
                .iter()
                .any(|d| d.code == "PARAMETRIC_BOUND_VALUE_UNSET"),
            unset
        );
        assert!(!r.execution_performed);
    }
    let key = set(
        &mut e,
        Form::Binding {
            specification: a,
            id: Some(b),
            parameter: x.0.to_string(),
            source: SourceForm::TypedValue {
                property: "1020".into(),
                usage: "1011".into(),
                unit: c::UnitId::Metre,
            },
        },
    );
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::Apply(key)).is_err());
    assert_eq!(e.snapshot, before);
    assert_eq!(e.snapshot.records, records);
}
#[test]
fn parametric_boolean_text_and_foreign_value_sources_are_actionable_without_coercion() {
    let (mut e, a, x, _, b, _) = graph();
    let mut s = t::load(&e.snapshot).unwrap();
    let value = Id(e.snapshot.next_id);
    let prop = Id(value.0 + 1);
    let template = s.definitions[&Id(1005)].clone();
    s.definitions.insert(
        value,
        t::Definition {
            id: value,
            name: "Boolean source".into(),
            kind: t::Kind::Value(t::Scalar::Boolean),
            ..template
        },
    );
    s.properties.insert(
        prop,
        t::Property {
            id: prop,
            name: "Boolean source property".into(),
            value_type: value,
            ..s.properties[&Id(1020)].clone()
        },
    );
    e.snapshot.next_id = prop.0 + 1;
    put(&mut e, &s);
    e.snapshot.validate().unwrap();
    let key = set(
        &mut e,
        Form::Binding {
            specification: a,
            id: Some(b),
            parameter: x.0.to_string(),
            source: SourceForm::TypedValue {
                property: prop.0.to_string(),
                usage: "".into(),
                unit: c::UnitId::Metre,
            },
        },
    );
    assert!(
        run(&mut e, Action::Apply(key))
            .unwrap_err()
            .contains("NUMERIC_SCALAR")
    );
    let mut changed = t::load(&e.snapshot).unwrap();
    changed.definitions.get_mut(&value).unwrap().kind = t::Kind::Value(t::Scalar::Text);
    put(&mut e, &changed);
    e.snapshot.validate().unwrap();
    let key = set(
        &mut e,
        Form::Binding {
            specification: a,
            id: Some(b),
            parameter: x.0.to_string(),
            source: SourceForm::TypedValue {
                property: prop.0.to_string(),
                usage: "".into(),
                unit: c::UnitId::Metre,
            },
        },
    );
    assert!(
        run(&mut e, Action::Apply(key))
            .unwrap_err()
            .contains("NUMERIC_SCALAR")
    );
    let mut foreign = e.snapshot.records[&RecordId(1)].clone();
    foreign.id = RecordId(20000);
    foreign.scope = foreign.id;
    e.snapshot.records.insert(foreign.id, foreign);
    let mut s = t::load(&e.snapshot).unwrap();
    let component = Id(20001);
    let value = Id(20002);
    let prop = Id(20003);
    s.definitions.insert(
        component,
        t::Definition {
            id: component,
            system: RecordId(20000),
            name: "Foreign component".into(),
            kind: t::Kind::Component,
        },
    );
    s.definitions.insert(
        value,
        t::Definition {
            id: value,
            system: RecordId(20000),
            name: "Foreign value".into(),
            kind: t::Kind::Value(t::Scalar::Real),
        },
    );
    s.properties.insert(
        prop,
        t::Property {
            id: prop,
            owner: component,
            name: "Foreign length".into(),
            value_type: value,
            multiplicity: t::Multiplicity {
                lower: 1,
                upper: Some(1),
            },
        },
    );
    e.snapshot.next_id = 20004;
    put(&mut e, &s);
    e.snapshot.validate().unwrap();
    let key = set(
        &mut e,
        Form::Binding {
            specification: a,
            id: Some(b),
            parameter: x.0.to_string(),
            source: SourceForm::TypedValue {
                property: prop.0.to_string(),
                usage: "".into(),
                unit: c::UnitId::Metre,
            },
        },
    );
    assert!(
        run(&mut e, Action::Apply(key))
            .unwrap_err()
            .contains("FOREIGN_SYSTEM")
    );
}
#[test]
fn parametric_removed_or_changed_calculation_retains_identity_bindings_and_diagnostics() {
    let (mut e, a, _, _, _, _) = graph();
    let mut unused = e.snapshot.records[&RecordId(1003)].clone();
    unused.id = RecordId(20000);
    unused.name = "Independent native calculation source".into();
    unused.fields.remove("_kernspanCalculationRawOwners");
    unused.fields.get_mut("calculation").unwrap()["id"] = json!(20000);
    unused.fields.get_mut("calculation").unwrap()["owner"] = json!(20000);
    e.snapshot.records.insert(unused.id, unused);
    e.snapshot.next_id = 20001;
    let mut store = t::load(&e.snapshot).unwrap();
    let owned = store.parametrics.get_mut(&a).unwrap();
    owned.calculation = Some(c::CalculationId(20000));
    for b in owned.bindings.values_mut() {
        if let Source::NativeInput { calculation, .. } | Source::NativeOutput { calculation } =
            &mut b.source
        {
            *calculation = c::CalculationId(20000);
        }
    }
    put(&mut e, &store);
    e.snapshot.validate().unwrap();
    let accepted = t::load(&e.snapshot).unwrap().parametrics;
    let removed = e.snapshot.records.remove(&RecordId(20000)).unwrap();
    e.snapshot.validate().unwrap();
    let r = p::report(&e.snapshot, a).unwrap();
    assert!(!r.accepted_bindings_complete);
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "PARAMETRIC_CALCULATION_UNRESOLVED")
    );
    assert_eq!(t::load(&e.snapshot).unwrap().parametrics, accepted);
    e.snapshot.records.insert(removed.id, removed);
    let record = e.snapshot.records.get_mut(&RecordId(20000)).unwrap();
    record.fields.get_mut("calculation").unwrap()["bindings"]
        .as_object_mut()
        .unwrap()
        .remove("x");
    e.snapshot.validate().unwrap();
    let report = p::report(&e.snapshot, a).unwrap();
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("INPUT_MISSING"))
    );
    assert_eq!(t::load(&e.snapshot).unwrap().parametrics, accepted);
}
#[test]
fn parametric_model_history_retains_newer_invalid_raw_and_in_use_removals_are_explicit() {
    let (mut e, a, x, _, b, _) = graph();
    let key = set(
        &mut e,
        Form::Binding {
            specification: a,
            id: Some(b),
            parameter: "not a reference".into(),
            source: input("x"),
        },
    );
    let forms = p::forms(&e.snapshot).unwrap();
    let before = e.snapshot.clone();
    assert!(run(&mut e, Action::Apply(key.clone())).is_err());
    assert_eq!(e.snapshot, before);
    assert!(
        run(
            &mut e,
            Action::Remove {
                specification: a,
                element: x
            }
        )
        .unwrap_err()
        .contains("IN_USE")
    );
    run(
        &mut e,
        Action::Remove {
            specification: a,
            element: b,
        },
    )
    .unwrap();
    let high = e.snapshot.next_id;
    e.undo_model(&e.freshness()).unwrap();
    assert_eq!(p::forms(&e.snapshot).unwrap(), forms);
    assert_eq!(e.snapshot.next_id, high);
    e.redo_model(&e.freshness()).unwrap();
    run(
        &mut e,
        Action::Remove {
            specification: a,
            element: x,
        },
    )
    .unwrap();
    run(
        &mut e,
        Action::Remove {
            specification: a,
            element: a,
        },
    )
    .unwrap();
    assert!(t::load(&e.snapshot).unwrap().parametrics.is_empty());
    assert_eq!(p::forms(&e.snapshot).unwrap(), forms);
    run(&mut e, Action::Discard(key)).unwrap();
}
#[test]
fn parametric_wire_schema_canonical_nested_ids_global_collisions_and_future_reference_reservation_are_strict()
 {
    let (e, a, x, _, b, _) = graph();
    let original = serde_json::to_value(&e.snapshot).unwrap();
    let base = original["extensions"]["namespaces"][t::NAMESPACE].clone();
    let path = a.0.to_string();
    for mode in 0..6 {
        let mut wire = original.clone();
        let s = &mut wire["extensions"]["namespaces"][t::NAMESPACE];
        match mode {
            0 => {
                s["parametrics"][&path]["future"] = json!(true);
            }
            1 => {
                let root = s["parametrics"].as_object_mut().unwrap();
                let v = root.remove(&path).unwrap();
                root.insert(format!("0{path}"), v);
            }
            2 => {
                let root = s["parametrics"][&path]["parameters"]
                    .as_object_mut()
                    .unwrap();
                let v = root.remove(&x.0.to_string()).unwrap();
                root.insert(format!("0{}", x.0), v);
            }
            3 | 5 => {
                let target = if mode == 3 { 1003 } else { 1005 };
                let parameters = s["parametrics"][&path]["parameters"]
                    .as_object_mut()
                    .unwrap();
                let mut p = parameters.remove(&x.0.to_string()).unwrap();
                p["id"] = json!(target);
                parameters.insert(target.to_string(), p);
                s["parametrics"][&path]["bindings"][b.0.to_string()]["parameter"] = json!(target);
            }
            _ => {
                s["parametrics"][&path]["parameters"][x.0.to_string()]["quantity"]["unit"] =
                    json!("Second");
            }
        }
        let result = serde_json::from_value::<Snapshot>(wire);
        assert!(result.is_err() || result.unwrap().validate().is_err());
    }
    assert_eq!(original["extensions"]["namespaces"][t::NAMESPACE], base);
    let mut future = e.snapshot.clone();
    let mut s = t::load(&future).unwrap();
    s.parametrics
        .get_mut(&a)
        .unwrap()
        .bindings
        .get_mut(&b)
        .unwrap()
        .source = Source::NativeOutput {
        calculation: c::CalculationId(999999),
    };
    future
        .extensions
        .as_mut()
        .unwrap()
        .namespaces
        .insert(t::NAMESPACE.into(), serde_json::to_value(s).unwrap());
    assert!(future.validate().is_err());
    future.next_id = 1000000;
    future.validate().unwrap();
    assert!(
        p::report(&future, a)
            .unwrap()
            .diagnostics
            .iter()
            .any(|d| d.code == "PARAMETRIC_BINDING_UNRESOLVED")
    );
    let mut reserved = Editor::new(future, "future:allocator".into()).unwrap();
    let next = spec(&mut reserved, false);
    assert_eq!(next, Id(1000000));
    assert!(
        p::report(&reserved.snapshot, a)
            .unwrap()
            .diagnostics
            .iter()
            .any(|d| d.code == "PARAMETRIC_BINDING_UNRESOLVED")
    );
    let mut wire = serde_json::to_value(&e.snapshot).unwrap();
    wire["extensions"]["namespaces"][t::DRAFTS]["scenario_forms"][format!("scenario:{}", a.0)] =
        serde_json::to_value(t::scenarios::Form::Scenario {
            id: Some(a),
            system: "1".into(),
            name: "Foreign raw collision".into(),
        })
        .unwrap();
    let s: Snapshot = serde_json::from_value(wire).unwrap();
    assert!(s.validate().unwrap_err().contains("COLLISION"));
}
#[test]
fn parametric_raw_utf8_quota_shared512_and_native_binding_limits_refuse_without_partial_acceptance()
{
    let mut e = editor();
    let a = spec(&mut e, false);
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            Action::Set(Form::Specification {
                id: Some(a),
                system: "1".into(),
                name: "é".repeat(2500),
                calculation: "".into()
            })
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    e.snapshot.next_id = 50000; // Explicit retained raw IDs are already retired, not fresh allocations.
    for i in 0..127 {
        set(
            &mut e,
            Form::Specification {
                id: Some(Id(30000 + i)),
                system: "1".into(),
                name: "retained raw".into(),
                calculation: "".into(),
            },
        );
    }
    assert_eq!(p::forms(&e.snapshot).unwrap().len(), 128);
    let before = e.snapshot.clone();
    assert!(
        run(
            &mut e,
            Action::Set(Form::Specification {
                id: Some(Id(40000)),
                system: "1".into(),
                name: "overflow".into(),
                calculation: "".into()
            })
        )
        .is_err()
    );
    assert_eq!(e.snapshot, before);
    let mut n = Snapshot::new(DocumentId("parametric:capacity".into()));
    n.extensions = Some(Extensions::default());
    let mut s = t::Store::default();
    for i in 2..513 {
        s.parametrics.insert(
            Id(i),
            p::Specification {
                id: Id(i),
                system: RecordId(1),
                name: format!("Constraint {i}"),
                calculation: None,
                parameters: Default::default(),
                bindings: Default::default(),
            },
        );
    }
    s.definitions.insert(
        Id(513),
        t::Definition {
            id: Id(513),
            system: RecordId(1),
            name: "Existing component shares the same capacity".into(),
            kind: t::Kind::Component,
        },
    );
    s.validate().unwrap();
    let ext = n.extensions.as_mut().unwrap();
    ext.required_namespaces.push(t::NAMESPACE.into());
    ext.namespaces
        .insert(t::NAMESPACE.into(), serde_json::to_value(s).unwrap());
    n.next_id = 514;
    n.validate().unwrap();
    let mut e = Editor::new(n, "capacity".into()).unwrap();
    let key = set(
        &mut e,
        Form::Specification {
            id: None,
            system: "1".into(),
            name: "513th".into(),
            calculation: "".into(),
        },
    );
    let before = e.snapshot.clone();
    assert!(
        run(&mut e, Action::Apply(key))
            .unwrap_err()
            .contains("LIMIT")
    );
    assert_eq!(e.snapshot, before);
    let mut e = editor();
    let a = spec(&mut e, false);
    let mut first = None;
    for i in 0..32 {
        let id = parameter(&mut e, a, &format!("p{i}"), c::UnitId::Metre);
        first.get_or_insert(id);
        binding(&mut e, a, id, input("x"));
    }
    assert_eq!(
        t::load(&e.snapshot).unwrap().parametrics[&a]
            .parameters
            .len(),
        32
    );
    let key = set(
        &mut e,
        Form::Parameter {
            specification: a,
            id: None,
            name: "p32".into(),
            quantity: q(c::UnitId::Metre),
        },
    );
    let before = e.snapshot.clone();
    assert!(
        run(&mut e, Action::Apply(key))
            .unwrap_err()
            .contains("LIMIT")
    );
    assert_eq!(e.snapshot, before);
    let key = set(
        &mut e,
        Form::Binding {
            specification: a,
            id: None,
            parameter: first.unwrap().0.to_string(),
            source: input("x"),
        },
    );
    let before = e.snapshot.clone();
    assert!(
        run(&mut e, Action::Apply(key))
            .unwrap_err()
            .contains("LIMIT")
    );
    assert_eq!(e.snapshot, before);
}
#[test]
fn parametric_legacy_deletion_and_refactor_disclose_retained_exact_references_and_proposals() {
    use t::{deletion as d, refactor as f};
    let (mut e, a, _, _, _, _) = graph();
    assert!(
        d::unknown_impact(&e)
            .locations
            .iter()
            .any(|s| s.contains("ParametricSpecification"))
    );
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::Refactor(f::Action::SetForm(f::Form {
            mode: f::Mode::CloneDefinition,
            definition: "1004".into(),
            name: "Explicit copy".into(),
            usages: "".into(),
            owner: "".into(),
            keep_external: true,
        }))),
    )
    .unwrap();
    let prepared = f::prepare(&e).unwrap();
    assert!(
        prepared
            .summary()
            .uninterpreted_locations
            .iter()
            .any(|s| s.contains("ParametricSpecification"))
    );
    assert!(!prepared.summary().exhaustive_global_reference_verdict);
    run(
        &mut e,
        Action::Remove {
            specification: a,
            element: a,
        },
    )
    .unwrap();
    assert!(
        d::unknown_impact(&e)
            .locations
            .iter()
            .any(|s| s.contains("Retained parametric proposals"))
    );
}
#[test]
fn parametric_bound_native_cold_and_captured_recovery_keep_whole_prior_raw_opaque_and_all_eight_maps()
 {
    use mbse_model::{
        native_views::{self as v, Views},
        native_views_io as io,
    };
    let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let mut session = Session::new(area.path()).unwrap();
    session.editor =
        Editor::new(editor().snapshot, session.editor.freshness().instance_id).unwrap();
    let before = session.editor.snapshot.clone();
    let mut doc:v::Document=serde_json::from_slice(include_bytes!("../../../docs/parity/p34c/verification/native-02/run/scenario-final-cold-copy.mbse-views.json")).unwrap();
    doc.profile = v::scenarios::RECOVERY_PROFILE.into();
    doc.binding = None;
    let views = Views::restore(&doc.encoded().unwrap(), &session.editor).unwrap();
    let a = spec(&mut session.editor, true);
    let x = parameter(&mut session.editor, a, "x", c::UnitId::Metre);
    let duration = parameter(&mut session.editor, a, "x1", c::UnitId::Second);
    binding(&mut session.editor, a, duration, input("x1"));
    binding(&mut session.editor, a, x, input("x"));
    let key = set(
        &mut session.editor,
        Form::Parameter {
            specification: a,
            id: Some(x),
            name: "x +".into(),
            quantity: q(c::UnitId::Metre),
        },
    );
    assert!(run(&mut session.editor, Action::Apply(key)).is_err());
    let expected = session.editor.snapshot.clone();
    assert_eq!(expected.records, before.records);
    assert_eq!(expected.drafts, before.drafts);
    assert_eq!(expected.creation_drafts, before.creation_drafts);
    let mut ns = expected.extensions.as_ref().unwrap().namespaces.clone();
    ns.get_mut(t::NAMESPACE)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("parametrics");
    ns.get_mut(t::DRAFTS)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("parametric_forms");
    assert_eq!(ns, before.extensions.as_ref().unwrap().namespaces);
    let path = area.path().join("parametric.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let mut writes = io::Io::default();
    let side = area.path().join("parametric.mbse-views.json");
    let job = writes
        .begin(&session, &views, &side, io::Kind::Bound)
        .unwrap();
    assert!(writes.complete(&session, &views, io::execute(job)));
    let cold = Session::open(area.path(), &path).unwrap();
    assert_eq!(cold.editor.snapshot, expected);
    let decoded = Views::decode(
        &std::fs::read(&side).unwrap(),
        &cold.editor,
        &cold.native_checkpoint().unwrap().unwrap(),
    )
    .unwrap();
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
    let old = serde_json::to_value(views.document()).unwrap();
    let data = serde_json::to_value(decoded.document()).unwrap();
    for k in maps {
        assert_eq!(data.get(k), old.get(k), "bound/cold {k}");
    }
    let job = session.begin_recovery().unwrap();
    assert!(session.complete(execute_write(job)));
    let restored = Session::restore(area.path(), session.last_recovery().unwrap()).unwrap();
    let mut recovered = restored.editor.snapshot;
    recovered.document_id = expected.document_id.clone();
    assert_eq!(recovered, expected);
    let recovery = area.path().join("parametric-recovery.mbse-views.json");
    let destination = writes
        .freeze_destination(&session, &views, &recovery, io::Kind::Recovery)
        .unwrap();
    let capture = views.capture(&session.editor);
    let job = writes
        .begin_captured(&session, &views, capture, destination)
        .unwrap();
    assert!(writes.complete(&session, &views, io::execute(job)));
    let restored = Views::restore(&std::fs::read(recovery).unwrap(), &session.editor).unwrap();
    let data = serde_json::to_value(restored.document()).unwrap();
    for k in maps {
        assert_eq!(data.get(k), old.get(k), "captured recovery {k}");
    }
    assert!(!decoded.can_layout_history(false));
}
