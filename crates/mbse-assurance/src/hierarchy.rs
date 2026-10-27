//! Requirement containment over the existing Board; no derivation or status roll-up.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HierarchyRow {
    pub id: RequirementId,
    pub parent: Option<RequirementId>,
    pub depth: usize,
    pub child_count: usize,
}
fn children(s: &Register) -> Result<BTreeMap<RequirementId, Vec<RequirementId>>, String> {
    let mut children: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (id, r) in &s.requirements {
        if id != &r.id || id.0 == 0 {
            return Err("ASSURANCE_REQUIREMENT_ID".into());
        }
        if let Some(parent) = r.parent {
            if parent == *id {
                return Err("ASSURANCE_PARENT_CYCLE".into());
            }
            if !s.requirements.contains_key(&parent) {
                return Err("ASSURANCE_PARENT_MISSING".into());
            }
            children.entry(parent).or_default().push(*id);
        }
    }
    Ok(children)
}
pub(super) fn validate(s: &Register) -> Result<(), String> {
    if s.requirements.len() > 10000 {
        return Err("ASSURANCE_RECORD_LIMIT".into());
    }
    let mut identifiers = BTreeSet::new();
    for r in s.requirements.values() {
        if !r.identifier.is_empty() {
            identifier_syntax(&r.identifier)?;
            if !identifiers.insert(r.identifier.as_str()) {
                return Err("ASSURANCE_DUPLICATE_IDENTIFIER".into());
            }
        }
    }
    let children = children(s)?;
    let mut pending: Vec<_> = s
        .requirements
        .values()
        .filter(|r| r.parent.is_none())
        .map(|r| r.id)
        .collect();
    let mut visited = 0;
    while let Some(id) = pending.pop() {
        visited += 1;
        if let Some(ids) = children.get(&id) {
            pending.extend(ids);
        }
    }
    if visited != s.requirements.len() {
        return Err("ASSURANCE_PARENT_CYCLE".into());
    }
    // Serialized raw flags cannot promote a proposal to accepted source truth.
    for r in s.requirements.values() {
        if s.drafts
            .get(&format!("{}:identifier", r.id.0))
            .is_some_and(|d| {
                d.valid && (identifier_syntax(&d.text).is_err() || d.text != r.identifier)
            })
        {
            return Err("ASSURANCE_IDENTIFIER_DRAFT_VALIDITY_MISMATCH".into());
        }
        if s.drafts
            .get(&format!("{}:parent", r.id.0))
            .is_some_and(|d| d.valid && parse_parent(&d.text).ok() != Some(r.parent))
        {
            return Err("ASSURANCE_PARENT_DRAFT_VALIDITY_MISMATCH".into());
        }
        if s.drafts
            .get(&format!("{}:statement", r.id.0))
            .is_some_and(|d| d.valid && (d.text.trim().is_empty() || d.text != r.statement))
        {
            return Err("ASSURANCE_STATEMENT_DRAFT_VALIDITY_MISMATCH".into());
        }
    }
    Ok(())
}
/// Reject numeric-map aliases before typed map deserialization can overwrite a record.
pub(super) fn validate_wire_ids(root: &Value) -> Result<(), String> {
    let s = root
        .get("register")
        .and_then(Value::as_object)
        .ok_or("ASSURANCE_REGISTER_OBJECT_REQUIRED")?;
    for field in ["requirements", "evidence", "decisions", "traces", "test_cases"] {
        if matches!(field,"traces"|"test_cases") && s.get(field).is_none(){continue;}
        let map = s
            .get(field)
            .and_then(Value::as_object)
            .ok_or("ASSURANCE_REGISTER_MAP_REQUIRED")?;
        for key in map.keys() {
            if key
                .parse::<u64>()
                .ok()
                .is_none_or(|id| id == 0 || id.to_string() != *key)
            {
                return Err("ASSURANCE_NONCANONICAL_OR_ALIASED_ID".into());
            }
        }
    }
    Ok(())
}
fn parse_parent(text: &str) -> Result<Option<RequirementId>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    let n = text
        .parse::<u64>()
        .map_err(|_| "ASSURANCE_PARENT_CANONICAL_ID_REQUIRED")?;
    if n == 0 || n.to_string() != text {
        return Err("ASSURANCE_PARENT_CANONICAL_ID_REQUIRED".into());
    }
    Ok(Some(RequirementId(n)))
}
fn identifier_syntax(text: &str) -> Result<(), String> {
    if text.is_empty()
        || text.len() > 128
        || text.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("ASSURANCE_IDENTIFIER_REQUIRED_OR_FORMAT".into());
    }
    Ok(())
}
impl Board {
    pub fn set_new_identifier_draft(&mut self, text: String) -> Result<(), String> {
        if text.len() > 4096 {
            return Err("ASSURANCE_IDENTIFIER_DRAFT_LIMIT".into());
        }
        self.transaction(|s| {
            if text.is_empty() {
                s.drafts.remove("new:identifier");
            } else {
                let valid = identifier_syntax(&text).is_ok()
                    && !s.requirements.values().any(|r| r.identifier == text);
                s.drafts
                    .insert("new:identifier".into(), RawDraft { text, valid });
            }
            Ok(())
        })
    }
    pub fn edit_identifier(&mut self, id: RequirementId, text: String) -> Result<bool, String> {
        if text.len() > 4096 {
            return Err("ASSURANCE_IDENTIFIER_DRAFT_LIMIT".into());
        }
        self.transaction(|s| {
            if !s.requirements.contains_key(&id) {
                return Err("ASSURANCE_REQUIREMENT_MISSING".into());
            }
            let valid = identifier_syntax(&text).is_ok()
                && !s
                    .requirements
                    .values()
                    .any(|r| r.id != id && r.identifier == text);
            s.drafts.insert(
                format!("{}:identifier", id.0),
                RawDraft {
                    text: text.clone(),
                    valid,
                },
            );
            if valid {
                let r = s.requirements.get_mut(&id).unwrap();
                if r.identifier != text {
                    r.identifier = text;
                    r.revision = r
                        .revision
                        .checked_add(1)
                        .ok_or("ASSURANCE_REVISION_LIMIT")?;
                }
            }
            Ok(valid)
        })
    }
    pub fn add_requirement_under(
        &mut self,
        statement: String,
        parent: Option<RequirementId>,
    ) -> Result<RequirementId, String> {
        self.add_with_identifier(statement, None, parent)
    }
    pub fn add_identified_requirement(
        &mut self,
        statement: String,
        identifier: String,
        parent: Option<RequirementId>,
    ) -> Result<RequirementId, String> {
        identifier_syntax(&identifier)?;
        self.add_with_identifier(statement, Some(identifier), parent)
    }
    fn add_with_identifier(
        &mut self,
        statement: String,
        identifier: Option<String>,
        parent: Option<RequirementId>,
    ) -> Result<RequirementId, String> {
        self.transaction(|s| {
            if identifier
                .as_ref()
                .is_some_and(|id| s.requirements.values().any(|r| &r.identifier == id))
            {
                return Err("ASSURANCE_DUPLICATE_IDENTIFIER".into());
            }
            if s.requirements.len() >= 10000 {
                return Err("ASSURANCE_RECORD_LIMIT".into());
            }
            if statement.trim().is_empty() {
                return Err("ASSURANCE_STATEMENT_REQUIRED".into());
            }
            if parent.is_some_and(|p| !s.requirements.contains_key(&p)) {
                return Err("ASSURANCE_PARENT_MISSING".into());
            }
            let id = RequirementId(s.next_id);
            if s.requirements.contains_key(&id) || s.evidence.contains_key(&EvidenceId(id.0)) {
                return Err("ASSURANCE_DUPLICATE_ID".into());
            }
            s.next_id = s.next_id.checked_add(1).ok_or("ASSURANCE_ID_LIMIT")?;
            s.drafts.remove("new:statement");
            if identifier.is_some() {
                s.drafts.remove("new:identifier");
            }
            let identifier = identifier.unwrap_or_default();
            s.requirements.insert(
                id,
                Requirement {
                    id,
                    parent,
                    identifier,
                    statement,
                    rationale: String::new(),
                    source: String::new(),
                    owner: String::new(),
                    user_version: String::new(),
                    revision: 0,
                    maturity: Maturity::Draft,
                    implementation: ImplementationStatus::UnderReview,
                    verification: VerificationStatus::NotStarted,
                    anchor: None,
                    unknown: BTreeMap::new(),
                },
            );
            Ok(id)
        })
    }
    /// Raw parent text is retained even when missing, noncanonical, self or cyclic.
    pub fn edit_parent(&mut self, id: RequirementId, text: String) -> Result<bool, String> {
        if text.len() > 4096 {
            return Err("ASSURANCE_PARENT_DRAFT_LIMIT".into());
        }
        self.transaction(|s| {
            if !s.requirements.contains_key(&id) {
                return Err("ASSURANCE_REQUIREMENT_MISSING".into());
            }
            let parsed = parse_parent(&text);
            let mut valid = parsed.is_ok();
            if let Ok(Some(parent)) = &parsed {
                let parent = *parent;
                valid = s.requirements.contains_key(&parent);
                let mut cursor = Some(parent);
                for _ in 0..=s.requirements.len() {
                    let Some(at) = cursor else { break };
                    if at == id {
                        valid = false;
                        break;
                    }
                    cursor = s.requirements.get(&at).and_then(|r| r.parent);
                }
            }
            s.drafts
                .insert(format!("{}:parent", id.0), RawDraft { text, valid });
            if valid {
                let parent = parsed?;
                let r = s.requirements.get_mut(&id).unwrap();
                if r.parent != parent {
                    r.parent = parent;
                    r.revision = r
                        .revision
                        .checked_add(1)
                        .ok_or("ASSURANCE_REVISION_LIMIT")?;
                }
            }
            Ok(valid)
        })
    }
    /// Iterative deterministic preorder; depth is data, never an executable recursion budget.
    pub fn hierarchy(&self) -> Result<Vec<HierarchyRow>, String> {
        validate(&self.register)?;
        let children = children(&self.register)?;
        let mut pending: Vec<_> = self
            .register
            .requirements
            .values()
            .rev()
            .filter(|r| r.parent.is_none())
            .map(|r| (r.id, 0))
            .collect();
        let mut rows = Vec::with_capacity(self.register.requirements.len());
        while let Some((id, depth)) = pending.pop() {
            let r = &self.register.requirements[&id];
            rows.push(HierarchyRow {
                id,
                parent: r.parent,
                depth,
                child_count: children.get(&id).map_or(0, Vec::len),
            });
            if let Some(ids) = children.get(&id) {
                pending.extend(ids.iter().rev().map(|id| (*id, depth + 1)));
            }
        }
        Ok(rows)
    }
}
