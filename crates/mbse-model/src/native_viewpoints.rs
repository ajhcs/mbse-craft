//! App-owned documentary viewpoint scope and read-only native-card filtering.
//! This does not create SysML Viewpoint/Concern model elements or semantic links.
use super::*;
pub const PROFILE: &str = "mbse-craft.native-views/5";
pub const RECOVERY_PROFILE: &str = "mbse-craft.native-views-recovery/5";
pub fn profile(s: &str) -> bool {
    matches!(s, PROFILE | RECOVERY_PROFILE)||super::activities::profile(s)
}
mod optional_id {
    use super::*;
    pub fn serialize<S: Serializer>(id: &Option<RecordId>, s: S) -> Result<S::Ok, S::Error> {
        match id {
            Some(id) => s.serialize_some(&id.0.to_string()),
            None => s.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<RecordId>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| {
                let id = s.parse::<u64>().map_err(de::Error::custom)?;
                if id == 0 || id.to_string() != s {
                    return Err(de::Error::custom("canonical positive native ID required"));
                }
                Ok(RecordId(id))
            })
            .transpose()
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub source_document: DocumentId,
    pub name: String,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "optional_id")]
    pub stakeholder: Option<RecordId>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "optional_id")]
    pub concern: Option<RecordId>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "optional_id")]
    pub system: Option<RecordId>,
    pub kinds: BTreeSet<RecordKind>,
    pub name_contains: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub source_document: DocumentId,
    pub name: String,
    pub enabled: bool,
    pub stakeholder: String,
    pub concern: String,
    pub system: String,
    pub kinds: BTreeSet<RecordKind>,
    pub name_contains: String,
}
impl Form {
    pub fn new(editor: &Editor) -> Self {
        Self {
            source_document: editor.snapshot.document_id.clone(),
            name: String::new(),
            enabled: true,
            stakeholder: String::new(),
            concern: String::new(),
            system: String::new(),
            kinds: BTreeSet::new(),
            name_contains: String::new(),
        }
    }
    pub fn from_spec(s: &Spec) -> Self {
        Self {
            source_document: s.source_document.clone(),
            name: s.name.clone(),
            enabled: s.enabled,
            stakeholder: s.stakeholder.map(|n| n.0.to_string()).unwrap_or_default(),
            concern: s.concern.map(|n| n.0.to_string()).unwrap_or_default(),
            system: s.system.map(|n| n.0.to_string()).unwrap_or_default(),
            kinds: s.kinds.clone(),
            name_contains: s.name_contains.clone(),
        }
    }
    fn validate_raw(&self) -> Result<(), String> {
        if [
            &self.source_document.0,
            &self.name,
            &self.stakeholder,
            &self.concern,
            &self.system,
            &self.name_contains,
        ]
        .iter()
        .any(|s| s.len() > 4096)
        {
            Err("VIEWPOINT_FORM_LIMIT".into())
        } else {
            Ok(())
        }
    }
    fn parse(&self) -> Result<Spec, String> {
        self.validate_raw()?;
        let id = |s: &str| -> Result<Option<RecordId>, String> {
            if s.is_empty() {
                return Ok(None);
            }
            let n = s.parse::<u64>().map_err(|_| "VIEWPOINT_CANONICAL_ID")?;
            if n == 0 || n.to_string() != s {
                return Err("VIEWPOINT_CANONICAL_ID".into());
            }
            Ok(Some(RecordId(n)))
        };
        let spec = Spec {
            source_document: self.source_document.clone(),
            name: self.name.clone(),
            enabled: self.enabled,
            stakeholder: id(&self.stakeholder)?,
            concern: id(&self.concern)?,
            system: id(&self.system)?,
            kinds: self.kinds.clone(),
            name_contains: self.name_contains.clone(),
        };
        spec.validate()?;
        Ok(spec)
    }
}
impl Spec {
    fn validate(&self) -> Result<(), String> {
        if self.source_document.0.is_empty()
            || self.source_document.0.len() > 4096
            || self.name.trim().is_empty()
            || self.name.trim() != self.name
            || self.name.len() > 128
            || self.name.chars().any(char::is_control)
            || self.name_contains.len() > 256
            || self.name_contains.chars().any(char::is_control)
            || self.stakeholder.is_some_and(|id| id.0 == 0)
            || self.concern.is_some_and(|id| id.0 == 0)
            || self.system.is_some_and(|id| id.0 == 0)
            || self.concern.is_some() && self.stakeholder.is_none()
        {
            return Err("VIEWPOINT_SPEC_FIELDS".into());
        }
        Ok(())
    }
    fn context(&self, snapshot: &Snapshot) -> Vec<String> {
        let mut errors = vec![];
        if self.source_document != snapshot.document_id {
            return vec![format!(
                "Foreign viewpoint source {} differs from current {}; original IDs retained without retargeting. Filter inactive.",
                self.source_document.0, snapshot.document_id.0
            )];
        }
        for (id, kind, label) in [
            (self.stakeholder, RecordKind::Stakeholder, "Stakeholder"),
            (
                self.concern,
                RecordKind::Expectation,
                "Concern (native Expectation)",
            ),
            (self.system, RecordKind::System, "System scope"),
        ] {
            if let Some(id) = id {
                if snapshot.records.get(&id).is_none_or(|r| r.kind != kind) {
                    errors.push(format!(
                        "Detached {label} #{}; original reference retained.",
                        id.0
                    ));
                }
            }
        }
        if let (Some(concern), Some(stakeholder)) = (self.concern, self.stakeholder) {
            if let Some(r) = snapshot
                .records
                .get(&concern)
                .filter(|r| r.kind == RecordKind::Expectation)
            {
                if r.fields
                    .get("expressedBy")
                    .and_then(serde_json::Value::as_u64)
                    != Some(stakeholder.0)
                {
                    errors.push(
                        "Concern is not expressed by the declared Stakeholder; filter inactive."
                            .into(),
                    );
                }
                if self.system.is_some_and(|s| {
                    r.fields
                        .get("concernsSystem")
                        .and_then(serde_json::Value::as_u64)
                        != Some(s.0)
                }) {
                    errors.push(
                        "Concern does not reference the declared System scope; filter inactive."
                            .into(),
                    );
                }
            }
        }
        if let (Some(stakeholder), Some(system)) = (self.stakeholder, self.system) {
            if snapshot
                .records
                .get(&stakeholder)
                .is_some_and(|r| r.kind == RecordKind::Stakeholder && r.scope != system)
            {
                errors.push("Stakeholder belongs to another System; filter inactive.".into());
            }
        }
        errors
    }
}
#[derive(Clone, Debug)]
pub enum Action {
    SetForm(Form),
    Apply,
    Clear,
    RebindFormSource,
}
pub(super) fn validate(view: &View, doc_profile: &str) -> Result<(), String> {
    if (view.viewpoint.is_some() || view.viewpoint_form.is_some()) && !profile(doc_profile) {
        return Err("VIEWPOINT_REQUIRES_V5".into());
    }
    if let Some(s) = &view.viewpoint {
        s.validate()?;
    }
    if let Some(f) = &view.viewpoint_form {
        f.validate_raw()?;
    }
    Ok(())
}
pub(super) fn apply(
    doc: &mut Document,
    editor: &Editor,
    view: ViewId,
    action: Action,
) -> Result<(), String> {
    let current = doc.views.get_mut(&view).ok_or("VIEWS_NO_VIEW")?;
    match action {
        Action::SetForm(form) => {
            form.validate_raw()?;
            current.viewpoint_form = Some(form);
        }
        Action::Apply => {
            let s = current
                .viewpoint_form
                .as_ref()
                .ok_or("VIEWPOINT_FORM_MISSING")?
                .parse()?;
            let errors = s.context(&editor.snapshot);
            if !errors.is_empty() {
                return Err(format!("VIEWPOINT_CONTEXT: {}", errors.join(" ")));
            }
            current.viewpoint = Some(s);
        }
        Action::Clear => current.viewpoint = None,
        Action::RebindFormSource => {
            current
                .viewpoint_form
                .as_mut()
                .ok_or("VIEWPOINT_FORM_MISSING")?
                .source_document = editor.snapshot.document_id.clone();
        }
    }
    if !super::activities::profile(&doc.profile){
    doc.profile = if bound_profile(&doc.profile) {
        PROFILE
    } else {
        RECOVERY_PROFILE
    }
    .into();
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize)]
pub struct Projection {
    active: bool,
    context_resolved: bool,
    matched: BTreeSet<RecordId>,
    limitations: Vec<String>,
}
impl Projection {
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn context_resolved(&self) -> bool {
        self.context_resolved
    }
    pub fn allows(&self, id: RecordId) -> bool {
        !self.active || self.matched.contains(&id)
    }
    pub fn matched(&self) -> &BTreeSet<RecordId> {
        &self.matched
    }
    pub fn limitations(&self) -> &[String] {
        &self.limitations
    }
}
pub fn project(view: &View, snapshot: &Snapshot) -> Projection {
    let mut p=Projection{active:false,context_resolved:true,matched:BTreeSet::new(),limitations:vec!["Filters affect native record cards only; typed port/connector rendering, manual visibility and the complete model/ledger retain their existing scope. No normative SysML Viewpoint/Concern or engineering approval is inferred.".into()]};
    let Some(spec) = &view.viewpoint else {
        return p;
    };
    let errors = spec.context(snapshot);
    p.context_resolved = errors.is_empty();
    p.limitations.extend(errors);
    p.active = spec.enabled && p.context_resolved;
    if p.active {
        let needle = spec.name_contains.to_lowercase();
        for r in snapshot.records.values() {
            if spec.system.is_none_or(|s| r.scope == s)
                && (spec.kinds.is_empty() || spec.kinds.contains(&r.kind))
                && r.name.to_lowercase().contains(&needle)
            {
                p.matched.insert(r.id);
            }
        }
    }
    p
}
pub(super) fn check_wire(
    root: &serde_json::Map<String, serde_json::Value>,
    doc_profile: &str,
) -> Result<(), String> {
    if let Some(views) = root.get("views").and_then(serde_json::Value::as_object) {
        for view in views.values() {
            if (view.get("viewpoint").is_some() || view.get("viewpoint_form").is_some())
                && !profile(doc_profile)
            {
                return Err("VIEWPOINT_REQUIRES_V5".into());
            }
        }
    }
    Ok(())
}
