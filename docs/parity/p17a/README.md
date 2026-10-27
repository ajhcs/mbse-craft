# P17a: interface contracts and durable port authoring

First child of P17, based on merged P16 PR #19
(`4370e879411479aa79b3d5aabbba5f2ff80d47ce`) in isolated
`codex/m4-p17a-ports-interfaces`. Tools / Ports and interfaces (Ctrl/Cmd+Alt+I)
authors one-payload interfaces and Component-owned ports. Assignment requires the
same exact Value-definition identity, flow direction and System. Different
interface IDs may share that explicit contract. P17b separately adds typed port
occurrences and cross-view reuse; **P17 remains incomplete**.

| Acceptance | Exact evidence |
| --- | --- |
| Compatible assignment | Native In interfaces1012/1013 both use Value1005; Port1015 owner1007 declares1012 and assigns distinct compatible1013 |
| Incompatible assignment refuses atomically | Native Out1014 and Component1004 as assignment show refusal/disabled Apply with accepted store/model revision unchanged. API cases cover different exact payload identities, missing targets, wrong owner/payload kinds and cross-System refusal |
| Accepted data and raw forms stay separate | Corrective native run immediately asserts exact delivered assignment1013?, interface payload1005? and usage target incomplete before history or Save. Pixels show red refusals/disabled Apply beside accepted contracts/usages |
| Model history retains newer invalid forms | Corrective run performs real Part1009 owner move and Port1015 rename, then Undo/Redo, preserving all three exact invalid forms and semantic IDs. API cases also cover no-op Apply and monotonic allocation |
| Native Save/cold reopen retains exact invalid text | Saved journal equals exact expected forms; actual cold app retains all three invalid forms beside accepted ports/interfaces/parts and prior P15 property draft. Core records/document ID/raw9e/creation journal/95metre/2.5km stay exact; paired views hash matches and reopened view history is empty |
| Real field focus remains stable | Actual egui renderer keeps port assignment field live/focused with unfinished text through interface-row insertion. Port editor scopes history shortcuts and blocks file commands until closed |
| P16 file compatibility | Existing native P16 store/journal serializes to equal JSON without empty new port/interface fields. No dependency/vendor changes |

The687-file semantic source
`2b017a65ef5e2bb4f3e39a0a7026184732bcfbef7b7360736d527b6ca6681982`
passed **422 distinct workspace tests**, including **eight new P17a cases**, and
desktop build exit0 at `ae2d819`. Semantic binary:
`2aea49ae4c98925c0a43d34a4f18b748de9123491e87c5dc365a4855dd5843a7`.
[Compiler receipt](verification/COMPILER_RECEIPT.json), [test pairs](verification/TEST_RESULTS.json),
[frozen source/job](before-wording-review/SOURCE_MANIFEST.json) and [contract](CONTRACT.md)
record boundaries. Original two native runs verify actual authoring and incompatible
assignment refusal. Their KeyError from assuming an empty ports key survives Undo
is retained and corrected; serialized empty fields are intentionally omitted.

**Input fidelity correction:** artifact review found that the old driver dropped
trailing question marks. Earlier native invalid-draft claims for P16 and the first
P17a native run were unsupported: actual audits/saved bytes contain1007/1013/1005.
[Erratum](INPUT_FIDELITY_ERRATUM.md) and [corrective review](exact-draft-verification/REVIEW.json)
withdraw those specific claims without changing raw receipts/events/pixels. The
new driver prevalidates printable ASCII and uses named shifted punctuation; four
[helper contract tests](exact-draft-verification/INPUT_CONTRACT_TESTS.json) pass.
Two fresh corrective app/display runs assert exact delivered strings before
history/persistence, then verify native Save and cold reopen. They have zero action
errors, normal exits, unchanged source inputs and exact owned-process/auth cleanup.
The [corrective raw receipt](exact-draft-verification/native/RECEIPT.json) retains
`actions_completed_review_pending`; same-agent pixel/byte review is separate.
All28 corrective artifact original/stored hashes are in
[ARCHIVES.json](exact-draft-verification/ARCHIVES.json). P16 correction is descendant
regression evidence, not a retroactive execution on the original P16 binary.

Final runtime closure:
`1b71881d2890bdbec4de39059fd549b3adb936be5d7b4eefae0a5d52adac60fd`;
final binary:
`cdf9a94e0eaf6a148687896bba875a6f5699c57cccb686cde0315fbc57463920`.
Only one display sentence differs from the422-test semantic source: the product
states current placement availability without an internal milestone identifier.
Model/test files are byte-exact. Changed desktop build exit0 and one targeted
wording native run pass, with unchanged model/input hashes and normal cleanup.
[Wording build/review](final-wording-verification/REVIEW.json) and11 artifact hashes
are separate from original33 functional artifacts and28 corrective artifacts.
No counts are summed, and no full-suite rerun is claimed for the display string.
The corrective invalid-draft runs use this final runtime binary and repaired driver.

[Final preservation](exact-draft-verification/PRESERVATION_AFTER.json) verifies29
available pre-existing checkouts unchanged. Eighteen user-removed historical dirs
remain absent with registered branch/commit heads retained; their contents cannot
currently be verified. No recreation/pruning/reset was performed. All output stayed
on Samsung /workspace under fresh local checks, exclusive target lock and one
Cargo job/test thread. No provider/service/release/signing or external skill edits.

This is app-owned one-payload interface/port semantics with exact payload/direction/
System assignment rules. No full SysML/NoMagic/Craft/platform/domain/independent
acceptance, connectors or configured instances are asserted. Port presentation and
cross-view reuse remain P17b; Milestone4 exit stays open and all56 broad parity
specifications remain NOT_RUN.
