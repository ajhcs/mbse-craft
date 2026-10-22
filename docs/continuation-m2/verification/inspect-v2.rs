//! Independent count-only reader of the actual V2 public API. Never emits model names,
//! IDs, raw expressions, attributes, source bytes or diagnostic detail text.
use std::{collections::BTreeMap, path::Path};
use serde_json::json;
fn tally(items: impl Iterator<Item=String>) -> BTreeMap<String, usize> {
    let mut result=BTreeMap::new();
    for value in items { *result.entry(value).or_default()+=1; }
    result
}
fn main() {
    let path=std::env::args().nth(1).expect("one absolute source path");
    let path=Path::new(&path);assert!(path.is_absolute());
    let report=mbse_exchange::v2::open(path).expect("actual bounded V2 open");
    let selected:Vec<_>=report.nodes.iter().take(8).map(|n|n.key.clone()).collect();
    let preview=report.admission_preview(&selected);
    assert!(!preview.admitted,"V2 inspection must not admit native edits");
    let result=json!({
        "profile":report.profile,"primary_sha256":report.primary_sha256,
        "source_set_digest":report.source_set_digest,"primary_bytes":report.primary_bytes().len(),
        "stage":report.stage,"asset_count":report.assets.len(),"manifest_count":report.manifest.len(),
        "node_count":report.nodes.len(),"reference_count":report.references.len(),
        "kind_counts":tally(report.nodes.iter().map(|n|format!("{:?}",n.kind))),
        "reference_resolution_counts":tally(report.references.iter().map(|r|format!("{:?}",r.resolution))),
        "reference_provenance_counts":tally(report.references.iter().map(|r|format!("{:?}",r.provenance))),
        "role_resolution_counts":tally(report.references.iter().map(|r|format!("{}::{} / {} / {:?}",r.declaring_type.namespace,r.declaring_type.local,r.feature,r.resolution))),
        "diagnostic_code_counts":tally(report.diagnostics.iter().map(|d|d.code.clone())),
        "loss_code_counts":tally(report.losses.iter().map(|l|l.code.clone())),
        "version_claim_count":report.source_versions.len(),
        "recognized_contract_count":report.source_versions.iter().filter(|v|v.recognized_contract).count(),
        "counters":report.counters,
        "preview":{"selected_count":preview.selected.len(),"admitted":preview.admitted,
            "loss_code_counts":tally(preview.losses.iter().map(|l|l.code.clone())),
            "omitted_losses":preview.omitted_losses,"action_count":preview.actions.len()}
    });
    println!("{}",serde_json::to_string_pretty(&result).unwrap());
}
