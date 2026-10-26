# P15: typed definitions, properties and durable forms

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
| Pending definition and property text survive Save/reopen | Native definition `Unsubmitted quantity`, System1, Integer choice and property raw name `pending length text`, upper `1..` and exact type/owner IDs cold-reopen alongside accepted property `length`,1..1. Original Stage1 records, raw9e, creation journal, document ID,95metre and2.5km remain exact |
| Undo/Redo retains newer drafts | Native Model Undo removes the accepted property while retaining the newer invalid form; Redo restores the same accepted IDs/data while preserving that form. API tests undo all definitions, retain detached journals and keep allocator monotonic |
| Identity and history stay safe | Definitions/properties share the native identity allocator, with core/nested/retained journal collision and noncanonical/future schema refusal. Repeated Apply and identical raw form are no-ops. Actual renderer keeps field ID/focus and raw text through row insertion; IDs include document instance/property/field |
| Existing work is preserved within user cleanup | The25 remaining pre-existing checkouts match original heads/status. During this run the user confirmed removing18 historical worktree directories; their registered branch/commit heads remain exact, while current directory status cannot be reverified |

The reviewed683-file source closure
`e4576fb1b0a1477c602cde2fed9bdd0f7557e580cc4a465d98814154af4b42dd`
passed **407 distinct workspace tests**, including **ten new P15 cases**, and
desktop build exit0. Executed checkout `fc781b0`; full IDs are in the
[final compiler receipt](final-verification/COMPILER_RECEIPT.json).
Binary SHA-256:
`2ea8fe78524596e4e76253fde071695f8c4c5e456693a0233177fb07d3da4de0`.
See [manifest](SOURCE_MANIFEST.json), [job](JOB.json),
[final test pairs](final-verification/TEST_RESULTS.json) and [contract](CONTRACT.md).
Runtime source matches this passing job; dependencies/vendor are unchanged.

Two final private Linux app/display runs exited normally with exact owned-process
cleanup and absent authentication files. Actual keyboard/menu/field/button input
created definitions and a typed property, refused wrong-kind/invalid multiplicity,
retained both journals through model Undo/Redo, saved native/views and cold-reopened
both unfinished forms alongside the accepted property. One harness assertion
incorrectly expected System1000; the unchanged source fixture scopes record1003 to
System1. The correction verified System1 and retained the original error. The
[raw receipt](final-verification/native/RECEIPT.json) keeps its original
`actions_completed_review_pending`; [same-agent pixel/byte review](final-verification/NATIVE_REVIEW.json)
is separate. All33 final original/stored/lossless artifact hashes are indexed in
[ARCHIVES.json](final-verification/ARCHIVES.json). Original input bytes remain exact.

[PR18 review follow-up](REVIEW_FOLLOWUP.md) records the definition-form durability
fix and its zero-test borrow-check refusal/correction. The earlier405-test/eight-new
candidate and two zero-error native runs remain separately in [verification](verification/NATIVE_REVIEW.json),
with frozen source/job in [before-definition-journal](before-definition-journal/README.md).
These counts are separate and are not summed.

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
are separate from the earlier405 and are not summed.

[Preservation reconciliation](PRESERVATION_AFTER.json) records25 unchanged
checkouts and18 user-removed directories. The user explicitly confirmed the
removals while the audit ran; current working files in those absent directories
cannot be verified. Their registered/branch heads and commit objects remain
retained. This task did not recreate/prune them. The final review audit additionally verifies
the original P15 implementation checkout, for26 unchanged available checkouts in
[final preservation](final-verification/PRESERVATION_AFTER.json). The first packaging audit stopped
on the missing path before any all-unchanged claim was published.

This is app-owned structural typing, with four primitive value definitions and
same-System properties. Property assignments/defaults/derived results, inheritance,
parts/ports/connectors and typed diagram presentation remain later slices. Definition and
property forms are durable journals; model history retains newer unfinished work. P14 explicit copy migration is required before authoring types.
Independent, full-platform/accessibility, domain and full NoMagic/Craft acceptance
are not asserted. Milestone4 exit remains open; all56 broad specifications remain
NOT_RUN. Outputs stayed on Samsung /workspace under fresh preflight, target lock
and one Cargo job/thread; no provider/service/release or shared-skill mutation.
