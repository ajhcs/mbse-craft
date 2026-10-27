//! Bounded typed dependencies in the existing register. Labels never choose semantics.
use super::*;
use mbse_model::{
    stage1::Snapshot,
    types::{self, Id, tags::TargetKind},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceKind {
    Derive,
    Refine,
    Satisfy,
    Verify,
    Copy,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEndpoint {
    pub document: String,
    pub id: u64,
    pub expected: TargetKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TraceEndpoint {
    Requirement(RequirementId),
    Native(NativeEndpoint),
    TestCase(u64),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementTrace {
    pub id: u64,
    pub kind: TraceKind,
    pub source: TraceEndpoint,
    pub target: RequirementId,
    pub label: String,
}
/// Documentary test specification, never an execution receipt or verification decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestCase {
    pub id: u64,
    pub name: String,
    pub procedure: String,
    pub expected_result: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TraceProjection {
    pub trace: u64,
    pub active: bool,
    pub reason: String,
}

pub fn native_endpoint(snapshot: &Snapshot, id: u64) -> Result<TraceEndpoint, String> {
    let store = types::load(snapshot)?;
    let expected = types::tags::target_kind(snapshot, &store, Id(id))?;
    Ok(TraceEndpoint::Native(NativeEndpoint {
        document: snapshot.document_id.0.clone(),
        id,
        expected,
    }))
}
pub fn parse_endpoint(text: &str, snapshot: &Snapshot) -> Result<TraceEndpoint, String> {
    let (prefix, raw) = text
        .split_once(':')
        .ok_or("TRACE_ENDPOINT_PREFIX_REQUIRED")?;
    let id = raw
        .parse::<u64>()
        .map_err(|_| "TRACE_CANONICAL_ID_REQUIRED")?;
    if id == 0 || id.to_string() != raw {
        return Err("TRACE_CANONICAL_ID_REQUIRED".into());
    }
    match prefix {
        "req" => Ok(TraceEndpoint::Requirement(RequirementId(id))),
        "native" => native_endpoint(snapshot, id),
        "test" => Ok(TraceEndpoint::TestCase(id)),
        _ => Err("TRACE_ENDPOINT_PREFIX_REQUIRED".into()),
    }
}
fn native_current(endpoint: &NativeEndpoint, snapshot: &Snapshot) -> Result<(), String> {
    if endpoint.document != snapshot.document_id.0 {
        return Err("TRACE_NATIVE_DOCUMENT_MISMATCH".into());
    }
    if native_endpoint(snapshot, endpoint.id)? != TraceEndpoint::Native(endpoint.clone()) {
        return Err("TRACE_NATIVE_KIND_MISMATCH".into());
    }
    Ok(())
}
fn endpoints(s: &Register, t: &RequirementTrace) -> Result<(), String> {
    if !s.requirements.contains_key(&t.target) {
        return Err("TRACE_TARGET_REQUIREMENT_MISSING".into());
    }
    match &t.source {
        TraceEndpoint::Requirement(id) => {
            if !s.requirements.contains_key(id) {
                return Err("TRACE_SOURCE_REQUIREMENT_MISSING".into());
            }
            if *id == t.target {
                return Err("TRACE_SELF_DEPENDENCY".into());
            }
        }
        TraceEndpoint::Native(n) => {
            if n.id == 0 || n.document.trim().is_empty() || n.document.len() > 4096 {
                return Err("TRACE_NATIVE_IDENTITY".into());
            }
        }
        TraceEndpoint::TestCase(id) => {
            if !s.test_cases.contains_key(id) {
                return Err("TRACE_TEST_CASE_MISSING".into());
            }
        }
    }
    let compatible = match (&t.kind, &t.source) {
        (TraceKind::Derive | TraceKind::Copy, TraceEndpoint::Requirement(_)) => true,
        (TraceKind::Refine, TraceEndpoint::Requirement(_) | TraceEndpoint::Native(_)) => true,
        (TraceKind::Satisfy, TraceEndpoint::Native(n)) => matches!(
            n.expected,
            TargetKind::System | TargetKind::ComponentDefinition | TargetKind::Part
        ),
        (TraceKind::Verify, TraceEndpoint::TestCase(_)) => true,
        _ => false,
    };
    if !compatible {
        return Err("TRACE_ENDPOINT_KIND".into());
    }
    if t.label.len() > 512 {
        return Err("TRACE_LABEL_LIMIT".into());
    }
    Ok(())
}
pub(super) fn validate(s: &Register) -> Result<(), String> {
    if s.traces.len() > 10000 || s.test_cases.len() > 10000 {
        return Err("TRACE_RECORD_LIMIT".into());
    }
    let mut masters = BTreeMap::new();
    let mut keys = BTreeSet::new();
    for (id, c) in &s.test_cases {
        if *id != c.id
            || *id == 0
            || c.name.trim().is_empty()
            || c.name.len() > 512
            || c.procedure.trim().is_empty()
            || c.expected_result.trim().is_empty()
            || c.procedure.len() > 16384
            || c.expected_result.len() > 16384
        {
            return Err("TRACE_TEST_CASE_FIELDS".into());
        }
    }
    for (id, t) in &s.traces {
        if *id != t.id || *id == 0 {
            return Err("TRACE_ID".into());
        }
        endpoints(s, t)?;
        let key =
            serde_json::to_string(&(t.kind, &t.source, t.target)).map_err(|e| e.to_string())?;
        if !keys.insert(key) {
            return Err("TRACE_DUPLICATE".into());
        }
        if t.kind == TraceKind::Copy {
            let TraceEndpoint::Requirement(client) = t.source else {
                unreachable!()
            };
            if masters.insert(client, t.target).is_some() {
                return Err("TRACE_COPY_MULTIPLE_MASTERS".into());
            }
            if s.requirements[&client].statement != s.requirements[&t.target].statement {
                return Err("TRACE_COPY_TEXT_MISMATCH".into());
            }
        }
    }
    // Each client has at most one master. Walk paths iteratively, memoizing completed paths.
    let mut done = BTreeSet::new();
    for start in masters.keys() {
        let mut path = BTreeSet::new();
        let mut at = *start;
        while !done.contains(&at) {
            if !path.insert(at) {
                return Err("TRACE_COPY_CYCLE".into());
            }
            let Some(next) = masters.get(&at) else {
                break;
            };
            at = *next;
        }
        done.extend(path);
    }
    Ok(())
}
pub(super) fn is_copy(s: &Register, id: RequirementId) -> bool {
    s.traces
        .values()
        .any(|t| t.kind == TraceKind::Copy && t.source == TraceEndpoint::Requirement(id))
}
pub(super) fn propagate_statement(s: &mut Register, master: RequirementId) -> Result<(), String> {
    let mut pending = vec![master];
    while let Some(at) = pending.pop() {
        let text = s.requirements[&at].statement.clone();
        let clients: Vec<_> = s
            .traces
            .values()
            .filter(|t| t.kind == TraceKind::Copy && t.target == at)
            .filter_map(|t| {
                if let TraceEndpoint::Requirement(id) = t.source {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();
        for id in clients {
            let r = s.requirements.get_mut(&id).unwrap();
            if r.statement != text {
                r.statement = text.clone();
                r.revision = r
                    .revision
                    .checked_add(1)
                    .ok_or("ASSURANCE_REVISION_LIMIT")?;
            }
            if let Some(d) = s.drafts.get_mut(&format!("{}:statement", id.0)) {
                if d.valid {
                    d.text = text.clone();
                }
            }
            pending.push(id);
        }
    }
    Ok(())
}
impl Board {
    pub fn copy_master(&self, id: RequirementId) -> Option<RequirementId> {
        self.register
            .traces
            .values()
            .find(|t| t.kind == TraceKind::Copy && t.source == TraceEndpoint::Requirement(id))
            .map(|t| t.target)
    }
    pub fn set_trace_draft(&mut self, field: &str, text: String) -> Result<(), String> {
        if !matches!(
            field,
            "source" | "label" | "case_name" | "procedure" | "expected"
        ) {
            return Err("TRACE_DRAFT_FIELD".into());
        }
        if text.len() > 16384 {
            return Err("TRACE_DRAFT_LIMIT".into());
        }
        self.transaction(|s| {
            s.drafts
                .insert(format!("trace:{field}"), RawDraft { text, valid: false });
            Ok(())
        })
    }
    pub fn add_test_case(
        &mut self,
        name: String,
        procedure: String,
        expected_result: String,
    ) -> Result<u64, String> {
        self.transaction(|s| {
            let id = s.next_id;
            s.next_id = id.checked_add(1).ok_or("ASSURANCE_ID_LIMIT")?;
            s.test_cases.insert(
                id,
                TestCase {
                    id,
                    name,
                    procedure,
                    expected_result,
                },
            );
            s.drafts.insert(
                "trace:source".into(),
                RawDraft {
                    text: format!("test:{id}"),
                    valid: false,
                },
            );
            Ok(id)
        })
    }
    pub fn add_trace(
        &mut self,
        kind: TraceKind,
        source: TraceEndpoint,
        target: RequirementId,
        label: String,
        snapshot: &Snapshot,
    ) -> Result<u64, String> {
        if let TraceEndpoint::Native(n) = &source {
            native_current(n, snapshot)?;
        }
        self.transaction(|s| {
            let id = s.next_id;
            s.next_id = id.checked_add(1).ok_or("ASSURANCE_ID_LIMIT")?;
            s.traces.insert(
                id,
                RequirementTrace {
                    id,
                    kind,
                    source,
                    target,
                    label,
                },
            );
            Ok(id)
        })
    }
    pub fn rename_trace(&mut self, id: u64, label: String) -> Result<(), String> {
        self.transaction(|s| {
            s.traces.get_mut(&id).ok_or("TRACE_MISSING")?.label = label;
            Ok(())
        })
    }
    pub fn remove_trace(&mut self, id: u64) -> Result<(), String> {
        self.transaction(|s| {
            s.traces.remove(&id).ok_or("TRACE_MISSING")?;
            Ok(())
        })
    }
    pub fn trace_projection(&self, snapshot: &Snapshot) -> Vec<TraceProjection> {
        self.register
            .traces
            .values()
            .map(|t| {
                let result = endpoints(&self.register, t).and_then(|_| match &t.source {
                    TraceEndpoint::Native(n) => native_current(n, snapshot),
                    _ => Ok(()),
                });
                TraceProjection {
                    trace: t.id,
                    active: result.is_ok(),
                    reason: result.err().unwrap_or_else(|| {
                        "Declared dependency only; no fulfillment or verification claim".into()
                    }),
                }
            })
            .collect()
    }
}
