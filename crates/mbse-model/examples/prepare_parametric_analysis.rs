//! Prepare source fixtures only. Expected arithmetic remains the hand-authored JSON oracle.
use mbse_model::{
    native_views::{self as v, Views},
    native_views_io as io,
    stage1::*,
    stage1_io::{Session, execute_write},
    types::{
        self as t,
        parametrics::{self as p, Form, SourceForm},
    },
};
use serde_json::{Value, json};
use std::path::PathBuf;
#[path = "../tests/support/parametric_source.rs"]
mod support;
fn main() {
    let args = std::env::args()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        4,
        "base native, base views, independent oracle, fresh /workspace output directory"
    );
    let output = &args[3];
    assert!(output.is_absolute() && output.starts_with("/workspace") && !output.exists());
    std::fs::create_dir_all(output).unwrap();
    let mut session = Session::open(output, &args[0]).unwrap();
    let before = session.editor.snapshot.clone();
    let original_types = t::load(&before).unwrap();
    let mut doc: v::Document = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    doc.profile = v::scenarios::RECOVERY_PROFILE.into();
    doc.binding = None;
    let views = Views::restore(&doc.encoded().unwrap(), &session.editor).unwrap();
    let oracle: Value = serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
    let mut cases = Vec::new();
    let mut added_calculations = Vec::new();
    let mut added_specs = Vec::new();
    session.editor.snapshot.next_id = session.editor.snapshot.next_id.max(60000);
    for row in oracle["cases"].as_array().unwrap() {
        let calc = session.editor.snapshot.next_id;
        session
            .editor
            .snapshot
            .records
            .insert(RecordId(calc), support::native(calc, 1002, row));
        session.editor.snapshot.next_id = calc + 1;
        session.editor.snapshot.model_revision += 1;
        session.editor.snapshot.validate().unwrap();
        let (spec, params) = support::selected(&mut session.editor, calc);
        support::accept(
            &mut session.editor,
            Form::Specification {
                id: Some(spec),
                system: "1".into(),
                name: row["name"].as_str().unwrap().into(),
                calculation: calc.to_string(),
            },
        );
        added_calculations.push(RecordId(calc));
        added_specs.push(spec);
        cases.push(json!({"name":row["name"],"calculation":calc,"specification":spec,"parameters_and_bindings":params,"expected":row}));
    }
    let calc = session.editor.snapshot.next_id;
    let mut row = support::data("scalar_equal");
    row["name"] = json!("cycle");
    session
        .editor
        .snapshot
        .records
        .insert(RecordId(calc), support::native(calc, 1002, &row));
    session.editor.snapshot.next_id = calc + 1;
    session.editor.snapshot.model_revision += 1;
    let (spec, params) = support::selected(&mut session.editor, calc);
    support::accept(
        &mut session.editor,
        Form::Specification {
            id: Some(spec),
            system: "1".into(),
            name: "cycle".into(),
            calculation: calc.to_string(),
        },
    );
    let (parameter, binding) = params["a"];
    support::accept(
        &mut session.editor,
        Form::Binding {
            specification: spec,
            id: Some(binding),
            parameter: parameter.0.to_string(),
            source: SourceForm::NativeOutput {
                calculation: calc.to_string(),
            },
        },
    );
    added_calculations.push(RecordId(calc));
    added_specs.push(spec);
    cases.push(json!({"name":"cycle","calculation":calc,"specification":spec,"expected":{"expected_failure":"Cycle"}}));
    let (spec, _) = support::selected(&mut session.editor, 1003);
    support::accept(
        &mut session.editor,
        Form::Specification {
            id: Some(spec),
            system: "1".into(),
            name: "pending_original_criterion".into(),
            calculation: "1003".into(),
        },
    );
    added_specs.push(spec);
    cases.push(json!({"name":"pending_original_criterion","calculation":1003,"specification":spec,"expected":{"expected_value":2.5,"expected_assessment_failure":"IncompleteInput"}}));
    let mut preserved = session.editor.snapshot.records.clone();
    for id in added_calculations {
        preserved.remove(&id);
    }
    assert_eq!(preserved, before.records);
    assert_eq!(session.editor.snapshot.drafts, before.drafts);
    assert_eq!(
        session.editor.snapshot.creation_drafts,
        before.creation_drafts
    );
    let mut types = t::load(&session.editor.snapshot).unwrap();
    for id in &added_specs {
        types.parametrics.remove(id);
    }
    assert_eq!(types, original_types);
    let path = output.join("native.kernspan");
    let job = session.begin_save(&path).unwrap();
    assert!(session.complete(execute_write(job)));
    let mut writes = io::Io::default();
    let side = output.join("native.mbse-views.json");
    let job = writes
        .begin(&session, &views, &side, io::Kind::Bound)
        .unwrap();
    assert!(writes.complete(&session, &views, io::execute(job)));
    let decoded = Views::decode(
        &std::fs::read(&side).unwrap(),
        &session.editor,
        &session.native_checkpoint().unwrap().unwrap(),
    )
    .unwrap();
    let expected = serde_json::to_value(views.document()).unwrap();
    let actual = serde_json::to_value(decoded.document()).unwrap();
    for map in [
        "views",
        "occurrences",
        "edges",
        "typed_ports",
        "connector_edges",
        "activity_diagrams",
        "state_machine_diagrams",
        "scenario_diagrams",
    ] {
        assert_eq!(actual[map], expected[map]);
    }
    std::fs::write(output.join("EXPECTED.json"),serde_json::to_vec_pretty(&json!({"schema":"mbse-craft.parametric-native-fixtures/1","preparation":"Source data only, never evaluates its oracle or parametric programs. Numeric expectations copied from the pre-implementation hand-fixed fixture.","cases":&cases,"preserved_original_records_raw_opaque_and_eight_maps":true,"counters":{"model_revision":session.editor.snapshot.model_revision,"raw_version":session.editor.snapshot.raw_version,"next_id":session.editor.snapshot.next_id}})).unwrap()).unwrap();
    println!(
        "Prepared {} source-only native scenarios at {}",
        cases.len(),
        output.display()
    );
}
