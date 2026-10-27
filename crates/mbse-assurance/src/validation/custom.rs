//! Finite declarative checks and sealed previews over the existing requirement authority.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextField {
    Identifier,
    Statement,
}
impl TextField {
    fn key(self) -> &'static str {
        match self {
            Self::Identifier => "identifier",
            Self::Statement => "statement",
        }
    }
    fn target(self) -> RequirementField {
        match self {
            Self::Identifier => RequirementField::Identifier,
            Self::Statement => RequirementField::Statement,
        }
    }
    fn value<'a>(self, r: &'a Requirement) -> &'a str {
        match self {
            Self::Identifier => &r.identifier,
            Self::Statement => &r.statement,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TextPredicate {
    NonBlank,
    StartsWith { prefix: String },
    Equals { value: String },
}
impl TextPredicate {
    fn matches(&self, s: &str) -> bool {
        match self {
            Self::NonBlank => !s.trim().is_empty(),
            Self::StartsWith { prefix } => s.starts_with(prefix),
            Self::Equals { value } => s == value,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomRule {
    pub id: String,
    pub requirement: RequirementId,
    pub field: TextField,
    pub predicate: TextPredicate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}
fn literal(value: &str) -> Result<(), String> {
    if value.len() > 4096 || value.chars().any(char::is_control) {
        Err("CUSTOM_RULE_LITERAL_LIMIT_OR_CONTROL".into())
    } else {
        Ok(())
    }
}
pub(super) fn validate_rules(rules: &[CustomRule]) -> Result<(), String> {
    if rules.len() > 32 {
        return Err("CUSTOM_RULE_LIMIT".into());
    }
    let mut ids = BTreeSet::new();
    for r in rules {
        if r.id.is_empty()
            || r.id.len() > 64
            || !r
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            || !ids.insert(&r.id)
            || r.requirement.0 == 0
        {
            return Err("CUSTOM_RULE_ID_OR_SCOPE".into());
        }
        match &r.predicate {
            TextPredicate::StartsWith { prefix } => {
                literal(prefix)?;
                if prefix.is_empty() {
                    return Err("CUSTOM_RULE_EMPTY_PREFIX".into());
                }
            }
            TextPredicate::Equals { value } => literal(value)?,
            TextPredicate::NonBlank => (),
        }
        if let Some(v) = &r.replacement {
            literal(v)?;
            if !r.predicate.matches(v) {
                return Err("CUSTOM_RULE_FIX_DOES_NOT_SATISFY_PREDICATE".into());
            }
        }
    }
    Ok(())
}
impl CustomRule {
    pub(super) fn evaluate(&self, s: &Register) -> Result<Option<(String, Target)>, String> {
        let r = s.requirements.get(&self.requirement).ok_or_else(|| {
            format!(
                "Custom rule {}: exact requirement #{} missing; no retargeting.",
                self.id, self.requirement.0
            )
        })?;
        Ok((!self.predicate.matches(self.field.value(r))).then(||(format!("Custom requirement text predicate {:?} failed on accepted text. Pending drafts are not accepted by this check.",self.predicate),Target::Requirement{requirement:self.requirement,field:self.field.target()})))
    }
}
/// Runtime review of exact suite proposal. Loading rule data never restores this authority.
#[derive(Clone, Debug, Serialize)]
pub struct SuiteReview {
    name: String,
    suite: Suite,
    #[serde(skip)]
    source: String,
}
impl SuiteReview {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn suite(&self) -> &Suite {
        &self.suite
    }
}
impl Board {
    pub fn preview_validation_suite(&self) -> Result<SuiteReview, String> {
        validate_register(&self.register)?;
        envelope_size(&self.register)?;
        let name = self
            .register
            .drafts
            .get("validation:name")
            .map(|d| d.text.clone())
            .unwrap_or_default();
        let raw = self
            .register
            .drafts
            .get("validation:rules")
            .map(|d| d.text.as_str())
            .unwrap_or("");
        let suite: Suite =
            serde_json::from_str(raw).map_err(|e| format!("VALIDATION_DECLARATIVE_SUITE: {e}"))?;
        let mut proposed = self.clone();
        proposed.admit_suite(name.clone(), suite.clone())?;
        Ok(SuiteReview {
            name,
            suite,
            source: digest(&self.register)?,
        })
    }
    pub fn admit_validation_review(&mut self, review: &SuiteReview) -> Result<(), String> {
        if digest(&self.register)? != review.source {
            return Err("CUSTOM_RULE_REVIEW_STALE".into());
        }
        self.admit_suite(review.name.clone(), review.suite.clone())
    }
    pub(super) fn admit_suite(&mut self, name: String, suite: Suite) -> Result<(), String> {
        self.transaction(|s| {
            s.validation_suites.insert(name, suite);
            Ok(())
        })
    }
}
/// Complete requirement and raw-draft changes produced by the existing edit authority.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FixChange {
    pub requirement: RequirementId,
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct QuickFixPreview {
    issue: String,
    changes: Vec<FixChange>,
    #[serde(skip)]
    guard: Guard,
    #[serde(skip)]
    after: Register,
}
impl QuickFixPreview {
    pub fn issue(&self) -> &str {
        &self.issue
    }
    pub fn changes(&self) -> &[FixChange] {
        &self.changes
    }
    pub fn check(&self, b: &Board, e: &Editor) -> Result<(), String> {
        self.guard.check(b, e)
    }
}
fn changes(before: &Register, after: &Register) -> Vec<FixChange> {
    let mut out = vec![];
    for (id, r) in &before.requirements {
        let next = &after.requirements[id];
        for (field, a, b) in [
            ("identifier", &r.identifier, &next.identifier),
            ("statement", &r.statement, &next.statement),
        ] {
            if a != b {
                out.push(FixChange {
                    requirement: *id,
                    field: field.into(),
                    before: Some(a.clone()),
                    after: Some(b.clone()),
                });
            }
            let key = format!("{}:{field}", id.0);
            let a = before.drafts.get(&key).map(|d| d.text.clone());
            let b = after.drafts.get(&key).map(|d| d.text.clone());
            if a != b {
                out.push(FixChange {
                    requirement: *id,
                    field: format!("raw:{field}"),
                    before: a,
                    after: b,
                });
            }
        }
    }
    out
}
impl Report {
    pub fn preview_quick_fix(
        &self,
        b: &Board,
        e: &Editor,
        id: &str,
    ) -> Result<QuickFixPreview, String> {
        self.check(b, e)?;
        if !self.issues.iter().any(|i| i.id == id) {
            return Err("VALIDATION_ISSUE_MISSING".into());
        }
        let rule = self.fixes.get(id).ok_or("CUSTOM_RULE_NO_EXPLICIT_FIX")?;
        let value = rule
            .replacement
            .as_ref()
            .ok_or("CUSTOM_RULE_NO_EXPLICIT_FIX")?;
        let key = format!("{}:{}", rule.requirement.0, rule.field.key());
        if b.register.drafts.get(&key).is_some_and(|d| !d.valid) {
            return Err("CUSTOM_FIX_PENDING_DRAFT_PRESERVED".into());
        }
        let mut prospective = b.clone();
        let valid = match rule.field {
            TextField::Identifier => {
                prospective.edit_identifier(rule.requirement, value.clone())?
            }
            TextField::Statement => {
                prospective.edit_field(rule.requirement, "statement", value.clone())?
            }
        };
        if !valid {
            return Err("CUSTOM_FIX_REFUSED_BY_EXISTING_AUTHORITY".into());
        }
        if rule.evaluate(&prospective.register)?.is_some() {
            return Err("CUSTOM_FIX_PREDICATE_REMAINS".into());
        }
        // Master edits may propagate through Copy. Never overwrite an invalid slave draft.
        for (key, draft) in &b.register.drafts {
            if !draft.valid && prospective.register.drafts.get(key) != Some(draft) {
                return Err("CUSTOM_FIX_PENDING_DRAFT_PRESERVED".into());
            }
        }
        let changes = changes(&b.register, &prospective.register);
        if changes.is_empty() {
            return Err("CUSTOM_FIX_NO_CHANGE".into());
        }
        Ok(QuickFixPreview {
            issue: id.into(),
            changes,
            guard: self.guard.clone(),
            after: prospective.register,
        })
    }
}
impl Board {
    pub fn apply_quick_fix(&mut self, e: &Editor, preview: &QuickFixPreview) -> Result<(), String> {
        preview.check(self, e)?;
        self.transaction(|s| {
            *s = preview.after.clone();
            Ok(())
        })
    }
}
