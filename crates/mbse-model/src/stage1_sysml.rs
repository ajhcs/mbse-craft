//! Closed, source-preserving Stage1 SysML textual interchange.
//!
//! Real custom draft parts, typed values, references and calculations carry the
//! supported meaning. This module does not implement general SysML resolution,
//! normative execution, conformance, filesystem I/O or native session history.
use crate::stage1::{DocumentId, Record, RecordId, RecordKind, Snapshot};
use crate::stage1_calc::{
    self as calc, CalculationId, Definition as Calculation, Expr as CalculationExpr, Function,
    Input, Literal as QuantityLiteral, QuantityKind, QuantityType, UnitId,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use sysmlv2_syntax::{ast::*, lexer::tokenize, parser::parse_source, span::Span, token::TokenKind};

pub const PROFILE: &str = "kernspan.stage1.sysml/1";
pub const PARSER_REVISION: &str = "821221767c3c56cb1ebe7da22666197a47c9c645";
const ONTOLOGY: &str = "KernspanStage1DraftOntology";
const SOURCE_FIELD: &str = "_kernspanSysmlSource";
const LIBRARY_NAMESPACES: &[&str] = &[
    "ScalarValues",
    "MeasurementReferences",
    "ISQ",
    "SI",
    "RealFunctions",
    "Calculations",
    "Quantities",
    ONTOLOGY,
];
fn symbol_name(name: &str, span: Option<Span>) -> Result<()> {
    if name.contains("::") || LIBRARY_NAMESPACES.contains(&name) {
        return Err(issue(ExchangeCode::AmbiguousSymbol, span,
            "Local symbol collides with a registered namespace or embeds a qualified-path separator"));
    }
    Ok(())
}
fn qualified_name(name: &QualifiedName) -> Result<String> {
    if name.is_global
        || name
            .segments
            .iter()
            .any(|segment| segment.value.contains("::"))
    {
        return Err(issue(ExchangeCode::UnsupportedConstruct, Some(name.span),
            "Qualified paths require actual name segments; global or embedded-separator spellings are outside this profile"));
    }
    Ok(name.to_display_string())
}

const MAX_DIAGNOSTICS: usize = 256;

/// Original, independently authored, fixed application ontology. These types are
/// explicitly draft concepts, not normative mission/requirement/measure types.
pub const ONTOLOGY_SOURCE: &str = r#"package KernspanStage1DraftOntology {
    enum def Comparator { enum GreaterOrEqual; enum LessOrEqual; enum Equal; }
    enum def LegacyQuantity { enum Dimensionless; enum Length; }
    enum def LegacyUnit { enum One; enum Percent; enum Metre; }
    enum def AffiliationKind { enum Unknown; enum Internal; enum External; }
    enum def ContextKind { enum Description; enum Assumption; enum AcceptedConstraint; }
    attribute def RatioValue :> MeasurementReferences::DimensionOneValue;
    attribute percent : MeasurementReferences::DimensionOneUnit {
        attribute :>> unitConversion : MeasurementReferences::ConversionByConvention {
            attribute :>> referenceUnit = MeasurementReferences::one;
            attribute :>> conversionFactor = 0.01;
        }
    }
    part def RecordDraft {
        attribute id : ScalarValues::Integer;
        attribute displayName : ScalarValues::String;
        ref part scope : SystemDraft;
    }
    part def SystemDraft :> RecordDraft { attribute purpose : ScalarValues::String[0..1]; }
    part def MissionDraft :> RecordDraft { attribute shortLabel : ScalarValues::String[0..1]; }
    part def CapabilityDraft :> RecordDraft { ref part contributesTo : MissionDraft[0..*]; }
    part def ObjectiveDraft :> RecordDraft {
        attribute outcome : ScalarValues::String[0..1];
        ref part 'subject' : RecordDraft[0..1];
    }
    part def MeasureDraft :> RecordDraft {
        ref part forObjective : ObjectiveDraft;
        attribute quantitySelection : LegacyQuantity[0..1];
        attribute unitSelection : LegacyUnit[0..1];
        attribute criterion : Quantities::ScalarQuantityValue[0..1];
        attribute comparator : Comparator[0..1];
        attribute assessmentMethod : ScalarValues::String[0..1];
        attribute conditions : ScalarValues::String[0..1];
        attribute uncertainty : ScalarValues::String[0..1];
        ref calc calculation : Calculations::Calculation[0..1];
    }
    part def StakeholderDraft :> RecordDraft { attribute interest : ScalarValues::String[0..1]; }
    part def ExpectationDraft :> RecordDraft {
        ref part expressedBy : StakeholderDraft[0..1];
        ref part concernsSystem : SystemDraft[0..1];
        attribute text : ScalarValues::String[0..1];
    }
    part def EnvironmentDraft :> RecordDraft;
    part def OrganizationDraft :> RecordDraft;
    part def ActorDraft :> RecordDraft;
    part def ContextDraft :> RecordDraft {
        attribute kind : ContextKind;
        attribute text : ScalarValues::String;
    }
    part def AffiliationDraft {
        ref part organization : OrganizationDraft;
        attribute relationship : AffiliationKind;
    }
}"#;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}
impl From<Span> for SourceSpan {
    fn from(span: Span) -> Self {
        Self {
            byte_start: span.start as usize,
            byte_end: span.end as usize,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticId {
    Record(RecordId),
    Context(RecordId),
    Calculation(CalculationId),
    Binding(CalculationId, String),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSymbol {
    pub stable_id: SemanticId,
    pub qualified_name: Vec<String>,
    pub declaration: SourceSpan,
    pub value: Option<SourceSpan>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetainedSysmlSource {
    pub logical_key: String,
    pub source_utf8: String,
    pub sha256: String,
    pub profile: String,
    pub parser_revision: String,
    pub symbols: Vec<SourceSymbol>,
}
/// An oversized asset stays owned by the caller; copying it here would violate
/// the admission bound. In-bound failures, including invalid UTF8, retain bytes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceAsset {
    pub logical_key: String,
    pub original_bytes: Option<Vec<u8>>,
    pub byte_len: usize,
    pub sha256: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExchangeCode {
    Syntax,
    UnsupportedConstruct,
    UnsupportedField,
    Limit,
    DuplicateIdentity,
    AmbiguousSymbol,
    MissingSymbol,
    WrongType,
    Scope,
    Unit,
    QuantityKind,
    Calculation,
    SemanticMismatch,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExchangeDiagnostic {
    pub code: ExchangeCode,
    pub span: Option<SourceSpan>,
    pub symbol: Option<String>,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExchangeFailure {
    pub source_asset: SourceAsset,
    pub diagnostics: Vec<ExchangeDiagnostic>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExcludedState {
    NativeDocumentIdentity,
    RevisionCounters,
    RawDrafts,
    CreationDrafts,
    InteractiveHistory,
    DerivedResults,
    ViewState,
    SourceFormattingAndArchive,
    NativeJournalOwnershipMetadata,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExchangeModel {
    pub records: BTreeMap<RecordId, Record>,
    /// Derived checked view, never a second authoritative persistent store.
    pub calculations: BTreeMap<CalculationId, Calculation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportCandidate {
    pub semantics: ExchangeModel,
    pub source: RetainedSysmlSource,
    pub excluded_session_state: Vec<ExcludedState>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExportCandidate {
    pub utf8: String,
    pub sha256: String,
    pub semantic_digest: String,
    pub report: Vec<ExchangeDiagnostic>,
    pub excluded_session_state: Vec<ExcludedState>,
}

type Result<T> = std::result::Result<T, ExchangeDiagnostic>;
fn issue(code: ExchangeCode, span: Option<Span>, message: impl Into<String>) -> ExchangeDiagnostic {
    ExchangeDiagnostic {
        code,
        span: span.map(Into::into),
        symbol: None,
        message: message.into(),
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn exclusions() -> Vec<ExcludedState> {
    vec![
        ExcludedState::NativeDocumentIdentity,
        ExcludedState::RevisionCounters,
        ExcludedState::RawDrafts,
        ExcludedState::CreationDrafts,
        ExcludedState::InteractiveHistory,
        ExcludedState::DerivedResults,
        ExcludedState::ViewState,
        ExcludedState::SourceFormattingAndArchive,
        ExcludedState::NativeJournalOwnershipMetadata,
    ]
}
fn failure(key: &str, bytes: &[u8], diagnostics: Vec<ExchangeDiagnostic>) -> ExchangeFailure {
    ExchangeFailure {
        source_asset: SourceAsset {
            logical_key: key.into(),
            original_bytes: (bytes.len() <= crate::boundary::MAX_SOURCE).then(|| bytes.to_vec()),
            byte_len: bytes.len(),
            sha256: digest(bytes),
        },
        diagnostics,
    }
}

/// Construct the interchange view from authoritative native record fields.
/// Unknown semantic fields are checked by export/digest, not silently discarded.
pub fn from_records(records: &BTreeMap<RecordId, Record>) -> Result<ExchangeModel> {
    let calculations = collect_calculations(records)?;
    Ok(ExchangeModel {
        records: records.clone(),
        calculations,
    })
}
fn collect_calculations(
    records: &BTreeMap<RecordId, Record>,
) -> Result<BTreeMap<CalculationId, Calculation>> {
    let mut definitions = BTreeMap::new();
    for record in records.values() {
        if let Some(value) = record.fields.get("calculation") {
            let def: Calculation = serde_json::from_value(value.clone()).map_err(|e| {
                issue(
                    ExchangeCode::Calculation,
                    None,
                    format!("Malformed calculation on Record{}: {e}", record.id.0),
                )
            })?;
            if record.kind != RecordKind::Measure
                || def.owner != record.id
                || def.id != CalculationId(record.id.0)
            {
                return Err(issue(
                    ExchangeCode::Calculation,
                    None,
                    "Calculation identity/owner must be its actual Measure",
                ));
            }
            calc::validate(&def, &calc::Limits::default())
                .map_err(|e| issue(ExchangeCode::Calculation, None, format!("{e:?}")))?;
            definitions.insert(def.id, def);
        }
    }
    calc::validate_document_resources(&definitions, &calc::Limits::default())
        .map_err(|error| issue(ExchangeCode::Limit, None, error.to_string()))?;
    Ok(definitions)
}

pub fn import(key: &str, bytes: &[u8]) -> std::result::Result<ImportCandidate, ExchangeFailure> {
    let source = crate::boundary::preflight(key, bytes).map_err(|e| {
        failure(
            key,
            bytes,
            vec![issue(
                if e.contains("LIMIT") || e.contains("LARGE") {
                    ExchangeCode::Limit
                } else {
                    ExchangeCode::Syntax
                },
                None,
                e,
            )],
        )
    })?;
    // Parsing, AST validation, projection and drop all use the qualified stack.
    let parsed = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("stage1-sysml-parser".into())
            .stack_size(sysmlv2_syntax::parser::MAX_NESTING_STACK_BYTES)
            .spawn_scoped(scope, || read_model(&source))
            .map_err(|e| {
                vec![issue(
                    ExchangeCode::Limit,
                    None,
                    format!("PARSER_STACK_UNAVAILABLE: {e}"),
                )]
            })?
            .join()
            .map_err(|_| vec![issue(ExchangeCode::Limit, None, "PARSER_THREAD_FAILED")])?
    })
    .map_err(|diagnostics| failure(key, bytes, diagnostics))?;
    Ok(ImportCandidate {
        semantics: parsed.model,
        source: RetainedSysmlSource {
            logical_key: key.into(),
            source_utf8: source,
            sha256: digest(bytes),
            profile: PROFILE.into(),
            parser_revision: PARSER_REVISION.into(),
            symbols: parsed.symbols,
        },
        excluded_session_state: exclusions(),
    })
}

/// Write a new textual candidate and re-import it before releasing any bytes.
/// Root owns create-new disk I/O and the distinct non-native-Saved export receipt.
pub fn export(
    model: &ExchangeModel,
) -> std::result::Result<ExportCandidate, Vec<ExchangeDiagnostic>> {
    let expected = semantic_digest(model).map_err(|e| vec![e])?;
    let utf8 = write_model(model).map_err(|e| vec![e])?;
    let reparsed = import("export.sysml", utf8.as_bytes()).map_err(|e| e.diagnostics)?;
    let actual = semantic_digest(&reparsed.semantics).map_err(|e| vec![e])?;
    if actual != expected {
        return Err(vec![issue(
            ExchangeCode::SemanticMismatch,
            None,
            "Supported semantic digest changed across generated SysML roundtrip",
        )]);
    }
    Ok(ExportCandidate {
        sha256: digest(utf8.as_bytes()),
        utf8,
        semantic_digest: expected,
        report: vec![],
        excluded_session_state: exclusions(),
    })
}

/// Canonical semantic equality excludes session state, source bytes and caches.
/// Only known optional null/empty-container absence is normalized. Authored zero,
/// empty text, quantity kind/unit and calculation definitions remain meaningful.
pub fn semantic_digest(model: &ExchangeModel) -> Result<String> {
    check_model(model)?;
    let mut records = model.records.clone();
    for record in records.values_mut() {
        record.fields.remove(SOURCE_FIELD);
        record.fields.remove("_kernspanCalculationRawOwners");
        record
            .fields
            .retain(|key, value| !optional_absence(key, value));
        if let Some(value) = record.fields.get_mut("criterion") {
            if let Some(number) = value.as_f64() {
                *value = json!(if number == 0.0 { 0.0 } else { number });
            }
        }
        // Decimal spelling stays in the native/source asset. Semantic numeric
        // comparison treats +1,1.0 and1e0 as the same finite literal value.
        if let Some(value) = record.fields.get_mut("calculation") {
            let mut definition: Calculation = serde_json::from_value(value.clone())
                .map_err(|e| issue(ExchangeCode::Calculation, None, e.to_string()))?;
            normalize_definition(&mut definition)?;
            *value = serde_json::to_value(definition)
                .map_err(|e| issue(ExchangeCode::Calculation, None, e.to_string()))?;
        }
    }
    serde_json::to_vec(&(PROFILE, records))
        .map(|bytes| digest(&bytes))
        .map_err(|e| issue(ExchangeCode::SemanticMismatch, None, e.to_string()))
}
fn optional_absence(key: &str, value: &Value) -> bool {
    if value.is_null() {
        return matches!(key, "subject" | "quantity" | "unit" | "criterion");
    }
    (matches!(
        key,
        "contributesTo" | "observations" | "actors" | "inputs" | "context"
    ) && value.as_array().is_some_and(Vec::is_empty))
        || (key == "affiliations" && value.as_object().is_some_and(serde_json::Map::is_empty))
}
fn canonical_decimal(number: f64) -> String {
    if number == 0.0 {
        return "0".into();
    }
    let fixed = number.to_string();
    if fixed.len() <= 128 {
        fixed
    } else {
        format!("{number:e}")
    }
}
fn normalize_literal(literal: &mut QuantityLiteral) -> Result<()> {
    let number = calc::decimal(&literal.decimal)
        .map_err(|e| issue(ExchangeCode::Calculation, None, format!("{e:?}")))?;
    literal.decimal = canonical_decimal(number);
    Ok(())
}
fn normalize_expression(expr: &mut CalculationExpr) -> Result<()> {
    match expr {
        CalculationExpr::Literal(value) => normalize_literal(value)?,
        CalculationExpr::Binding(_) => {}
        CalculationExpr::Neg(a) => {
            normalize_expression(a)?;
            if let CalculationExpr::Literal(value) = a.as_ref() {
                let mut literal = value.clone();
                let number = calc::decimal(&literal.decimal)
                    .map_err(|e| issue(ExchangeCode::Calculation, None, format!("{e:?}")))?;
                literal.decimal = canonical_decimal(-number);
                *expr = CalculationExpr::Literal(literal);
            }
        }
        CalculationExpr::Add(a, b)
        | CalculationExpr::Sub(a, b)
        | CalculationExpr::Mul(a, b)
        | CalculationExpr::Div(a, b) => {
            normalize_expression(a)?;
            normalize_expression(b)?;
        }
        CalculationExpr::Call(_, args) => {
            for arg in args {
                normalize_expression(arg)?;
            }
        }
    }
    Ok(())
}
fn normalize_definition(definition: &mut Calculation) -> Result<()> {
    for input in definition.bindings.values_mut() {
        if let Input::Literal(literal) = input {
            normalize_literal(literal)?;
        }
    }
    normalize_expression(&mut definition.expression)
}

fn text_fields(kind: RecordKind) -> &'static [&'static str] {
    match kind {
        RecordKind::System => &["purpose"],
        RecordKind::Mission => &["shortLabel"],
        RecordKind::Objective => &["outcome"],
        RecordKind::Measure => &["assessmentMethod", "conditions", "uncertainty"],
        RecordKind::Stakeholder => &["interest"],
        RecordKind::Expectation => &["text"],
        _ => &[],
    }
}
fn other_fields(kind: RecordKind) -> &'static [&'static str] {
    match kind {
        RecordKind::Capability => &["contributesTo"],
        RecordKind::Objective => &["subject"],
        RecordKind::Measure => &[
            "forObjective",
            "quantity",
            "unit",
            "criterion",
            "comparator",
            "observations",
            "calculation",
        ],
        RecordKind::Stakeholder => &["affiliations", "actors"],
        RecordKind::Expectation => &["expressedBy", "concernsSystem"],
        RecordKind::Environment => &["context", "inputs"],
        _ => &[],
    }
}
fn kind_type(kind: RecordKind) -> String {
    format!("{ONTOLOGY}::{}Draft", kind.label())
}
fn parse_kind(name: &str) -> Option<RecordKind> {
    [
        RecordKind::System,
        RecordKind::Mission,
        RecordKind::Capability,
        RecordKind::Objective,
        RecordKind::Measure,
        RecordKind::Stakeholder,
        RecordKind::Expectation,
        RecordKind::Environment,
        RecordKind::Organization,
        RecordKind::Actor,
    ]
    .into_iter()
    .find(|kind| kind_type(*kind) == name)
}
fn scalar_type() -> QuantityType {
    QuantityType {
        kind: QuantityKind::Scalar,
        unit: UnitId::ScalarOne,
    }
}
fn quantity_name(kind: QuantityKind) -> &'static str {
    match kind {
        QuantityKind::Scalar => "ScalarValues::Real",
        QuantityKind::Ratio => "KernspanStage1DraftOntology::RatioValue",
        QuantityKind::Count => "MeasurementReferences::CountValue",
        QuantityKind::Length => "ISQ::LengthValue",
        QuantityKind::Duration => "ISQ::DurationValue",
        QuantityKind::Speed => "ISQ::SpeedValue",
    }
}
fn quantity_kind(name: &str) -> Option<QuantityKind> {
    [
        QuantityKind::Scalar,
        QuantityKind::Ratio,
        QuantityKind::Count,
        QuantityKind::Length,
        QuantityKind::Duration,
        QuantityKind::Speed,
    ]
    .into_iter()
    .find(|kind| quantity_name(*kind) == name)
}
fn unit_text(unit: UnitId) -> &'static str {
    match unit {
        UnitId::ScalarOne => "",
        UnitId::RatioOne | UnitId::Count => "MeasurementReferences::one",
        UnitId::Percent => "KernspanStage1DraftOntology::percent",
        UnitId::Metre => "SI::m",
        UnitId::Kilometre => "SI::km",
        UnitId::Second => "SI::s",
        UnitId::Minute => "SI::min",
        UnitId::MetrePerSecond => "SI::m / SI::s",
        UnitId::KilometrePerHour => "SI::km / SI::h",
    }
}
fn legacy_type(quantity: &str) -> Result<&'static str> {
    match quantity {
        "dimensionless" => Ok("MeasurementReferences::DimensionOneValue"),
        "length" => Ok("ISQ::LengthValue"),
        _ => Err(issue(
            ExchangeCode::QuantityKind,
            None,
            "Unsupported criterion quantity kind",
        )),
    }
}
fn legacy_unit(unit: &str) -> Result<&'static str> {
    match unit {
        "one" => Ok("MeasurementReferences::one"),
        "percent" => Ok("KernspanStage1DraftOntology::percent"),
        "metre" => Ok("SI::m"),
        _ => Err(issue(
            ExchangeCode::Unit,
            None,
            "Unsupported criterion unit",
        )),
    }
}
fn check_model(model: &ExchangeModel) -> Result<()> {
    if model.records.len() > crate::boundary::MAX_ELEMENTS {
        return Err(issue(ExchangeCode::Limit, None, "Element limit"));
    }
    if collect_calculations(&model.records)? != model.calculations {
        return Err(issue(
            ExchangeCode::Calculation,
            None,
            "Derived calculation map differs from authoritative record fields",
        ));
    }
    // Resolve actual output identities before generic Snapshot allocator checks,
    // so a dangling dependency retains its precise missing-symbol diagnosis.
    for def in model.calculations.values() {
        for name in def.bindings.keys() {
            symbol_name(name, None)?;
            if name == "result" {
                return Err(issue(
                    ExchangeCode::AmbiguousSymbol,
                    None,
                    "Input name result collides with the profile's explicit return feature",
                ));
            }
        }
        for dependency in calc::dependencies(def) {
            let target = model.calculations.get(&dependency).ok_or_else(|| {
                issue(
                    ExchangeCode::MissingSymbol,
                    None,
                    format!("Missing calculation{}", dependency.0),
                )
            })?;
            if model.records[&def.owner].scope != model.records[&target.owner].scope {
                return Err(issue(
                    ExchangeCode::Scope,
                    None,
                    "Calculation dependency crosses System scope",
                ));
            }
        }
    }
    let max = model
        .records
        .values()
        .flat_map(|r| {
            std::iter::once(r.id.0).chain(
                r.fields
                    .get("context")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|x| x.get("id").and_then(Value::as_u64)),
            )
        })
        .max()
        .unwrap_or(0);
    let mut structural_records = model.records.clone();
    for record in structural_records.values_mut() {
        record.fields.remove(SOURCE_FIELD);
        record.fields.remove("_kernspanCalculationRawOwners");
    }
    let snapshot = Snapshot {
        document_id: DocumentId("sysml-semantic-check".into()),
        records: structural_records,
        drafts: vec![],
        creation_drafts: BTreeMap::new(),
        next_id: max
            .checked_add(1)
            .ok_or_else(|| issue(ExchangeCode::Limit, None, "Identity allocator exhausted"))?,
        model_revision: 0,
        raw_version: 0,
    };
    snapshot
        .validate()
        .map_err(|e| issue(ExchangeCode::WrongType, None, e))?;
    for record in model.records.values() {
        for (field, value) in &record.fields {
            if field == "_kernspanCalculationRawOwners" {
                continue;
            }
            if field == SOURCE_FIELD {
                if record.kind != RecordKind::System {
                    return Err(issue(
                        ExchangeCode::UnsupportedField,
                        None,
                        "Source archive belongs only to System",
                    ));
                }
                continue;
            }
            if !text_fields(record.kind).contains(&field.as_str())
                && !other_fields(record.kind).contains(&field.as_str())
            {
                return Err(issue(
                    ExchangeCode::UnsupportedField,
                    None,
                    format!(
                        "Unsupported {:?}.{} on Record{}",
                        record.kind, field, record.id.0
                    ),
                ));
            }
            if matches!(field.as_str(), "observations" | "actors" | "inputs")
                && !value.as_array().is_some_and(Vec::is_empty)
            {
                return Err(issue(
                    ExchangeCode::UnsupportedField,
                    None,
                    format!("Nonempty {field} needs a qualified typed mapping"),
                ));
            }
            if text_fields(record.kind).contains(&field.as_str()) {
                quoted(value.as_str().ok_or_else(|| {
                    issue(ExchangeCode::WrongType, None, "Text field needs String")
                })?)?;
            }
        }
        quoted(&record.name)?;
        if let Some(contexts) = record.fields.get("context") {
            check_contexts(record, contexts)?;
        }
        if let Some(affiliations) = record.fields.get("affiliations") {
            for (target, relation) in affiliations.as_object().ok_or_else(|| {
                issue(
                    ExchangeCode::WrongType,
                    None,
                    "Affiliations must be an object",
                )
            })? {
                let id = target.parse::<u64>().map(RecordId).map_err(|_| {
                    issue(
                        ExchangeCode::WrongType,
                        None,
                        "Affiliation needs Organization ID",
                    )
                })?;
                require_target(model, record, id, RecordKind::Organization)?;
                if !matches!(relation.as_str(), Some("Unknown" | "Internal" | "External")) {
                    return Err(issue(
                        ExchangeCode::WrongType,
                        None,
                        "Unqualified affiliation relationship",
                    ));
                }
            }
        }
    }
    // Bounded iterative cycle check: no recursion or execution of calculations.
    let mut remaining: BTreeMap<_, _> = model
        .calculations
        .iter()
        .map(|(id, def)| (*id, calc::dependencies(def)))
        .collect();
    while !remaining.is_empty() {
        let ready: Vec<_> = remaining
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(id, _)| *id)
            .collect();
        if ready.is_empty() {
            return Err(issue(
                ExchangeCode::Calculation,
                None,
                "Cyclic calculation bindings cannot be imported/exported by this profile",
            ));
        }
        for id in ready {
            remaining.remove(&id);
            for deps in remaining.values_mut() {
                deps.remove(&id);
            }
        }
    }
    Ok(())
}
fn require_target(
    model: &ExchangeModel,
    source: &Record,
    id: RecordId,
    kind: RecordKind,
) -> Result<()> {
    let target = model.records.get(&id).ok_or_else(|| {
        issue(
            ExchangeCode::MissingSymbol,
            None,
            format!("Missing Record{}", id.0),
        )
    })?;
    if target.kind != kind {
        return Err(issue(
            ExchangeCode::WrongType,
            None,
            "Reference target has wrong draft type",
        ));
    }
    if target.scope != source.scope {
        return Err(issue(
            ExchangeCode::Scope,
            None,
            "Reference target crosses System scope",
        ));
    }
    Ok(())
}
fn check_contexts(record: &Record, value: &Value) -> Result<()> {
    for item in value
        .as_array()
        .ok_or_else(|| issue(ExchangeCode::WrongType, None, "Context must be an array"))?
    {
        let object = item
            .as_object()
            .ok_or_else(|| issue(ExchangeCode::WrongType, None, "Context must be an object"))?;
        let allowed = ["id", "type", "name", "scope", "kind", "text"];
        if object.len() != allowed.len()
            || object.keys().any(|key| !allowed.contains(&key.as_str()))
            || item["type"] != "Context"
            || item["scope"].as_u64() != Some(record.scope.0)
            || item["id"].as_u64().is_none_or(|id| id == 0)
            || !matches!(
                item["kind"].as_str(),
                Some("Description" | "Assumption" | "AcceptedConstraint")
            )
        {
            return Err(issue(
                ExchangeCode::UnsupportedField,
                None,
                "Context shape/identity/kind is outside the closed profile",
            ));
        }
        quoted(
            item["name"]
                .as_str()
                .ok_or_else(|| issue(ExchangeCode::WrongType, None, "Context name must be text"))?,
        )?;
        quoted(
            item["text"]
                .as_str()
                .ok_or_else(|| issue(ExchangeCode::WrongType, None, "Context text must be text"))?,
        )?;
    }
    Ok(())
}
fn quoted(value: &str) -> Result<String> {
    let mut text = String::from("\"");
    for c in value.chars() {
        match c {
            '\"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            '\u{0008}' => text.push_str("\\b"),
            '\u{000c}' => text.push_str("\\f"),
            c if c.is_control() => {
                return Err(issue(
                    ExchangeCode::UnsupportedField,
                    None,
                    "String control character has no admitted SysML escape",
                ))
            }
            c => text.push(c),
        }
    }
    text.push('"');
    Ok(text)
}
fn binding_name(value: &str) -> String {
    format!("'{value}'")
}
fn literal_text(literal: &QuantityLiteral) -> Result<String> {
    calc::validate_literal(literal)
        .map_err(|e| issue(ExchangeCode::Calculation, None, format!("{e:?}")))?;
    let decimal = literal.decimal.trim();
    if literal.quantity.unit == UnitId::ScalarOne {
        Ok(decimal.into())
    } else {
        Ok(format!("{decimal}[{}]", unit_text(literal.quantity.unit)))
    }
}
fn input_type(input: &Input, model: &ExchangeModel) -> Result<QuantityType> {
    match input {
        Input::Literal(literal) => Ok(literal.quantity),
        Input::Output(id) => model
            .calculations
            .get(id)
            .map(|definition| definition.output)
            .ok_or_else(|| {
                issue(
                    ExchangeCode::MissingSymbol,
                    None,
                    "Missing calculation output",
                )
            }),
    }
}
fn expression_kind(
    expr: &CalculationExpr,
    def: &Calculation,
    model: &ExchangeModel,
) -> Result<QuantityKind> {
    use QuantityKind::*;
    match expr {
        CalculationExpr::Literal(literal) => {
            if literal.quantity != scalar_type() {
                return Err(issue(
                    ExchangeCode::UnsupportedConstruct,
                    None,
                    "Physical expression literals must use typed bindings",
                ));
            }
            Ok(Scalar)
        }
        CalculationExpr::Binding(name) => input_type(
            def.bindings.get(name).ok_or_else(|| {
                issue(
                    ExchangeCode::MissingSymbol,
                    None,
                    format!("Missing input {name}"),
                )
            })?,
            model,
        )
        .map(|t| t.kind),
        CalculationExpr::Neg(a) => expression_kind(a, def, model),
        CalculationExpr::Add(a, b) | CalculationExpr::Sub(a, b) => {
            let left = expression_kind(a, def, model)?;
            let right = expression_kind(b, def, model)?;
            if left != right {
                return Err(issue(
                    ExchangeCode::QuantityKind,
                    None,
                    "Different nominal kinds in arithmetic",
                ));
            }
            Ok(left)
        }
        CalculationExpr::Mul(a, b) | CalculationExpr::Div(a, b) => {
            let left = expression_kind(a, def, model)?;
            let right = expression_kind(b, def, model)?;
            let divide = matches!(expr, CalculationExpr::Div(..));
            let kind = if divide {
                match (left, right) {
                    (a, Scalar) => Some(a),
                    (Ratio, Ratio)
                    | (Count, Count)
                    | (Length, Length)
                    | (Duration, Duration)
                    | (Speed, Speed) => Some(Ratio),
                    (Length, Duration) => Some(Speed),
                    (Length, Speed) => Some(Duration),
                    _ => None,
                }
            } else {
                match (left, right) {
                    (Scalar, a) | (a, Scalar) => Some(a),
                    (Ratio, Ratio) => Some(Ratio),
                    (Speed, Duration) | (Duration, Speed) => Some(Length),
                    _ => None,
                }
            };
            kind.ok_or_else(|| {
                issue(
                    ExchangeCode::QuantityKind,
                    None,
                    "Unsupported nominal algebra in SysML export",
                )
            })
        }
        CalculationExpr::Call(_, args) => {
            for arg in args {
                if expression_kind(arg, def, model)? != Scalar {
                    return Err(issue(
                        ExchangeCode::UnsupportedConstruct,
                        None,
                        "Only Scalar function signatures are qualified for SysML exchange",
                    ));
                }
            }
            Ok(Scalar)
        }
    }
}
fn expression_text(
    expr: &CalculationExpr,
    def: &Calculation,
    model: &ExchangeModel,
) -> Result<String> {
    expression_kind(expr, def, model)?;
    match expr {
        CalculationExpr::Literal(value) => literal_text(value),
        CalculationExpr::Binding(name) => Ok(binding_name(name)),
        CalculationExpr::Neg(value) => Ok(format!("(-{})", expression_text(value, def, model)?)),
        CalculationExpr::Add(a, b)
        | CalculationExpr::Sub(a, b)
        | CalculationExpr::Mul(a, b)
        | CalculationExpr::Div(a, b) => {
            let op = match expr {
                CalculationExpr::Add(..) => "+",
                CalculationExpr::Sub(..) => "-",
                CalculationExpr::Mul(..) => "*",
                _ => "/",
            };
            Ok(format!(
                "({} {op} {})",
                expression_text(a, def, model)?,
                expression_text(b, def, model)?
            ))
        }
        CalculationExpr::Call(function, args) => {
            let name = match function {
                Function::Abs => "abs",
                Function::Min => "min",
                Function::Max => "max",
                Function::Sqrt => "sqrt",
            };
            Ok(format!(
                "RealFunctions::{name}({})",
                args.iter()
                    .map(|arg| expression_text(arg, def, model))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ")
            ))
        }
    }
}
fn write_model(model: &ExchangeModel) -> Result<String> {
    let mut out = format!("package KernspanStage1Exchange {{\n{ONTOLOGY_SOURCE}\n");
    for record in model.records.values() {
        out.push_str(&format!(
            "    part <ks_r_{}> r_{} : {} {{\n",
            record.id.0,
            record.id.0,
            kind_type(record.kind)
        ));
        out.push_str(&format!("        attribute :>> id = {};\n        attribute :>> displayName = {};\n        ref part :>> scope = r_{};\n",record.id.0,quoted(&record.name)?,record.scope.0));
        for field in text_fields(record.kind) {
            if let Some(value) = record.fields.get(*field) {
                out.push_str(&format!(
                    "        attribute :>> '{field}' = {};\n",
                    quoted(value.as_str().ok_or_else(|| issue(
                        ExchangeCode::WrongType,
                        None,
                        "Text field"
                    ))?)?
                ));
            }
        }
        for field in ["forObjective", "expressedBy", "concernsSystem"] {
            if let Some(id) = record.fields.get(field).and_then(Value::as_u64) {
                out.push_str(&format!("        ref part :>> {field} = r_{id};\n"));
            }
        }
        if let Some(subject) = record.fields.get("subject").filter(|v| !v.is_null()) {
            out.push_str(&format!(
                "        ref part :>> 'subject' = r_{};\n",
                subject["id"].as_u64().ok_or_else(|| issue(
                    ExchangeCode::WrongType,
                    None,
                    "Subject ID"
                ))?
            ));
        }
        if let Some(links) = record
            .fields
            .get("contributesTo")
            .and_then(Value::as_array)
            .filter(|links| !links.is_empty())
        {
            out.push_str(&format!(
                "        ref part :>> contributesTo = ({});\n",
                links
                    .iter()
                    .map(|id| format!("r_{}", id.as_u64().unwrap_or(0)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(quantity) = record.fields.get("quantity").and_then(Value::as_str) {
            let variant = match quantity {
                "dimensionless" => "Dimensionless",
                "length" => "Length",
                _ => {
                    return Err(issue(
                        ExchangeCode::QuantityKind,
                        None,
                        "Unknown criterion quantity",
                    ))
                }
            };
            out.push_str(&format!("        attribute :>> quantitySelection = {ONTOLOGY}::LegacyQuantity::{variant};\n"));
        }
        if let Some(unit) = record.fields.get("unit").and_then(Value::as_str) {
            let variant = match unit {
                "one" => "One",
                "percent" => "Percent",
                "metre" => "Metre",
                _ => return Err(issue(ExchangeCode::Unit, None, "Unknown criterion unit")),
            };
            out.push_str(&format!(
                "        attribute :>> unitSelection = {ONTOLOGY}::LegacyUnit::{variant};\n"
            ));
        }
        if let Some(value) = record.fields.get("criterion").filter(|v| !v.is_null()) {
            out.push_str(&format!(
                "        attribute :>> criterion : {} = {}[{}];\n",
                legacy_type(record.text("quantity"))?,
                value,
                legacy_unit(record.text("unit"))?
            ));
        }
        if let Some(comparator) = record.fields.get("comparator").and_then(Value::as_str) {
            out.push_str(&format!(
                "        attribute :>> comparator = {ONTOLOGY}::Comparator::{comparator};\n"
            ));
        }
        if let Some(affiliations) = record.fields.get("affiliations").and_then(Value::as_object) {
            for (id, relationship) in affiliations {
                out.push_str(&format!("        part affiliation_{id} : {ONTOLOGY}::AffiliationDraft {{\n            ref part :>> organization = r_{id};\n            attribute :>> relationship = {ONTOLOGY}::AffiliationKind::{};\n        }}\n",relationship.as_str().ok_or_else(||issue(ExchangeCode::WrongType,None,"Affiliation kind"))?));
            }
        }
        if let Some(contexts) = record.fields.get("context").and_then(Value::as_array) {
            for item in contexts {
                let id = item["id"]
                    .as_u64()
                    .ok_or_else(|| issue(ExchangeCode::WrongType, None, "Context ID"))?;
                out.push_str(&format!("        part <ks_r_{id}> context_{id} : {ONTOLOGY}::ContextDraft {{\n            attribute :>> id = {id};\n            attribute :>> displayName = {};\n            ref part :>> scope = r_{};\n            attribute :>> kind = {ONTOLOGY}::ContextKind::{};\n            attribute :>> text = {};\n        }}\n",quoted(item["name"].as_str().unwrap_or_default())?,record.scope.0,item["kind"].as_str().unwrap_or_default(),quoted(item["text"].as_str().unwrap_or_default())?));
            }
        }
        if model.calculations.contains_key(&CalculationId(record.id.0)) {
            out.push_str(&format!(
                "        ref calc :>> calculation = run_{};\n",
                record.id.0
            ));
        }
        out.push_str("    }\n");
    }
    for definition in model.calculations.values() {
        if expression_kind(&definition.expression, definition, model)? != definition.output.kind {
            return Err(issue(
                ExchangeCode::QuantityKind,
                None,
                "Calculation output kind disagrees with expression",
            ));
        }
        out.push_str(&format!(
            "    calc def <ks_c_{}> calc_{} {{\n",
            definition.id.0, definition.id.0
        ));
        for (name, input) in &definition.bindings {
            out.push_str(&format!(
                "        in {} : {};\n",
                binding_name(name),
                quantity_name(input_type(input, model)?.kind)
            ));
        }
        out.push_str(&format!(
            "        return result : {};\n        {}\n    }}\n",
            quantity_name(definition.output.kind),
            expression_text(&definition.expression, definition, model)?
        ));
        out.push_str(&format!(
            "    calc run_{} : calc_{} {{\n",
            definition.id.0, definition.id.0
        ));
        for (name, input) in &definition.bindings {
            let value = match input {
                Input::Literal(value) => literal_text(value)?,
                Input::Output(id) => format!("run_{}.result", id.0),
            };
            out.push_str(&format!("        in {} = {value};\n", binding_name(name)));
        }
        // Authored output unit selection is a real typed, unbound feature.
        // It is not a cached numerical result or an executed calculation.
        if definition.output.kind == QuantityKind::Scalar {
            out.push_str("        return result : ScalarValues::Real;\n");
        } else {
            out.push_str(&format!(
                "        return result : {} {{ attribute :>> mRef = {}; }}\n",
                quantity_name(definition.output.kind),
                unit_text(definition.output.unit)
            ));
        }
        out.push_str("    }\n");
    }
    out.push_str("}\n");
    if out.len() > crate::boundary::MAX_SOURCE {
        return Err(issue(
            ExchangeCode::Limit,
            None,
            "Generated SysML exceeds source bound",
        ));
    }
    Ok(out)
}

struct Parsed {
    model: ExchangeModel,
    symbols: Vec<SourceSymbol>,
}
struct PendingRef {
    owner: RecordId,
    field: String,
    targets: Vec<(String, Span)>,
}
struct ParsedDefinition {
    id: CalculationId,
    name: String,
    parameters: BTreeMap<String, QuantityKind>,
    parameter_spans: BTreeMap<String, Span>,
    expression: CalculationExpr,
    result_kind: QuantityKind,
    span: Span,
}
struct ParsedUsage {
    name: String,
    definition: String,
    values: BTreeMap<String, Expr>,
    output: QuantityType,
    span: Span,
}
struct Reader {
    outer: String,
    imports: BTreeMap<String, String>,
    records: BTreeMap<RecordId, Record>,
    names: BTreeMap<String, RecordId>,
    ids: BTreeSet<u64>,
    pending: Vec<PendingRef>,
    symbols: Vec<SourceSymbol>,
    definitions: BTreeMap<String, ParsedDefinition>,
    usages: BTreeMap<String, ParsedUsage>,
    calculation_links: BTreeMap<RecordId, (String, Span)>,
}
fn read_model(source: &str) -> std::result::Result<Parsed, Vec<ExchangeDiagnostic>> {
    let parsed = parse_source(source);
    if !parsed.diagnostics.is_empty() {
        return Err(parsed
            .diagnostics
            .iter()
            .take(MAX_DIAGNOSTICS)
            .map(|d| issue(ExchangeCode::Syntax, Some(d.span), d.message.clone()))
            .collect());
    }
    read_unit(source, &parsed.unit).map_err(|e| vec![e])
}
fn plain_member(member: &Member) -> Result<()> {
    if member.visibility.is_some()
        || member.leading_then
        || member.leading_then_multiplicity.is_some()
    {
        return Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(member.span),
            "Visibility/succession prefixes are not supported here",
        ));
    }
    Ok(())
}
fn plain_package(package: &Package, span: Span) -> Result<String> {
    if package.is_library
        || package.is_standard
        || package.is_namespace
        || !package.metadata.is_empty()
        || package.id.short_name.is_some()
    {
        return Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(span),
            "Only plain named packages are admitted",
        ));
    }
    package
        .id
        .name
        .as_ref()
        .map(|name| name.value.clone())
        .ok_or_else(|| {
            issue(
                ExchangeCode::UnsupportedConstruct,
                Some(span),
                "Package needs a name",
            )
        })
}
fn source_slice(source: &str, span: Span) -> Option<&str> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    source.get(start..end)
}
fn tokens_equal(left: &str, right: &str) -> bool {
    fn tokens(source: &str) -> Option<Vec<(TokenKind, String)>> {
        let (tokens, errors) = tokenize(source);
        if !errors.is_empty() {
            return None;
        }
        tokens
            .into_iter()
            .filter(|t| !t.kind.is_trivia() && t.kind != TokenKind::Eof)
            .map(|t| Some((t.kind, source_slice(source, t.span)?.to_owned())))
            .collect()
    }
    tokens(left).zip(tokens(right)).is_some_and(|(a, b)| a == b)
}
fn read_unit(source: &str, unit: &SourceUnit) -> Result<Parsed> {
    let substantive: Vec<_> = unit
        .members
        .iter()
        .filter(|m| !matches!(m.kind, MemberKind::Comment(_) | MemberKind::Doc(_)))
        .collect();
    if substantive.len() != 1 {
        return Err(issue(
            ExchangeCode::UnsupportedConstruct,
            None,
            "Exactly one outer package is required",
        ));
    }
    let outer_member = substantive[0];
    plain_member(outer_member)?;
    let MemberKind::Package(package) = &outer_member.kind else {
        return Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(outer_member.span),
            "Outer member must be a package",
        ));
    };
    let outer = plain_package(package, outer_member.span)?;
    symbol_name(&outer, Some(outer_member.span))?;
    let mut reader = Reader {
        outer,
        imports: BTreeMap::new(),
        records: BTreeMap::new(),
        names: BTreeMap::new(),
        ids: BTreeSet::new(),
        pending: vec![],
        symbols: vec![],
        definitions: BTreeMap::new(),
        usages: BTreeMap::new(),
        calculation_links: BTreeMap::new(),
    };
    let members = package.body.as_ref().ok_or_else(|| {
        issue(
            ExchangeCode::UnsupportedConstruct,
            Some(outer_member.span),
            "Outer package body required",
        )
    })?;
    let mut ontology_seen = false;
    let mut top_names: BTreeSet<String> = LIBRARY_NAMESPACES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    // Imports and exact ontology are checked before uses, regardless source order.
    for member in members {
        if let MemberKind::Import(import) = &member.kind {
            reader.read_import(member, import)?;
        }
        if let MemberKind::Package(package) = &member.kind {
            plain_member(member)?;
            if plain_package(package, member.span)? != ONTOLOGY
                || ontology_seen
                || !source_slice(source, member.span)
                    .is_some_and(|slice| tokens_equal(slice, ONTOLOGY_SOURCE))
            {
                return Err(issue(ExchangeCode::UnsupportedConstruct,Some(member.span),"The exact closed draft ontology is required; arbitrary user inheritance/definitions refuse"));
            }
            ontology_seen = true;
        }
    }
    top_names.extend(reader.imports.keys().cloned());
    if !ontology_seen {
        return Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(outer_member.span),
            "Missing reviewed draft ontology",
        ));
    }
    for member in members {
        match &member.kind {
            MemberKind::Import(_) | MemberKind::Package(_) => {}
            MemberKind::Comment(_) | MemberKind::Doc(_) => {
                plain_member(member)?;
            }
            MemberKind::Usage(usage) if usage.kind == UsageKind::Part => {
                plain_member(member)?;
                let name = usage
                    .declaration
                    .id
                    .name
                    .as_ref()
                    .map(|n| n.value.clone())
                    .ok_or_else(|| {
                        issue(
                            ExchangeCode::UnsupportedConstruct,
                            Some(member.span),
                            "Record declaration name required",
                        )
                    })?;
                symbol_name(&name, Some(member.span))?;
                if !top_names.insert(name.clone()) {
                    return Err(issue(
                        ExchangeCode::AmbiguousSymbol,
                        Some(member.span),
                        "Duplicate outer symbol",
                    ));
                }
                reader.read_record(member, usage, &name)?;
            }
            MemberKind::Definition(definition) if definition.kind == DefKind::Calc => {
                plain_member(member)?;
                let def = reader.read_definition(member, definition)?;
                symbol_name(&def.name, Some(member.span))?;
                if !top_names.insert(def.name.clone())
                    || reader.definitions.values().any(|d| d.id == def.id)
                {
                    return Err(issue(
                        ExchangeCode::DuplicateIdentity,
                        Some(member.span),
                        "Duplicate calculation identity or name",
                    ));
                }
                reader.definitions.insert(def.name.clone(), def);
            }
            MemberKind::Usage(usage) if usage.kind == UsageKind::Calc => {
                plain_member(member)?;
                let used = reader.read_calculation_usage(member, usage)?;
                symbol_name(&used.name, Some(member.span))?;
                if !top_names.insert(used.name.clone()) {
                    return Err(issue(
                        ExchangeCode::AmbiguousSymbol,
                        Some(member.span),
                        "Duplicate outer symbol",
                    ));
                }
                reader.usages.insert(used.name.clone(), used);
            }
            _ => {
                return Err(issue(
                    ExchangeCode::UnsupportedConstruct,
                    Some(member.span),
                    "Unsupported outer member; no partial import",
                ))
            }
        }
    }
    reader.resolve_records()?;
    let calculations = reader.resolve_calculations()?;
    if reader.symbols.len() > crate::boundary::MAX_ELEMENTS {
        return Err(issue(ExchangeCode::Limit, None, "Semantic symbol limit"));
    }
    let model = ExchangeModel {
        records: reader.records,
        calculations,
    };
    check_model(&model).map_err(|mut diagnostic| {
        if diagnostic.span.is_none() {
            diagnostic.span = Some(outer_member.span.into());
        }
        diagnostic
    })?;
    reader
        .symbols
        .sort_by(|a, b| (&a.stable_id, &a.qualified_name).cmp(&(&b.stable_id, &b.qualified_name)));
    Ok(Parsed {
        model,
        symbols: reader.symbols,
    })
}
fn base_usage(usage: &Usage, expected: UsagePrefix, span: Span) -> Result<()> {
    if usage.prefix != expected || !matches!(usage.detail, UsageDetail::None) || usage.is_parallel {
        return Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(span),
            "Unsupported usage prefix/detail/body context",
        ));
    }
    let mut declaration = usage.declaration.clone();
    declaration.id = Identification {
        short_name: None,
        name: None,
    };
    declaration.specializations.clear();
    if declaration != FeatureDeclaration::default() {
        return Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(span),
            "Unsupported multiplicity/ordering/conjugation/specialization",
        ));
    }
    Ok(())
}
fn named_type(usage: &Usage, span: Span) -> Result<String> {
    base_usage(usage, UsagePrefix::default(), span)?;
    if usage.value.is_some() {
        return Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(span),
            "Typed record/calc usage cannot carry an unreviewed feature value",
        ));
    }
    match usage.declaration.specializations.as_slice() {
        [FeatureSpecialization::TypedBy(types)] if types.len() == 1 && !types[0].is_conjugated => {
            target_name(&types[0].target)
        }
        _ => Err(issue(
            ExchangeCode::WrongType,
            Some(span),
            "Exactly one explicit nonconjugated type is required",
        )),
    }
}
fn target_name(target: &TargetRef) -> Result<String> {
    match target {
        TargetRef::Name(name) => qualified_name(name),
        _ => Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(target.span()),
            "Unsupported qualified/feature-chain target",
        )),
    }
}
fn ref_expression(expr: &Expr) -> Result<String> {
    match &expr.kind {
        ExprKind::Ref(name) => qualified_name(name),
        _ => Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(expr.span),
            "An explicit named reference is required",
        )),
    }
}
fn identity(id: &Identification, prefix: &str, span: Span) -> Result<u64> {
    let text = id
        .short_name
        .as_ref()
        .map(|n| n.value.as_str())
        .ok_or_else(|| {
            issue(
                ExchangeCode::DuplicateIdentity,
                Some(span),
                "Stable short ID required",
            )
        })?;
    let digits = text.strip_prefix(prefix).ok_or_else(|| {
        issue(
            ExchangeCode::DuplicateIdentity,
            Some(span),
            "Short ID namespace disagrees with semantic kind",
        )
    })?;
    let number = digits
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| {
            issue(
                ExchangeCode::DuplicateIdentity,
                Some(span),
                "Nonzero numeric identity required",
            )
        })?;
    if digits != number.to_string() {
        return Err(issue(
            ExchangeCode::DuplicateIdentity,
            Some(span),
            "Stable ID must use canonical decimal digits",
        ));
    }
    Ok(number)
}
fn numeric_id(expr: &Expr) -> Result<u64> {
    match &expr.kind {
        ExprKind::Literal(Literal::Integer(raw)) => {
            raw.parse::<u64>().ok().filter(|n| *n > 0).ok_or_else(|| {
                issue(
                    ExchangeCode::DuplicateIdentity,
                    Some(expr.span),
                    "Nonzero numeric ID required",
                )
            })
        }
        _ => Err(issue(
            ExchangeCode::DuplicateIdentity,
            Some(expr.span),
            "Numeric identity must be an integer literal",
        )),
    }
}
fn string_value(expr: &Expr) -> Result<String> {
    match &expr.kind {
        ExprKind::Literal(Literal::String(text)) => Ok(text.clone()),
        _ => Err(issue(
            ExchangeCode::WrongType,
            Some(expr.span),
            "String literal required",
        )),
    }
}
fn assigned(
    usage: &Usage,
    kind: UsageKind,
    span: Span,
    allow_type: bool,
) -> Result<(String, &Expr, Option<String>)> {
    let prefix = UsagePrefix {
        is_ref: matches!(kind, UsageKind::Part | UsageKind::Calc),
        ..UsagePrefix::default()
    };
    base_usage(usage, prefix, span)?;
    if usage.kind != kind || !usage.declaration.id.is_empty() || usage.body.is_some() {
        return Err(issue(
            ExchangeCode::UnsupportedConstruct,
            Some(span),
            "Only a bound redefinition of an admitted feature is supported",
        ));
    }
    let mut field = None;
    let mut ty = None;
    for specialization in &usage.declaration.specializations {
        match specialization {
            FeatureSpecialization::Redefines(refs) if refs.len() == 1 && field.is_none() => {
                let name = target_name(&refs[0])?;
                if name.contains("::") {
                    return Err(issue(
                        ExchangeCode::UnsupportedConstruct,
                        Some(span),
                        "Feature redefinition must name the local inherited field",
                    ));
                }
                field = Some(name);
            }
            FeatureSpecialization::TypedBy(types)
                if allow_type && types.len() == 1 && !types[0].is_conjugated && ty.is_none() =>
            {
                ty = Some(target_name(&types[0].target)?)
            }
            _ => {
                return Err(issue(
                    ExchangeCode::UnsupportedConstruct,
                    Some(span),
                    "Unsupported feature specialization",
                ))
            }
        }
    }
    let field = field.ok_or_else(|| {
        issue(
            ExchangeCode::UnsupportedConstruct,
            Some(span),
            "Explicit inherited field redefinition required",
        )
    })?;
    let value = usage
        .value
        .as_ref()
        .filter(|v| v.kind == ValueKind::Bound)
        .ok_or_else(|| {
            issue(
                ExchangeCode::UnsupportedConstruct,
                Some(span),
                "Bound value required; initial/default values refuse",
            )
        })?;
    Ok((field, &value.expr, ty))
}
impl Reader {
    fn read_import(&mut self, member: &Member, import: &Import) -> Result<()> {
        if member.visibility != Some(Visibility::Private)
            || member.leading_then
            || member.leading_then_multiplicity.is_some()
            || import.is_import_all
            || import.is_namespace
            || import.is_recursive
            || !import.filters.is_empty()
            || import.target.is_global
        {
            return Err(issue(
                ExchangeCode::UnsupportedConstruct,
                Some(member.span),
                "Only registered private single-member imports are supported",
            ));
        }
        let full = qualified_name(&import.target)?;
        let allowed = [
            "ScalarValues::Real",
            "ScalarValues::String",
            "ScalarValues::Integer",
            "MeasurementReferences::one",
            "MeasurementReferences::CountValue",
            "MeasurementReferences::DimensionOneValue",
            "ISQ::LengthValue",
            "ISQ::DurationValue",
            "ISQ::SpeedValue",
            "SI::m",
            "SI::km",
            "SI::s",
            "SI::min",
            "SI::h",
            "RealFunctions::abs",
            "RealFunctions::min",
            "RealFunctions::max",
            "RealFunctions::sqrt",
        ];
        if !allowed.contains(&full.as_str()) {
            return Err(issue(
                ExchangeCode::UnsupportedConstruct,
                Some(member.span),
                "Unregistered import; no external filesystem or network resolution",
            ));
        }
        let alias = import
            .target
            .segments
            .last()
            .map(|n| n.value.clone())
            .unwrap_or_default();
        if self.imports.insert(alias, full).is_some() {
            return Err(issue(
                ExchangeCode::AmbiguousSymbol,
                Some(member.span),
                "Duplicate import candidate",
            ));
        }
        Ok(())
    }
    fn type_name(&self, name: String) -> String {
        self.imports.get(&name).cloned().unwrap_or(name)
    }
    fn local(&self, name: &str, span: Span) -> Result<String> {
        if let Some((outer, local)) = name.split_once("::") {
            if outer == self.outer && !local.contains("::") {
                return Ok(local.into());
            }
        } else {
            return Ok(name.into());
        }
        Err(issue(
            ExchangeCode::MissingSymbol,
            Some(span),
            "Reference does not resolve in this exact outer package",
        ))
    }
    fn enum_value(&self, expr: &Expr, ty: &str, allowed: &[&str]) -> Result<String> {
        let full = ref_expression(expr)?;
        let prefix = format!("{ONTOLOGY}::{ty}::");
        let value = full
            .strip_prefix(&prefix)
            .filter(|value| allowed.contains(value))
            .ok_or_else(|| {
                issue(
                    ExchangeCode::WrongType,
                    Some(expr.span),
                    "Exact declared enum member required",
                )
            })?;
        Ok(value.into())
    }
    fn read_record(&mut self, member: &Member, usage: &Usage, name: &str) -> Result<()> {
        let ty = self.type_name(named_type(usage, member.span)?);
        let kind = parse_kind(&ty).ok_or_else(|| {
            issue(
                ExchangeCode::WrongType,
                Some(member.span),
                "Record must use an exact custom draft ontology type",
            )
        })?;
        let id = RecordId(identity(&usage.declaration.id, "ks_r_", member.span)?);
        if !self.ids.insert(id.0) {
            return Err(issue(
                ExchangeCode::DuplicateIdentity,
                Some(member.span),
                "Duplicate record/context identity",
            ));
        }
        self.names.insert(name.into(), id);
        let mut record = Record {
            id,
            kind,
            name: String::new(),
            scope: RecordId(0),
            fields: BTreeMap::new(),
        };
        let mut seen = BTreeSet::new();
        let mut child_names = BTreeSet::new();
        let mut criterion = None;
        for child in usage.body.as_ref().ok_or_else(|| {
            issue(
                ExchangeCode::UnsupportedConstruct,
                Some(member.span),
                "Record body required",
            )
        })? {
            plain_member(child)?;
            match &child.kind {
                MemberKind::Comment(_) | MemberKind::Doc(_) => {}
                MemberKind::Usage(value)
                    if value.kind == UsageKind::Part && !value.prefix.is_ref =>
                {
                    let nested_type = self.type_name(named_type(value, child.span)?);
                    let nested_name = value
                        .declaration
                        .id
                        .name
                        .as_ref()
                        .map(|n| n.value.clone())
                        .ok_or_else(|| {
                            issue(
                                ExchangeCode::AmbiguousSymbol,
                                Some(child.span),
                                "Owned draft part name required",
                            )
                        })?;
                    symbol_name(&nested_name, Some(child.span))?;
                    if ["id", "displayName", "scope"].contains(&nested_name.as_str())
                        || !child_names.insert(nested_name)
                    {
                        return Err(issue(
                            ExchangeCode::AmbiguousSymbol,
                            Some(child.span),
                            "Duplicate or inherited owned-part name",
                        ));
                    }
                    if kind == RecordKind::Environment
                        && nested_type == format!("{ONTOLOGY}::ContextDraft")
                    {
                        self.read_context(&mut record, child, value, name)?;
                    } else if kind == RecordKind::Stakeholder
                        && nested_type == format!("{ONTOLOGY}::AffiliationDraft")
                    {
                        self.read_affiliation(&mut record, child, value)?;
                    } else {
                        return Err(issue(
                            ExchangeCode::UnsupportedConstruct,
                            Some(child.span),
                            "Unsupported owned draft part",
                        ));
                    }
                }
                MemberKind::Usage(value) if value.kind == UsageKind::Attribute => {
                    let (field, expr, ty) =
                        assigned(value, UsageKind::Attribute, child.span, true)?;
                    if !seen.insert(field.clone()) {
                        return Err(issue(
                            ExchangeCode::AmbiguousSymbol,
                            Some(child.span),
                            "Duplicate record feature",
                        ));
                    }
                    match field.as_str() {
                        "id" => {
                            if ty.is_some() || numeric_id(expr)? != id.0 {
                                return Err(issue(
                                    ExchangeCode::DuplicateIdentity,
                                    Some(child.span),
                                    "Numeric ID and stable short ID disagree",
                                ));
                            }
                        }
                        "displayName" => {
                            if ty.is_some() {
                                return Err(issue(
                                    ExchangeCode::WrongType,
                                    Some(child.span),
                                    "Unexpected displayName type override",
                                ));
                            }
                            record.name = string_value(expr)?;
                        }
                        "quantitySelection" if kind == RecordKind::Measure => {
                            if ty.is_some() {
                                return Err(issue(
                                    ExchangeCode::WrongType,
                                    Some(child.span),
                                    "Unexpected enum type override",
                                ));
                            }
                            let variant = self.enum_value(
                                expr,
                                "LegacyQuantity",
                                &["Dimensionless", "Length"],
                            )?;
                            record.fields.insert(
                                "quantity".into(),
                                json!(if variant == "Dimensionless" {
                                    "dimensionless"
                                } else {
                                    "length"
                                }),
                            );
                        }
                        "unitSelection" if kind == RecordKind::Measure => {
                            if ty.is_some() {
                                return Err(issue(
                                    ExchangeCode::WrongType,
                                    Some(child.span),
                                    "Unexpected enum type override",
                                ));
                            }
                            let variant =
                                self.enum_value(expr, "LegacyUnit", &["One", "Percent", "Metre"])?;
                            record.fields.insert(
                                "unit".into(),
                                json!(match variant.as_str() {
                                    "One" => "one",
                                    "Percent" => "percent",
                                    _ => "metre",
                                }),
                            );
                        }
                        "criterion" if kind == RecordKind::Measure => {
                            criterion = Some((
                                expr.clone(),
                                ty.ok_or_else(|| {
                                    issue(
                                        ExchangeCode::WrongType,
                                        Some(child.span),
                                        "Criterion needs explicit quantity type",
                                    )
                                })?,
                            ));
                        }
                        "comparator" if kind == RecordKind::Measure => {
                            if ty.is_some() {
                                return Err(issue(
                                    ExchangeCode::WrongType,
                                    Some(child.span),
                                    "Unexpected enum type override",
                                ));
                            }
                            record.fields.insert(
                                field,
                                self.enum_value(
                                    expr,
                                    "Comparator",
                                    &["GreaterOrEqual", "LessOrEqual", "Equal"],
                                )?
                                .into(),
                            );
                        }
                        _ if text_fields(kind).contains(&field.as_str()) => {
                            if ty.is_some() {
                                return Err(issue(
                                    ExchangeCode::WrongType,
                                    Some(child.span),
                                    "Unexpected text type override",
                                ));
                            }
                            record.fields.insert(field, json!(string_value(expr)?));
                        }
                        _ => {
                            return Err(issue(
                                ExchangeCode::UnsupportedField,
                                Some(child.span),
                                format!("Unsupported {kind:?} attribute {field}"),
                            ))
                        }
                    }
                }
                MemberKind::Usage(value)
                    if value.kind == UsageKind::Part && value.prefix.is_ref =>
                {
                    let (field, expr, _) = assigned(value, UsageKind::Part, child.span, false)?;
                    if !seen.insert(field.clone()) {
                        return Err(issue(
                            ExchangeCode::AmbiguousSymbol,
                            Some(child.span),
                            "Duplicate record reference",
                        ));
                    }
                    let allowed = field == "scope"
                        || match kind {
                            RecordKind::Capability => field == "contributesTo",
                            RecordKind::Objective => field == "subject",
                            RecordKind::Measure => field == "forObjective",
                            RecordKind::Expectation => {
                                matches!(field.as_str(), "expressedBy" | "concernsSystem")
                            }
                            _ => false,
                        };
                    if !allowed {
                        return Err(issue(
                            ExchangeCode::UnsupportedField,
                            Some(child.span),
                            "Unsupported reference role",
                        ));
                    }
                    let expressions = match &expr.kind {
                        ExprKind::Sequence(values) if field == "contributesTo" => {
                            values.iter().collect::<Vec<_>>()
                        }
                        _ => vec![expr],
                    };
                    let mut targets = vec![];
                    for expression in expressions {
                        targets.push((
                            self.local(&ref_expression(expression)?, expression.span)?,
                            expression.span,
                        ));
                    }
                    if targets.is_empty() {
                        return Err(issue(
                            ExchangeCode::UnsupportedConstruct,
                            Some(expr.span),
                            "Empty reference expression must be absent",
                        ));
                    }
                    self.pending.push(PendingRef {
                        owner: id,
                        field,
                        targets,
                    });
                }
                MemberKind::Usage(value)
                    if value.kind == UsageKind::Calc
                        && value.prefix.is_ref
                        && kind == RecordKind::Measure =>
                {
                    let (field, expr, _) = assigned(value, UsageKind::Calc, child.span, false)?;
                    if field != "calculation" || !seen.insert(field) {
                        return Err(issue(
                            ExchangeCode::UnsupportedField,
                            Some(child.span),
                            "Only one explicit calculation link is admitted",
                        ));
                    }
                    self.calculation_links.insert(
                        id,
                        (self.local(&ref_expression(expr)?, expr.span)?, expr.span),
                    );
                }
                _ => {
                    return Err(issue(
                        ExchangeCode::UnsupportedConstruct,
                        Some(child.span),
                        "Unsupported record body member; no partial import",
                    ))
                }
            }
        }
        if !["id", "displayName", "scope"]
            .iter()
            .all(|key| seen.contains(*key))
        {
            return Err(issue(
                ExchangeCode::WrongType,
                Some(member.span),
                "Record id/displayName/scope are required",
            ));
        }
        if let Some((expr, ty)) = criterion {
            let expected = legacy_type(record.text("quantity"))?;
            if self.type_name(ty) != expected {
                return Err(issue(
                    ExchangeCode::QuantityKind,
                    Some(expr.span),
                    "Criterion type disagrees with quantity selection",
                ));
            }
            let kind = if record.text("quantity") == "length" {
                QuantityKind::Length
            } else {
                QuantityKind::Ratio
            };
            let literal = self.read_literal(&expr, kind)?;
            let unit = match literal.quantity.unit {
                UnitId::Metre => "metre",
                UnitId::RatioOne => "one",
                UnitId::Percent => "percent",
                _ => {
                    return Err(issue(
                        ExchangeCode::Unit,
                        Some(expr.span),
                        "Unsupported criterion unit",
                    ))
                }
            };
            if record.text("unit") != unit {
                return Err(issue(
                    ExchangeCode::Unit,
                    Some(expr.span),
                    "Criterion unit disagrees with authored unit selection",
                ));
            }
            let number = calc::decimal(&literal.decimal)
                .map_err(|e| issue(ExchangeCode::Calculation, Some(expr.span), format!("{e:?}")))?;
            record.fields.insert("criterion".into(), json!(number));
        }
        self.symbols.push(SourceSymbol {
            stable_id: SemanticId::Record(id),
            qualified_name: vec![self.outer.clone(), name.into()],
            declaration: member.span.into(),
            value: None,
        });
        self.records.insert(id, record);
        Ok(())
    }
    fn read_context(
        &mut self,
        record: &mut Record,
        member: &Member,
        usage: &Usage,
        parent_name: &str,
    ) -> Result<()> {
        let id = RecordId(identity(&usage.declaration.id, "ks_r_", member.span)?);
        if !self.ids.insert(id.0) {
            return Err(issue(
                ExchangeCode::DuplicateIdentity,
                Some(member.span),
                "Duplicate context/record identity",
            ));
        }
        let name = usage
            .declaration
            .id
            .name
            .as_ref()
            .map(|n| n.value.clone())
            .ok_or_else(|| {
                issue(
                    ExchangeCode::AmbiguousSymbol,
                    Some(member.span),
                    "Context symbol needs name",
                )
            })?;
        let mut item = json!({"id":id.0,"type":"Context","name":"","scope":0,"kind":"","text":""});
        let mut seen = BTreeSet::new();
        for child in usage.body.as_ref().ok_or_else(|| {
            issue(
                ExchangeCode::WrongType,
                Some(member.span),
                "Context body required",
            )
        })? {
            plain_member(child)?;
            let MemberKind::Usage(value) = &child.kind else {
                return Err(issue(
                    ExchangeCode::UnsupportedConstruct,
                    Some(child.span),
                    "Context admits only fixed fields",
                ));
            };
            let (field, expr, _) = assigned(
                value,
                if value.kind == UsageKind::Part {
                    UsageKind::Part
                } else {
                    UsageKind::Attribute
                },
                child.span,
                false,
            )?;
            if !seen.insert(field.clone()) {
                return Err(issue(
                    ExchangeCode::AmbiguousSymbol,
                    Some(child.span),
                    "Duplicate Context field",
                ));
            }
            match (value.kind, field.as_str()) {
                (UsageKind::Attribute, "id") => {
                    if numeric_id(expr)? != id.0 {
                        return Err(issue(
                            ExchangeCode::DuplicateIdentity,
                            Some(expr.span),
                            "Context numeric ID disagrees",
                        ));
                    }
                }
                (UsageKind::Attribute, "displayName") => item["name"] = json!(string_value(expr)?),
                (UsageKind::Attribute, "text") => item["text"] = json!(string_value(expr)?),
                (UsageKind::Attribute, "kind") => {
                    item["kind"] = json!(self.enum_value(
                        expr,
                        "ContextKind",
                        &["Description", "Assumption", "AcceptedConstraint"]
                    )?)
                }
                (UsageKind::Part, "scope") => self.pending.push(PendingRef {
                    owner: record.id,
                    field: format!("contextscope:{}", id.0),
                    targets: vec![(self.local(&ref_expression(expr)?, expr.span)?, expr.span)],
                }),
                _ => {
                    return Err(issue(
                        ExchangeCode::UnsupportedField,
                        Some(child.span),
                        "Unsupported Context field",
                    ))
                }
            }
        }
        if seen.len() != 5 {
            return Err(issue(
                ExchangeCode::WrongType,
                Some(member.span),
                "Context needs all five fixed fields",
            ));
        }
        record
            .fields
            .entry("context".into())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| issue(ExchangeCode::WrongType, Some(member.span), "Context array"))?
            .push(item);
        self.symbols.push(SourceSymbol {
            stable_id: SemanticId::Context(id),
            qualified_name: vec![self.outer.clone(), parent_name.into(), name],
            declaration: member.span.into(),
            value: None,
        });
        Ok(())
    }
    fn read_affiliation(
        &mut self,
        record: &mut Record,
        member: &Member,
        usage: &Usage,
    ) -> Result<()> {
        if usage.declaration.id.short_name.is_some() {
            return Err(issue(
                ExchangeCode::UnsupportedConstruct,
                Some(member.span),
                "Affiliation identity is target Organization, not an invented record",
            ));
        }
        let mut organization = None;
        let mut relationship = None;
        for child in usage.body.as_ref().ok_or_else(|| {
            issue(
                ExchangeCode::WrongType,
                Some(member.span),
                "Affiliation body required",
            )
        })? {
            plain_member(child)?;
            let MemberKind::Usage(value) = &child.kind else {
                return Err(issue(
                    ExchangeCode::UnsupportedConstruct,
                    Some(child.span),
                    "Affiliation fixed fields only",
                ));
            };
            let (field, expr, _) = assigned(
                value,
                if value.kind == UsageKind::Part {
                    UsageKind::Part
                } else {
                    UsageKind::Attribute
                },
                child.span,
                false,
            )?;
            match (value.kind, field.as_str()) {
                (UsageKind::Part, "organization") if organization.is_none() => {
                    organization = Some((self.local(&ref_expression(expr)?, expr.span)?, expr.span))
                }
                (UsageKind::Attribute, "relationship") if relationship.is_none() => {
                    relationship = Some(self.enum_value(
                        expr,
                        "AffiliationKind",
                        &["Unknown", "Internal", "External"],
                    )?)
                }
                _ => {
                    return Err(issue(
                        ExchangeCode::UnsupportedField,
                        Some(child.span),
                        "Unknown/duplicate affiliation field",
                    ))
                }
            }
        }
        let relation = relationship.ok_or_else(|| {
            issue(
                ExchangeCode::WrongType,
                Some(member.span),
                "Affiliation relationship required",
            )
        })?;
        self.pending.push(PendingRef {
            owner: record.id,
            field: format!("affiliation:{relation}"),
            targets: vec![organization.ok_or_else(|| {
                issue(
                    ExchangeCode::WrongType,
                    Some(member.span),
                    "Affiliation target required",
                )
            })?],
        });
        Ok(())
    }
    fn resolve_records(&mut self) -> Result<()> {
        // Resolve System scopes first so all later same-scope tests see actual IDs.
        self.pending.sort_by_key(|pending| pending.field != "scope");
        for pending in &self.pending {
            let mut targets = vec![];
            let mut unique = BTreeSet::new();
            for (name, span) in &pending.targets {
                let id = *self.names.get(name).ok_or_else(|| {
                    issue(
                        ExchangeCode::MissingSymbol,
                        Some(*span),
                        format!("Unresolved record symbol {name}"),
                    )
                })?;
                if !unique.insert(id) {
                    return Err(issue(
                        ExchangeCode::DuplicateIdentity,
                        Some(*span),
                        "Repeated reference target",
                    ));
                }
                targets.push(id);
            }
            let span = pending.targets[0].1;
            let id = targets[0];
            let target = &self.records[&id];
            let expected = match pending.field.as_str() {
                "scope" | "concernsSystem" => Some(RecordKind::System),
                "contributesTo" => Some(RecordKind::Mission),
                "forObjective" => Some(RecordKind::Objective),
                "expressedBy" => Some(RecordKind::Stakeholder),
                _ if pending.field.starts_with("affiliation:") => Some(RecordKind::Organization),
                _ if pending.field.starts_with("contextscope:") => Some(RecordKind::System),
                _ => None,
            };
            for target_id in &targets {
                let target = &self.records[target_id];
                if expected.is_some_and(|kind| target.kind != kind)
                    || (pending.field == "subject"
                        && !matches!(
                            target.kind,
                            RecordKind::System | RecordKind::Mission | RecordKind::Capability
                        ))
                {
                    return Err(issue(
                        ExchangeCode::WrongType,
                        Some(span),
                        "Reference endpoint has wrong custom draft type",
                    ));
                }
            }
            let source = &self.records[&pending.owner];
            if pending.field != "scope"
                && targets
                    .iter()
                    .any(|id| self.records[id].scope != source.scope)
            {
                return Err(issue(
                    ExchangeCode::Scope,
                    Some(span),
                    "Reference crosses System scope",
                ));
            }
            let subject_kind = target.kind.label();
            let record = self.records.get_mut(&pending.owner).ok_or_else(|| {
                issue(
                    ExchangeCode::MissingSymbol,
                    Some(span),
                    "Missing reference owner",
                )
            })?;
            match pending.field.as_str() {
                "scope" => record.scope = id,
                "subject" => {
                    record
                        .fields
                        .insert("subject".into(), json!({"kind":subject_kind,"id":id.0}));
                }
                "contributesTo" => {
                    record.fields.insert(
                        "contributesTo".into(),
                        json!(targets.iter().map(|id| id.0).collect::<Vec<_>>()),
                    );
                }
                field if field.starts_with("contextscope:") => {
                    let context_id = field[13..].parse::<u64>().map_err(|_| {
                        issue(
                            ExchangeCode::DuplicateIdentity,
                            Some(span),
                            "Context scope ID",
                        )
                    })?;
                    let items = record
                        .fields
                        .get_mut("context")
                        .and_then(Value::as_array_mut)
                        .ok_or_else(|| {
                            issue(ExchangeCode::WrongType, Some(span), "Context array missing")
                        })?;
                    let item = items
                        .iter_mut()
                        .find(|item| item["id"].as_u64() == Some(context_id))
                        .ok_or_else(|| {
                            issue(
                                ExchangeCode::MissingSymbol,
                                Some(span),
                                "Context identity missing",
                            )
                        })?;
                    item["scope"] = json!(id.0);
                }
                field if field.starts_with("affiliation:") => {
                    let relationships = record
                        .fields
                        .entry("affiliations".into())
                        .or_insert_with(|| json!({}))
                        .as_object_mut()
                        .ok_or_else(|| {
                            issue(ExchangeCode::WrongType, Some(span), "Affiliation object")
                        })?;
                    if relationships
                        .insert(id.0.to_string(), json!(&field[12..]))
                        .is_some()
                    {
                        return Err(issue(
                            ExchangeCode::DuplicateIdentity,
                            Some(span),
                            "Duplicate affiliation target",
                        ));
                    }
                }
                field => {
                    record.fields.insert(field.into(), json!(id.0));
                }
            }
        }
        Ok(())
    }
    fn read_definition(
        &self,
        member: &Member,
        definition: &Definition,
    ) -> Result<ParsedDefinition> {
        if definition.prefix.is_abstract
            || definition.prefix.is_variation
            || definition.prefix.is_individual
            || !definition.prefix.metadata.is_empty()
            || !definition.specializes.is_empty()
            || definition.is_parallel
            || definition.is_sufficient
            || definition.multiplicity.is_some()
            || !definition.conjugates.is_empty()
            || !definition.disjoint_from.is_empty()
            || !definition.unions.is_empty()
            || !definition.intersects.is_empty()
            || !definition.differences.is_empty()
        {
            return Err(issue(
                ExchangeCode::UnsupportedConstruct,
                Some(member.span),
                "Unsupported calc definition prefix/inheritance",
            ));
        }
        let id = CalculationId(identity(&definition.id, "ks_c_", member.span)?);
        let name = definition
            .id
            .name
            .as_ref()
            .map(|n| n.value.clone())
            .ok_or_else(|| {
                issue(
                    ExchangeCode::AmbiguousSymbol,
                    Some(member.span),
                    "Calc definition name required",
                )
            })?;
        let mut parameters = BTreeMap::new();
        let mut parameter_spans = BTreeMap::new();
        let mut result_kind = None;
        let mut expression = None;
        let mut last_name = String::new();
        for child in definition.body.as_ref().ok_or_else(|| {
            issue(
                ExchangeCode::UnsupportedConstruct,
                Some(member.span),
                "Calc body required",
            )
        })? {
            plain_member(child)?;
            match &child.kind {
                MemberKind::Usage(usage)
                    if usage.prefix.direction == Some(FeatureDirection::In)
                        && result_kind.is_none()
                        && expression.is_none() =>
                {
                    let (parameter, kind) = self.read_parameter(usage, child.span, true)?;
                    if parameter <= last_name || parameters.len() >= 32 {
                        return Err(issue(
                            ExchangeCode::UnsupportedConstruct,
                            Some(child.span),
                            "Unique lexically ordered parameters, maximum32, are required",
                        ));
                    }
                    last_name = parameter.clone();
                    parameters.insert(parameter.clone(), kind);
                    parameter_spans.insert(parameter, child.span);
                }
                MemberKind::Return(usage) if result_kind.is_none() && expression.is_none() => {
                    let (parameter, kind) = self.read_parameter(usage, child.span, false)?;
                    if parameter != "result" {
                        return Err(issue(
                            ExchangeCode::UnsupportedConstruct,
                            Some(child.span),
                            "The explicit return parameter must be result",
                        ));
                    }
                    result_kind = Some(kind);
                }
                MemberKind::Result(expr) if result_kind.is_some() && expression.is_none() => {
                    expression = Some(self.read_expression(expr, 0)?)
                }
                _ => {
                    return Err(issue(
                        ExchangeCode::UnsupportedConstruct,
                        Some(child.span),
                        "Unsupported calc body member/order or duplicate result",
                    ))
                }
            }
        }
        Ok(ParsedDefinition {
            id,
            name,
            parameters,
            parameter_spans,
            expression: expression.ok_or_else(|| {
                issue(
                    ExchangeCode::WrongType,
                    Some(member.span),
                    "Trailing result expression required",
                )
            })?,
            result_kind: result_kind.ok_or_else(|| {
                issue(
                    ExchangeCode::WrongType,
                    Some(member.span),
                    "Typed return required",
                )
            })?,
            span: member.span,
        })
    }
    fn read_parameter(
        &self,
        usage: &Usage,
        span: Span,
        input: bool,
    ) -> Result<(String, QuantityKind)> {
        let prefix = UsagePrefix {
            direction: input.then_some(FeatureDirection::In),
            ..UsagePrefix::default()
        };
        base_usage(usage, prefix, span)?;
        if !matches!(
            usage.kind,
            UsageKind::Default | UsageKind::Ref | UsageKind::Attribute
        ) || usage.declaration.id.short_name.is_some()
            || usage.value.is_some()
            || usage.body.is_some()
        {
            return Err(issue(
                ExchangeCode::UnsupportedConstruct,
                Some(span),
                "Only a named typed scalar parameter is admitted",
            ));
        }
        let name = usage
            .declaration
            .id
            .name
            .as_ref()
            .map(|n| n.value.clone())
            .ok_or_else(|| {
                issue(
                    ExchangeCode::WrongType,
                    Some(span),
                    "Parameter name required",
                )
            })?;
        symbol_name(&name, Some(span))?;
        if self.imports.contains_key(&name) {
            return Err(issue(
                ExchangeCode::AmbiguousSymbol,
                Some(span),
                "Calculation parameter shadows an imported member",
            ));
        }
        let ty = match usage.declaration.specializations.as_slice() {
            [FeatureSpecialization::TypedBy(types)]
                if types.len() == 1 && !types[0].is_conjugated =>
            {
                self.type_name(target_name(&types[0].target)?)
            }
            _ => {
                return Err(issue(
                    ExchangeCode::WrongType,
                    Some(span),
                    "One explicit parameter type required",
                ))
            }
        };
        let kind = quantity_kind(&ty).ok_or_else(|| {
            issue(
                ExchangeCode::QuantityKind,
                Some(span),
                "Unregistered parameter quantity kind",
            )
        })?;
        Ok((name, kind))
    }
    fn read_calculation_usage(&self, member: &Member, usage: &Usage) -> Result<ParsedUsage> {
        let definition = self.local(&named_type(usage, member.span)?, member.span)?;
        if usage.declaration.id.short_name.is_some() {
            return Err(issue(
                ExchangeCode::UnsupportedConstruct,
                Some(member.span),
                "Calc usage identity is its typed definition, not a second calc ID",
            ));
        }
        let name = usage
            .declaration
            .id
            .name
            .as_ref()
            .map(|n| n.value.clone())
            .ok_or_else(|| {
                issue(
                    ExchangeCode::WrongType,
                    Some(member.span),
                    "Calc usage name required",
                )
            })?;
        let mut values = BTreeMap::new();
        let mut output = None;
        for child in usage.body.as_ref().ok_or_else(|| {
            issue(
                ExchangeCode::WrongType,
                Some(member.span),
                "Calc usage body required",
            )
        })? {
            plain_member(child)?;
            match &child.kind {
                MemberKind::Usage(value) if output.is_none() => {
                    base_usage(
                        value,
                        UsagePrefix {
                            direction: Some(FeatureDirection::In),
                            ..UsagePrefix::default()
                        },
                        child.span,
                    )?;
                    if !matches!(
                        value.kind,
                        UsageKind::Default | UsageKind::Ref | UsageKind::Attribute
                    ) || !value.declaration.specializations.is_empty()
                        || value.declaration.id.short_name.is_some()
                        || value.body.is_some()
                    {
                        return Err(issue(
                            ExchangeCode::UnsupportedConstruct,
                            Some(child.span),
                            "Only named bound input values are supported",
                        ));
                    }
                    let parameter = value
                        .declaration
                        .id
                        .name
                        .as_ref()
                        .map(|n| n.value.clone())
                        .ok_or_else(|| {
                            issue(
                                ExchangeCode::WrongType,
                                Some(child.span),
                                "Input name required",
                            )
                        })?;
                    let expr = &value
                        .value
                        .as_ref()
                        .filter(|v| v.kind == ValueKind::Bound)
                        .ok_or_else(|| {
                            issue(
                                ExchangeCode::UnsupportedConstruct,
                                Some(child.span),
                                "Bound input required",
                            )
                        })?
                        .expr;
                    if values.insert(parameter, expr.clone()).is_some() {
                        return Err(issue(
                            ExchangeCode::AmbiguousSymbol,
                            Some(child.span),
                            "Duplicate input binding",
                        ));
                    }
                }
                MemberKind::Return(value) if output.is_none() => {
                    let mut plain = value.clone();
                    plain.body = None;
                    let (parameter, kind) = self.read_parameter(&plain, child.span, false)?;
                    if parameter != "result" {
                        return Err(issue(
                            ExchangeCode::WrongType,
                            Some(child.span),
                            "Return parameter must be result",
                        ));
                    }
                    let unit = if kind == QuantityKind::Scalar {
                        if value.body.is_some() {
                            return Err(issue(
                                ExchangeCode::UnsupportedConstruct,
                                Some(child.span),
                                "Scalar Real has no quantity mRef feature",
                            ));
                        }
                        UnitId::ScalarOne
                    } else {
                        let body = value
                            .body
                            .as_ref()
                            .filter(|body| body.len() == 1)
                            .ok_or_else(|| {
                                issue(
                                    ExchangeCode::Unit,
                                    Some(child.span),
                                    "Explicit single output measurement reference is required",
                                )
                            })?;
                        plain_member(&body[0])?;
                        let MemberKind::Usage(attribute) = &body[0].kind else {
                            return Err(issue(
                                ExchangeCode::UnsupportedConstruct,
                                Some(body[0].span),
                                "Output measurement reference attribute required",
                            ));
                        };
                        let (field, expr, _) =
                            assigned(attribute, UsageKind::Attribute, body[0].span, false)?;
                        if field != "mRef" {
                            return Err(issue(
                                ExchangeCode::UnsupportedField,
                                Some(body[0].span),
                                "Only genuine quantity mRef is supported",
                            ));
                        }
                        self.read_unit(expr, kind)?
                    };
                    output = Some(QuantityType { kind, unit });
                }
                _ => {
                    return Err(issue(
                        ExchangeCode::UnsupportedConstruct,
                        Some(child.span),
                        "Unsupported calc usage member/order",
                    ))
                }
            }
        }
        Ok(ParsedUsage {
            name,
            definition,
            values,
            output: output.ok_or_else(|| {
                issue(
                    ExchangeCode::Unit,
                    Some(member.span),
                    "Authored output type/unit selection required",
                )
            })?,
            span: member.span,
        })
    }
    fn read_unit(&self, expr: &Expr, kind: QuantityKind) -> Result<UnitId> {
        let unit = match &expr.kind {
            ExprKind::Ref(name) => {
                let name = self.type_name(qualified_name(name)?);
                match (name.as_str(), kind) {
                    ("MeasurementReferences::one", QuantityKind::Ratio) => UnitId::RatioOne,
                    ("MeasurementReferences::one", QuantityKind::Count) => UnitId::Count,
                    ("KernspanStage1DraftOntology::percent", QuantityKind::Ratio) => {
                        UnitId::Percent
                    }
                    ("SI::m", QuantityKind::Length) => UnitId::Metre,
                    ("SI::km", QuantityKind::Length) => UnitId::Kilometre,
                    ("SI::s", QuantityKind::Duration) => UnitId::Second,
                    ("SI::min", QuantityKind::Duration) => UnitId::Minute,
                    _ => {
                        return Err(issue(
                            ExchangeCode::Unit,
                            Some(expr.span),
                            "Unit is not registered for the declared nominal kind",
                        ))
                    }
                }
            }
            ExprKind::Binary {
                op: BinaryOp::Div,
                lhs,
                rhs,
            } if kind == QuantityKind::Speed => {
                let left = self.type_name(ref_expression(lhs)?);
                let right = self.type_name(ref_expression(rhs)?);
                match (left.as_str(), right.as_str()) {
                    ("SI::m", "SI::s") => UnitId::MetrePerSecond,
                    ("SI::km", "SI::h") => UnitId::KilometrePerHour,
                    _ => {
                        return Err(issue(
                            ExchangeCode::Unit,
                            Some(expr.span),
                            "Unregistered speed unit expression",
                        ))
                    }
                }
            }
            _ => {
                return Err(issue(
                    ExchangeCode::Unit,
                    Some(expr.span),
                    "Unsupported unit expression; no general unit algebra",
                ))
            }
        };
        Ok(unit)
    }
    fn read_literal(&self, expr: &Expr, kind: QuantityKind) -> Result<QuantityLiteral> {
        fn number(expr: &Expr) -> Result<String> {
            match &expr.kind {
                ExprKind::Literal(Literal::Integer(raw) | Literal::Real(raw)) => Ok(raw.clone()),
                ExprKind::Unary {
                    op: UnaryOp::Plus,
                    operand,
                } => number(operand),
                ExprKind::Unary {
                    op: UnaryOp::Minus,
                    operand,
                } => {
                    let value = number(operand)?;
                    Ok(if let Some(positive) = value.strip_prefix('-') {
                        positive.into()
                    } else {
                        format!("-{value}")
                    })
                }
                _ => Err(issue(
                    ExchangeCode::WrongType,
                    Some(expr.span),
                    "Input must be a finite decimal literal, not an expression or cached value",
                )),
            }
        }
        let (decimal, unit) = match &expr.kind {
            ExprKind::Bracket { target, arg } => (number(target)?, self.read_unit(arg, kind)?),
            ExprKind::Unary {
                op: UnaryOp::Plus | UnaryOp::Minus,
                operand,
            } if matches!(operand.kind, ExprKind::Bracket { .. }) => {
                let mut literal = self.read_literal(operand, kind)?;
                if matches!(
                    expr.kind,
                    ExprKind::Unary {
                        op: UnaryOp::Minus,
                        ..
                    }
                ) {
                    literal.decimal = if let Some(positive) = literal.decimal.strip_prefix('-') {
                        positive.into()
                    } else {
                        format!("-{}", literal.decimal)
                    };
                }
                return Ok(literal);
            }
            _ if kind == QuantityKind::Scalar => (number(expr)?, UnitId::ScalarOne),
            _ => {
                return Err(issue(
                    ExchangeCode::Unit,
                    Some(expr.span),
                    "Typed non-Scalar binding requires a real unit-bearing literal",
                ))
            }
        };
        let literal = QuantityLiteral {
            decimal,
            quantity: QuantityType { kind, unit },
        };
        calc::validate_literal(&literal)
            .map_err(|e| issue(ExchangeCode::Calculation, Some(expr.span), format!("{e:?}")))?;
        Ok(literal)
    }
    fn read_expression(&self, expr: &Expr, depth: usize) -> Result<CalculationExpr> {
        if depth >= 64 {
            return Err(issue(
                ExchangeCode::Limit,
                Some(expr.span),
                "Calculation expression nesting limit",
            ));
        }
        let next = |value: &Expr| self.read_expression(value, depth + 1);
        match &expr.kind {
            ExprKind::Literal(Literal::Integer(_) | Literal::Real(_)) => Ok(
                CalculationExpr::Literal(self.read_literal(expr, QuantityKind::Scalar)?),
            ),
            ExprKind::Ref(name) if !name.is_global && name.segments.len() == 1 => {
                Ok(CalculationExpr::Binding(name.segments[0].value.clone()))
            }
            ExprKind::Unary {
                op: UnaryOp::Plus,
                operand,
            } => next(operand),
            ExprKind::Unary {
                op: UnaryOp::Minus,
                operand,
            } => Ok(CalculationExpr::Neg(Box::new(next(operand)?))),
            ExprKind::Binary { op, lhs, rhs } => {
                let left = Box::new(next(lhs)?);
                let right = Box::new(next(rhs)?);
                match op {
                    BinaryOp::Add => Ok(CalculationExpr::Add(left, right)),
                    BinaryOp::Sub => Ok(CalculationExpr::Sub(left, right)),
                    BinaryOp::Mul => Ok(CalculationExpr::Mul(left, right)),
                    BinaryOp::Div => Ok(CalculationExpr::Div(left, right)),
                    _ => Err(issue(
                        ExchangeCode::UnsupportedConstruct,
                        Some(expr.span),
                        "Unsupported arithmetic operator",
                    )),
                }
            }
            ExprKind::Invocation { ty, args } => {
                let name = self.type_name(target_name(ty)?);
                let function = match name.as_str() {
                    "RealFunctions::abs" => Function::Abs,
                    "RealFunctions::min" => Function::Min,
                    "RealFunctions::max" => Function::Max,
                    "RealFunctions::sqrt" => Function::Sqrt,
                    _ => {
                        return Err(issue(
                            ExchangeCode::UnsupportedConstruct,
                            Some(expr.span),
                            "Unregistered pure function signature",
                        ))
                    }
                };
                let mut values = vec![];
                for arg in args {
                    if arg.name.is_some() {
                        return Err(issue(
                            ExchangeCode::UnsupportedConstruct,
                            Some(arg.value.span),
                            "Named invocation arguments not admitted",
                        ));
                    }
                    values.push(next(&arg.value)?);
                }
                Ok(CalculationExpr::Call(function, values))
            }
            _ => Err(issue(
                ExchangeCode::UnsupportedConstruct,
                Some(expr.span),
                "Unsupported expression form; physical values must use named typed bindings",
            )),
        }
    }
    fn resolve_calculations(&mut self) -> Result<BTreeMap<CalculationId, Calculation>> {
        let mut calculations = BTreeMap::new();
        let mut used_definitions = BTreeSet::new();
        let mut used_usages = BTreeSet::new();
        for (owner, (usage_name, link_span)) in &self.calculation_links {
            let usage = self.usages.get(usage_name).ok_or_else(|| {
                issue(
                    ExchangeCode::MissingSymbol,
                    Some(*link_span),
                    "Unresolved actual calc usage",
                )
            })?;
            let definition = self.definitions.get(&usage.definition).ok_or_else(|| {
                issue(
                    ExchangeCode::MissingSymbol,
                    Some(usage.span),
                    "Unresolved calc definition type",
                )
            })?;
            if definition.id != CalculationId(owner.0)
                || usage.output.kind != definition.result_kind
            {
                return Err(issue(
                    ExchangeCode::Calculation,
                    Some(*link_span),
                    "Calculation definition ID/owner/result type disagreement",
                ));
            }
            if !used_definitions.insert(definition.id) || !used_usages.insert(usage_name.clone()) {
                return Err(issue(
                    ExchangeCode::DuplicateIdentity,
                    Some(*link_span),
                    "A calculation definition/usage belongs to exactly one Measure",
                ));
            }
            if definition.parameters.keys().ne(usage.values.keys()) {
                return Err(issue(
                    ExchangeCode::MissingSymbol,
                    Some(usage.span),
                    "Input bindings must exactly match typed parameter names",
                ));
            }
            let mut bindings = BTreeMap::new();
            for (name, kind) in &definition.parameters {
                let expression = &usage.values[name];
                let input = match &expression.kind {
                    ExprKind::ChainStep { target, member } if target_name(member)? == "result" => {
                        let dependency_name = self.local(&ref_expression(target)?, target.span)?;
                        let dependency = self.usages.get(&dependency_name).ok_or_else(|| {
                            issue(
                                ExchangeCode::MissingSymbol,
                                Some(expression.span),
                                "Unresolved actual calc output usage",
                            )
                        })?;
                        let target =
                            self.definitions
                                .get(&dependency.definition)
                                .ok_or_else(|| {
                                    issue(
                                        ExchangeCode::MissingSymbol,
                                        Some(expression.span),
                                        "Unresolved calc output definition",
                                    )
                                })?;
                        if target.result_kind != *kind {
                            return Err(issue(
                                ExchangeCode::QuantityKind,
                                Some(expression.span),
                                "Dependency output type differs from parameter kind",
                            ));
                        }
                        Input::Output(target.id)
                    }
                    _ => Input::Literal(self.read_literal(expression, *kind)?),
                };
                bindings.insert(name.clone(), input);
                self.symbols.push(SourceSymbol {
                    stable_id: SemanticId::Binding(definition.id, name.clone()),
                    qualified_name: vec![self.outer.clone(), definition.name.clone(), name.clone()],
                    declaration: definition.parameter_spans[name].into(),
                    value: Some(expression.span.into()),
                });
            }
            let calculation = Calculation {
                id: definition.id,
                owner: *owner,
                bindings,
                expression: definition.expression.clone(),
                output: usage.output,
            };
            calc::validate(&calculation, &calc::Limits::default()).map_err(|e| {
                issue(
                    ExchangeCode::Calculation,
                    Some(definition.span),
                    format!("{e:?}"),
                )
            })?;
            self.symbols.push(SourceSymbol {
                stable_id: SemanticId::Calculation(definition.id),
                qualified_name: vec![self.outer.clone(), definition.name.clone()],
                declaration: definition.span.into(),
                value: None,
            });
            calculations.insert(definition.id, calculation);
        }
        if used_definitions.len() != self.definitions.len()
            || used_usages.len() != self.usages.len()
        {
            return Err(issue(
                ExchangeCode::Calculation,
                None,
                "All calc definitions/usages must be explicitly attached to their matching Measure",
            ));
        }
        for (id, definition) in &calculations {
            let value = serde_json::to_value(definition)
                .map_err(|e| issue(ExchangeCode::Calculation, None, e.to_string()))?;
            self.records
                .get_mut(&RecordId(id.0))
                .ok_or_else(|| {
                    issue(
                        ExchangeCode::MissingSymbol,
                        None,
                        "Calculation owner missing",
                    )
                })?
                .fields
                .insert("calculation".into(), value);
        }
        let model = ExchangeModel {
            records: self.records.clone(),
            calculations: calculations.clone(),
        };
        for definition in calculations.values() {
            if expression_kind(&definition.expression, definition, &model)?
                != definition.output.kind
            {
                return Err(issue(
                    ExchangeCode::QuantityKind,
                    None,
                    "Calculation output kind differs from expression",
                ));
            }
        }
        Ok(calculations)
    }
}
