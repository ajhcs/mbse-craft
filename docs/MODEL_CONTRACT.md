# Model and persistence contract

The `mbse-model` crate reuses the bounded MIT Kernspan Stage1 model from clean
commit `c80037d2259cd67a8e813cf3bc6c01bb1226a460`. Exact copied-file hashes and
notices are retained in `licenses/kernspan`. Seven source modules are reused;
`stage1_io` is adapted to call pinned upstream `craft-atomic` for native saves
and recovery. This is an actual persistence dependency, not file-format or
whole-product ArtCraft compatibility.

## Shared Rust interface

`stage1::{Snapshot, Editor, Record, RecordId, RecordKind, RawKey, Command,
SubjectRef}` owns stable model identity. Use `Editor::command(&freshness,
command)` for atomic validated edits; take `editor.freshness()` immediately
before the command. Invalid/incomplete raw input remains separate from the last
accepted value. `display_text` exposes retained text. Model and focused-text
Undo/Redo are separate operations. Model Undo does not erase detached journals.

System `purpose` describes why the System exists. Mission name is neutral
intended accomplishment, with optional short label. Capability `contributesTo`
references unique Missions of the same System. Objective `subject` is explicitly
System, Mission or Capability of the same System. Measure `forObjective`
references an actual Objective. A custom Measure is not automatically MOE/MOP;
any additional classification remains opaque metadata, not normative adoption.
Stakeholders, Expectations, Actors, Organizations and Environments keep typed
records and system scope. Unknown record fields remain opaque JSON.

`stage1_graph::derive_graph` projects authoritative relation endpoints from
records. Geometry and proximity never create semantic relationships.

`stage1_calc` evaluates bounded typed expressions and dependency graphs with
explicit unit conversion. Scalar, Ratio, Count, Length, Duration and Speed are
supported. Domain, divide-by-zero, cycle, missing-reference, unit/kind and
resource failures remain explicit. Results are derived evidence, separate from
criteria, observations and engineering acceptance.

## Disk contract

`stage1_io::Session` owns `editor`, `path` and state directory. `begin_save`,
`begin_recovery`, `execute_write` and `complete` bind a frozen accepted/raw snapshot
to document, instance, request, revision and hash. A stale write cannot acknowledge
later edits. Recovery remains an independent dirty document and never marks the
native destination saved. Fresh copies use upstream `WriteOptions::write_new`;
known destinations use `atomic_write` with private staging. The application keeps
cooperative locks, expected source hashes, retained prior versions and strict
parent-directory sync. Noncooperating writers can race final replacement; no
filesystem compare-and-swap or power-loss qualification is claimed.

Native storage retains `kernspan.stage1/2`, schema 2 for explicit interoperability
with this bounded baseline; do not rename the profile while claiming unchanged
interchange. `stage1_sysml` accepts only the custom `kernspan.stage1.sysml/1`
ontology/profile and preserves source bytes, spans and hashes. General SysML,
normative classifier adoption and conformance are outside its scope.

The vendored Apache-2.0 `sysmlv2-syntax` source is pinned at
`821221767c3c56cb1ebe7da22666197a47c9c645`. Its source is unchanged; only Cargo
workspace inheritance is replaced with standalone metadata. It has no build
script and no default dependencies. No CSE verifier or hosted service is invoked.

## Executable acceptance

Run `cargo test -p mbse-model --offline` with workspace-local target and temporary
directories and one Cargo job after dependency/build-script review. Retained
module tests cover typed reference refusals, atomic edit rollback, raw drafts,
model/text histories, calculation failures, fresh writes, stale receipts, disk
conflicts and independent recovery. GUI acceptance remains separate.
