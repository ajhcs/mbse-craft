//! Read-only local coverage and conservative change impact. No verification decisions.
use super::*;
use mbse_model::stage1::Editor;

#[derive(Clone, Debug, Serialize)]
pub struct CoverageRow {
    pub requirement: RequirementId,
    pub identifier: String,
    pub missing_satisfy: bool,
    pub missing_verify: bool,
    pub orphan: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct EvidenceRow {
    pub evidence: EvidenceId,
    pub requirement: RequirementId,
    pub reasons: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CoverageReport {
    pub register_sha256: String,
    pub native_sha256: String,
    pub config_sha256: String,
    pub requirements: Vec<CoverageRow>,
    pub detached_traces: Vec<traces::TraceProjection>,
    pub evidence: Vec<EvidenceRow>,
    pub evidence_total: usize,
    pub evidence_unexamined: usize,
    pub context_error: Option<String>,
    pub unsupported_count: usize,
    pub unsupported_locations: Vec<String>,
    pub impact_root: Option<RequirementId>,
    pub potential_requirements: Vec<RequirementId>,
    pub potential_traces: Vec<u64>,
    pub potential_evidence: Vec<EvidenceId>,
}

/// Hash only semantic supplier context; labels and verification declarations do not select meaning.
/// Parent containment contributes conservative context; it never derives text or inherits status.
pub(super) fn dependency_hash(s: &Register, root: RequirementId) -> Result<Option<String>, String> {
    let mut pending = vec![root];
    let mut nodes = BTreeMap::new();
    let mut edges = BTreeMap::new();
    let mut cases = BTreeMap::new();
    let mut has_dependency = false;
    let mut dependencies: BTreeMap<RequirementId, Vec<&RequirementTrace>> = BTreeMap::new();
    for t in s.traces.values() {
        let owner = if let TraceEndpoint::Requirement(id) = t.source {
            id
        } else {
            t.target
        };
        dependencies.entry(owner).or_default().push(t);
    }
    while let Some(id) = pending.pop() {
        if nodes.contains_key(&id) {
            continue;
        }
        let r = s
            .requirements
            .get(&id)
            .ok_or("COVERAGE_REQUIREMENT_MISSING")?;
        nodes.insert(id, requirement_hash(r)?);
        if let Some(parent) = r.parent {
            has_dependency = true;
            pending.push(parent);
        }
        for t in dependencies.get(&id).into_iter().flatten() {
            let outgoing = t.source == TraceEndpoint::Requirement(id);
            let incoming_nonrequirement =
                t.target == id && !matches!(t.source, TraceEndpoint::Requirement(_));
            if outgoing || incoming_nonrequirement {
                has_dependency = true;
                edges.insert(t.id, (t.kind, t.source.clone(), t.target));
                if outgoing {
                    pending.push(t.target);
                }
                if let TraceEndpoint::TestCase(case) = t.source {
                    cases.insert(
                        case,
                        s.test_cases.get(&case).ok_or("TRACE_TEST_CASE_MISSING")?,
                    );
                }
            }
        }
    }
    if has_dependency {
        digest(&(nodes, edges, cases, &s.unknown)).map(Some)
    } else {
        Ok(None)
    }
}

impl Board {
    pub fn coverage(
        &self,
        editor: &Editor,
        config: &Value,
        impact_root: Option<RequirementId>,
    ) -> Result<CoverageReport, String> {
        validate_register(&self.register)?;
        envelope_size(&self.register)?;
        if impact_root.is_some_and(|id| !self.register.requirements.contains_key(&id)) {
            return Err("COVERAGE_REQUIREMENT_MISSING".into());
        }
        let projections = self.trace_projection(&editor.snapshot);
        let active: BTreeSet<_> = projections
            .iter()
            .filter(|p| p.active)
            .map(|p| p.trace)
            .collect();
        let mut satisfied = BTreeSet::new();
        let mut verified = BTreeSet::new();
        let mut linked = BTreeSet::new();
        for t in self.register.traces.values() {
            linked.insert(t.target);
            if let TraceEndpoint::Requirement(id) = t.source {
                linked.insert(id);
            }
            if active.contains(&t.id) {
                match t.kind {
                    TraceKind::Satisfy => {
                        satisfied.insert(t.target);
                    }
                    TraceKind::Verify => {
                        verified.insert(t.target);
                    }
                    _ => (),
                }
            }
        }
        let parents: BTreeSet<_> = self
            .register
            .requirements
            .values()
            .filter_map(|r| r.parent)
            .collect();
        let requirements = self
            .register
            .requirements
            .values()
            .map(|r| CoverageRow {
                requirement: r.id,
                identifier: r.identifier.clone(),
                missing_satisfy: !satisfied.contains(&r.id),
                missing_verify: !verified.contains(&r.id),
                orphan: r.parent.is_none() && !parents.contains(&r.id) && !linked.contains(&r.id),
            })
            .collect();
        let context = EvidenceContext::from_editor(editor, config);
        let mut evidence = vec![];
        for e in self.register.evidence.values().take(256) {
            let mut reasons =
                self.evidence_staleness(e.id, context.as_ref().unwrap_or(&e.context))?;
            if let Err(error) = &context {
                reasons.push(format!("Current model/config context unavailable: {error}"));
            }
            evidence.push(EvidenceRow {
                evidence: e.id,
                requirement: e.requirement,
                reasons,
            });
        }
        let unknown = mbse_model::types::deletion::unknown_impact(editor);
        let mut unsupported_count = unknown.count;
        let mut unsupported_locations = unknown.locations;
        let mut disclose = |location: String| {
            unsupported_count += 1;
            if unsupported_locations.len() < 128 {
                unsupported_locations.push(location);
            }
        };
        for key in self.register.unknown.keys() {
            disclose(format!(
                "Register unknown metadata {key}; relation impact uninterpreted"
            ));
        }
        for r in self.register.requirements.values() {
            for key in r.unknown.keys() {
                disclose(format!("Requirement #{} unknown metadata {key}", r.id.0));
            }
            if let Some(a) = &r.anchor {
                for key in a.unknown.keys() {
                    disclose(format!(
                        "Requirement #{} source-anchor metadata {key}",
                        r.id.0
                    ));
                }
            }
        }
        for e in self.register.evidence.values() {
            for key in e.unknown.keys() {
                disclose(format!("Evidence #{} unknown metadata {key}", e.id.0));
            }
        }
        let mut affected = BTreeSet::new();
        let mut pending: Vec<_> = impact_root.into_iter().collect();
        while let Some(id) = pending.pop() {
            if !affected.insert(id) {
                continue;
            }
            for r in self.register.requirements.values() {
                if r.parent == Some(id) {
                    pending.push(r.id);
                }
            }
            for t in self.register.traces.values() {
                if t.target == id {
                    if let TraceEndpoint::Requirement(client) = t.source {
                        pending.push(client);
                    }
                }
            }
        }
        let potential_traces = self
            .register
            .traces
            .values()
            .filter(|t| {
                affected.contains(&t.target)
                    || matches!(t.source,TraceEndpoint::Requirement(id)if affected.contains(&id))
            })
            .map(|t| t.id)
            .collect();
        let potential_evidence = self
            .register
            .evidence
            .values()
            .filter(|e| affected.contains(&e.requirement))
            .map(|e| e.id)
            .collect();
        Ok(CoverageReport {
            register_sha256: digest(&self.register)?,
            native_sha256: editor.snapshot.digest(),
            config_sha256: digest(config)?,
            requirements,
            detached_traces: projections.into_iter().filter(|p| !p.active).collect(),
            evidence,
            evidence_total: self.register.evidence.len(),
            evidence_unexamined: self.register.evidence.len().saturating_sub(256),
            context_error: context.err(),
            unsupported_count,
            unsupported_locations,
            impact_root,
            potential_requirements: affected.into_iter().collect(),
            potential_traces,
            potential_evidence,
        })
    }
}
