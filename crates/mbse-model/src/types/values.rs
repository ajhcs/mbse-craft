//! Authored defaults and explicit Part overrides; pure derived previews never write either.
use super::*;
use crate::{stage1::Editor, stage1_calc as calc};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Literal {
    pub kind: Scalar,
    pub text: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Target {
    Default { property: Id },
    Override { usage: Id, property: Id },
}
impl Target {
    pub fn property(self) -> Id {
        match self {
            Self::Default { property } | Self::Override { property, .. } => property,
        }
    }
    pub fn key(self) -> String {
        match self {
            Self::Default { property } => format!("default:{}", property.0),
            Self::Override { usage, property } => format!("override:{}:{}", usage.0, property.0),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub target: Target,
    pub text: String,
    pub expression: String,
}
#[derive(Clone, Debug)]
pub enum Action {
    Begin(Target),
    Set(Form),
    Apply(Target),
    Clear(Target),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    Default,
    Override,
    Unset,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effective {
    pub value: Option<Literal>,
    pub origin: Origin,
}
pub fn forms(s: &Snapshot) -> Result<BTreeMap<String, Form>, String> {
    Ok(journal(s)?.values)
}
fn property(store: &Store, target: Target) -> Result<&Property, String> {
    let p = store
        .properties
        .get(&target.property())
        .ok_or("VALUE_PROPERTY_MISSING_OR_KIND")?;
    if p.multiplicity.lower != 1 || p.multiplicity.upper != Some(1) {
        return Err("VALUE_SCALAR_MULTIPLICITY_REQUIRED".into());
    }
    if let Target::Override { usage, .. } = target {
        let u = store
            .usages
            .get(&usage)
            .ok_or("VALUE_PART_MISSING_OR_KIND")?;
        if u.kind != UsageKind::Part || u.target != p.owner {
            return Err("VALUE_INSTANCE_PROPERTY_CONTEXT".into());
        }
    }
    Ok(p)
}
fn scalar(store: &Store, target: Target) -> Result<Scalar, String> {
    let p = property(store, target)?;
    match store.definitions.get(&p.value_type).map(|d| d.kind) {
        Some(Kind::Value(kind)) => Ok(kind),
        _ => Err("VALUE_TYPE_MISSING_OR_KIND".into()),
    }
}
fn literal(kind: Scalar, text: &str) -> Result<Literal, String> {
    if text.len() > 4096 {
        return Err("VALUE_LITERAL_LIMIT".into());
    }
    let parsed = if kind == Scalar::Text {
        text
    } else {
        text.trim()
    };
    match kind {
        Scalar::Boolean => {
            if !matches!(parsed, "true" | "false") {
                return Err("VALUE_BOOLEAN_LITERAL".into());
            }
        }
        Scalar::Integer => {
            parsed.parse::<i64>().map_err(|_| "VALUE_INTEGER_LITERAL")?;
        }
        Scalar::Real => {
            calc::decimal(parsed).map_err(|e| format!("VALUE_REAL_LITERAL: {e}"))?;
        }
        Scalar::Text => {}
    }
    Ok(Literal {
        kind,
        text: parsed.into(),
    })
}
fn explicit(store: &Store, target: Target) -> Option<&Literal> {
    match target {
        Target::Default { property } => store.defaults.get(&property),
        Target::Override { usage, property } => {
            store.overrides.get(&usage).and_then(|m| m.get(&property))
        }
    }
}
pub fn effective(store: &Store, target: Target) -> Result<Effective, String> {
    property(store, target)?;
    if let Some(value) = explicit(store, target) {
        return Ok(Effective {
            value: Some(value.clone()),
            origin: if matches!(target, Target::Override { .. }) {
                Origin::Override
            } else {
                Origin::Default
            },
        });
    }
    if let Target::Override { property, .. } = target {
        if let Some(value) = store.defaults.get(&property) {
            return Ok(Effective {
                value: Some(value.clone()),
                origin: Origin::Default,
            });
        }
    }
    Ok(Effective {
        value: None,
        origin: Origin::Unset,
    })
}
pub(super) fn validate_store(store: &Store) -> Result<(), String> {
    if store.defaults.len() > 512
        || store.overrides.len() > 512
        || store.overrides.values().map(|m| m.len()).sum::<usize>() + store.defaults.len() > 1024
    {
        return Err("VALUE_COUNT_LIMIT".into());
    }
    let check = |target, value: &Literal| -> Result<(), String> {
        let kind = scalar(store, target)?;
        if value.kind != kind || literal(kind, &value.text)? != *value {
            return Err("VALUE_LITERAL_TYPE_OR_SHAPE".into());
        }
        Ok(())
    };
    for (id, value) in &store.defaults {
        check(Target::Default { property: *id }, value)?;
    }
    for (usage, map) in &store.overrides {
        if store
            .usages
            .get(usage)
            .is_none_or(|u| u.kind != UsageKind::Part)
        {
            return Err("VALUE_OVERRIDE_PART_KIND".into());
        }
        for (property, value) in map {
            check(
                Target::Override {
                    usage: *usage,
                    property: *property,
                },
                value,
            )?;
        }
    }
    Ok(())
}
pub(super) fn canonical_maps(value: &Value) -> Result<(), String> {
    if value.get("defaults").is_some() {
        canonical(value, "defaults")?;
    }
    if let Some(overrides) = value.get("overrides") {
        canonical(value, "overrides")?;
        for map in overrides.as_object().unwrap().values() {
            let v = serde_json::json!({"items":map});
            canonical(&v, "items")?;
        }
    }
    Ok(())
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    if j.values.len() > 512
        || j.values.iter().any(|(key, f)| {
            *key != f.target.key()
                || f.target.property().0 == 0
                || matches!(f.target,Target::Override{usage,..}if usage.0==0)
                || f.text.len() > 4096
                || f.expression.len() > 4096
        })
    {
        return Err("VALUE_FORM_SHAPE_OR_LIMIT".into());
    }
    Ok(())
}
pub(super) fn draft_high(j: &Journal) -> u64 {
    j.values
        .values()
        .map(|f| match f.target {
            Target::Default { property } => property.0,
            Target::Override { usage, property } => usage.0.max(property.0),
        })
        .max()
        .unwrap_or(0)
}
pub(super) fn apply_inner(s: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut store = load(s)?;
    let mut j = journal(s)?;
    match action {
        Action::Begin(target) => {
            property(&store, target)?;
            if !j.values.contains_key(&target.key()) {
                let text = effective(&store, target)?
                    .value
                    .map_or(String::new(), |v| v.text);
                j.values.insert(
                    target.key(),
                    Form {
                        target,
                        text,
                        expression: "x".into(),
                    },
                );
            }
            put(s, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Set(form) => {
            if !j.values.contains_key(&form.target.key()) {
                return Err("VALUE_FORM_UNAVAILABLE".into());
            }
            j.values.insert(form.target.key(), form);
            put(s, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Apply(target) => {
            let f = j
                .values
                .get(&target.key())
                .ok_or("VALUE_FORM_UNAVAILABLE")?;
            let value = literal(scalar(&store, target)?, &f.text)?;
            match target {
                Target::Default { property } => {
                    store.defaults.insert(property, value);
                }
                Target::Override { usage, property } => {
                    store
                        .overrides
                        .entry(usage)
                        .or_default()
                        .insert(property, value);
                }
            }
            store.validate()?;
            put(s, NAMESPACE, &store)?;
            Ok(Change::Model)
        }
        Action::Clear(target) => {
            property(&store, target)?;
            match target {
                Target::Default { property } => {
                    store.defaults.remove(&property);
                }
                Target::Override { usage, property } => {
                    if let Some(map) = store.overrides.get_mut(&usage) {
                        map.remove(&property);
                        if map.is_empty() {
                            store.overrides.remove(&usage);
                        }
                    }
                }
            }
            store.validate()?;
            put(s, NAMESPACE, &store)?;
            Ok(Change::Model)
        }
    }
}
pub fn preview(s: &Snapshot, target: Target) -> Result<Literal, String> {
    let store = load(s)?;
    let f = forms(s)?
        .remove(&target.key())
        .ok_or("VALUE_FORM_UNAVAILABLE")?;
    literal(scalar(&store, target)?, &f.text)
}
fn stamp(e: &Editor) -> calc::DocumentStamp {
    calc::DocumentStamp {
        document_id: e.snapshot.document_id.0.clone(),
        instance_id: e.freshness().instance_id,
        model_revision: e.snapshot.model_revision,
        raw_version: e.snapshot.raw_version,
        snapshot_hash: e.snapshot.digest(),
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Derived {
    target: Target,
    effective: Effective,
    expression: String,
    receipt: calc::EvaluationReceipt,
}
impl Derived {
    pub fn diagnostic(&self) -> Option<&calc::Diagnostic> {
        self.receipt
            .outcomes
            .get(&calc::CalculationId(1))
            .and_then(|o| o.diagnostic())
    }
    pub fn effective(&self) -> &Effective {
        &self.effective
    }
    pub fn current(&self, e: &Editor) -> bool {
        self.receipt.stamp == stamp(e)
    }
    pub fn value(&self) -> Option<f64> {
        self.receipt
            .outcomes
            .get(&calc::CalculationId(1))
            .and_then(|o| o.value())
            .map(|v| v.value)
    }
}
pub fn derive(e: &Editor, target: Target, expression: &str) -> Result<Derived, String> {
    let store = load(&e.snapshot)?;
    let accepted = effective(&store, target)?;
    let value = accepted.value.as_ref().ok_or("VALUE_EFFECTIVE_UNSET")?;
    if !matches!(value.kind, Scalar::Integer | Scalar::Real) {
        return Err("VALUE_DERIVED_NUMERIC_ONLY".into());
    }
    if value.kind == Scalar::Integer
        && value
            .text
            .trim()
            .parse::<i64>()
            .map_err(|_| "VALUE_INTEGER_LITERAL")?
            .unsigned_abs()
            > calc::MAX_EXACT_COUNT as u64
    {
        return Err("VALUE_DERIVED_INTEGER_PRECISION".into());
    }
    let limits = calc::Limits::default();
    let quantity = calc::QuantityType {
        kind: calc::QuantityKind::Scalar,
        unit: calc::UnitId::ScalarOne,
    };
    let definition = calc::Definition {
        id: calc::CalculationId(1),
        owner: store.definitions[&property(&store, target)?.owner].system,
        bindings: BTreeMap::from([(
            "x".into(),
            calc::Input::Literal(calc::Literal {
                decimal: value.text.clone(),
                quantity,
            }),
        )]),
        expression: calc::parse(expression, &limits).map_err(|e| e.to_string())?,
        output: quantity,
    };
    let receipt = calc::evaluate(
        &BTreeMap::from([(definition.id, definition)]),
        &BTreeMap::new(),
        &stamp(e),
        &limits,
    );
    Ok(Derived {
        target,
        effective: accepted,
        expression: expression.into(),
        receipt,
    })
}
