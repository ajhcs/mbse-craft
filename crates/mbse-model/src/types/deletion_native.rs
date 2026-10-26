//! Native accepted references used by deletion; calculation meaning stays in Stage1.
use super::*;
use crate::stage1::{RecordKind, Snapshot};
#[derive(Clone, Debug, Serialize)]
pub struct OwnedContext {
    pub owner: Id,
    pub id: Id,
    pub name: String,
}
pub(super) fn contexts(s: &Snapshot, removed: &BTreeSet<Id>) -> Vec<OwnedContext> {
    s.records
        .values()
        .filter(|r| removed.contains(&Id(r.id.0)))
        .flat_map(|r| {
            r.fields
                .get("context")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(move |v| {
                    Some(OwnedContext {
                        owner: Id(r.id.0),
                        id: Id(v["id"].as_u64()?),
                        name: v["name"].as_str().unwrap_or("").into(),
                    })
                })
        })
        .collect()
}
pub(super) fn references(s: &Snapshot) -> Result<Vec<refactor::Reference>, String> {
    let mut rows = Vec::new();
    let mut add = |owner: u64, role: String, target: u64| {
        rows.push(refactor::Reference {
            owner: Id(owner),
            role,
            target: Id(target),
        })
    };
    for r in s.records.values() {
        if r.id != r.scope {
            add(r.id.0, "Native System scope".into(), r.scope.0);
        }
        match r.kind {
            RecordKind::Capability => {
                for target in r.links() {
                    add(r.id.0, "Native Mission contribution".into(), target.0);
                }
            }
            RecordKind::Objective => {
                if let Some(target) = r.fields.get("subject").and_then(|v| v["id"].as_u64()) {
                    add(r.id.0, "Native Objective subject".into(), target);
                }
            }
            RecordKind::Measure => {
                if let Some(target) = r.fields.get("forObjective").and_then(Value::as_u64) {
                    add(r.id.0, "Native Objective owner".into(), target);
                }
            }
            RecordKind::Expectation => {
                for key in ["expressedBy", "concernsSystem"] {
                    if let Some(target) = r.fields.get(key).and_then(Value::as_u64) {
                        add(r.id.0, format!("Native Expectation {key}"), target);
                    }
                }
            }
            RecordKind::Stakeholder => {
                if let Some(values) = r.fields.get("affiliations").and_then(Value::as_object) {
                    for key in values.keys() {
                        if let Ok(target) = key.parse::<u64>() {
                            add(r.id.0, "Native Organization affiliation".into(), target);
                        }
                    }
                }
                if let Some(values) = r.fields.get("actors").and_then(Value::as_array) {
                    for target in values.iter().filter_map(Value::as_u64) {
                        add(r.id.0, "Native Actor membership".into(), target);
                    }
                }
            }
            _ => {}
        }
    }
    for calc in s.calculations()?.values() {
        for (name, input) in &calc.bindings {
            if let crate::stage1_calc::Input::Output(target) = input {
                add(
                    calc.owner.0,
                    format!("Native calculation output binding {name}"),
                    target.0,
                );
            }
        }
    }
    Ok(rows)
}
