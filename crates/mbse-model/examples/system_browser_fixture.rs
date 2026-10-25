//! Explicit public synthetic GUI fixture, written through the checked native Save protocol.
//! It does not add in-product System creation or claim engineering acceptance.
use mbse_model::{
    stage1::{Record, RecordId, RecordKind},
    stage1_io::{execute_write, Session},
};
use std::{collections::BTreeMap, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Explicit output path required")?,
    );
    let root =
        PathBuf::from("/workspace/build/builderbob/mbse-craft/native-system-browser/fixture");
    if out != root.join("two-systems.kernspan")
        || out.exists()
        || out.is_symlink()
        || root.exists()
        || root.is_symlink()
    {
        return Err("Refuse any other/existing/symlink fixture destination".into());
    }
    std::fs::create_dir(&root)?;
    if root.canonicalize()? != root {
        return Err("Fixture parent must be the exact non-symlink workspace directory".into());
    }
    let state = root.join("state");
    if state.exists() || state.is_symlink() {
        return Err("Fresh fixture state required".into());
    }
    std::fs::create_dir(&state)?;
    let mut session = Session::new(&state)?;
    session
        .editor
        .snapshot
        .records
        .get_mut(&RecordId(1))
        .unwrap()
        .name = "Synthetic System A".into();
    session
        .editor
        .snapshot
        .records
        .get_mut(&RecordId(1))
        .unwrap()
        .fields
        .insert(
            "purpose".into(),
            serde_json::json!(
                "Synthetic native browser QA fixture, not accepted engineering content."
            ),
        );
    session.editor.snapshot.records.insert(
        RecordId(2),
        Record {
            id: RecordId(2),
            kind: RecordKind::System,
            name: "Synthetic System B".into(),
            scope: RecordId(2),
            fields: BTreeMap::from([(
                "purpose".into(),
                serde_json::json!(
                    "Synthetic native browser QA fixture, not accepted engineering content."
                ),
            )]),
        },
    );
    for (id, scope) in [(1000, 1), (1001, 2)] {
        session.editor.snapshot.records.insert(
            RecordId(id),
            Record {
                id: RecordId(id),
                kind: RecordKind::Mission,
                name: "Duplicate mission".into(),
                scope: RecordId(scope),
                fields: BTreeMap::new(),
            },
        );
    }
    session.editor.snapshot.next_id = 1002;
    session.editor.snapshot.validate()?;
    let write = session.begin_save(&out)?;
    if !session.complete(execute_write(write)) {
        return Err("Fixture Save not acknowledged".into());
    }
    let cp = session
        .native_checkpoint()?
        .ok_or("Missing actual fixture checkpoint")?;
    let receipt = serde_json::json!({"status":"WrittenThroughExistingCheckedSessionSave","qualification":"Public synthetic QA fixture; no engineering acceptance or in-product System creation claim","path":out,"native_sha256":cp.artifact_sha256(),"snapshot_sha256":cp.snapshot_sha256(),"system_ids":["1","2"],"mission_ids":["1000","1001"],"memberships":[["1000","1"],["1001","2"]]});
    std::fs::write(
        root.join("FIXTURE-RECEIPT.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
