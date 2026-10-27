//! Frozen binding adapters over the existing native calculation engine. No model writes.
use super::{Source, Specification, accepted_form, forms, report};
use crate::{
    stage1::{Editor, RawKey, RecordId, RecordKind, Validity},
    stage1_calc as calc,
    types::{self as t, Id, values},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
pub const RULES_REVISION: &str = "mbse-craft.parametric-analysis/1";
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BindingEvidence {
    pub binding: Id,
    pub parameter: Id,
    pub input_name: String,
    pub source: Source,
    pub declared: calc::QuantityType,
    pub frozen_input: calc::Input,
    pub value_origin: Option<values::Origin>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Criterion {
    pub measure: RecordId,
    pub threshold: calc::Literal,
    pub comparator: String,
}
/// Only prepare constructs execution authority. Stored/copied reports cannot reconstruct it.
/// ```compile_fail
/// use mbse_model::types::parametrics::analysis::Program;
/// let _: Program = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct Program {
    stamp: calc::DocumentStamp,
    specification: Id,
    calculation: calc::CalculationId,
    definitions: BTreeMap<calc::CalculationId, calc::Definition>,
    raw: calc::CalculationRawState,
    bindings: Vec<BindingEvidence>,
    criterion: Result<Criterion, calc::Diagnostic>,
}
impl Program {
    pub fn stamp(&self) -> &calc::DocumentStamp {
        &self.stamp
    }
    pub fn specification(&self) -> Id {
        self.specification
    }
    pub fn is_current(&self, e: &Editor) -> bool {
        self.stamp == e.calculation_stamp()
    }
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Assessment {
    Met,
    NotMet,
    Unavailable(calc::Diagnostic),
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Run {
    pub rules_revision: String,
    pub specification: Id,
    pub calculation: calc::CalculationId,
    pub stamp: calc::DocumentStamp,
    pub bindings: Vec<BindingEvidence>,
    pub criterion: Result<Criterion, calc::Diagnostic>,
    pub outcome: calc::Outcome,
    pub assessment: Assessment,
    pub native_receipt: calc::EvaluationReceipt,
    pub parametric_execution_performed: bool,
    pub verification_written: bool,
    pub limits: Vec<String>,
}
impl Run {
    pub fn is_current(&self, e: &Editor) -> bool {
        self.rules_revision == RULES_REVISION
            && self.stamp == e.calculation_stamp()
            && self.native_receipt.algorithm_revision == calc::ALGORITHM_REVISION
            && self.native_receipt.unit_table_revision == calc::UNIT_TABLE_REVISION
    }
}
fn error(
    code: calc::DiagnosticCode,
    message: impl Into<String>,
    related: Vec<calc::CalculationId>,
) -> calc::Diagnostic {
    calc::Diagnostic {
        code,
        message: message.into(),
        span: None,
        related,
        cause: None,
    }
}
fn invalid(message: impl Into<String>) -> calc::Diagnostic {
    error(calc::DiagnosticCode::InvalidInput, message, vec![])
}
fn native_config_current(e: &Editor, id: calc::CalculationId) -> Result<(), calc::Diagnostic> {
    if let Some(key) = e.snapshot.calculation_config_id(RecordId(id.0)) {
        if e.snapshot.creation_drafts.get(&key).is_some_and(|d| {
            d.owner == RecordId(id.0)
                && d.kind == RecordKind::Measure
                && d.fields.get("profile").map(String::as_str)
                    == Some("kernspan.stage1.calc-config/1")
        }) {
            return Err(invalid(format!(
                "PARAMETRIC_NATIVE_CONFIG_PENDING: calculation #{}; finish or discard its current configuration proposal.",
                id.0
            )));
        }
    }
    Ok(())
}
fn native_raw_current(
    e: &Editor,
    d: &calc::Definition,
    only_input: Option<&str>,
    raw: &calc::CalculationRawState,
) -> Result<(), calc::Diagnostic> {
    native_config_current(e, d.id)?;
    if let Some(drafts) = raw.get(&d.id) {
        for draft in drafts {
            if only_input.is_none_or(|name| draft.key.field == format!("calc.input.{name}")) {
                calc::classify_raw(d, draft, &calc::Limits::default()).map_err(|mut issue| {
                    issue.message = format!(
                        "PARAMETRIC_NATIVE_RAW_PENDING: calculation #{} field {}: {}",
                        d.id.0, draft.key.field, issue.message
                    );
                    if issue.related.is_empty() {
                        issue.related.push(d.id);
                    }
                    issue
                })?;
            }
        }
    }
    Ok(())
}
fn current_typed_literal(
    e: &Editor,
    s: &t::Store,
    target: values::Target,
    unit: calc::UnitId,
) -> Result<calc::Literal, calc::Diagnostic> {
    let effective = values::effective(s, target).map_err(invalid)?;
    let value = effective.value.as_ref().ok_or_else(|| {
        error(
            calc::DiagnosticCode::MissingReference,
            "PARAMETRIC_TYPED_VALUE_UNSET: no accepted value is invented.",
            vec![],
        )
    })?;
    if !matches!(value.kind, t::Scalar::Integer | t::Scalar::Real) {
        return Err(invalid("PARAMETRIC_NUMERIC_SCALAR_REQUIRED"));
    }
    if value.kind == t::Scalar::Integer
        && value
            .text
            .parse::<i64>()
            .map_err(|_| invalid("PARAMETRIC_INTEGER_LITERAL"))?
            .unsigned_abs()
            > calc::MAX_EXACT_COUNT as u64
    {
        return Err(invalid(
            "PARAMETRIC_INTEGER_PRECISION: integer exceeds the existing native exact-number bound.",
        ));
    }
    let drafts = values::forms(&e.snapshot).map_err(invalid)?;
    let explicit = |target| match target {
        values::Target::Default { property } => s.defaults.get(&property),
        values::Target::Override { usage, property } => {
            s.overrides.get(&usage).and_then(|m| m.get(&property))
        }
    };
    let current = |target: values::Target| -> Result<(), calc::Diagnostic> {
        if drafts.contains_key(&target.key()) {
            let accepted = explicit(target).ok_or_else(|| {
                invalid(format!(
                    "PARAMETRIC_TYPED_VALUE_DRAFT_PENDING: {} has no accepted explicit value.",
                    target.key()
                ))
            })?;
            let proposed = values::preview(&e.snapshot, target).map_err(|message| {
                invalid(format!(
                    "PARAMETRIC_TYPED_VALUE_DRAFT_PENDING: {}: {message}",
                    target.key()
                ))
            })?;
            if &proposed != accepted {
                return Err(invalid(format!(
                    "PARAMETRIC_TYPED_VALUE_DRAFT_PENDING: {} differs from its accepted literal.",
                    target.key()
                )));
            }
        }
        Ok(())
    };
    current(target)?;
    if effective.origin == values::Origin::Default {
        current(values::Target::Default {
            property: target.property(),
        })?;
    }
    let literal = calc::Literal {
        decimal: value.text.clone(),
        quantity: calc::QuantityType {
            kind: calc::unit_kind(unit),
            unit,
        },
    };
    calc::validate_literal(&literal)?;
    Ok(literal)
}
fn criterion(e: &Editor, measure: RecordId) -> Result<Criterion, calc::Diagnostic> {
    let r = e.snapshot.records.get(&measure).ok_or_else(|| {
        error(
            calc::DiagnosticCode::MissingReference,
            "PARAMETRIC_CRITERION_OWNER_MISSING",
            vec![],
        )
    })?;
    let value = r.number("criterion").ok_or_else(|| {
        error(
            calc::DiagnosticCode::MissingReference,
            "PARAMETRIC_CRITERION_UNSET: no threshold is invented.",
            vec![],
        )
    })?;
    let unit = match r.text("unit") {
        "metre" => calc::UnitId::Metre,
        "percent" => calc::UnitId::Percent,
        "one" => calc::UnitId::ScalarOne,
        _ => {
            return Err(error(
                calc::DiagnosticCode::UnitMismatch,
                "PARAMETRIC_CRITERION_UNIT_UNSUPPORTED",
                vec![],
            ));
        }
    };
    let key = RawKey {
        record: measure,
        field: "criterion".into(),
        item: None,
    };
    if let Some(raw) = e.raw(&key) {
        if raw.validity != Validity::Valid {
            return Err(error(
                if raw.validity == Validity::Incomplete {
                    calc::DiagnosticCode::IncompleteInput
                } else {
                    calc::DiagnosticCode::InvalidInput
                },
                format!(
                    "PARAMETRIC_CRITERION_RAW_PENDING: Measure #{} retains {:?} raw {:?}; accepted criterion is not substituted.",
                    measure.0, raw.validity, raw.text
                ),
                vec![calc::CalculationId(measure.0)],
            ));
        }
        if raw
            .unit_context
            .as_deref()
            .is_some_and(|u| u != r.text("unit"))
        {
            return Err(error(
                calc::DiagnosticCode::UnitMismatch,
                "PARAMETRIC_CRITERION_RAW_UNIT_MISMATCH",
                vec![],
            ));
        }
        if calc::decimal(&raw.text)? != value {
            return Err(invalid("PARAMETRIC_CRITERION_RAW_DIFFERS_FROM_ACCEPTED"));
        }
    }
    let comparator = r.text("comparator");
    if !matches!(comparator, "GreaterOrEqual" | "LessOrEqual" | "Equal") {
        return Err(invalid("PARAMETRIC_CRITERION_COMPARATOR_UNSUPPORTED"));
    }
    let threshold = calc::Literal {
        decimal: value.to_string(),
        quantity: calc::QuantityType {
            kind: calc::unit_kind(unit),
            unit,
        },
    };
    calc::validate_literal(&threshold)?;
    Ok(Criterion {
        measure,
        threshold,
        comparator: comparator.into(),
    })
}
fn selected_forms_current(e: &Editor, a: &Specification) -> Result<(), calc::Diagnostic> {
    for f in forms(&e.snapshot)
        .map_err(invalid)?
        .values()
        .filter(|f| f.specification() == Some(a.id))
    {
        let expected = f.id().and_then(|id| accepted_form(a, id));
        if expected.as_ref() != Some(f) {
            return Err(invalid(format!(
                "PARAMETRIC_PROPOSAL_PENDING: Specification #{} form {} must be applied or explicitly discarded before preparation.",
                a.id.0,
                f.key()
            )));
        }
    }
    Ok(())
}
/// Validate sources and freeze explicit overrides; preparation never evaluates a calculation.
pub fn prepare(e: &Editor, specification: Id) -> Result<Program, calc::Diagnostic> {
    e.snapshot.validate().map_err(invalid)?;
    let s = t::load(&e.snapshot).map_err(invalid)?;
    let a = s.parametrics.get(&specification).ok_or_else(|| {
        error(
            calc::DiagnosticCode::MissingReference,
            "PARAMETRIC_SPECIFICATION_MISSING",
            vec![],
        )
    })?;
    selected_forms_current(e, a)?;
    let review = report(&e.snapshot, specification).map_err(invalid)?;
    if !review.accepted_bindings_complete {
        return Err(invalid(format!(
            "PARAMETRIC_TYPE_REVIEW_BLOCKED: {}",
            review
                .diagnostics
                .iter()
                .take(8)
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }
    let calculation = a.calculation.ok_or_else(|| {
        error(
            calc::DiagnosticCode::MissingReference,
            "PARAMETRIC_CALCULATION_UNASSIGNED",
            vec![],
        )
    })?;
    let originals = e.snapshot.calculations().map_err(invalid)?;
    let original = originals.get(&calculation).ok_or_else(|| {
        error(
            calc::DiagnosticCode::MissingReference,
            "PARAMETRIC_CALCULATION_MISSING",
            vec![calculation],
        )
    })?;
    let native_raw = e.snapshot.calculation_raw_state();
    native_raw_current(e, original, None, &native_raw)?;
    let mut selected = original.clone();
    let mut bindings = Vec::new();
    for p in a.parameters.values() {
        let b = a
            .bindings
            .values()
            .find(|b| b.parameter == p.id)
            .ok_or_else(|| invalid("PARAMETRIC_PARAMETER_UNBOUND"))?;
        let mut value_origin = None;
        let input = match &b.source {
            Source::NativeInput { calculation, name } => {
                let d = originals.get(calculation).ok_or_else(|| {
                    error(
                        calc::DiagnosticCode::MissingReference,
                        "PARAMETRIC_SOURCE_CALCULATION_MISSING",
                        vec![*calculation],
                    )
                })?;
                native_raw_current(e, d, Some(name), &native_raw)?;
                d.bindings.get(name).cloned().ok_or_else(|| {
                    error(
                        calc::DiagnosticCode::MissingReference,
                        "PARAMETRIC_SOURCE_INPUT_MISSING",
                        vec![*calculation],
                    )
                })?
            }
            Source::NativeOutput { calculation } => calc::Input::Output(*calculation),
            Source::TypedValue { target, unit } => {
                let literal = current_typed_literal(e, &s, *target, *unit)?;
                value_origin = Some(values::effective(&s, *target).map_err(invalid)?.origin);
                calc::Input::Literal(literal)
            }
        };
        selected.bindings.insert(p.name.clone(), input.clone());
        bindings.push(BindingEvidence {
            binding: b.id,
            parameter: p.id,
            input_name: p.name.clone(),
            source: b.source.clone(),
            declared: p.quantity,
            frozen_input: input,
            value_origin,
        });
    }
    let mut definitions = BTreeMap::new();
    let mut todo = vec![calculation];
    while let Some(id) = todo.pop() {
        if definitions.contains_key(&id) {
            continue;
        }
        let d = if id == calculation {
            selected.clone()
        } else {
            originals.get(&id).cloned().ok_or_else(|| {
                error(
                    calc::DiagnosticCode::MissingReference,
                    "PARAMETRIC_DEPENDENCY_MISSING",
                    vec![id],
                )
            })?
        };
        if e.snapshot
            .records
            .get(&d.owner)
            .is_none_or(|r| r.scope != a.system)
        {
            return Err(invalid(format!(
                "PARAMETRIC_DEPENDENCY_FOREIGN_SYSTEM: #{}",
                id.0
            )));
        }
        let original = originals.get(&id).ok_or_else(|| {
            error(
                calc::DiagnosticCode::MissingReference,
                "PARAMETRIC_DEPENDENCY_MISSING",
                vec![id],
            )
        })?;
        native_raw_current(e, original, None, &native_raw)?;
        calc::validate(&d, &calc::Limits::default())?;
        todo.extend(calc::dependencies(&d));
        definitions.insert(id, d);
        if definitions.len() > calc::Limits::default().definitions {
            return Err(error(
                calc::DiagnosticCode::ResourceLimit,
                "PARAMETRIC_DEFINITION_LIMIT",
                vec![],
            ));
        }
    }
    calc::validate_document_resources(&definitions, &calc::Limits::default())?;
    let overridden: BTreeSet<_> = bindings
        .iter()
        .map(|b| format!("calc.input.{}", b.input_name))
        .collect();
    let mut raw: BTreeMap<_, _> = native_raw
        .into_iter()
        .filter(|(id, _)| definitions.contains_key(id))
        .collect();
    if let Some(drafts) = raw.get_mut(&calculation) {
        drafts.retain(|d| !overridden.contains(&d.key.field));
    }
    Ok(Program {
        stamp: e.calculation_stamp(),
        specification,
        calculation,
        definitions,
        raw,
        bindings,
        criterion: criterion(e, original.owner),
    })
}
fn assess(outcome: &calc::Outcome, criterion: &Result<Criterion, calc::Diagnostic>) -> Assessment {
    let result = (|| -> Result<bool, calc::Diagnostic> {
        let value = match outcome {
            calc::Outcome::Value(v) => v,
            calc::Outcome::Failure(d) => {
                let mut e = error(
                    calc::DiagnosticCode::BlockedBy,
                    "PARAMETRIC_ASSESSMENT_BLOCKED_BY_NUMERIC_FAILURE",
                    d.related.clone(),
                );
                e.cause = Some(d.code);
                return Err(e);
            }
        };
        let criterion = criterion.clone()?;
        let threshold = calc::convert_literal(&criterion.threshold, value.quantity.unit)?;
        let threshold = calc::decimal(&threshold.decimal)?;
        Ok(match criterion.comparator.as_str() {
            "GreaterOrEqual" => value.value >= threshold,
            "LessOrEqual" => value.value <= threshold,
            "Equal" => value.value == threshold,
            _ => return Err(invalid("PARAMETRIC_CRITERION_COMPARATOR_UNSUPPORTED")),
        })
    })();
    match result {
        Ok(true) => Assessment::Met,
        Ok(false) => Assessment::NotMet,
        Err(d) => Assessment::Unavailable(d),
    }
}
/// Execute one still-current frozen program. Results and computed assessments never mutate source.
pub fn evaluate(e: &Editor, program: &Program) -> Result<Run, calc::Diagnostic> {
    if !program.is_current(e) {
        return Err(invalid(
            "PARAMETRIC_PROGRAM_STALE_OR_FOREIGN_INSTANCE: prepare from the current source; historic results retained.",
        ));
    }
    let native_receipt = calc::evaluate(
        &program.definitions,
        &program.raw,
        &program.stamp,
        &calc::Limits::default(),
    );
    let outcome = native_receipt
        .outcomes
        .get(&program.calculation)
        .cloned()
        .unwrap_or_else(|| {
            calc::Outcome::Failure(native_receipt.diagnostics.first().cloned().unwrap_or_else(
                || {
                    error(
                        calc::DiagnosticCode::ResourceLimit,
                        "PARAMETRIC_NATIVE_OUTCOME_UNAVAILABLE",
                        vec![program.calculation],
                    )
                },
            ))
        });
    let assessment = assess(&outcome, &program.criterion);
    Ok(Run{rules_revision:RULES_REVISION.into(),specification:program.specification,calculation:program.calculation,stamp:program.stamp.clone(),bindings:program.bindings.clone(),criterion:program.criterion.clone(),outcome,assessment,native_receipt,parametric_execution_performed:true,verification_written:false,limits:vec!["Ephemeral explicit input bindings over the existing native engine; native model/criteria/observations/verification are unchanged.".into(),"Native finite arithmetic and exact comparator semantics. Native criteria support metre/one/percent only; no uncertainty/tolerance/general solver/engineering acceptance.".into(),"Current literal and context drafts are checked; typed derived-expression scratch is neither executed nor copied.".into()]})
}
