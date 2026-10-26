# P15: typed definitions, properties and durable property forms

Built on merged P14 PR #17 (`46d18c4509bd1dce8c51bd1f41ab55638c889e96`)
in isolated `codex/m4-p15-types-properties`. Tools / Types and properties
(Ctrl/Cmd+Alt+T) authors Component definitions, Boolean/Integer/Real/Text value
definitions and same-System typed properties on a migrated native copy.
Required namespaces have explicit profile/version/shape admission; optional opaque
entries using the same names refuse editing rather than being silently promoted.

| Acceptance | Concrete evidence |
| --- | --- |
| Wrong-kind references refuse atomically | Native component1004 used as a property's value type shows TYPE_REFERENCE_WRONG_KIND and disabled Apply; accepted property1006 still references Real value1005, with model revision unchanged. API cases also reject a value definition as owner |
| Invalid multiplicities refuse atomically | Native lower2/upper1 shows lower-exceeds-upper and leaves accepted1..1 untouched. API checks reject invalid/incomplete/negative/over-range/noncanonical bounds and admit0..* |
| Pending property text survives Save/reopen | Native raw name `pending length text`, upper `1..` and exact type/owner IDs cold-reopen alongside accepted property `length`,1..1. Original Stage1 records, raw9e, creation journal, document ID,95metre and2.5km remain exact |
| Undo/Redo retains newer drafts | Native Model Undo removes the accepted property while retaining the newer invalid form; Redo restores the same accepted IDs/data while preserving that form. API tests undo all definitions, retain detached journals and keep allocator monotonic |
| Identity and history stay safe | Definitions/properties share the native identity allocator, with core/nested/retained journal collision and noncanonical/future schema refusal. Repeated Apply and identical raw form are no-ops. Actual renderer keeps field ID/focus and raw text through row insertion; IDs include document instance/property/field |
| Existing work is preserved within user cleanup | The25 remaining pre-existing checkouts match original heads/status. During this run the user confirmed removing18 historical worktree directories; their registered branch/commit heads remain exact, while current directory status cannot be reverified |

The final683-file source closure
`7507442f0023c7311fe49459d5c89367829cdfd11869836572018b81f4d7c1a9`
passed **405 distinct workspace tests**, including **eight new P15 cases**, and
desktop build exit0. Runtime/source correction `5cb091b`, executed checkout
`ab0d9f7`; full IDs are in the [compiler receipt](verification/COMPILER_RECEIPT.json).
Binary SHA-256:
`8a0b991d8e8f732684154203f963860bcb83088aeff4090dc55ad706c4882715`.
See [manifest](SOURCE_MANIFEST.json), [job](JOB.json),
[test pairs](verification/TEST_RESULTS.json) and [contract](CONTRACT.md).
No runtime source changed after this passing job. Dependencies/vendor are unchanged.

Two private Linux app/display runs exited normally with zero action errors,
exact owned-process cleanup and absent authentication files. They used real menu/
keyboard, field and button input, actual native/views Save and a cold native app.
The [raw receipt](verification/native/RECEIPT.json) retains its original
`actions_completed_review_pending`; [same-agent pixel/byte review](verification/NATIVE_REVIEW.json)
is separate. All40 original/stored/lossless artifact hashes are indexed in
[ARCHIVES.json](verification/ARCHIVES.json). Original fixture bytes are unchanged.

The initial compile refusal (zero tests) and two129-pass/one-failed renderer
attempts remain in their original failed-* directories. The first renderer
clipping diagnosis was incomplete: a focused actual AreaState trace proved that
the TextEdit becomes live on the third normal startup frame. The final test uses
three startup frames with the same live/focus assertions. The larger initial
window height provides form space; it is not claimed as the cause of the focused
pass. The focused diagnostic passed one test/110 filtered; its nocapture output
interleaves the test-name line, so the generic receipt's line regex counted zero.
See [renderer diagnosis](RENDERER_DIAGNOSIS.md) and
[diagnostic raw receipt/logs](verification/diagnostic/RECEIPT.json). Those counts
are separate from the final405 and are not summed.

[Preservation reconciliation](PRESERVATION_AFTER.json) records25 unchanged
checkouts and18 user-removed directories. The user explicitly confirmed the
removals while the audit ran; current working files in those absent directories
cannot be verified. Their registered/branch heads and commit objects remain
retained. This task did not recreate/prune them. The first packaging audit stopped
on the missing path before any all-unchanged claim was published.

This is app-owned structural typing, with four primitive value definitions and
same-System properties. Property assignments/defaults/derived results, inheritance,
parts/ports/connectors and typed diagram presentation remain later slices. New
property forms are durable journals; an unapplied definition-name UI buffer is
session-only. P14 explicit copy migration is required before authoring types.
Independent, full-platform/accessibility, domain and full NoMagic/Craft acceptance
are not asserted. Milestone4 exit remains open; all56 broad specifications remain
NOT_RUN. Outputs stayed on Samsung /workspace under fresh preflight, target lock
and one Cargo job/thread; no provider/service/release or shared-skill mutation.
