//! Versioned, inert presentation-copy provenance. No Apply or runtime authority.
use crate::{
    native_views::{self as v, Document, NativeBinding, OccurrenceId, ViewId, VisualEdgeId},
    native_views_inspect::Inspection,
    stage1::{DocumentId, RecordId, RecordKind},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE: &str = "mbse-craft.presentation-copy-origin/1";
pub const MAX_PROVENANCE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyOrigin {
    pub profile: String,
    pub source_sha256: String,
    pub source_profile: String,
    pub source_document_id: DocumentId,
    pub source_sidecar_id: String,
    pub source_view: ViewId,
    // An explicitly declared source binding, never a native checkpoint.
    pub declared_native_binding: Option<NativeBinding>,
    pub occurrences: BTreeMap<OccurrenceId, OccurrenceMapping>,
    pub edges: BTreeMap<VisualEdgeId, EdgeMapping>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccurrenceMapping {
    #[serde(with = "v::native_id")]
    pub source_record: RecordId,
    pub expected_kind: RecordKind,
    pub outcome: OccurrenceOutcome,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum OccurrenceOutcome {
    Mapped {
        target_occurrence: OccurrenceId,
        #[serde(with = "v::native_id")]
        target_record: RecordId,
    },
    Skipped {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeMapping {
    pub source_occurrence: OccurrenceId,
    pub target_occurrence: OccurrenceId,
    pub outcome: EdgeOutcome,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub enum EdgeOutcome {
    Mapped { target_edge: VisualEdgeId },
    Skipped { reason: String },
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn reason(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 1024
}

impl CopyOrigin {
    /// Structural provenance validation, including exact endpoints when still
    /// present. Removed endpoints remain retired historical IDs, not live refs.
    pub(crate) fn validate(&self, document: &Document, view: ViewId) -> Result<(), String> {
        if self.profile != PROFILE
            || !v::supported_profile(&self.source_profile)
            || !digest(&self.source_sha256)
            || self.source_document_id.0.is_empty()
            || self.source_document_id.0.len() > 1024
            || !self.source_sidecar_id.starts_with("views:")
            || self.source_sidecar_id.len() > 1024
        {
            return Err("VIEWS_COPY_SOURCE_IDENTITY".into());
        }
        if v::bound_profile(&self.source_profile) != self.declared_native_binding.is_some()
            || self.declared_native_binding.as_ref().is_some_and(|b| {
                b.document_id != self.source_document_id
                    || !digest(&b.artifact_sha256)
                    || !digest(&b.snapshot_sha256)
            })
        {
            return Err("VIEWS_COPY_DECLARED_BINDING".into());
        }
        if self.occurrences.len() > v::MAX_PER_VIEW || self.edges.len() > v::MAX_EDGES {
            return Err("VIEWS_COPY_MAPPING_BUDGET".into());
        }
        let mut source_ids = BTreeSet::from([self.source_view.get()]);
        let mut target_ids = BTreeSet::new();
        for (source, mapping) in &self.occurrences {
            if !source_ids.insert(source.get()) || mapping.source_record.0 == 0 {
                return Err("VIEWS_COPY_SOURCE_MAPPING".into());
            }
            match &mapping.outcome {
                OccurrenceOutcome::Mapped {
                    target_occurrence,
                    target_record,
                } => {
                    if target_record.0 == 0
                        || target_occurrence.get() >= document.next_id()
                        || !target_ids.insert(target_occurrence.get())
                        || document
                            .views
                            .keys()
                            .any(|id| id.get() == target_occurrence.get())
                        || document
                            .edges
                            .keys()
                            .any(|id| id.get() == target_occurrence.get())
                    {
                        return Err("VIEWS_COPY_TARGET_MAPPING".into());
                    }
                    if let Some(row) = document.occurrences.get(target_occurrence) {
                        if row.view != view
                            || row.record != *target_record
                            || row.expected_kind != mapping.expected_kind
                        {
                            return Err("VIEWS_COPY_TARGET_IDENTITY".into());
                        }
                    }
                }
                OccurrenceOutcome::Skipped { reason: text } if !reason(text) => {
                    return Err("VIEWS_COPY_SKIP_REASON".into())
                }
                OccurrenceOutcome::Skipped { .. } => {}
            }
        }
        for (source, mapping) in &self.edges {
            if !source_ids.insert(source.get()) {
                return Err("VIEWS_COPY_SOURCE_MAPPING".into());
            }
            let a = self
                .occurrences
                .get(&mapping.source_occurrence)
                .ok_or("VIEWS_COPY_EDGE_SOURCE")?;
            let b = self
                .occurrences
                .get(&mapping.target_occurrence)
                .ok_or("VIEWS_COPY_EDGE_TARGET")?;
            match &mapping.outcome {
                EdgeOutcome::Mapped { target_edge } => {
                    let (
                        OccurrenceOutcome::Mapped {
                            target_occurrence: source,
                            ..
                        },
                        OccurrenceOutcome::Mapped {
                            target_occurrence: target,
                            ..
                        },
                    ) = (&a.outcome, &b.outcome)
                    else {
                        return Err("VIEWS_COPY_EDGE_UNMAPPED_ENDPOINT".into());
                    };
                    if target_edge.get() >= document.next_id()
                        || !target_ids.insert(target_edge.get())
                        || document
                            .views
                            .keys()
                            .any(|id| id.get() == target_edge.get())
                        || document
                            .occurrences
                            .keys()
                            .any(|id| id.get() == target_edge.get())
                    {
                        return Err("VIEWS_COPY_TARGET_MAPPING".into());
                    }
                    if let Some(edge) = document.edges.get(target_edge) {
                        if edge.view != view || edge.source != *source || edge.target != *target {
                            return Err("VIEWS_COPY_EDGE_IDENTITY".into());
                        }
                    }
                }
                EdgeOutcome::Skipped { reason: text } if !reason(text) => {
                    return Err("VIEWS_COPY_SKIP_REASON".into())
                }
                EdgeOutcome::Skipped { .. } => {}
            }
        }
        Ok(())
    }

    /// Explicit source recheck only. No ambient acquisition, target mutation,
    /// identity inference, checkpoint creation or allocation is performed.
    pub fn verify_source(&self, source: &Inspection) -> Result<(), String> {
        source.verify_unchanged()?;
        let doc = source.declared();
        if self.source_sha256 != source.source_sha256()
            || self.source_profile != doc.profile
            || self.source_document_id != doc.document_id
            || self.source_sidecar_id != doc.sidecar_id
            || self.declared_native_binding != doc.binding
            || !doc.views.contains_key(&self.source_view)
        {
            return Err("VIEWS_COPY_SOURCE_MISMATCH".into());
        }
        let rows: BTreeMap<_, _> = doc
            .occurrences
            .iter()
            .filter(|(_, o)| o.view == self.source_view)
            .collect();
        let edges: BTreeMap<_, _> = doc
            .edges
            .iter()
            .filter(|(_, e)| e.view == self.source_view)
            .collect();
        if self.occurrences.len() != rows.len() || self.edges.len() != edges.len() {
            return Err("VIEWS_COPY_SOURCE_COVERAGE".into());
        }
        for (id, mapping) in &self.occurrences {
            let row = rows.get(id).ok_or("VIEWS_COPY_SOURCE_OCCURRENCE")?;
            if row.record != mapping.source_record || row.expected_kind != mapping.expected_kind {
                return Err("VIEWS_COPY_SOURCE_OCCURRENCE".into());
            }
        }
        for (id, mapping) in &self.edges {
            let row = edges.get(id).ok_or("VIEWS_COPY_SOURCE_EDGE")?;
            if row.source != mapping.source_occurrence || row.target != mapping.target_occurrence {
                return Err("VIEWS_COPY_SOURCE_EDGE".into());
            }
        }
        Ok(())
    }
}

/// Check raw counts/bytes before allocating typed provenance maps. v1 remains
/// strict: even a null copy_origin is an unknown v1 field, never an upgrade.
pub(crate) fn check_wire_budget(
    root: &serde_json::Map<String, serde_json::Value>,
    v2: bool,
) -> Result<(), String> {
    let views = root
        .get("views")
        .and_then(serde_json::Value::as_object)
        .ok_or("VIEWS_COLLECTION_REQUIRED")?;
    if views.len() > v::MAX_VIEWS {
        return Err("VIEWS_COUNT_BUDGET".into());
    }
    let mut bytes = 0;
    for view in views.values() {
        if let Some(origin) = view.get("copy_origin") {
            if !v2 {
                return Err("VIEWS_COPY_REQUIRES_V2".into());
            }
            for (key, limit) in [("occurrences", v::MAX_PER_VIEW), ("edges", v::MAX_EDGES)] {
                if origin
                    .get(key)
                    .and_then(serde_json::Value::as_object)
                    .ok_or("VIEWS_COPY_MAPPING_REQUIRED")?
                    .len()
                    > limit
                {
                    return Err("VIEWS_COPY_MAPPING_BUDGET".into());
                }
            }
            bytes += serde_json::to_vec(origin).map_err(|e| e.to_string())?.len();
            if bytes > MAX_PROVENANCE_BYTES {
                return Err("VIEWS_COPY_PROVENANCE_BUDGET".into());
            }
        }
    }
    Ok(())
}
