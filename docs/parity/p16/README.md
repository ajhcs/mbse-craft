# P16: parts, references and identity-preserving decomposition moves

Built on merged P15 PR #18 (`19cb6516350d180fa3f693034c9acc9dbd2b1821`)
in isolated `codex/m4-p16-parts-references`. Tools / Types and properties now
creates Component-owned Part and Reference usages. Parts form an acyclic type
decomposition; References reuse types without implying containment. The browser
labels System membership separately from owned Part and Reference rows.

| Acceptance | Exact evidence |
| --- | --- |
| Containment cycles refuse | API direct/indirect cycle cases refuse atomically. Native converting reverse Reference1011 to Part displays TYPE_CONTAINMENT_CYCLE/disabled Apply while accepted usages and model revision stay unchanged |
| Reuse does not duplicate type identity | Actual native Parts1009/1010 have distinct usage IDs and both target Device1007. Reference1011 points from Device1007 back to Subsystem1004 without a containment cycle |
| Moves preserve identity | Native owner change moves Part1009 from Component1004 to1008 with same ID and target1007. Undo/Redo restore ownership; original intended trailing-question-mark input was not delivered. Exact invalid target incomplete is verified separately in the corrective descendant run below |
| Wrong targets refuse | Native primitive value1005 as usage target refuses without accepted mutation. API covers wrong owner/target kinds and cross-System refusal |
| Invalid drafts persist | Native Save/cold app retains the usage1009 form, existing P15 pending property text/upper1.. and accepted property1006. Original invalid usage-text claim is withdrawn; corrective exact native evidence is below. Original core records/document ID/raw9e/creation journals/95metre/2.5km remain exact |
| Browser distinguishes membership/containment | Actual scrolled browser pixels show member of System1, Part containment and Reference not containment. A third run completed the lower-row visual check clipped in the first capture; it did not repeat authoring checks |
| P15 data compatibility | Regression parses actual P15 native fixture and serializes type store/journal to equal JSON without inserting empty P16 fields. No dependency/vendor change |

The684-file closure
`fc0bc968cd4df7e19f4bef2fae59edfecb0bb018645ac79879201481e25ed434`
passed **414 distinct workspace tests**, including **seven new P16 cases**, and
desktop build exit0 at checkout `2d2f786`. Binary SHA-256:
`4e9e71efa56f86a92995b9cf975a25d974c5b1c47160781977132eac6c12d0bd`.
[Compiler receipt](verification/COMPILER_RECEIPT.json), [test pairs](verification/TEST_RESULTS.json),
[behavior source](before-label-review/SOURCE_MANIFEST.json), [behavior job](before-label-review/JOB.json) and [contract](CONTRACT.md)
record exact source/execution/input boundaries. The final review below changes
only two display strings; model/test files remain exact.

Three private Linux app/display runs had zero action errors, normal app/driver
exits, exact owned-process cleanup and absent authentication files. Native/views
Save and cold reopen use actual file bytes and checked views hash; reopened view
history is empty. The [raw native receipt](verification/native/RECEIPT.json) retains
`actions_completed_review_pending`; [same-agent pixel/byte review](verification/NATIVE_REVIEW.json)
is separate. All44 original/stored/lossless artifact hashes are indexed in
[ARCHIVES.json](verification/ARCHIVES.json). Fixture bytes remain exact.

The first413-test/build pass remains separately in `before-form-review`. Review
split pending property and usage edits into independent slots, preventing same-frame
replacement by another form action. It also replaced repeated reachability scans
with topological cycle admission and added P15 serialization compatibility coverage.
No test counts are summed. There were no P16 compile/test failures.

[Preservation](verification/PRESERVATION_AFTER.json) verifies27 pre-existing
available checkouts unchanged. The18 user-removed historical directories still
have their registered branch/commit heads retained; their contents cannot currently
be verified. No recreation, pruning, reset or cleanup was performed.

This is bounded same-agent Linux structural evidence. Part usages are definitions
owned by a Component, with same-System Component targets; configured instance
values, ports, exchanges, inherited features and typed occurrence diagrams remain
later work. One current usage form is retained; explicit New/Edit replaces it.
Existing per-property and definition journals remain independent. The PR19 review replaced two unsupported arrow separators with plain
uses type wording, verified in actual browser/editor pixels. Independent/platform/font/domain/full NoMagic/Craft acceptance and the
Milestone4 exit remain open; all56 broad parity specifications remain NOT_RUN.


PR19 review ran in isolated `codex/review-p16` and corrected the missing arrow
separator glyph with supported words. Final684-file source closure:
`c93573db4ea8a2c519462565505961892b0ec1258277d70ac91046658bc817e4`;
final binary:
`7c03dba24fdaffbf72ee7da0aa7701330911c3876b37a3669cfb45f6e8ff340a`.
The changed desktop built exit0 at `43681f8`. One final native display run verified
both actual label surfaces, exact unchanged model hash and input bytes, zero action
errors and normal process/authentication cleanup. [Review/patch](final-label-verification/REVIEW.json),
[build receipt](final-label-verification/BUILD_RECEIPT.json), [raw native receipt](final-label-verification/native/RECEIPT.json)
and13 original/stored artifact hashes are separate from the414-test/three-run
functional acceptance. No new test-suite execution is claimed on the label-only
source. [Final preservation](final-label-verification/PRESERVATION_AFTER.json)
verifies28 available prior checkouts unchanged, including the original P16 candidate;
the18 user-removed historical directories remain absent with commit heads retained.


Input fidelity erratum recorded during P17a review: the old native driver omitted
trailing ? and the helper did not assert requested text immediately. Thus the
original1007? native invalid usage-form claim was unsupported; actual saved/audit
text is1007. Raw receipts/events/pixels remain exact. The original review is
preserved in before-input-fidelity-review and annotated with this qualification.
[Correction](../p17a/INPUT_FIDELITY_ERRATUM.md) repairs the driver; [fresh exact
native acceptance](../p17a/exact-draft-verification/REVIEW.json) on the P17a reviewed
descendant binary verifies immediate incomplete target input, real move Undo/Redo,
Save and cold reopen, preserving accepted IDs and prior journals. It does not claim
the original P16 binary received that missing input. Existing API/native-IO invalid
text tests, actual cycle/wrong-kind refusals and move identity checks remain valid.
