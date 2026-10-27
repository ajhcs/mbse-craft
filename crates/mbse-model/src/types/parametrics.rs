//! Documentary constraints bind existing calculation/value authorities; never evaluate here.
use super::*;
use crate::stage1_calc as calc;
pub const RULES_REVISION: &str = "mbse-craft.parametrics/1";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub id: Id,
    pub name: String,
    pub quantity: calc::QuantityType,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Source {
    NativeInput {
        calculation: calc::CalculationId,
        name: String,
    },
    NativeOutput {
        calculation: calc::CalculationId,
    },
    TypedValue {
        target: values::Target,
        unit: calc::UnitId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: Id,
    pub parameter: Id,
    pub source: Source,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Specification {
    pub id: Id,
    pub system: RecordId,
    pub name: String,
    pub calculation: Option<calc::CalculationId>,
    pub parameters: BTreeMap<Id, Parameter>,
    pub bindings: BTreeMap<Id, Binding>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum SourceForm {
    NativeInput {
        calculation: String,
        name: String,
    },
    NativeOutput {
        calculation: String,
    },
    TypedValue {
        property: String,
        usage: String,
        unit: calc::UnitId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Form {
    Specification {
        id: Option<Id>,
        system: String,
        name: String,
        calculation: String,
    },
    Parameter {
        specification: Id,
        id: Option<Id>,
        name: String,
        quantity: calc::QuantityType,
    },
    Binding {
        specification: Id,
        id: Option<Id>,
        parameter: String,
        source: SourceForm,
    },
}
#[derive(Clone, Debug)]
pub enum Action {
    Set(Form),
    Apply(String),
    Discard(String),
    Remove { specification: Id, element: Id },
}
impl Form {
    pub fn id(&self) -> Option<Id> {
        match self {
            Self::Specification { id, .. }
            | Self::Parameter { id, .. }
            | Self::Binding { id, .. } => *id,
        }
    }
    pub fn specification(&self) -> Option<Id> {
        match self {
            Self::Specification { id, .. } => *id,
            Self::Parameter { specification, .. } | Self::Binding { specification, .. } => {
                Some(*specification)
            }
        }
    }
    pub fn key(&self) -> String {
        let id = self.id().map_or("new".into(), |id| id.0.to_string());
        match self {
            Self::Specification { .. } => format!("parametric:{id}"),
            Self::Parameter { specification, .. } => format!("parameter:{}:{id}", specification.0),
            Self::Binding { specification, .. } => format!("binding:{}:{id}", specification.0),
        }
    }
    fn set_id(&mut self, new: Id) {
        match self {
            Self::Specification { id, .. }
            | Self::Parameter { id, .. }
            | Self::Binding { id, .. } => *id = Some(new),
        }
    }
    fn matching(&self, s: &Store, id: Id) -> bool {
        match self {
            Self::Specification { .. } => s.parametrics.contains_key(&id),
            Self::Parameter { specification, .. } => s
                .parametrics
                .get(specification)
                .is_some_and(|a| a.parameters.contains_key(&id)),
            Self::Binding { specification, .. } => s
                .parametrics
                .get(specification)
                .is_some_and(|a| a.bindings.contains_key(&id)),
        }
    }
    fn raw(&self) -> Vec<&str> {
        match self {
            Self::Specification {
                system,
                name,
                calculation,
                ..
            } => vec![system, name, calculation],
            Self::Parameter { name, .. } => vec![name],
            Self::Binding {
                parameter, source, ..
            } => {
                let mut v = vec![parameter.as_str()];
                match source {
                    SourceForm::NativeInput { calculation, name } => {
                        v.extend([calculation.as_str(), name.as_str()])
                    }
                    SourceForm::NativeOutput { calculation } => v.push(calculation),
                    SourceForm::TypedValue {
                        property, usage, ..
                    } => v.extend([property.as_str(), usage.as_str()]),
                }
                v
            }
        }
    }
}
pub fn forms(snapshot: &Snapshot) -> Result<BTreeMap<String, Form>, String> {
    Ok(journal(snapshot)?.parametric_forms)
}
pub(super) fn ids(s: &Store) -> impl Iterator<Item = Id> + '_ {
    s.parametrics.values().flat_map(|a| {
        std::iter::once(a.id)
            .chain(a.parameters.keys().copied())
            .chain(a.bindings.keys().copied())
    })
}
pub(super) fn canonical_maps(value: &Value) -> Result<(), String> {
    if let Some(map) = value.get("parametrics") {
        canonical(value, "parametrics")?;
        for a in map.as_object().ok_or("PARAMETRIC_MAP")?.values() {
            canonical(a, "parameters")?;
            canonical(a, "bindings")?;
        }
    }
    Ok(())
}
fn reference(raw: &str) -> Result<Id, String> {
    let n = raw
        .parse::<u64>()
        .map_err(|_| "PARAMETRIC_CANONICAL_REFERENCE")?;
    if n == 0 || n.to_string() != raw {
        Err("PARAMETRIC_CANONICAL_REFERENCE".into())
    } else {
        Ok(Id(n))
    }
}
fn identifier(name: &str) -> Result<(), String> {
    if name.len() > 64 {
        return Err("PARAMETRIC_INPUT_NAME_LIMIT".into());
    }
    match calc::parse(name, &calc::Limits::default()).map_err(|e| e.to_string())? {
        calc::Expr::Binding(n) if n == name => Ok(()),
        _ => Err("PARAMETRIC_EXACT_INPUT_IDENTIFIER".into()),
    }
}
fn quantity(q: calc::QuantityType) -> Result<(), String> {
    if calc::unit_kind(q.unit) == q.kind {
        Ok(())
    } else {
        Err("PARAMETRIC_QUANTITY_UNIT_KIND_MISMATCH".into())
    }
}
fn source_shape(source: &Source) -> Result<(), String> {
    match source {
        Source::NativeInput { calculation, name } => {
            if calculation.0 == 0 {
                return Err("PARAMETRIC_CALCULATION_ID".into());
            }
            identifier(name)
        }
        Source::NativeOutput { calculation } => {
            if calculation.0 == 0 {
                Err("PARAMETRIC_CALCULATION_ID".into())
            } else {
                Ok(())
            }
        }
        Source::TypedValue { target, .. } => {
            if target.property().0 == 0
                || matches!(target,values::Target::Override{usage,..}if usage.0==0)
            {
                Err("PARAMETRIC_VALUE_ID".into())
            } else {
                Ok(())
            }
        }
    }
}
pub(super) fn validate_store(s: &Store, all: &mut BTreeSet<Id>) -> Result<(), String> {
    for (key, a) in &s.parametrics {
        identity(*key, a.id, all)?;
        name(&a.name)?;
        if a.system.0 == 0
            || a.calculation.is_some_and(|id| id.0 == 0)
            || a.parameters.len() > calc::Limits::default().bindings
            || a.bindings.len() > calc::Limits::default().bindings
        {
            return Err("PARAMETRIC_SPECIFICATION_SHAPE_OR_LIMIT".into());
        }
        let mut names = BTreeSet::new();
        for (key, p) in &a.parameters {
            identity(*key, p.id, all)?;
            identifier(&p.name)?;
            quantity(p.quantity)?;
            if !names.insert(&p.name) {
                return Err("PARAMETRIC_DUPLICATE_PARAMETER_NAME".into());
            }
        }
        let mut bound = BTreeSet::new();
        for (key, b) in &a.bindings {
            identity(*key, b.id, all)?;
            if !a.parameters.contains_key(&b.parameter) || !bound.insert(b.parameter) {
                return Err("PARAMETRIC_PARAMETER_MISSING_OR_DUPLICATE_BINDING".into());
            }
            source_shape(&b.source)?;
        }
    }
    Ok(())
}
fn identity(key: Id, id: Id, all: &mut BTreeSet<Id>) -> Result<(), String> {
    if key != id || id.0 == 0 || !all.insert(id) {
        Err("PARAMETRIC_ID_COLLISION_OR_MAP_KEY".into())
    } else {
        Ok(())
    }
}
pub(super) fn validate_targets(snapshot: &Snapshot, s: &Store) -> Result<(), String> {
    for a in s.parametrics.values() {
        if snapshot
            .records
            .get(&a.system)
            .is_none_or(|r| r.kind != RecordKind::System)
        {
            return Err("PARAMETRIC_SYSTEM_MISSING_OR_WRONG_KIND".into());
        }
    }
    Ok(())
}
pub(super) fn target_high(s: &Store) -> u64 {
    s.parametrics
        .values()
        .flat_map(|a| {
            std::iter::once(a.system.0)
                .chain(a.calculation.map(|id| id.0))
                .chain(a.bindings.values().flat_map(|b| match b.source {
                    Source::NativeInput { calculation, .. }
                    | Source::NativeOutput { calculation } => vec![calculation.0],
                    Source::TypedValue { target, .. } => match target {
                        values::Target::Default { property } => vec![property.0],
                        values::Target::Override { usage, property } => vec![usage.0, property.0],
                    },
                }))
        })
        .max()
        .unwrap_or(0)
}
pub(super) fn validate_journal(j: &Journal) -> Result<(), String> {
    if j.parametric_forms.len() > 128 {
        return Err("PARAMETRIC_FORM_COUNT".into());
    }
    let mut ids = BTreeSet::new();
    for (key, f) in &j.parametric_forms {
        if *key != f.key()
            || f.id().is_some_and(|id| id.0 == 0 || !ids.insert(id))
            || f.specification().is_some_and(|id| id.0 == 0)
            || f.raw().iter().any(|text| text.len() > 4096)
        {
            return Err("PARAMETRIC_FORM_KEY_ID_OR_LIMIT".into());
        }
    }
    Ok(())
}
pub(super) fn validate_draft_ids(
    s: &Store,
    j: &Journal,
    native: &BTreeSet<u64>,
) -> Result<u64, String> {
    let owned: BTreeSet<_> = ids(s).collect();
    let occupied: BTreeSet<_> = s
        .definitions
        .keys()
        .chain(s.properties.keys())
        .chain(s.usages.keys())
        .chain(s.interfaces.keys())
        .chain(s.ports.keys())
        .chain(s.connectors.keys())
        .copied()
        .chain(activities::ids(s))
        .chain(state_machines::ids(s))
        .chain(scenarios::ids(s))
        .chain(ids(s))
        .collect();
    let other: BTreeSet<_> = j
        .forms
        .keys()
        .chain(j.ports.keys())
        .chain(j.connectors.keys())
        .copied()
        .chain(j.usage.as_ref().and_then(|f| f.id))
        .chain(j.activity_flow.as_ref().and_then(|f| f.id))
        .chain(j.activity_forms.values().filter_map(|f| f.id()))
        .chain(j.state_machine_forms.values().filter_map(|f| f.id()))
        .chain(j.scenario_forms.values().filter_map(|f| f.id()))
        .collect();
    if other.iter().any(|id| owned.contains(id)) {
        return Err("PARAMETRIC_OTHER_DRAFT_ID_COLLISION".into());
    }
    let mut high = 0;
    for f in j.parametric_forms.values() {
        high = high.max(f.specification().map_or(0, |id| id.0));
        if let Some(id) = f.id() {
            if other.contains(&id)
                || ((native.contains(&id.0) || occupied.contains(&id)) && !f.matching(s, id))
            {
                return Err("PARAMETRIC_FORM_FOREIGN_ID_COLLISION".into());
            }
            high = high.max(id.0);
        }
    }
    Ok(high)
}
fn definition<'a>(
    snapshot: &Snapshot,
    definitions: &'a BTreeMap<calc::CalculationId, calc::Definition>,
    system: RecordId,
    id: calc::CalculationId,
) -> Result<&'a calc::Definition, String> {
    let d = definitions
        .get(&id)
        .ok_or_else(|| format!("PARAMETRIC_CALCULATION_MISSING: #{}", id.0))?;
    if snapshot
        .records
        .get(&d.owner)
        .is_none_or(|r| r.kind != RecordKind::Measure || r.scope != system)
    {
        return Err(format!(
            "PARAMETRIC_CALCULATION_FOREIGN_SYSTEM_OR_KIND: #{}",
            id.0
        ));
    }
    Ok(d)
}
fn input_type(
    defs: &BTreeMap<calc::CalculationId, calc::Definition>,
    input: &calc::Input,
) -> Result<calc::QuantityType, String> {
    match input {
        calc::Input::Literal(l) => Ok(l.quantity),
        calc::Input::Output(id) => defs
            .get(id)
            .map(|d| d.output)
            .ok_or_else(|| format!("PARAMETRIC_INPUT_OUTPUT_MISSING: #{}", id.0)),
    }
}
fn declared_type(
    snapshot: &Snapshot,
    defs: &BTreeMap<calc::CalculationId, calc::Definition>,
    a: &Specification,
    p: &Parameter,
) -> Result<(), String> {
    if let Some(id) = a.calculation {
        let d = definition(snapshot, defs, a.system, id)?;
        let input = d
            .bindings
            .get(&p.name)
            .ok_or_else(|| format!("PARAMETRIC_DECLARED_INPUT_MISSING: {}", p.name))?;
        compatible(p.quantity, input_type(defs, input)?)?;
    }
    Ok(())
}
fn compatible(expected: calc::QuantityType, actual: calc::QuantityType) -> Result<(), String> {
    quantity(expected)?;
    quantity(actual)?;
    if expected.kind != actual.kind {
        Err(format!(
            "PARAMETRIC_BINDING_QUANTITY_KIND_MISMATCH: expected {:?} ({:?}), source {:?} ({:?})",
            expected.kind, expected.unit, actual.kind, actual.unit
        ))
    } else {
        Ok(())
    }
}
fn source_type(
    snapshot: &Snapshot,
    s: &Store,
    defs: &BTreeMap<calc::CalculationId, calc::Definition>,
    a: &Specification,
    source: &Source,
) -> Result<(calc::QuantityType, bool), String> {
    match source {
        Source::NativeInput { calculation, name } => {
            let d = definition(snapshot, defs, a.system, *calculation)?;
            let input = d
                .bindings
                .get(name)
                .ok_or_else(|| format!("PARAMETRIC_SOURCE_INPUT_MISSING: {name}"))?;
            if let calc::Input::Output(id) = input {
                definition(snapshot, defs, a.system, *id)?;
            }
            Ok((input_type(defs, input)?, true))
        }
        Source::NativeOutput { calculation } => Ok((
            definition(snapshot, defs, a.system, *calculation)?.output,
            true,
        )),
        Source::TypedValue { target, unit } => {
            let p = s
                .properties
                .get(&target.property())
                .ok_or("PARAMETRIC_VALUE_PROPERTY_MISSING")?;
            let owner = s
                .definitions
                .get(&p.owner)
                .ok_or("PARAMETRIC_VALUE_OWNER_MISSING")?;
            if owner.system != a.system {
                return Err("PARAMETRIC_VALUE_FOREIGN_SYSTEM".into());
            }
            let scalar = match s.definitions.get(&p.value_type).map(|d| d.kind) {
                Some(Kind::Value(Scalar::Real)) => Scalar::Real,
                Some(Kind::Value(Scalar::Integer)) => Scalar::Integer,
                _ => return Err("PARAMETRIC_NUMERIC_SCALAR_VALUE_REQUIRED".into()),
            };
            let effective = values::effective(s, *target)?;
            let q = calc::QuantityType {
                kind: calc::unit_kind(*unit),
                unit: *unit,
            };
            if let Some(value) = &effective.value {
                if value.kind != scalar {
                    return Err("PARAMETRIC_VALUE_TYPE_CHANGED".into());
                }
                calc::validate_literal(&calc::Literal {
                    decimal: value.text.clone(),
                    quantity: q,
                })
                .map_err(|e| e.to_string())?;
            }
            Ok((q, effective.value.is_some()))
        }
    }
}
fn source_from_form(f: &SourceForm) -> Result<Source, String> {
    Ok(match f {
        SourceForm::NativeInput { calculation, name } => {
            identifier(name)?;
            Source::NativeInput {
                calculation: calc::CalculationId(reference(calculation)?.0),
                name: name.clone(),
            }
        }
        SourceForm::NativeOutput { calculation } => Source::NativeOutput {
            calculation: calc::CalculationId(reference(calculation)?.0),
        },
        SourceForm::TypedValue {
            property,
            usage,
            unit,
        } => {
            let property = reference(property)?;
            Source::TypedValue {
                target: if usage.is_empty() {
                    values::Target::Default { property }
                } else {
                    values::Target::Override {
                        usage: reference(usage)?,
                        property,
                    }
                },
                unit: *unit,
            }
        }
    })
}
pub(super) fn apply_inner(snapshot: &mut Snapshot, action: Action) -> Result<Change, String> {
    let mut j = journal(snapshot)?;
    match action {
        Action::Set(f) => {
            j.parametric_forms.insert(f.key(), f);
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Discard(key) => {
            if j.parametric_forms.remove(&key).is_none() {
                return Err("PARAMETRIC_FORM_UNAVAILABLE".into());
            }
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Raw)
        }
        Action::Apply(key) => {
            let mut f = j
                .parametric_forms
                .get(&key)
                .cloned()
                .ok_or("PARAMETRIC_FORM_UNAVAILABLE")?;
            let mut s = load(snapshot)?;
            if f.id().is_some_and(|id| !f.matching(&s, id)) {
                return Err("PARAMETRIC_DETACHED_EDIT_CREATE_EXPLICITLY".into());
            }
            let id = if let Some(id) = f.id() {
                id
            } else {
                allocate(snapshot)?
            };
            let defs = snapshot.calculations()?;
            match &f {
                Form::Specification {
                    system,
                    name,
                    calculation,
                    ..
                } => {
                    let system = RecordId(reference(system)?.0);
                    if snapshot
                        .records
                        .get(&system)
                        .is_none_or(|r| r.kind != RecordKind::System)
                    {
                        return Err("PARAMETRIC_SYSTEM_MISSING_OR_WRONG_KIND".into());
                    }
                    let calculation = if calculation.is_empty() {
                        None
                    } else {
                        let c = calc::CalculationId(reference(calculation)?.0);
                        definition(snapshot, &defs, system, c)?;
                        Some(c)
                    };
                    let a = s.parametrics.entry(id).or_insert(Specification {
                        id,
                        system,
                        name: name.clone(),
                        calculation,
                        parameters: BTreeMap::new(),
                        bindings: BTreeMap::new(),
                    });
                    if a.system != system {
                        return Err("PARAMETRIC_SYSTEM_MOVE_UNSUPPORTED".into());
                    }
                    a.name = name.clone();
                    a.calculation = calculation;
                }
                Form::Parameter {
                    specification,
                    name,
                    quantity,
                    ..
                } => {
                    identifier(name)?;
                    self::quantity(*quantity)?;
                    let a = s
                        .parametrics
                        .get_mut(specification)
                        .ok_or("PARAMETRIC_SPECIFICATION_MISSING_OR_FOREIGN")?;
                    let p = Parameter {
                        id,
                        name: name.clone(),
                        quantity: *quantity,
                    };
                    declared_type(snapshot, &defs, a, &p)?;
                    a.parameters.insert(id, p);
                }
                Form::Binding {
                    specification,
                    parameter,
                    source,
                    ..
                } => {
                    let a = s
                        .parametrics
                        .get(specification)
                        .ok_or("PARAMETRIC_SPECIFICATION_MISSING_OR_FOREIGN")?;
                    let parameter = reference(parameter)?;
                    let p = a
                        .parameters
                        .get(&parameter)
                        .ok_or("PARAMETRIC_PARAMETER_MISSING_OR_FOREIGN")?;
                    let source = source_from_form(source)?;
                    let (q, _) = source_type(snapshot, &s, &defs, a, &source)?;
                    declared_type(snapshot, &defs, a, p)?;
                    compatible(p.quantity, q)?;
                    s.parametrics
                        .get_mut(specification)
                        .unwrap()
                        .bindings
                        .insert(
                            id,
                            Binding {
                                id,
                                parameter,
                                source,
                            },
                        );
                }
            }
            s.validate()?;
            f.set_id(id);
            j.parametric_forms.remove(&key);
            j.parametric_forms.insert(f.key(), f);
            put(snapshot, NAMESPACE, &s)?;
            put(snapshot, DRAFTS, &j)?;
            Ok(Change::Model)
        }
        Action::Remove {
            specification,
            element,
        } => {
            let mut s = load(snapshot)?;
            if specification == element {
                if s.parametrics.remove(&specification).is_none() {
                    return Err("PARAMETRIC_SPECIFICATION_MISSING".into());
                }
            } else {
                let a = s
                    .parametrics
                    .get_mut(&specification)
                    .ok_or("PARAMETRIC_SPECIFICATION_MISSING_OR_FOREIGN")?;
                if a.bindings.values().any(|b| b.parameter == element) {
                    return Err("PARAMETRIC_PARAMETER_IN_USE".into());
                }
                if a.parameters.remove(&element).is_none() && a.bindings.remove(&element).is_none()
                {
                    return Err("PARAMETRIC_ELEMENT_MISSING_OR_FOREIGN".into());
                }
            }
            put(snapshot, NAMESPACE, &s)?;
            Ok(Change::Model)
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub elements: Vec<Id>,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    pub rules_revision: String,
    pub native_algorithm_revision: String,
    pub unit_table_revision: String,
    pub document_id: crate::stage1::DocumentId,
    pub model_revision: u64,
    pub raw_version: u64,
    pub snapshot_hash: String,
    pub specification: Id,
    pub unbound: Vec<Id>,
    pub diagnostics: Vec<Diagnostic>,
    pub accepted_bindings_complete: bool,
    pub execution_performed: bool,
    pub limits: Vec<String>,
}
pub fn report(snapshot: &Snapshot, id: Id) -> Result<Report, String> {
    snapshot.validate()?;
    let s = load(snapshot)?;
    let a = s
        .parametrics
        .get(&id)
        .ok_or("PARAMETRIC_SPECIFICATION_MISSING")?;
    let defs = snapshot.calculations()?;
    let mut diagnostics = Vec::new();
    let mut add = |code: &str, elements: Vec<Id>, message: String| {
        diagnostics.push(Diagnostic {
            code: code.into(),
            elements,
            message,
        })
    };
    if let Some(calc) = a.calculation {
        match definition(snapshot, &defs, a.system, calc) {
            Ok(d) => {
                for name in d.bindings.keys() {
                    if !a.parameters.values().any(|p| &p.name == name) {
                        add(
                            "PARAMETRIC_INPUT_UNDECLARED",
                            vec![id],
                            format!(
                                "Calculation #{} input {name} has no declared parameter.",
                                calc.0
                            ),
                        );
                    }
                }
            }
            Err(e) => add("PARAMETRIC_CALCULATION_UNRESOLVED", vec![id], e),
        }
    } else {
        add(
            "PARAMETRIC_CALCULATION_UNASSIGNED",
            vec![id],
            "No native calculation attached; specification is retained independently of execution."
                .into(),
        );
    }
    let mut unbound = Vec::new();
    for p in a.parameters.values() {
        if let Err(e) = declared_type(snapshot, &defs, a, p) {
            add("PARAMETRIC_DECLARATION_UNRESOLVED", vec![id, p.id], e);
        }
        if let Some(b) = a.bindings.values().find(|b| b.parameter == p.id) {
            match source_type(snapshot,&s,&defs,a,&b.source).and_then(|(q,set)|compatible(p.quantity,q).map(|_|set)){Ok(false)=>add("PARAMETRIC_BOUND_VALUE_UNSET",vec![id,p.id,b.id],"Exact typed-value binding has no accepted default/override; no value is invented.".into()),Ok(true)=>{},Err(e)=>add("PARAMETRIC_BINDING_UNRESOLVED",vec![id,p.id,b.id],e)}
        } else {
            unbound.push(p.id);
            add(
                "PARAMETRIC_PARAMETER_UNBOUND",
                vec![id, p.id],
                format!(
                    "Input {} is unbound; no native literal/default is used implicitly.",
                    p.name
                ),
            );
        }
    }
    if a.parameters.is_empty() {
        add(
            "PARAMETRIC_PARAMETERS_EMPTY",
            vec![id],
            "No parameters declared; specification remains incomplete.".into(),
        );
    }
    Ok(Report{rules_revision:RULES_REVISION.into(),native_algorithm_revision:calc::ALGORITHM_REVISION.into(),unit_table_revision:calc::UNIT_TABLE_REVISION.into(),document_id:snapshot.document_id.clone(),model_revision:snapshot.model_revision,raw_version:snapshot.raw_version,snapshot_hash:snapshot.digest(),specification:id,unbound,accepted_bindings_complete:diagnostics.is_empty(),diagnostics,execution_performed:false,limits:vec!["Accepted source types only; no evaluation, current raw validity, cycles, equation solving or engineering acceptance.".into(),"Typed-value units are authored on the binding, not inherited from scalar definitions. Same quantity kind permits native unit conversion; no conversion is executed here.".into()]})
}

/// Canonical authored form for one exact accepted object; reused by UI and execution freshness.
pub fn accepted_form(a: &Specification, id: Id) -> Option<Form> {
    if id == a.id {
        return Some(Form::Specification {
            id: Some(id),
            system: a.system.0.to_string(),
            name: a.name.clone(),
            calculation: a.calculation.map_or(String::new(), |id| id.0.to_string()),
        });
    }
    if let Some(p) = a.parameters.get(&id) {
        return Some(Form::Parameter {
            specification: a.id,
            id: Some(id),
            name: p.name.clone(),
            quantity: p.quantity,
        });
    }
    a.bindings.get(&id).map(|b| Form::Binding {
        specification: a.id,
        id: Some(id),
        parameter: b.parameter.0.to_string(),
        source: match &b.source {
            Source::NativeInput { calculation, name } => SourceForm::NativeInput {
                calculation: calculation.0.to_string(),
                name: name.clone(),
            },
            Source::NativeOutput { calculation } => SourceForm::NativeOutput {
                calculation: calculation.0.to_string(),
            },
            Source::TypedValue { target, unit } => SourceForm::TypedValue {
                property: target.property().0.to_string(),
                usage: match target {
                    values::Target::Override { usage, .. } => usage.0.to_string(),
                    _ => String::new(),
                },
                unit: *unit,
            },
        },
    })
}

pub mod analysis;
