//! Closed, pure Stage1 calculations. Results are derived evidence, never criteria
//! or observations; no host execution, implicit solve or legacy formula fallback.
use crate::stage1::{RawDraft, RecordId, Validity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const ALGORITHM_REVISION: &str = "kernspan.stage1.calculation/1";
pub const UNIT_TABLE_REVISION: &str = "kernspan.stage1.units/1";
pub const MAX_EXACT_COUNT: f64 = 9_007_199_254_740_991.0;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct CalculationId(pub u64);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuantityKind {
    Scalar,
    Ratio,
    Count,
    Length,
    Duration,
    Speed,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum UnitId {
    ScalarOne,
    RatioOne,
    Percent,
    Count,
    Metre,
    Kilometre,
    Second,
    Minute,
    MetrePerSecond,
    KilometrePerHour,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuantityType {
    pub kind: QuantityKind,
    pub unit: UnitId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Literal {
    pub decimal: String,
    pub quantity: QuantityType,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Input {
    Literal(Literal),
    Output(CalculationId),
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Function {
    Abs,
    Min,
    Max,
    Sqrt,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Expr {
    Literal(Literal),
    Binding(String),
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Call(Function, Vec<Expr>),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: CalculationId,
    pub owner: RecordId,
    pub bindings: BTreeMap<String, Input>,
    pub expression: Expr,
    pub output: QuantityType,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DocumentStamp {
    pub document_id: String,
    pub instance_id: String,
    pub model_revision: u64,
    pub raw_version: u64,
    pub snapshot_hash: String,
}
pub type CalculationRawState = BTreeMap<CalculationId, Vec<RawDraft>>;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiagnosticCode {
    IncompleteInput,
    InvalidInput,
    Unsupported,
    UnsupportedAlgebra,
    MissingReference,
    Cycle,
    BlockedBy,
    QuantityKindMismatch,
    UnitMismatch,
    DivisionByZero,
    DomainError,
    Overflow,
    Underflow,
    ResourceLimit,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub related: Vec<CalculationId>,
    pub cause: Option<DiagnosticCode>,
}
pub type ParseDiagnostic = Diagnostic;
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}
impl std::error::Error for Diagnostic {}
fn error(code: DiagnosticCode, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code,
        message: message.into(),
        span: None,
        related: Vec::new(),
        cause: None,
    }
}
fn located(
    code: DiagnosticCode,
    message: impl Into<String>,
    start: usize,
    end: usize,
) -> Diagnostic {
    Diagnostic {
        span: Some(SourceSpan { start, end }),
        ..error(code, message)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub definitions: usize,
    pub bindings: usize,
    pub expression_bytes: usize,
    pub ast_nodes: usize,
    pub ast_depth: usize,
    pub document_ast_nodes: usize,
    pub dependency_edges: usize,
    pub decimal_bytes: usize,
    pub diagnostics: usize,
    pub node_visits: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            definitions: 1024,
            bindings: 32,
            expression_bytes: 8192,
            ast_nodes: 1024,
            ast_depth: 64,
            document_ast_nodes: 65536,
            dependency_edges: 16384,
            decimal_bytes: 128,
            diagnostics: 256,
            node_visits: 262144,
        }
    }
}
impl Limits {
    fn bounded(&self) -> Self {
        let hard = Self::default();
        Self {
            definitions: self.definitions.min(hard.definitions),
            bindings: self.bindings.min(hard.bindings),
            expression_bytes: self.expression_bytes.min(hard.expression_bytes),
            ast_nodes: self.ast_nodes.min(hard.ast_nodes),
            ast_depth: self.ast_depth.min(hard.ast_depth),
            document_ast_nodes: self.document_ast_nodes.min(hard.document_ast_nodes),
            dependency_edges: self.dependency_edges.min(hard.dependency_edges),
            decimal_bytes: self.decimal_bytes.min(hard.decimal_bytes),
            diagnostics: self.diagnostics.min(hard.diagnostics),
            node_visits: self.node_visits.min(hard.node_visits),
        }
    }
}
pub fn unit_kind(unit: UnitId) -> QuantityKind {
    match unit {
        UnitId::ScalarOne => QuantityKind::Scalar,
        UnitId::RatioOne | UnitId::Percent => QuantityKind::Ratio,
        UnitId::Count => QuantityKind::Count,
        UnitId::Metre | UnitId::Kilometre => QuantityKind::Length,
        UnitId::Second | UnitId::Minute => QuantityKind::Duration,
        UnitId::MetrePerSecond | UnitId::KilometrePerHour => QuantityKind::Speed,
    }
}
pub fn unit_key(unit: UnitId) -> &'static str {
    match unit {
        UnitId::ScalarOne => "ScalarOne",
        UnitId::RatioOne => "RatioOne",
        UnitId::Percent => "Percent",
        UnitId::Count => "Count",
        UnitId::Metre => "Metre",
        UnitId::Kilometre => "Kilometre",
        UnitId::Second => "Second",
        UnitId::Minute => "Minute",
        UnitId::MetrePerSecond => "MetrePerSecond",
        UnitId::KilometrePerHour => "KilometrePerHour",
    }
}
pub fn unit_from_key(key: &str) -> Option<UnitId> {
    [
        UnitId::ScalarOne,
        UnitId::RatioOne,
        UnitId::Percent,
        UnitId::Count,
        UnitId::Metre,
        UnitId::Kilometre,
        UnitId::Second,
        UnitId::Minute,
        UnitId::MetrePerSecond,
        UnitId::KilometrePerHour,
    ]
    .into_iter()
    .find(|unit| unit_key(*unit) == key)
}
pub fn unit_label(unit: UnitId) -> &'static str {
    match unit {
        UnitId::ScalarOne => "one (scalar)",
        UnitId::RatioOne => "one (ratio)",
        UnitId::Percent => "percent",
        UnitId::Count => "count",
        UnitId::Metre => "m",
        UnitId::Kilometre => "km",
        UnitId::Second => "s",
        UnitId::Minute => "min",
        UnitId::MetrePerSecond => "m/s",
        UnitId::KilometrePerHour => "km/h",
    }
}
pub fn quantity_label(kind: QuantityKind) -> &'static str {
    match kind {
        QuantityKind::Scalar => "Scalar",
        QuantityKind::Ratio => "Ratio",
        QuantityKind::Count => "Count",
        QuantityKind::Length => "Length",
        QuantityKind::Duration => "Duration",
        QuantityKind::Speed => "Speed",
    }
}
fn dimension(kind: QuantityKind) -> (i8, i8) {
    match kind {
        QuantityKind::Scalar | QuantityKind::Ratio | QuantityKind::Count => (0, 0),
        QuantityKind::Length => (1, 0),
        QuantityKind::Duration => (0, 1),
        QuantityKind::Speed => (1, -1),
    }
}
fn compatible(a: QuantityKind, b: QuantityKind) -> Result<(), Diagnostic> {
    if a == b {
        Ok(())
    } else if dimension(a) == dimension(b) {
        Err(error(
            DiagnosticCode::QuantityKindMismatch,
            "Equal physical dimensions do not make named quantity kinds compatible",
        ))
    } else {
        Err(error(
            DiagnosticCode::UnitMismatch,
            "Physical dimensions are incompatible",
        ))
    }
}
fn validate_quantity(quantity: QuantityType) -> Result<(), Diagnostic> {
    compatible(quantity.kind, unit_kind(quantity.unit))
}
fn scale(unit: UnitId) -> f64 {
    match unit {
        UnitId::Percent => 0.01,
        UnitId::Kilometre => 1000.0,
        UnitId::Minute => 60.0,
        UnitId::KilometrePerHour => 1000.0 / 3600.0,
        _ => 1.0,
    }
}
fn checked_number(number: f64) -> Result<f64, Diagnostic> {
    if !number.is_finite() {
        Err(error(
            DiagnosticCode::Overflow,
            "Arithmetic exceeds the finite numeric range",
        ))
    } else {
        Ok(if number == 0.0 { 0.0 } else { number })
    }
}
fn checked_product(a: f64, b: f64) -> Result<f64, Diagnostic> {
    let result = checked_number(a * b)?;
    if result == 0.0 && a != 0.0 && b != 0.0 {
        Err(error(
            DiagnosticCode::Underflow,
            "Nonzero multiplication/conversion underflows to zero",
        ))
    } else {
        Ok(result)
    }
}
fn checked_division(a: f64, b: f64) -> Result<f64, Diagnostic> {
    if b == 0.0 {
        return Err(error(DiagnosticCode::DivisionByZero, "Denominator is zero"));
    }
    let result = checked_number(a / b)?;
    if result == 0.0 && a != 0.0 {
        Err(error(
            DiagnosticCode::Underflow,
            "Nonzero division/conversion underflows to zero",
        ))
    } else {
        Ok(result)
    }
}
fn exact_count_literal(text: &str) -> Result<(), Diagnostic> {
    let text = text.trim();
    let negative = text.starts_with('-');
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = unsigned
        .split_once(['e', 'E'])
        .map_or((unsigned, 0i32), |(mantissa, exponent)| {
            (mantissa, exponent.parse::<i32>().unwrap_or(i32::MAX))
        });
    let fractional = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    let digits: String = mantissa
        .bytes()
        .filter(|byte| byte.is_ascii_digit())
        .map(char::from)
        .collect();
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Ok(());
    }
    let failure = || {
        error(
            DiagnosticCode::DomainError,
            "Count literal must be exactly integral, nonnegative and at most 2^53-1",
        )
    };
    if negative {
        return Err(failure());
    }
    let shift = i64::from(exponent) - fractional as i64;
    let integer = if shift < 0 {
        let remove = (-shift) as usize;
        if remove > digits.len()
            || !digits[digits.len() - remove..]
                .bytes()
                .all(|byte| byte == b'0')
        {
            return Err(failure());
        }
        digits[..digits.len() - remove].to_owned()
    } else {
        if shift > 16 || digits.len() + shift as usize > 16 {
            return Err(failure());
        }
        format!("{digits}{}", "0".repeat(shift as usize))
    };
    if integer.len() > 16 || (integer.len() == 16 && integer.as_str() > "9007199254740991") {
        return Err(failure());
    }
    Ok(())
}
fn count_range(value: f64, kind: QuantityKind) -> Result<(), Diagnostic> {
    if kind == QuantityKind::Count
        && (value < 0.0 || value.fract() != 0.0 || value > MAX_EXACT_COUNT)
    {
        Err(error(
            DiagnosticCode::DomainError,
            "Count must be a nonnegative integer at most 2^53-1",
        ))
    } else {
        Ok(())
    }
}
/// Finite decimal only; exact source text remains in the Literal/raw journal.
pub fn decimal(text: &str) -> Result<f64, Diagnostic> {
    if text.len() > Limits::default().decimal_bytes {
        return Err(error(
            DiagnosticCode::ResourceLimit,
            "Decimal literal exceeds 128 bytes",
        ));
    }
    let text = text.trim();
    if text.is_empty() {
        return Err(error(
            DiagnosticCode::IncompleteInput,
            "A decimal value is required",
        ));
    }
    let bytes = text.as_bytes();
    let mut i = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let mut digits = 0;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
        digits += 1;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return Err(error(
            if i == bytes.len() {
                DiagnosticCode::IncompleteInput
            } else {
                DiagnosticCode::InvalidInput
            },
            "Expected a decimal literal",
        ));
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if start == i {
            return Err(error(
                if i == bytes.len() {
                    DiagnosticCode::IncompleteInput
                } else {
                    DiagnosticCode::InvalidInput
                },
                "Exponent needs decimal digits",
            ));
        }
    }
    if i != bytes.len() {
        return Err(error(
            DiagnosticCode::InvalidInput,
            "Unexpected text in decimal literal",
        ));
    }
    let value = text
        .parse::<f64>()
        .map_err(|_| error(DiagnosticCode::InvalidInput, "Invalid decimal literal"))?;
    let value = checked_number(value)?;
    if value == 0.0
        && text
            .split(['e', 'E'])
            .next()
            .unwrap_or(text)
            .bytes()
            .any(|c| matches!(c, b'1'..=b'9'))
    {
        return Err(error(
            DiagnosticCode::Underflow,
            "Nonzero decimal literal underflows to zero",
        ));
    }
    Ok(value)
}
pub fn validate_literal(literal: &Literal) -> Result<(), Diagnostic> {
    validate_quantity(literal.quantity)?;
    let value = decimal(&literal.decimal)?;
    if literal.quantity.kind == QuantityKind::Count {
        exact_count_literal(&literal.decimal)?;
    }
    count_range(value, literal.quantity.kind)?;
    checked_product(value, scale(literal.quantity.unit))?;
    Ok(())
}
pub fn convert_literal(literal: &Literal, destination: UnitId) -> Result<Literal, Diagnostic> {
    validate_literal(literal)?;
    compatible(literal.quantity.kind, unit_kind(destination))?;
    if literal.quantity.unit == destination {
        return Ok(literal.clone());
    }
    let value = checked_division(
        checked_product(decimal(&literal.decimal)?, scale(literal.quantity.unit))?,
        scale(destination),
    )?;
    count_range(value, literal.quantity.kind)?;
    let decimal = value.to_string();
    let decimal = if decimal.len() > Limits::default().decimal_bytes {
        format!("{value:e}")
    } else {
        decimal
    };
    Ok(Literal {
        decimal,
        quantity: QuantityType {
            kind: literal.quantity.kind,
            unit: destination,
        },
    })
}
fn identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    text.len() <= 64
        && bytes
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
fn function_name(function: Function) -> &'static str {
    match function {
        Function::Abs => "abs",
        Function::Min => "min",
        Function::Max => "max",
        Function::Sqrt => "sqrt",
    }
}
fn function_arity(function: Function) -> usize {
    match function {
        Function::Abs | Function::Sqrt => 1,
        Function::Min | Function::Max => 2,
    }
}
fn resource_shape(expr: &Expr, limits: &Limits) -> Result<usize, Diagnostic> {
    let mut stack = vec![(expr, 1usize)];
    let mut nodes = 0usize;
    let mut bytes = 0usize;
    while let Some((expr, depth)) = stack.pop() {
        nodes += 1;
        if nodes > limits.ast_nodes || depth > limits.ast_depth {
            return Err(error(
                DiagnosticCode::ResourceLimit,
                "Expression AST resource limit exceeded",
            ));
        }
        match expr {
            Expr::Literal(literal) => {
                if literal.decimal.len() > limits.decimal_bytes {
                    return Err(error(
                        DiagnosticCode::ResourceLimit,
                        "Decimal byte limit exceeded",
                    ));
                }
                bytes = bytes.saturating_add(literal.decimal.len());
            }
            Expr::Binding(name) => {
                if name.len() > 64 {
                    return Err(error(
                        DiagnosticCode::ResourceLimit,
                        "Identifier byte limit exceeded",
                    ));
                }
                bytes = bytes.saturating_add(name.len());
            }
            Expr::Neg(inner) => {
                bytes = bytes.saturating_add(3);
                stack.push((inner, depth + 1));
            }
            Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
                bytes = bytes.saturating_add(3);
                stack.push((b, depth + 1));
                stack.push((a, depth + 1));
            }
            Expr::Call(function, args) => {
                if args.len() > limits.ast_nodes {
                    return Err(error(
                        DiagnosticCode::ResourceLimit,
                        "Function argument resource limit exceeded",
                    ));
                }
                bytes = bytes.saturating_add(function_name(*function).len() + 2 + args.len());
                for arg in args.iter().rev() {
                    stack.push((arg, depth + 1));
                }
            }
        }
        if bytes > limits.expression_bytes {
            return Err(error(
                DiagnosticCode::ResourceLimit,
                "Expression byte limit exceeded",
            ));
        }
    }
    Ok(nodes)
}
fn expression_shape(expr: &Expr, limits: &Limits) -> Result<usize, Diagnostic> {
    let mut stack = vec![(expr, 1usize)];
    let mut nodes = 0usize;
    let mut bytes = 0usize;
    while let Some((node, depth)) = stack.pop() {
        nodes += 1;
        if nodes > limits.ast_nodes || depth > limits.ast_depth {
            return Err(error(
                DiagnosticCode::ResourceLimit,
                "Expression AST node/depth limit exceeded",
            ));
        }
        match node {
            Expr::Literal(value) => {
                if value.decimal.len() > limits.decimal_bytes {
                    return Err(error(
                        DiagnosticCode::ResourceLimit,
                        "Decimal byte limit exceeded",
                    ));
                }
                validate_literal(value)?;
                bytes += value.decimal.len();
            }
            Expr::Binding(name) => {
                if !identifier(name) {
                    return Err(error(
                        DiagnosticCode::InvalidInput,
                        "Malformed binding identifier",
                    ));
                }
                bytes += name.len();
            }
            Expr::Neg(inner) => {
                bytes += 3;
                stack.push((inner, depth + 1));
            }
            Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
                bytes += 3;
                stack.push((b, depth + 1));
                stack.push((a, depth + 1));
            }
            Expr::Call(function, args) => {
                if args.len() != function_arity(*function) {
                    return Err(error(
                        DiagnosticCode::InvalidInput,
                        "Function arity is invalid",
                    ));
                }
                bytes += function_name(*function).len() + 2 + args.len().saturating_sub(1);
                for arg in args.iter().rev() {
                    stack.push((arg, depth + 1));
                }
            }
        }
        if bytes > limits.expression_bytes {
            return Err(error(
                DiagnosticCode::ResourceLimit,
                "Expression byte limit exceeded",
            ));
        }
    }
    Ok(nodes)
}
pub fn validate(definition: &Definition, limits: &Limits) -> Result<(), Diagnostic> {
    let limits = limits.bounded();
    if definition.id.0 == 0 || definition.id.0 != definition.owner.0 {
        return Err(error(
            DiagnosticCode::InvalidInput,
            "Calculation ID must equal its nonzero Measure owner ID",
        ));
    }
    if definition.bindings.len() > limits.bindings {
        return Err(error(
            DiagnosticCode::ResourceLimit,
            "Calculation binding limit exceeded",
        ));
    }
    resource_shape(&definition.expression, &limits)?;
    validate_quantity(definition.output)?;
    for (name, input) in &definition.bindings {
        if name.len() > 64 {
            return Err(error(
                DiagnosticCode::ResourceLimit,
                "Binding identifier byte limit exceeded",
            ));
        }
        if !identifier(name) {
            return Err(error(
                DiagnosticCode::InvalidInput,
                "Malformed binding identifier",
            ));
        }
        match input {
            Input::Literal(value) => {
                if value.decimal.len() > limits.decimal_bytes {
                    return Err(error(
                        DiagnosticCode::ResourceLimit,
                        "Decimal byte limit exceeded",
                    ));
                }
                validate_literal(value)?;
            }
            Input::Output(id) if id.0 == 0 => {
                return Err(error(
                    DiagnosticCode::InvalidInput,
                    "Output reference ID must be nonzero",
                ))
            }
            Input::Output(_) => {}
        }
    }
    expression_shape(&definition.expression, &limits)?;
    Ok(())
}
/// Shared admission bound for consumers that accept a complete program without
/// evaluating it. Runtime evaluation retains its explicit ResourceLimit outcomes.
pub fn validate_document_resources(
    definitions: &BTreeMap<CalculationId, Definition>,
    limits: &Limits,
) -> Result<(), Diagnostic> {
    let limits = limits.bounded();
    if definitions.len() > limits.definitions {
        return Err(error(
            DiagnosticCode::ResourceLimit,
            "Document calculation limit exceeded",
        ));
    }
    let mut nodes = 0usize;
    let mut edges = 0usize;
    for definition in definitions.values() {
        nodes = nodes.saturating_add(resource_shape(&definition.expression, &limits)?);
        edges = edges.saturating_add(dependencies(definition).len());
        if nodes > limits.document_ast_nodes || edges > limits.dependency_edges {
            return Err(error(
                DiagnosticCode::ResourceLimit,
                "Document calculation node/dependency limit exceeded",
            ));
        }
    }
    Ok(())
}
pub fn dependencies(definition: &Definition) -> BTreeSet<CalculationId> {
    definition
        .bindings
        .values()
        .filter_map(|input| match input {
            Input::Output(id) => Some(*id),
            _ => None,
        })
        .collect()
}
struct Parser<'a> {
    text: &'a str,
    pos: usize,
    nodes: usize,
    limits: Limits,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.space();
        self.text.as_bytes().get(self.pos).copied()
    }
    fn node(&mut self, expr: Expr) -> Result<Expr, Diagnostic> {
        self.nodes += 1;
        if self.nodes > self.limits.ast_nodes {
            Err(located(
                DiagnosticCode::ResourceLimit,
                "Expression node limit exceeded",
                self.pos,
                self.pos,
            ))
        } else {
            Ok(expr)
        }
    }
    fn depth(&self, depth: usize) -> Result<(), Diagnostic> {
        if depth > self.limits.ast_depth {
            Err(located(
                DiagnosticCode::ResourceLimit,
                "Expression depth limit exceeded",
                self.pos,
                self.pos,
            ))
        } else {
            Ok(())
        }
    }
    fn add(&mut self, depth: usize) -> Result<Expr, Diagnostic> {
        let mut left = self.multiply(depth)?;
        while let Some(op @ (b'+' | b'-')) = self.peek() {
            self.pos += 1;
            let right = self.multiply(depth)?;
            left = self.node(if op == b'+' {
                Expr::Add(Box::new(left), Box::new(right))
            } else {
                Expr::Sub(Box::new(left), Box::new(right))
            })?;
        }
        Ok(left)
    }
    fn multiply(&mut self, depth: usize) -> Result<Expr, Diagnostic> {
        let mut left = self.unary(depth)?;
        while let Some(op @ (b'*' | b'/')) = self.peek() {
            self.pos += 1;
            let right = self.unary(depth)?;
            left = self.node(if op == b'*' {
                Expr::Mul(Box::new(left), Box::new(right))
            } else {
                Expr::Div(Box::new(left), Box::new(right))
            })?;
        }
        Ok(left)
    }
    fn unary(&mut self, depth: usize) -> Result<Expr, Diagnostic> {
        self.depth(depth)?;
        match self.peek() {
            Some(b'+') => {
                self.pos += 1;
                self.unary(depth + 1)
            }
            Some(b'-') => {
                self.pos += 1;
                let expr = self.unary(depth + 1)?;
                self.node(Expr::Neg(Box::new(expr)))
            }
            _ => self.primary(depth),
        }
    }
    fn primary(&mut self, depth: usize) -> Result<Expr, Diagnostic> {
        self.depth(depth)?;
        let start = self.pos;
        match self.peek() {
            None => Err(located(
                DiagnosticCode::IncompleteInput,
                "Expression needs an operand",
                self.pos,
                self.pos,
            )),
            Some(b'(') => {
                self.pos += 1;
                let expr = self.add(depth + 1)?;
                match self.peek() {
                    Some(b')') => {
                        self.pos += 1;
                        Ok(expr)
                    }
                    None => Err(located(
                        DiagnosticCode::IncompleteInput,
                        "Unclosed parenthesis",
                        start,
                        self.pos,
                    )),
                    _ => Err(located(
                        DiagnosticCode::InvalidInput,
                        "Expected closing parenthesis",
                        self.pos,
                        self.pos + 1,
                    )),
                }
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let start = self.pos;
                while self
                    .text
                    .as_bytes()
                    .get(self.pos)
                    .is_some_and(u8::is_ascii_digit)
                {
                    self.pos += 1;
                }
                if self.text.as_bytes().get(self.pos) == Some(&b'.') {
                    self.pos += 1;
                    while self
                        .text
                        .as_bytes()
                        .get(self.pos)
                        .is_some_and(u8::is_ascii_digit)
                    {
                        self.pos += 1;
                    }
                }
                if matches!(self.text.as_bytes().get(self.pos), Some(b'e' | b'E')) {
                    self.pos += 1;
                    if matches!(self.text.as_bytes().get(self.pos), Some(b'+' | b'-')) {
                        self.pos += 1;
                    }
                    while self
                        .text
                        .as_bytes()
                        .get(self.pos)
                        .is_some_and(u8::is_ascii_digit)
                    {
                        self.pos += 1;
                    }
                }
                let text = &self.text[start..self.pos];
                if text.len() > self.limits.decimal_bytes {
                    return Err(located(
                        DiagnosticCode::ResourceLimit,
                        "Decimal byte limit exceeded",
                        start,
                        self.pos,
                    ));
                }
                decimal(text).map_err(|mut diagnostic| {
                    if diagnostic.code == DiagnosticCode::IncompleteInput
                        && self
                            .text
                            .as_bytes()
                            .get(self.pos)
                            .is_some_and(|byte| !byte.is_ascii_whitespace())
                    {
                        diagnostic.code = DiagnosticCode::InvalidInput;
                    }
                    diagnostic.span = Some(SourceSpan {
                        start,
                        end: self.pos,
                    });
                    diagnostic
                })?;
                self.node(Expr::Literal(Literal {
                    decimal: text.to_owned(),
                    quantity: QuantityType {
                        kind: QuantityKind::Scalar,
                        unit: UnitId::ScalarOne,
                    },
                }))
            }
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => {
                let start = self.pos;
                while self
                    .text
                    .as_bytes()
                    .get(self.pos)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    self.pos += 1;
                }
                let name = self.text[start..self.pos].to_owned();
                if !identifier(&name) {
                    return Err(located(
                        DiagnosticCode::ResourceLimit,
                        "Binding identifier exceeds 64 bytes",
                        start,
                        self.pos,
                    ));
                }
                if self.peek() != Some(b'(') {
                    return self.node(Expr::Binding(name));
                }
                let function = match name.as_str() {
                    "abs" => Function::Abs,
                    "min" => Function::Min,
                    "max" => Function::Max,
                    "sqrt" => Function::Sqrt,
                    _ => {
                        return Err(located(
                            DiagnosticCode::Unsupported,
                            "Function is not admitted",
                            start,
                            self.pos,
                        ))
                    }
                };
                self.pos += 1;
                let mut args = Vec::new();
                if self.peek() == Some(b')') {
                    self.pos += 1;
                } else {
                    loop {
                        if args.len() >= 2 {
                            return Err(located(
                                DiagnosticCode::InvalidInput,
                                "Too many function arguments",
                                start,
                                self.pos,
                            ));
                        }
                        args.push(self.add(depth + 1)?);
                        match self.peek() {
                            Some(b',') => {
                                self.pos += 1;
                            }
                            Some(b')') => {
                                self.pos += 1;
                                break;
                            }
                            None => {
                                return Err(located(
                                    DiagnosticCode::IncompleteInput,
                                    "Unclosed function call",
                                    start,
                                    self.pos,
                                ))
                            }
                            _ => {
                                return Err(located(
                                    DiagnosticCode::InvalidInput,
                                    "Expected comma or closing parenthesis",
                                    self.pos,
                                    self.pos + 1,
                                ))
                            }
                        }
                    }
                }
                if args.len() != function_arity(function) {
                    return Err(located(
                        DiagnosticCode::InvalidInput,
                        "Wrong function argument count",
                        start,
                        self.pos,
                    ));
                }
                self.node(Expr::Call(function, args))
            }
            Some(b'^' | b'=' | b'[' | b'{' | b';') => Err(located(
                DiagnosticCode::Unsupported,
                "Operator or literal syntax is not admitted",
                self.pos,
                self.pos + 1,
            )),
            _ => Err(located(
                DiagnosticCode::InvalidInput,
                "Unexpected expression token",
                self.pos,
                self.pos + 1,
            )),
        }
    }
}
pub fn parse(text: &str, limits: &Limits) -> Result<Expr, ParseDiagnostic> {
    let limits = limits.bounded();
    if text.len() > limits.expression_bytes {
        return Err(located(
            DiagnosticCode::ResourceLimit,
            "Expression byte limit exceeded",
            0,
            text.len(),
        ));
    }
    let mut parser = Parser {
        text,
        pos: 0,
        nodes: 0,
        limits,
    };
    let expression = parser.add(1)?;
    if let Some(token) = parser.peek() {
        return Err(located(
            if matches!(token, b'^' | b'=' | b'[' | b'{' | b';') {
                DiagnosticCode::Unsupported
            } else {
                DiagnosticCode::InvalidInput
            },
            "Unexpected trailing expression text",
            parser.pos,
            text.len(),
        ));
    }
    expression_shape(&expression, &parser.limits)?;
    Ok(expression)
}
/// Fully parenthesized scalar syntax; physical AST literals are visibly marked
/// and must be represented as bindings for textual export.
pub fn format_expression(expression: &Expr) -> String {
    enum Piece<'a> {
        Expr(&'a Expr),
        Text(&'static str),
        Owned(String),
    }
    let mut output = String::new();
    let mut stack = vec![Piece::Expr(expression)];
    let mut visited = 0usize;
    while let Some(piece) = stack.pop() {
        match piece {
            Piece::Text(text) => output.push_str(text),
            Piece::Owned(text) => output.push_str(&text),
            Piece::Expr(expr) => {
                visited += 1;
                if visited > 1024 {
                    return "<expression exceeds supported limits>".into();
                }
                match expr {
                    Expr::Literal(literal) => {
                        output.push_str(&literal.decimal);
                        if literal.quantity
                            != (QuantityType {
                                kind: QuantityKind::Scalar,
                                unit: UnitId::ScalarOne,
                            })
                        {
                            output.push('[');
                            output.push_str(unit_key(literal.quantity.unit));
                            output.push(']');
                        }
                    }
                    Expr::Binding(name) => output.push_str(name),
                    Expr::Neg(inner) => {
                        stack.push(Piece::Text(")"));
                        stack.push(Piece::Expr(inner));
                        stack.push(Piece::Text("(-"));
                    }
                    Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
                        let operator = match expr {
                            Expr::Add(_, _) => "+",
                            Expr::Sub(_, _) => "-",
                            Expr::Mul(_, _) => "*",
                            _ => "/",
                        };
                        stack.push(Piece::Text(")"));
                        stack.push(Piece::Expr(b));
                        stack.push(Piece::Text(operator));
                        stack.push(Piece::Expr(a));
                        stack.push(Piece::Text("("));
                    }
                    Expr::Call(function, args) => {
                        if args.len() > 2 {
                            return "<expression exceeds supported limits>".into();
                        }
                        stack.push(Piece::Text(")"));
                        for (index, arg) in args.iter().enumerate().rev() {
                            stack.push(Piece::Expr(arg));
                            if index > 0 {
                                stack.push(Piece::Text(","));
                            }
                        }
                        stack.push(Piece::Text("("));
                        stack.push(Piece::Owned(function_name(*function).into()));
                    }
                }
            }
        }
        if output.len() > 262144 {
            return "<expression exceeds supported limits>".into();
        }
    }
    output
}
fn validity(error: &Diagnostic) -> Validity {
    match error.code {
        DiagnosticCode::IncompleteInput => Validity::Incomplete,
        DiagnosticCode::Unsupported
        | DiagnosticCode::UnsupportedAlgebra
        | DiagnosticCode::Overflow
        | DiagnosticCode::Underflow
        | DiagnosticCode::ResourceLimit => Validity::Unsupported,
        _ => Validity::Invalid,
    }
}
pub fn expression_validity(text: &str, limits: &Limits) -> Validity {
    parse(text, limits).map_or_else(|diagnostic| validity(&diagnostic), |_| Validity::Valid)
}
fn active_literal<'a>(definition: &'a Definition, draft: &RawDraft) -> Option<&'a Literal> {
    draft
        .key
        .field
        .strip_prefix("calc.input.")
        .and_then(|name| definition.bindings.get(name))
        .and_then(|input| match input {
            Input::Literal(literal) => Some(literal),
            _ => None,
        })
}
fn draft_literal(definition: &Definition, draft: &RawDraft) -> Result<Option<Literal>, Diagnostic> {
    let Some(current) = active_literal(definition, draft) else {
        return Ok(None);
    };
    let quantity = match draft.unit_context.as_deref() {
        Some(context) => {
            let unit = unit_from_key(context).ok_or_else(|| {
                error(
                    DiagnosticCode::InvalidInput,
                    "Unknown calculation raw unit context",
                )
            })?;
            QuantityType {
                kind: unit_kind(unit),
                unit,
            }
        }
        None => current.quantity,
    };
    Ok(Some(Literal {
        decimal: draft.text.clone(),
        quantity,
    }))
}
/// None denotes a detached/unknown journal, not an active calculation input.
/// Historic recognized unit context classifies its own retained numeric text.
pub fn raw_validity(
    definition: &Definition,
    draft: &RawDraft,
    limits: &Limits,
) -> Option<Validity> {
    if draft.key.record != definition.owner || draft.key.item.is_some() {
        return None;
    }
    if draft.key.field == "calc.expression" {
        return Some(if draft.unit_context.is_some() {
            Validity::Invalid
        } else {
            expression_validity(&draft.text, limits)
        });
    }
    match draft_literal(definition, draft) {
        Ok(Some(literal)) => Some(
            validate_literal(&literal)
                .map_or_else(|diagnostic| validity(&diagnostic), |_| Validity::Valid),
        ),
        Ok(None) => None,
        Err(_) => Some(Validity::Invalid),
    }
}
pub fn classify_raw(
    definition: &Definition,
    draft: &RawDraft,
    limits: &Limits,
) -> Result<(), Diagnostic> {
    let Some(actual) = raw_validity(definition, draft, limits) else {
        return Ok(());
    };
    if actual != draft.validity {
        return Err(error(
            DiagnosticCode::InvalidInput,
            "Raw validity claim contradicts its current source text",
        ));
    }
    if draft.key.field == "calc.expression" {
        if draft.unit_context.is_some() {
            return Err(error(
                DiagnosticCode::InvalidInput,
                "Calculation expression has an unexpected unit context",
            ));
        }
        let expr = parse(&draft.text, limits)?;
        if expr != definition.expression {
            return Err(error(
                DiagnosticCode::InvalidInput,
                "Current formula raw text differs from the accepted expression",
            ));
        }
    } else if let Some(literal) = draft_literal(definition, draft)? {
        validate_literal(&literal)?;
        if Some(&literal) != active_literal(definition, draft) {
            return Err(error(
                DiagnosticCode::InvalidInput,
                "Current input raw text/unit differs from the accepted literal",
            ));
        }
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QuantityValue {
    pub value: f64,
    pub quantity: QuantityType,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Outcome {
    Value(QuantityValue),
    Failure(Diagnostic),
}
impl Outcome {
    pub fn diagnostic(&self) -> Option<&Diagnostic> {
        match self {
            Self::Failure(diagnostic) => Some(diagnostic),
            Self::Value(_) => None,
        }
    }
    pub fn value(&self) -> Option<&QuantityValue> {
        match self {
            Self::Value(value) => Some(value),
            Self::Failure(_) => None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvaluationReceipt {
    pub algorithm_revision: String,
    pub unit_table_revision: String,
    pub stamp: DocumentStamp,
    pub definitions_digest: String,
    pub raw_digest: String,
    pub input_closure_digest: String,
    pub dependencies: BTreeMap<CalculationId, BTreeSet<CalculationId>>,
    pub outcomes: BTreeMap<CalculationId, Outcome>,
    pub diagnostics: Vec<Diagnostic>,
    pub successful: bool,
    pub limits: Limits,
}
fn digest<T: Serialize>(value: &T) -> String {
    // All serializable fields are finite/text; no f64 values occur in definitions/raw.
    match serde_json::to_vec(value) {
        Ok(bytes) => format!("{:x}", Sha256::digest(bytes)),
        Err(_) => "unserializable".into(),
    }
}
fn closure_digest(definitions: &BTreeMap<CalculationId, Definition>) -> String {
    let bindings: BTreeMap<_, _> = definitions
        .iter()
        .map(|(id, definition)| (*id, &definition.bindings))
        .collect();
    digest(&bindings)
}
fn raw_bounded(raw: &CalculationRawState, limits: &Limits) -> bool {
    if raw.len() > limits.definitions {
        return false;
    }
    let mut bytes = 0usize;
    let mut entries = 0usize;
    for drafts in raw.values() {
        if drafts.len() > limits.bindings + 1 {
            return false;
        }
        for draft in drafts {
            entries += 1;
            bytes = bytes
                .saturating_add(draft.text.len())
                .saturating_add(draft.key.field.len())
                .saturating_add(draft.unit_context.as_ref().map_or(0, String::len));
            if draft.text.len() > limits.expression_bytes
                || draft.key.field.len() > 128
                || draft
                    .unit_context
                    .as_ref()
                    .is_some_and(|unit| unit.len() > 64)
                || entries > 33792
                || bytes > 8 * 1024 * 1024
            {
                return false;
            }
        }
    }
    true
}
impl EvaluationReceipt {
    /// Whether this resource refusal applies to the exact authoritative document
    /// stamp supplied by the owner. This is deliberately separate from
    /// `is_current`: an oversized graph was not hashed or successfully computed,
    /// and this method never makes its refusal reusable as a current value.
    /// The caller must derive the full stamp, including snapshot_hash, from its
    /// actual current document rather than trusting cached revision counters.
    pub fn refusal_is_current(&self, stamp: &DocumentStamp) -> bool {
        !self.successful
            && self.algorithm_revision == ALGORITHM_REVISION
            && self.unit_table_revision == UNIT_TABLE_REVISION
            && &self.stamp == stamp
            && self.definitions_digest == "resource-refused"
            && self.raw_digest == "resource-refused"
            && self.input_closure_digest == "resource-refused"
            && self.outcomes.len() <= Limits::default().definitions + 1
            && self.outcomes.values().all(|outcome| {
                matches!(outcome, Outcome::Failure(diagnostic)
                    if diagnostic.code == DiagnosticCode::ResourceLimit)
            })
            && !self.diagnostics.is_empty()
            && self.diagnostics.len() <= Limits::default().diagnostics
            && self
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit)
    }

    pub fn is_current(
        &self,
        definitions: &BTreeMap<CalculationId, Definition>,
        raw: &CalculationRawState,
        stamp: &DocumentStamp,
    ) -> bool {
        definitions.len() <= self.limits.definitions
            && definitions
                .values()
                .try_fold(0usize, |total, definition| {
                    resource_shape(&definition.expression, &self.limits)
                        .ok()
                        .and_then(|nodes| total.checked_add(nodes))
                })
                .is_some_and(|nodes| nodes <= self.limits.document_ast_nodes)
            && raw_bounded(raw, &self.limits)
            && definitions
                .values()
                .all(|definition| validate(definition, &self.limits).is_ok())
            && self.algorithm_revision == ALGORITHM_REVISION
            && self.unit_table_revision == UNIT_TABLE_REVISION
            && &self.stamp == stamp
            && self.definitions_digest == digest(definitions)
            && self.raw_digest == digest(raw)
            && self.input_closure_digest == closure_digest(definitions)
    }
}
#[derive(Clone, Copy)]
struct Number {
    value: f64,
    kind: QuantityKind,
}
fn base_literal(literal: &Literal) -> Result<Number, Diagnostic> {
    validate_literal(literal)?;
    Ok(Number {
        value: checked_product(decimal(&literal.decimal)?, scale(literal.quantity.unit))?,
        kind: literal.quantity.kind,
    })
}
fn result_number(value: f64, kind: QuantityKind) -> Result<Number, Diagnostic> {
    let value = checked_number(value)?;
    count_range(value, kind)?;
    Ok(Number { value, kind })
}
fn product_kind(
    a: QuantityKind,
    b: QuantityKind,
    divide: bool,
) -> Result<QuantityKind, Diagnostic> {
    use QuantityKind::*;
    let result = if divide {
        match (a, b) {
            (_, Scalar) => Some(a),
            (Ratio, Ratio) => Some(Ratio),
            (Count, Count) | (Length, Length) | (Duration, Duration) | (Speed, Speed) => {
                Some(Ratio)
            }
            (Length, Duration) => Some(Speed),
            (Length, Speed) => Some(Duration),
            _ => None,
        }
    } else {
        match (a, b) {
            (Scalar, _) => Some(b),
            (_, Scalar) => Some(a),
            (Ratio, Ratio) => Some(Ratio),
            (Speed, Duration) | (Duration, Speed) => Some(Length),
            _ => None,
        }
    };
    result.ok_or_else(|| {
        error(
            DiagnosticCode::UnsupportedAlgebra,
            "Product/quotient of these named quantity kinds is not admitted",
        )
    })
}
fn evaluate_expr(
    expression: &Expr,
    bindings: &BTreeMap<String, Number>,
    visits: &mut usize,
    limits: &Limits,
) -> Result<Number, Diagnostic> {
    *visits += 1;
    if *visits > limits.node_visits {
        return Err(error(
            DiagnosticCode::ResourceLimit,
            "Evaluation node-visit budget exhausted",
        ));
    }
    match expression {
        Expr::Literal(literal) => base_literal(literal),
        Expr::Binding(name) => bindings.get(name).copied().ok_or_else(|| {
            error(
                DiagnosticCode::InvalidInput,
                format!("Unbound identifier {name}"),
            )
        }),
        Expr::Neg(inner) => {
            let inner = evaluate_expr(inner, bindings, visits, limits)?;
            result_number(-inner.value, inner.kind)
        }
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
            let a = evaluate_expr(a, bindings, visits, limits)?;
            let b = evaluate_expr(b, bindings, visits, limits)?;
            match expression {
                Expr::Add(_, _) => {
                    compatible(a.kind, b.kind)?;
                    result_number(a.value + b.value, a.kind)
                }
                Expr::Sub(_, _) => {
                    compatible(a.kind, b.kind)?;
                    result_number(a.value - b.value, a.kind)
                }
                Expr::Mul(_, _) => {
                    let kind = product_kind(a.kind, b.kind, false)?;
                    result_number(checked_product(a.value, b.value)?, kind)
                }
                _ => {
                    let kind = product_kind(a.kind, b.kind, true)?;
                    result_number(checked_division(a.value, b.value)?, kind)
                }
            }
        }
        Expr::Call(function, args) => {
            if args.len() != function_arity(*function) {
                return Err(error(
                    DiagnosticCode::InvalidInput,
                    "Invalid function arity",
                ));
            }
            let a = evaluate_expr(&args[0], bindings, visits, limits)?;
            match function {
                Function::Abs => result_number(a.value.abs(), a.kind),
                Function::Sqrt => {
                    compatible(a.kind, QuantityKind::Scalar)?;
                    if a.value < 0.0 {
                        return Err(error(
                            DiagnosticCode::DomainError,
                            "sqrt requires a nonnegative Scalar",
                        ));
                    }
                    result_number(a.value.sqrt(), QuantityKind::Scalar)
                }
                Function::Min | Function::Max => {
                    let b = evaluate_expr(&args[1], bindings, visits, limits)?;
                    compatible(a.kind, b.kind)?;
                    result_number(
                        if *function == Function::Min {
                            a.value.min(b.value)
                        } else {
                            a.value.max(b.value)
                        },
                        a.kind,
                    )
                }
            }
        }
    }
}
fn evaluate_definition(
    definition: &Definition,
    outcomes: &BTreeMap<CalculationId, Outcome>,
    visits: &mut usize,
    limits: &Limits,
) -> Result<QuantityValue, Diagnostic> {
    let mut values = BTreeMap::new();
    for (name, input) in &definition.bindings {
        let value = match input {
            Input::Literal(literal) => base_literal(literal)?,
            Input::Output(id) => match outcomes.get(id) {
                Some(Outcome::Value(value)) => Number {
                    value: checked_product(value.value, scale(value.quantity.unit))?,
                    kind: value.quantity.kind,
                },
                Some(Outcome::Failure(cause)) => {
                    return Err(Diagnostic {
                        related: vec![*id],
                        cause: Some(cause.code),
                        ..error(
                            DiagnosticCode::BlockedBy,
                            format!("Dependency {} did not produce a current value", id.0),
                        )
                    })
                }
                None => {
                    return Err(Diagnostic {
                        related: vec![*id],
                        ..error(
                            DiagnosticCode::MissingReference,
                            format!("Calculation output {} is missing", id.0),
                        )
                    })
                }
            },
        };
        values.insert(name.clone(), value);
    }
    let result = evaluate_expr(&definition.expression, &values, visits, limits)?;
    compatible(result.kind, definition.output.kind)?;
    let value = checked_division(result.value, scale(definition.output.unit))?;
    count_range(value, result.kind)?;
    Ok(QuantityValue {
        value,
        quantity: definition.output,
    })
}
// Iterative Kosaraju: graph size is bounded before traversal; no dependency recursion.
fn cycle_components(
    graph: &BTreeMap<CalculationId, BTreeSet<CalculationId>>,
) -> Vec<Vec<CalculationId>> {
    let mut visited = BTreeSet::new();
    let mut finished = Vec::new();
    for id in graph.keys() {
        if visited.contains(id) {
            continue;
        }
        let mut stack = vec![(*id, false)];
        while let Some((id, expanded)) = stack.pop() {
            if expanded {
                finished.push(id);
                continue;
            }
            if !visited.insert(id) {
                continue;
            }
            stack.push((id, true));
            for dependency in graph[&id]
                .iter()
                .rev()
                .filter(|dependency| graph.contains_key(*dependency))
            {
                if !visited.contains(dependency) {
                    stack.push((*dependency, false));
                }
            }
        }
    }
    let mut reverse: BTreeMap<_, BTreeSet<_>> =
        graph.keys().map(|id| (*id, BTreeSet::new())).collect();
    for (id, dependencies) in graph {
        for dependency in dependencies {
            if let Some(target) = reverse.get_mut(dependency) {
                target.insert(*id);
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut cycles = Vec::new();
    for id in finished.into_iter().rev() {
        if seen.contains(&id) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![id];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            component.push(id);
            stack.extend(
                reverse[&id]
                    .iter()
                    .rev()
                    .filter(|item| !seen.contains(*item))
                    .copied(),
            );
        }
        component.sort();
        if component.len() > 1 || graph[&id].contains(&id) {
            cycles.push(component);
        }
    }
    cycles.sort();
    cycles
}
pub fn evaluate(
    definitions: &BTreeMap<CalculationId, Definition>,
    raw: &CalculationRawState,
    stamp: &DocumentStamp,
    limits: &Limits,
) -> EvaluationReceipt {
    let limits = limits.bounded();
    // Preflight sizes before hashing/traversing an unbounded definition graph.
    let mut outcomes = BTreeMap::new();
    let mut graph = BTreeMap::new();
    let mut global_error = if definitions.len() > limits.definitions || !raw_bounded(raw, &limits) {
        Some(error(
            DiagnosticCode::ResourceLimit,
            "Calculation definition/raw journal resource limit exceeded",
        ))
    } else {
        None
    };
    let mut total_nodes = 0usize;
    let mut total_edges = 0usize;
    if global_error.is_none() {
        for (id, definition) in definitions {
            if definition.bindings.len() > limits.bindings {
                global_error = Some(error(
                    DiagnosticCode::ResourceLimit,
                    "Calculation binding limit exceeded",
                ));
                break;
            }
            match resource_shape(&definition.expression, &limits) {
                Ok(nodes) => total_nodes += nodes,
                Err(diagnostic) => {
                    global_error = Some(diagnostic);
                    break;
                }
            }
            if total_nodes > limits.document_ast_nodes {
                global_error = Some(error(
                    DiagnosticCode::ResourceLimit,
                    "Document AST node limit exceeded",
                ));
                break;
            }
            if definition.bindings.keys().any(|name| name.len() > 64) {
                global_error = Some(error(
                    DiagnosticCode::ResourceLimit,
                    "Binding identifier byte limit exceeded",
                ));
                break;
            }
            if definition.bindings.values().any(|input| matches!(input, Input::Literal(literal) if literal.decimal.len() > limits.decimal_bytes)) {
                global_error = Some(error(DiagnosticCode::ResourceLimit, "Binding decimal byte limit exceeded"));
                break;
            }
            let dependencies = dependencies(definition);
            total_edges += dependencies.len();
            if total_edges > limits.dependency_edges {
                global_error = Some(error(
                    DiagnosticCode::ResourceLimit,
                    "Dependency edge limit exceeded",
                ));
                break;
            }
            graph.insert(*id, dependencies);
            let validation = if *id != definition.id {
                Err(error(
                    DiagnosticCode::InvalidInput,
                    "Definition map key does not match calculation ID",
                ))
            } else {
                validate(definition, &limits)
            };
            if let Err(diagnostic) = validation {
                if diagnostic.code == DiagnosticCode::ResourceLimit {
                    global_error = Some(diagnostic);
                    break;
                }
                outcomes.insert(*id, Outcome::Failure(diagnostic));
                continue;
            }
            if let Some(drafts) = raw.get(id) {
                let mut keys = BTreeSet::new();
                let mut failure = None;
                let mut ordered: Vec<_> = drafts.iter().collect();
                ordered.sort_by(|a, b| {
                    a.key
                        .cmp(&b.key)
                        .then(a.owner_generation.cmp(&b.owner_generation))
                });
                for draft in ordered {
                    if !keys.insert(draft.key.clone()) {
                        failure = Some(error(
                            DiagnosticCode::InvalidInput,
                            "Duplicate calculation raw field",
                        ));
                        break;
                    }
                    if let Err(diagnostic) = classify_raw(definition, draft, &limits) {
                        failure = Some(diagnostic);
                        break;
                    }
                }
                if let Some(diagnostic) = failure {
                    outcomes.insert(*id, Outcome::Failure(diagnostic));
                }
            }
        }
    }
    if let Some(diagnostic) = &global_error {
        // Resource failure never leaves an apparently successful partial graph.
        outcomes = definitions
            .keys()
            .take(1025)
            .map(|id| (*id, Outcome::Failure(diagnostic.clone())))
            .collect();
    } else {
        for component in cycle_components(&graph) {
            for id in &component {
                outcomes.insert(
                    *id,
                    Outcome::Failure(Diagnostic {
                        related: component.clone(),
                        ..error(
                            DiagnosticCode::Cycle,
                            "Cyclic calculation output dependency",
                        )
                    }),
                );
            }
        }
        let mut reverse: BTreeMap<CalculationId, BTreeSet<CalculationId>> = BTreeMap::new();
        let mut pending = BTreeMap::new();
        let mut ready = BTreeSet::new();
        for (id, dependencies) in &graph {
            if outcomes.contains_key(id) {
                continue;
            }
            let count = dependencies
                .iter()
                .filter(|dependency| {
                    definitions.contains_key(*dependency) && !outcomes.contains_key(*dependency)
                })
                .count();
            pending.insert(*id, count);
            if count == 0 {
                ready.insert(*id);
            }
            for dependency in dependencies {
                reverse.entry(*dependency).or_default().insert(*id);
            }
        }
        let mut visits = 0usize;
        while let Some(id) = ready.pop_first() {
            let outcome = evaluate_definition(&definitions[&id], &outcomes, &mut visits, &limits)
                .map_or_else(Outcome::Failure, Outcome::Value);
            outcomes.insert(id, outcome);
            if let Some(dependents) = reverse.get(&id) {
                for dependent in dependents {
                    if let Some(count) = pending.get_mut(dependent) {
                        if *count > 0 {
                            *count -= 1;
                            if *count == 0 {
                                ready.insert(*dependent);
                            }
                        }
                    }
                }
            }
        }
    }
    let mut diagnostics = Vec::new();
    let failures: Vec<_> = outcomes
        .iter()
        .filter_map(|(id, outcome)| outcome.diagnostic().map(|diagnostic| (*id, diagnostic)))
        .collect();
    let diagnostic_limit = limits.diagnostics;
    let truncated = failures.len() > diagnostic_limit;
    let retain = if truncated {
        diagnostic_limit.saturating_sub(1)
    } else {
        diagnostic_limit
    };
    for (id, diagnostic) in failures.iter().take(retain) {
        let mut diagnostic = (*diagnostic).clone();
        if diagnostic.related.is_empty() {
            diagnostic.related.push(*id);
        }
        diagnostics.push(diagnostic);
    }
    if truncated || global_error.is_some() {
        let diagnostic = error(
            DiagnosticCode::ResourceLimit,
            if truncated {
                "Diagnostic list truncated by explicit limit"
            } else {
                "Calculation graph refused by resource preflight"
            },
        );
        if diagnostics.len() < diagnostic_limit.max(1) {
            diagnostics.push(diagnostic);
        }
    }
    let successful = global_error.is_none()
        && !truncated
        && outcomes.len() == definitions.len()
        && outcomes
            .values()
            .all(|outcome| matches!(outcome, Outcome::Value(_)));
    // Definition/raw payloads have bounded normal producers; a rejected oversized
    // graph deliberately receives no reusable current stamp/digest.
    let hashable = global_error.is_none();
    EvaluationReceipt {
        algorithm_revision: ALGORITHM_REVISION.into(),
        unit_table_revision: UNIT_TABLE_REVISION.into(),
        stamp: stamp.clone(),
        definitions_digest: if hashable {
            digest(definitions)
        } else {
            "resource-refused".into()
        },
        raw_digest: if hashable {
            digest(raw)
        } else {
            "resource-refused".into()
        },
        input_closure_digest: if hashable {
            closure_digest(definitions)
        } else {
            "resource-refused".into()
        },
        dependencies: graph,
        outcomes,
        diagnostics,
        successful,
        limits,
    }
}
