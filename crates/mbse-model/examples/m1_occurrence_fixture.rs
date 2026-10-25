//! Public synthetic detached-occurrence QA fixture, validated and saved by real APIs.
//! This is fixture construction, not an authored engineering artifact or a UI claim.
use mbse_model::{native_views::{NativeBinding, Views}, native_views_io::{self, Io, Kind}, stage1_io::{self, Session}};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 { return Err("native input, views input and fresh output root required".into()); }
    let root = PathBuf::from(&args[2]);
    if root != PathBuf::from("/workspace/build/builderbob/mbse-craft/milestone1/p06-fixture") || root.exists() || root.is_symlink() {
        return Err("Only the fresh scoped fixture root is allowed".into());
    }
    std::fs::create_dir(&root)?;
    if root.canonicalize()? != root { return Err("Noncanonical fixture root".into()); }
    let mut session = Session::open(&root.join("state"), &PathBuf::from(&args[0]))?;
    let write = session.begin_save(&root.join("occurrence.kernspan"))?;
    if !session.complete(stage1_io::execute_write(write)) { return Err("Native fixture Save not acknowledged".into()); }
    let checkpoint = session.native_checkpoint()?.ok_or("Native checkpoint missing")?;
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    value["binding"] = serde_json::to_value(NativeBinding::from_checkpoint(&checkpoint))?;
    let mut detached = value["occurrences"]["10"].clone();
    if detached["record"] != "1002" || detached["view"] != "2" || value["occurrences"]["12"] != serde_json::Value::Null {
        return Err("Unexpected public input fixture identity".into());
    }
    detached["id"] = "12".into();
    detached["expected_kind"] = "Mission".into();
    detached["visible"] = true.into();
    detached["presence"] = serde_json::json!({"state":"Detached","reason":"Synthetic prior-kind row for read-only occurrence navigation QA"});
    value["occurrences"]["12"] = detached;
    value["next_id"] = "13".into();
    let views = Views::decode(&serde_json::to_vec(&value)?, &session.editor, &checkpoint)?;
    let mut io = Io::default();
    let job = io.begin(&session, &views, &root.join("occurrence.mbse-views.json"), Kind::Bound)?;
    if !io.complete(&session, &views, native_views_io::execute(job)) { return Err("Views fixture Save not acknowledged".into()); }
    io.verify_preserved(&session, &views, Kind::Bound)?;
    let receipt = serde_json::json!({"schema":"mbse-craft.m1-occurrence-fixture/1","result":"ValidatedAndSavedThroughActualNativeAndViewsProtocols","source_native":args[0],"source_views":args[1],"native_artifact_sha256":checkpoint.artifact_sha256(),"snapshot_sha256":checkpoint.snapshot_sha256(),"active_primary":"7","hidden_review":"10","detached_review":"12","target_native":"1002","detached_expected_kind":"Mission","actual_native_kind":"Objective","qualification":"Public synthetic QA fixture; no engineering or blank-author claim"});
    std::fs::write(root.join("RECEIPT.json"), serde_json::to_vec_pretty(&receipt)?)?;
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
