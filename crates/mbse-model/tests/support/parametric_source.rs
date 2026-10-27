use mbse_model::{
    extensions::Extensions,
    stage1::*,
    stage1_calc as c,
    types::{
        self as t, Id,
        parametrics::{Action, Form, SourceForm},
    },
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub fn data(name: &str) -> Value {
    let root: Value = serde_json::from_str(include_str!(
        "../../../../docs/parity/p36a/fixtures/quantities-and-thresholds.json"
    ))
    .unwrap();
    root["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone()
}
pub fn q(unit: c::UnitId) -> c::QuantityType {
    c::QuantityType {
        kind: c::unit_kind(unit),
        unit,
    }
}
pub fn unit(name: &str) -> c::UnitId {
    c::unit_from_key(name).unwrap()
}
pub fn native(id: u64, case: &Value) -> Record {
    let bindings = case["inputs"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, v)| {
            (
                name.clone(),
                c::Input::Literal(c::Literal {
                    decimal: v["decimal"].as_str().unwrap().into(),
                    quantity: q(unit(v["unit"].as_str().unwrap())),
                }),
            )
        })
        .collect();
    let d = c::Definition {
        id: c::CalculationId(id),
        owner: RecordId(id),
        bindings,
        expression: c::parse(case["expression"].as_str().unwrap(), &c::Limits::default()).unwrap(),
        output: q(unit(case["output"].as_str().unwrap())),
    };
    let criterion = &case["criterion"];
    let threshold = criterion
        .get("decimal")
        .and_then(Value::as_str)
        .map(|s| s.parse::<f64>().unwrap());
    let native_unit = criterion
        .get("unit")
        .and_then(Value::as_str)
        .unwrap_or("metre");
    Record {
        id: RecordId(id),
        kind: RecordKind::Measure,
        name: format!("{} source", case["name"].as_str().unwrap()),
        scope: RecordId(1),
        fields: BTreeMap::from([
            ("forObjective".into(), Value::Null),
            (
                "quantity".into(),
                json!(if native_unit == "metre" {
                    "length"
                } else {
                    "dimensionless"
                }),
            ),
            ("unit".into(), json!(native_unit)),
            ("criterion".into(), json!(threshold)),
            (
                "comparator".into(),
                json!(
                    criterion
                        .get("comparator")
                        .and_then(Value::as_str)
                        .unwrap_or("GreaterOrEqual")
                ),
            ),
            ("observations".into(), json!([])),
            ("calculation".into(), serde_json::to_value(d).unwrap()),
        ]),
    }
}
pub fn run(e: &mut Editor, action: Action) -> Result<(), String> {
    e.command(
        &e.freshness(),
        Command::Typed(t::Action::Parametric(action)),
    )
    .map(|_| ())
}
pub fn accept(e: &mut Editor, f: Form) -> Id {
    let id = f.id().unwrap_or(Id(e.snapshot.next_id));
    let key = f.key();
    run(e, Action::Set(f)).unwrap();
    run(e, Action::Apply(key)).unwrap();
    id
}
pub fn selected(e: &mut Editor, calc: u64) -> (Id, BTreeMap<String, (Id, Id)>) {
    let specification = accept(
        e,
        Form::Specification {
            id: None,
            system: "1".into(),
            name: "Frozen binding context".into(),
            calculation: calc.to_string(),
        },
    );
    let defs = e.snapshot.calculations().unwrap();
    let mut ids = BTreeMap::new();
    for (name, input) in &defs[&c::CalculationId(calc)].bindings {
        let quantity = match input {
            c::Input::Literal(l) => l.quantity,
            c::Input::Output(id) => defs[id].output,
        };
        let parameter = accept(
            e,
            Form::Parameter {
                specification,
                id: None,
                name: name.clone(),
                quantity,
            },
        );
        let binding = accept(
            e,
            Form::Binding {
                specification,
                id: None,
                parameter: parameter.0.to_string(),
                source: SourceForm::NativeInput {
                    calculation: calc.to_string(),
                    name: name.clone(),
                },
            },
        );
        ids.insert(name.clone(), (parameter, binding));
    }
    (specification, ids)
}
pub fn fixture(name: &str) -> (Editor, Id, BTreeMap<String, (Id, Id)>) {
    let case = data(name);
    let mut s = Snapshot::new(DocumentId("analysis:independent".into()));
    s.extensions = Some(Extensions::default());
    s.records.insert(RecordId(100), native(100, &case));
    s.next_id = 101;
    let mut e = Editor::new(s, "analysis:runtime".into()).unwrap();
    let (id, keys) = selected(&mut e, 100);
    (e, id, keys)
}
