//! Original bounded local rule/field oracles. Native rendered navigation is P26b.
use mbse_assurance::{validation::*, *};
use mbse_model::{
    stage1::{Command, Editor, RawKey, RecordId},
    types::{self, Id},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn setup() -> (Board, Editor) {
    let v: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/p25/fixtures/native.kernspan"
    ))
    .unwrap();
    let e = Editor::new(
        serde_json::from_value(v["snapshot"].clone()).unwrap(),
        "p26a:test".into(),
    )
    .unwrap();
    let mut b = Board::new();
    b.add_identified_requirement("Root requirement".into(), "R-1".into(), None)
        .unwrap();
    b.add_identified_requirement(
        "Child requirement".into(),
        "R-2".into(),
        Some(RequirementId(1)),
    )
    .unwrap();
    (b, e)
}
fn action(e: &mut Editor, a: types::Action) {
    e.command(&e.freshness(), Command::Typed(a)).unwrap();
}
fn faults(b: &mut Board, e: &mut Editor) {
    action(
        e,
        types::Action::SetForm {
            id: Id(1020),
            form: types::Form {
                owner: Id(1007),
                name: "instance gain".into(),
                value_type: "999999".into(),
                lower: "1".into(),
                upper: "1".into(),
            },
        },
    );
    action(
        e,
        types::Action::SetUsageForm {
            form: types::UsageForm {
                id: Some(Id(1023)),
                name: "primary device".into(),
                owner: "1021".into(),
                target: "1021".into(),
                kind: types::UsageKind::Part,
            },
        },
    );
    e.command(
        &e.freshness(),
        Command::EditText {
            key: RawKey {
                record: RecordId(1003),
                field: "calc.expression".into(),
                item: None,
            },
            text: "x + x1".into(),
        },
    )
    .unwrap();
    assert!(!b.edit_parent(RequirementId(1), "2".into()).unwrap());
}
fn area() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap()
}
#[test]
fn legacy_native_requirement_suite_does_not_require_type_extension_migration() {
    let (mut b, _) = setup();
    let e = Editor::new(
        mbse_model::stage1::Snapshot::new(mbse_model::stage1::DocumentId("p26a:legacy".into())),
        "p26a:legacy-runtime".into(),
    )
    .unwrap();
    assert!(e.snapshot.extensions.is_none());
    assert!(
        !b.edit_field(RequirementId(1), "statement", " ".into())
            .unwrap()
    );
    b.set_validation_suite(
        "Requirements only".into(),
        Suite {
            rules: BTreeSet::from([RuleFamily::RequirementDrafts]),
        },
    )
    .unwrap();
    let before = b.register.clone();
    let native = e.snapshot.clone();
    let report = b.run_validation("Requirements only", &e).unwrap();
    assert_eq!(report.issue_total(), 1);
    assert_eq!(
        report.issues()[0].id,
        "MBSE.REQUIREMENT.DRAFT:requirement1:Statement"
    );
    let builtin = b.run_validation(BUILTIN_NAME, &e).unwrap();
    assert!(
        builtin
            .limitations()
            .iter()
            .any(|l| l.contains("extensions are not migrated"))
    );
    assert_eq!(b.register, before);
    assert_eq!(e.snapshot, native);
}
#[test]
fn seeded_type_reference_containment_and_unit_faults_have_stable_exact_rule_targets() {
    let (mut b, mut e) = setup();
    faults(&mut b, &mut e);
    let native = e.snapshot.clone();
    let register = b.register.clone();
    let accepted = types::load(&e.snapshot).unwrap();
    assert_eq!(accepted.properties[&Id(1020)].value_type, Id(1005));
    assert_eq!(accepted.usages[&Id(1023)].target, Id(1007));
    let r = b.run_validation(BUILTIN_NAME, &e).unwrap();
    for (id, target) in [
        (
            "MBSE.TYPE.REFERENCE:property1020:ValueType",
            Target::Property {
                property: Id(1020),
                field: PropertyField::ValueType,
            },
        ),
        (
            "MBSE.STRUCT.PART_CONTAINMENT:usage1023:Target",
            Target::Usage {
                usage: Some(Id(1023)),
                field: UsageField::Target,
            },
        ),
        (
            "MBSE.CALC.UNIT_COMPATIBILITY:record1003:calc.expression",
            Target::NativeField(RawKey {
                record: RecordId(1003),
                field: "calc.expression".into(),
                item: None,
            }),
        ),
        (
            "MBSE.REQUIREMENT.DRAFT:requirement1:Parent",
            Target::Requirement {
                requirement: RequirementId(1),
                field: RequirementField::Parent,
            },
        ),
    ] {
        let issue = r
            .issues()
            .iter()
            .find(|i| i.id == id)
            .unwrap_or_else(|| panic!("missing {id}: {:?}", r.issues()));
        assert_eq!(issue.target, target);
        let location = r.navigate(&b, &e, id).unwrap();
        assert_eq!(location.resolve(&b, &e).unwrap(), target);
    }
    assert_eq!(b.register, register);
    assert_eq!(e.snapshot, native);
    let again = b.run_validation(BUILTIN_NAME, &e).unwrap();
    assert_eq!(
        r.issues().iter().map(|i| &i.id).collect::<Vec<_>>(),
        again.issues().iter().map(|i| &i.id).collect::<Vec<_>>()
    );
}
#[test]
fn containment_route_identifies_changed_owner_when_target_is_unchanged() {
    let (b, mut e) = setup();
    action(
        &mut e,
        types::Action::SetUsageForm {
            form: types::UsageForm {
                id: Some(Id(1023)),
                name: "primary device".into(),
                owner: "1007".into(),
                target: "1007".into(),
                kind: types::UsageKind::Part,
            },
        },
    );
    let r = b.run_validation(BUILTIN_NAME, &e).unwrap();
    assert!(
        r.issues()
            .iter()
            .any(|i| i.id == "MBSE.STRUCT.PART_CONTAINMENT:usage1023:Owner"
                && i.target
                    == Target::Usage {
                        usage: Some(Id(1023)),
                        field: UsageField::Owner
                    })
    );
}
#[test]
fn named_suite_scope_is_explicit_and_does_not_run_other_rule_families() {
    let (mut b, mut e) = setup();
    faults(&mut b, &mut e);
    b.set_validation_suite(
        "Structure only".into(),
        Suite {
            rules: BTreeSet::from([RuleFamily::PartContainment]),
        },
    )
    .unwrap();
    let r = b.run_validation("Structure only", &e).unwrap();
    assert_eq!(r.suite_name(), "Structure only");
    assert_eq!(r.rules(), &BTreeSet::from([RuleFamily::PartContainment]));
    assert_eq!(r.issue_total(), 1);
    assert_eq!(r.issues()[0].rule, "MBSE.STRUCT.PART_CONTAINMENT");
    assert!(r.limitations().iter().any(|s| s.contains("outside")));
    assert!(b.run_validation("Missing suite", &e).is_err());
}
#[test]
fn reports_and_sealed_locations_refuse_raw_changes_reused_counters_and_other_runtime() {
    let (mut b, mut e) = setup();
    faults(&mut b, &mut e);
    let r = b.run_validation(BUILTIN_NAME, &e).unwrap();
    let location = r.navigate(&b, &e, &r.issues()[0].id).unwrap();
    let mut other = e.clone();
    other
        .snapshot
        .records
        .get_mut(&RecordId(1003))
        .unwrap()
        .name = "Same revision different payload".into();
    assert!(location.resolve(&b, &other).unwrap_err().contains("STALE"));
    let cold = Editor::new(e.snapshot.clone(), "different-runtime".into()).unwrap();
    assert!(r.check(&b, &cold).is_err());
    let before = b.register.clone();
    b.register
        .requirements
        .get_mut(&RequirementId(1))
        .unwrap()
        .rationale = "Different register same counters".into();
    assert!(location.resolve(&b, &e).is_err());
    b.register = before;
    action(
        &mut e,
        types::Action::SetForm {
            id: Id(1020),
            form: types::Form {
                owner: Id(1007),
                name: "instance gain".into(),
                value_type: "999999?".into(),
                lower: "1".into(),
                upper: "1".into(),
            },
        },
    );
    assert!(location.resolve(&b, &e).is_err());
}
#[test]
fn selection_navigation_does_not_change_source_or_raw_history() {
    let (mut b, mut e) = setup();
    faults(&mut b, &mut e);
    let r = b.run_validation(BUILTIN_NAME, &e).unwrap();
    let before = e.snapshot.clone();
    let raw = b.register.drafts.clone();
    e.select(RecordId(1002)).unwrap();
    assert!(r.check(&b, &e).is_ok());
    let id = "MBSE.CALC.UNIT_COMPATIBILITY:record1003:calc.expression";
    let Target::NativeField(key) = r.navigate(&b, &e, id).unwrap().resolve(&b, &e).unwrap() else {
        panic!("exact native field expected")
    };
    e.select(key.record).unwrap();
    assert!(e.field_available(&key));
    assert_eq!(e.snapshot, before);
    assert_eq!(b.register.drafts, raw);
    assert!(r.navigate(&b, &e, "missing issue").is_err());
}
#[test]
fn named_suites_invalid_declarative_forms_and_undo_survive_atomic_save_cold_open() {
    let (mut b, mut e) = setup();
    faults(&mut b, &mut e);
    b.set_validation_form(
        "QA local".into(),
        serde_json::to_string(&Suite::local()).unwrap(),
    )
    .unwrap();
    b.apply_validation_form().unwrap();
    let accepted = b.register.clone();
    assert!(b.undo());
    assert!(!b.register.validation_suites.contains_key("QA local"));
    assert!(b.redo());
    assert_eq!(b.register, accepted);
    b.set_validation_form(
        "Retain invalid".into(),
        "{\"rules\":[\"ExecuteShell\"]}".into(),
    )
    .unwrap();
    let before = b.register.clone();
    assert!(b.apply_validation_form().is_err());
    assert_eq!(b.register, before);
    let dir = area();
    let path = dir.path().join("suites.json");
    b.save_new(&path).unwrap();
    let (cold, _) = Board::open(&path).unwrap();
    assert_eq!(cold.register, b.register);
    assert_eq!(
        cold.register.drafts["validation:rules"].text,
        "{\"rules\":[\"ExecuteShell\"]}"
    );
    let cold_editor = Editor::new(e.snapshot.clone(), "p26a:cold".into()).unwrap();
    let r = cold.run_validation("QA local", &cold_editor).unwrap();
    assert!(
        r.issues()
            .iter()
            .any(|i| i.rule == "MBSE.CALC.UNIT_COMPATIBILITY")
    );
    assert!(b.register.decisions.is_empty());
}
#[test]
fn unsupported_suite_fields_enums_empty_reserved_and_limits_refuse_without_overwrite() {
    let (mut b, e) = setup();
    for (name, suite) in [
        ("".into(), Suite::local()),
        (BUILTIN_NAME.into(), Suite::local()),
        (
            "Empty".into(),
            Suite {
                rules: BTreeSet::new(),
            },
        ),
    ] {
        let before = b.register.clone();
        assert!(b.set_validation_suite(name, suite).is_err());
        assert_eq!(b.register, before);
    }
    let dir = area();
    for raw in [
        json!({"rules":["ExecuteShell"]}),
        json!({"rules":["CalculationUnits"],"script":"must remain data"}),
    ] {
        let mut v = json!({"profile":PROFILE,"register":b.register});
        v["register"]["validation_suites"]["Unknown"] = raw;
        let bytes = serde_json::to_vec(&v).unwrap();
        let path = dir.path().join("bad-suite.json");
        std::fs::write(&path, &bytes).unwrap();
        assert!(Board::open(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    for i in 0..32 {
        b.set_validation_suite(format!("Suite{i}"), Suite::local())
            .unwrap();
    }
    let before = b.register.clone();
    assert!(
        b.set_validation_suite("Over limit".into(), Suite::local())
            .is_err()
    );
    assert_eq!(b.register, before);
    assert!(b.run_validation(BUILTIN_NAME, &e).is_ok());
}
#[test]
fn issue_cap_reports_unreported_count_without_changing_accepted_records() {
    let (mut b, e) = setup();
    let template = b.register.requirements[&RequirementId(1)].clone();
    for id in 3..=302 {
        let mut req = template.clone();
        req.id = RequirementId(id);
        req.identifier = format!("R-{id}");
        b.register.requirements.insert(req.id, req);
        b.register.drafts.insert(
            format!("{id}:statement"),
            RawDraft {
                text: "  ".into(),
                valid: false,
            },
        );
    }
    b.register.next_id = 303;
    b.set_validation_suite(
        "Requirements only".into(),
        Suite {
            rules: BTreeSet::from([RuleFamily::RequirementDrafts]),
        },
    )
    .unwrap();
    let before = b.register.clone();
    let r = b.run_validation("Requirements only", &e).unwrap();
    assert_eq!(
        (r.issue_total(), r.issues().len(), r.unreported_issues()),
        (300, 256, 44)
    );
    assert_eq!(b.register, before);
}
#[test]
fn unknown_or_unroutable_validator_diagnostics_remain_explicit_and_legacy_empty_maps_omit() {
    let (mut b, mut e) = setup();
    assert!(
        serde_json::to_value(&b.register)
            .unwrap()
            .get("validation_suites")
            .is_none()
    );
    b.register
        .unknown
        .insert("future.validation/1".into(), json!({"uninterpreted":true}));
    let mut f = types::forms(&e.snapshot).unwrap()[&Id(1020)].clone();
    f.owner = Id(1005);
    action(
        &mut e,
        types::Action::SetForm {
            id: Id(1020),
            form: f,
        },
    );
    let r = b.run_validation(BUILTIN_NAME, &e).unwrap();
    assert!(
        r.limitations()
            .iter()
            .any(|l| l.contains("no supported exact-field route"))
    );
    assert!(
        r.limitations()
            .iter()
            .any(|l| l.contains("metadata namespaces"))
    );
    assert!(
        !r.issues()
            .iter()
            .any(|i| i.id == "MBSE.TYPE.REFERENCE:property1020:ValueType")
    );
}
