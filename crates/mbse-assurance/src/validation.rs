//! Named, bounded local suites over existing validators; reports are not decisions.
use super::*;
mod custom;
pub use custom::{CustomRule, TextField, TextPredicate, QuickFixPreview, FixChange, SuiteReview};
use mbse_model::{
    stage1::{Editor, RawKey},
    stage1_calc::{DiagnosticCode, DocumentStamp},
    types::{self, Id},
};
pub const BUILTIN_NAME: &str = "Local working drafts";
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RuleFamily {
    PropertyDraft,
    PartContainment,
    CalculationUnits,
    RequirementDrafts,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suite {
    pub rules: BTreeSet<RuleFamily>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<CustomRule>,
}
impl Suite {
    pub fn local() -> Self {
        Self {
            custom: vec![],
            rules: BTreeSet::from([
                RuleFamily::PropertyDraft,
                RuleFamily::PartContainment,
                RuleFamily::CalculationUnits,
                RuleFamily::RequirementDrafts,
            ]),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PropertyField {
    Name,
    ValueType,
    Lower,
    Upper,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum UsageField {
    Owner,
    Target,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RequirementField {
    Identifier,
    Parent,
    Statement,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Target {
    Property {
        property: Id,
        field: PropertyField,
    },
    Usage {
        usage: Option<Id>,
        field: UsageField,
    },
    NativeField(RawKey),
    Requirement {
        requirement: RequirementId,
        field: RequirementField,
    },
}
impl Target {
    fn key(&self) -> String {
        match self {
            Self::Property { property, field } => format!("property{}:{field:?}", property.0),
            Self::Usage { usage, field } => format!(
                "usage{}:{field:?}",
                usage.map_or("new".into(), |id| id.0.to_string())
            ),
            Self::NativeField(key) => format!("record{}:{}", key.record.0, key.field),
            Self::Requirement { requirement, field } => {
                format!("requirement{}:{field:?}", requirement.0)
            }
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Issue {
    pub id: String,
    pub rule: String,
    pub message: String,
    pub target: Target,
    pub quick_fix: bool,
}
#[derive(Clone, Debug, Serialize)]
struct Guard {
    native: DocumentStamp,
    register: String,
}
impl Guard {
    fn new(b: &Board, e: &Editor) -> Result<Self, String> {
        Ok(Self {
            native: e.calculation_stamp(),
            register: digest(&b.register)?,
        })
    }
    fn check(&self, b: &Board, e: &Editor) -> Result<(), String> {
        if self.native != e.calculation_stamp() || self.register != digest(&b.register)? {
            return Err("VALIDATION_SOURCE_STALE".into());
        }
        Ok(())
    }
}
/// Runtime-only navigation proof: no Deserialize or public constructor.
#[derive(Clone, Debug)]
pub struct Navigation {
    guard: Guard,
    target: Target,
}
impl Navigation {
    pub fn resolve(&self, b: &Board, e: &Editor) -> Result<Target, String> {
        self.guard.check(b, e)?;
        Ok(self.target.clone())
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    guard: Guard,
    suite_name: String,
    rules: BTreeSet<RuleFamily>,
    issues: Vec<Issue>,
    issue_total: usize,
    unreported_issues: usize,
    limitations: Vec<String>,
    limitation_total: usize,
    #[serde(skip)]
    fixes: BTreeMap<String, CustomRule>,
    custom_rules: Vec<CustomRule>,
}
impl Report {
    pub fn custom_rules(&self) -> &[CustomRule] { &self.custom_rules }
    pub fn suite_name(&self) -> &str {
        &self.suite_name
    }
    pub fn rules(&self) -> &BTreeSet<RuleFamily> {
        &self.rules
    }
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }
    pub fn issue_total(&self) -> usize {
        self.issue_total
    }
    pub fn unreported_issues(&self) -> usize {
        self.unreported_issues
    }
    pub fn limitations(&self) -> &[String] {
        &self.limitations
    }
    pub fn limitation_total(&self) -> usize {
        self.limitation_total
    }

    pub fn check(&self, b: &Board, e: &Editor) -> Result<(), String> {
        self.guard.check(b, e)
    }
    pub fn navigate(&self, b: &Board, e: &Editor, id: &str) -> Result<Navigation, String> {
        self.check(b, e)?;
        let issue = self
            .issues
            .iter()
            .find(|i| i.id == id)
            .ok_or("VALIDATION_ISSUE_MISSING")?;
        Ok(Navigation {
            guard: self.guard.clone(),
            target: issue.target.clone(),
        })
    }
}
fn suite_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty()
        || name.trim() != name
        || name.len() > 128
        || name.chars().any(char::is_control)
    {
        Err("VALIDATION_SUITE_NAME".into())
    } else {
        Ok(())
    }
}
pub(super) fn validate_suites(s: &Register) -> Result<(), String> {
    if s.validation_suites.len() > 32 {
        return Err("VALIDATION_SUITE_LIMIT".into());
    }
    for (name, suite) in &s.validation_suites {
        suite_name(name)?;
        if name == BUILTIN_NAME || (suite.rules.is_empty() && suite.custom.is_empty()) {
            return Err("VALIDATION_RESERVED_OR_EMPTY_SUITE".into());
        }
        custom::validate_rules(&suite.custom)?;
    }
    Ok(())
}
impl Board {
    pub fn set_validation_form(&mut self, name: String, rules: String) -> Result<(), String> {
        if name.len() > 4096 || rules.len() > 16384 {
            return Err("VALIDATION_FORM_LIMIT".into());
        }
        self.transaction(|s| {
            s.drafts.insert(
                "validation:name".into(),
                RawDraft {
                    text: name,
                    valid: false,
                },
            );
            s.drafts.insert(
                "validation:rules".into(),
                RawDraft {
                    text: rules,
                    valid: false,
                },
            );
            Ok(())
        })
    }
    pub fn set_validation_suite(&mut self, name: String, suite: Suite) -> Result<(), String> {
        if !suite.custom.is_empty() { return Err("CUSTOM_RULE_REVIEW_REQUIRED".into()); }
        self.admit_suite(name, suite)
    }
    pub fn apply_validation_form(&mut self) -> Result<(), String> {
        let name = self
            .register
            .drafts
            .get("validation:name")
            .map(|d| d.text.clone())
            .unwrap_or_default();
        let rules = self
            .register
            .drafts
            .get("validation:rules")
            .map(|d| d.text.as_str())
            .unwrap_or("");
        let suite: Suite = serde_json::from_str(rules)
            .map_err(|e| format!("VALIDATION_DECLARATIVE_SUITE: {e}"))?;
        self.set_validation_suite(name, suite)
    }
    pub fn run_validation(&self, name: &str, e: &Editor) -> Result<Report, String> {
        validate_register(&self.register)?;
        envelope_size(&self.register)?;
        e.snapshot.validate()?;
        let suite = if name == BUILTIN_NAME {
            Suite::local()
        } else {
            self.register
                .validation_suites
                .get(name)
                .cloned()
                .ok_or("VALIDATION_SUITE_MISSING")?
        };
        let mut issues = vec![];
        let mut issue_total: usize = 0;
        let mut limitations=vec!["Local working-draft checks only; no full native/SysML/NoMagic, engineering or verification verdict.".into(),"Ports/interfaces/connectors/profile/value/refactor/deletion drafts and arbitrary external schemas are outside these four rule families.".into()];
        let mut limitation_total = limitations.len();
        let mut limit = |text: String| {
            limitation_total += 1;
            if limitations.len() < 128 {
                limitations.push(text);
            }
        };
        let mut add = |rule: &str, message: String, target: Target| {
            issue_total += 1;
            if issues.len() < 256 {
                issues.push(Issue {
                    id: format!("{rule}:{}", target.key()),
                    rule: rule.into(),
                    message,
                    target,
                    quick_fix: false,
                });
            }
        };
        let typed_requested = suite.rules.contains(&RuleFamily::PropertyDraft)
            || suite.rules.contains(&RuleFamily::PartContainment);
        let mut typed_available = false;
        let store = if typed_requested {
            match types::load(&e.snapshot) {
                Ok(store) => {
                    typed_available = true;
                    store
                }
                Err(error) => {
                    limit(if e.snapshot.extensions.is_none() {
                        "Type working-draft scope unavailable: native extensions are not migrated; other selected rule families remain available.".into()
                    } else {
                        format!(
                            "Type working-draft scope unavailable: {error}; original optional data retained without interpretation."
                        )
                    });
                    types::Store::default()
                }
            }
        } else {
            types::Store::default()
        };
        if typed_available && suite.rules.contains(&RuleFamily::PropertyDraft) {
            for (id, form) in types::forms(&e.snapshot)? {
                if let Err(error) = types::preview(&e.snapshot, id) {
                    let classification = if (error.starts_with("TYPE_REFERENCE")
                        || error == "TYPE_CROSS_SYSTEM_REFERENCE_UNSUPPORTED")
                        && store
                            .definitions
                            .get(&form.owner)
                            .is_some_and(|d| d.kind == types::Kind::Component)
                    {
                        Some(("MBSE.TYPE.REFERENCE", PropertyField::ValueType))
                    } else if error.starts_with("TYPE_MULTIPLICITY") {
                        Some((
                            "MBSE.TYPE.MULTIPLICITY",
                            if types::Multiplicity::parse(&form.lower, "*").is_err() {
                                PropertyField::Lower
                            } else {
                                PropertyField::Upper
                            },
                        ))
                    } else if error.starts_with("TYPE_NAME") {
                        Some(("MBSE.TYPE.NAME", PropertyField::Name))
                    } else {
                        None
                    };
                    if let Some((rule, field)) = classification {
                        add(
                            rule,
                            error,
                            Target::Property {
                                property: id,
                                field,
                            },
                        );
                    } else {
                        limit(format!(
                            "Property form #{} validator diagnostic has no supported exact-field route: {error}",
                            id.0
                        ));
                    }
                }
            }
        }
        if typed_available && suite.rules.contains(&RuleFamily::PartContainment) {
            if let Some(form) = types::usage_form(&e.snapshot)? {
                if let Err(error) = types::preview_usage(&e.snapshot) {
                    let classification = match error.as_str() {
                        "TYPE_CONTAINMENT_CYCLE" => Some((
                            "MBSE.STRUCT.PART_CONTAINMENT",
                            if form
                                .id
                                .and_then(|id| store.usages.get(&id))
                                .is_some_and(|u| {
                                    form.owner.parse::<u64>().ok() != Some(u.owner.0)
                                        && form.target.parse::<u64>().ok() == Some(u.target.0)
                                })
                            {
                                UsageField::Owner
                            } else {
                                UsageField::Target
                            },
                        )),
                        "TYPE_USAGE_TARGET_MISSING" => {
                            Some(("MBSE.REFERENCE.USAGE", UsageField::Target))
                        }
                        "TYPE_USAGE_OWNER_MISSING" => {
                            Some(("MBSE.REFERENCE.USAGE", UsageField::Owner))
                        }
                        _ => None,
                    };
                    if let Some((rule, field)) = classification {
                        add(
                            rule,
                            error,
                            Target::Usage {
                                usage: form.id,
                                field,
                            },
                        );
                    } else {
                        limit(format!(
                            "Usage validator diagnostic has no supported exact-field route: {error}"
                        ));
                    }
                }
            }
        }
        if suite.rules.contains(&RuleFamily::CalculationUnits) {
            let receipt = e.evaluate_calculations()?;
            for (id, outcome) in receipt.outcomes {
                if let Some(d) = outcome.diagnostic() {
                    if matches!(
                        d.code,
                        DiagnosticCode::QuantityKindMismatch | DiagnosticCode::UnitMismatch
                    ) {
                        let target = Target::NativeField(RawKey {
                            record: mbse_model::stage1::RecordId(id.0),
                            field: "calc.expression".into(),
                            item: None,
                        });
                        add(
                            "MBSE.CALC.UNIT_COMPATIBILITY",
                            format!("{:?}: {}", d.code, d.message),
                            target,
                        );
                    } else {
                        limit(format!(
                            "Calculation #{} diagnostic outside unit rule: {:?}",
                            id.0, d.code
                        ));
                    }
                }
            }
        }
        if suite.rules.contains(&RuleFamily::RequirementDrafts) {
            for (key, draft) in &self.register.drafts {
                if draft.valid {
                    continue;
                }
                let Some((raw, field)) = key.split_once(':') else {
                    continue;
                };
                let Ok(id) = raw.parse::<u64>() else {
                    continue;
                };
                if id == 0
                    || id.to_string() != raw
                    || !self.register.requirements.contains_key(&RequirementId(id))
                {
                    continue;
                }
                let field = match field {
                    "identifier" => Some(RequirementField::Identifier),
                    "parent" => Some(RequirementField::Parent),
                    "statement" => Some(RequirementField::Statement),
                    _ => None,
                };
                if let Some(field) = field {
                    add(
                        "MBSE.REQUIREMENT.DRAFT",
                        "Unapplied raw requirement proposal; accepted record is retained.".into(),
                        Target::Requirement {
                            requirement: RequirementId(id),
                            field,
                        },
                    );
                }
            }
        }
        let mut fixes = BTreeMap::new();
        for rule in &suite.custom {
            match rule.evaluate(&self.register) {
                Ok(Some((message, target))) => {
                    let code = format!("MBSE.CUSTOM.{}", rule.id);
                    let id = format!("{code}:{}", target.key());
                    add(&code, message, target);
                    if rule.replacement.is_some() { fixes.insert(id, rule.clone()); }
                }
                Ok(None) => (),
                Err(error) => limit(error),
            }
        }
        for issue in &mut issues { issue.quick_fix = fixes.contains_key(&issue.id); }
        let unknown = mbse_model::types::deletion::unknown_impact(e);
        if unknown.count > 0 {
            limit(format!(
                "{} native uninterpreted impact locations; completeness unproven.",
                unknown.count
            ));
            for location in unknown.locations {
                limit(location);
            }
        }
        if !self.register.unknown.is_empty() {
            limit(format!(
                "{} register metadata namespaces remain uninterpreted.",
                self.register.unknown.len()
            ));
        }
        issues.sort_by(|a, b| a.id.cmp(&b.id));
        let unreported_issues = issue_total.saturating_sub(issues.len());
        Ok(Report {
            guard: Guard::new(self, e)?,
            suite_name: name.into(),
            rules: suite.rules,
            issues,
            issue_total,
            unreported_issues,
            limitations,
            limitation_total,
            fixes,
            custom_rules: suite.custom,
        })
    }
}
