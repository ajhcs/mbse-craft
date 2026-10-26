# P17b: typed port occurrences and cross-view reuse

Built on merged P17a PR #20 (`c6b36918062bfb894df27b88ae9a32357d539bb5`)
in isolated `codex/m4-p17b-port-occurrences`. One semantic port can now appear in
two native views as separate port cards with independent geometry, visibility and
pin state. Port occurrences have their own admitted presentation records and
explicit views/3 bound/recovery profiles; they are never Stage1 record kinds.

| Acceptance | Concrete evidence |
| --- | --- |
| Cross-view reuse preserves port identity | Actual semantic Port1016 is reused as occurrences9/10 in views1/2. Native snapshots and all accepted interface contracts remain unchanged by presentation operations |
| Move/hide/remove preserve semantics | Final native pointer drag moves occurrence10 from45,160 to85,180; occurrence9 remains exact. Pin prevents moving; unpin, move-right20, Hide/Layout Undo and Remove/Layout Undo preserve the same IDs and model hash |
| Camera and gestures include typed ports | Actual first drag after closing editor survives frame reconciliation; camera blocks during gesture. Native Fit affects only the active view. Controller regressions cover distant/hidden typed geometry and typed gesture ownership without allowing core relationship endpoints to become ports |
| Model Undo/Redo preserves raw work | Initial immutable native captures show Port1016 removal detaching both occurrences, then same-runtime Redo restoring both while retaining exact newer assignment text unapplied. Final source API tests also cover this proof and detached cold restoration without runtime authority |
| Actual captured Save/cold reopen | Final native/views pair writes profile views/3 and cold app restores exact two-view IDs/geometry, original core occurrences, complete native snapshot and every port/interface/property/usage/definition journal. Reopened layout history is empty |
| Wrong/stale/malformed input refuses | Tests cover wrong port kind/target, missing view, pin/stale stamp, core/typed ID collision, noncanonical map IDs, old profile with typed content and combined per-view count limits |
| Copy/recovery retains meaning | Existing native-bound and explicit unbound-recovery paths preserve typed records. API source inspection/copy into populated typed target refuses explicitly, retaining source bytes and target state. Legacy wire values do not gain empty typed fields |

The final690-file closure
`86979cebf62dd472dc38cc5a342123a8606690699e25751418e8e4e87a0f6803`
passed **432 distinct workspace tests**, including **ten new P17b cases**, and
desktop build exit0 at checkout `dc1fd59`. Binary SHA-256:
`cf7319aceb4a3a0dbcf2d1b077e04a4aef959d3db3cbdf136c7ba0a2c2305571`.
[Compiler receipt](verification/COMPILER_RECEIPT.json), [test pairs](verification/TEST_RESULTS.json),
[manifest](SOURCE_MANIFEST.json), [job](JOB.json) and [contract](CONTRACT.md) record
exact source/build boundaries. Runtime source matches this passing job.
Dependencies/vendor are unchanged.

Two final private Linux app/display runs have zero action errors and verify actual
pointer/layout operations, captured native/views Save and cold app reopen.
[Raw receipt](verification/native/RECEIPT.json) retains its original
`actions_completed_review_pending`; [same-agent pixel/byte review](verification/NATIVE_REVIEW.json)
is separate. The exact drag midpoint/geometry/model-hash observations are in the
receipt. All31 final original/stored/lossless artifact hashes are in
[ARCHIVES.json](verification/ARCHIVES.json). Original native/views inputs are exact;
normal app/driver exits and owned-process/authentication cleanup are verified.

The evidence stages are deliberately separate:

- `failed-session-fixture` retains317 passed tests/one failure: the fixture changed the Session editor instance and the native checkpoint guard correctly refused. The fixture now retains the real Session instance; the guard was not weakened.
- `before-capture-diagnostic` retains the first431-test/build pass. The first native attempt in `failed-native-drag-capture` proved authoring, cross-view reuse and model detach/Redo, but failed pointer drag and captured views Save. It also retains a fixture suffix refusal and a helper tagged-Presence assertion correction. The native model saved; views did not. FINISH then correctly refused the still-open app; interrupted owned cleanup is verified without a normal-exit claim. Its original raw status remains active.
- `before-drag-reconcile-fix` retains another431-test pass, including the actual captured-write regression. `Capture.document` had selected views/2; it now retains views/3 for both bound and recovery captures.
- `pointer-diagnostic` records actual response delivery and a successful captured views/3 pair. The diagnostic recovered layout was derived from a document reconstruction whose digest exactly equals the recorded live document hash, then explicitly cleared historical binding and selected unbound recovery. No checkpoint authority was inferred. One missing recovery-widget audit ID was corrected using its actual adjacent UI button. Normal cleanup is recorded. Per-step printed pointer probes were tool-observed; no raw per-step probe log is claimed.
- `ViewWorkspace.reconcile` had cleared drag owners absent from the core occurrence map. The fix admits live visible unpinned typed-port gesture owners while core routing remains core-only. The final432-test source and final native remainder verify the correction. Passed authoring/model-history checks were not replayed or relabeled as a fresh whole-run PASS.

No test/run counts are summed. Every failed/partial receipt and original capture
is retained. The final native run starts from the diagnostic's actual saved bound
pair; its two-view persistence and layout behavior are independently checked again
on the final binary. [Preservation](verification/PRESERVATION_AFTER.json) verifies31
available pre-existing checkouts unchanged. Eighteen user-removed historical
paths retain registered branch/commit heads; their absent contents cannot be
reverified. No recreation, pruning or reset was performed.

This is bounded same-agent Linux port-card behavior. Nested port-on-owner glyphs,
connectors/conveyed items, configured instances, independent engineering and full
NoMagic/Craft/platform/domain acceptance remain open. Typed copy/source inspection
is explicitly unsupported rather than silently dropping content. Milestone4 exit
remains open; all56 broad parity specifications remain NOT_RUN. All output stays
on Samsung /workspace under fresh preflights, exclusive target locking and one
Cargo job/test thread; no service/provider/release/signing or shared-skill mutation.
