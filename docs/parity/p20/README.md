# P20: explicit clone/move with reference previews

Built on P19 PR24 merge3c628b2 in isolated `codex/m4-p20-clone-move`.
Tools > Clone and move with reference preview (Ctrl+Alt+B) now stages accepted
model changes with explicit shared-target treatment, fresh-ID mapping and a
sealed runtime-only candidate. Source raw work and presentation stay separate.

| Acceptance | Concrete evidence |
| --- | --- |
| Preview identifies reference treatment | Native clone preview maps Subsystem1004/property1006/Parts1009,1010/connectors1018,1019 to fresh1021..1026. It lists remapped owned references, retained shared targets and incomingReference1011 retaining originalSubsystem1004. Outside-scope native fields are disclosed without an exhaustive global verdict |
| Cancel and stale previews preserve source | Native Prepare/Cancel leave full snapshot hash and allocator unchanged. A newer clone-name edit makes old Apply unavailable and leaves accepted data unchanged. API tests additionally refuse foreign editor instances and replayed/stale plans |
| Clone allocates fresh IDs | Actual clone creates definition1021, property1022, Parts1023/1024 and connectors1025/1026. Internal Part/connector references remap; Device1007, ports/interfaces/Value1005 remain shared by explicit choice. Copied Part override23 remains explicit |
| Move keeps IDs and reference consistency | Moving onlyPart1009 toowner1008 blocks on boundary connectors1018/1019. Moving1009 and1010 together atomically updates both usage and connector owners, keeps all semantic IDs and preserves original qualified routes/explicit values and cloned fragment |
| Undo/Redo retains work | Native clone and move Undo/Redo restore exact accepted identities while keeping newer Future clone name raw text. API verifies monotonic allocation, cycles, policy/selection/member-budget refusals and source default/override retention |
| Durable native/views workflow | Actual captured native/views4 Save, cold app and cold second Save reproduce the complete snapshot with next_id1027. All prior raw forms, native opaque fields,9e draft,2.5km calculation and source occurrence/port/route maps remain exact. Cold preview authority is absent |
| Existing work preserved | All39 available prior checkouts retain exact heads/status. Eighteen user-removed directories retain registered heads; removed contents remain unverifiable. Original native/views fixture bytes unchanged |

Original702-file source
`e867f19811518d02f8b9359bc0b5336f1fb01f21311f753013785f68f224b613`
passed **465 distinct workspace tests**, including **eight new P20 cases**, and
desktop build. That original binary passed two native app runs with zero action errors
and normal app/display cleanup. Binary SHA-256:
`202a0149e1842aa365d7892e41406220a0e4b52b828c68cbff077d275764a1f3`.

[Contract](CONTRACT.md), [job](JOB.json), [manifest](SOURCE_MANIFEST.json),
[test pairs](verification/TEST_RESULTS.json), [compiler receipt](verification/COMPILER_RECEIPT.json),
[native receipt](verification/native/RECEIPT.json),
[same-agent review](verification/NATIVE_REVIEW.json) and
[45 hash-indexed artifacts](verification/ARCHIVES.json) retain exact evidence.
The original compiler/native job had no failed attempts. Isolated PR25 review
found that the move inventory labeled retained references between moved Parts and
connectors as external; actual accepted model/candidate publication was correct.
That inventory claim is corrected by separate internal-retained rows. Original
receipts remain unchanged; see [review followup](REVIEW_FOLLOWUP.md).

Current reviewed source
`b353f555ffd97faf90ec11b24ff13a6c0f5d136928d634a117fccc5b148c15f8`
passed **seven model regressions, one renderer test and build**. A separate native
preview/Cancel/Save run has zero action errors and verifies four internal Part
references with external ports/types kept separate. Accepted clone/move state and
all original views remain exact. Final binary:
`f1e10a7ea7b64ef49d310cb140f3208843d23fc723a884e2ad4f2e5409f71679`.
[Review evidence](reference-review-verification/REVIEW.json) and
[19 additional indexed artifacts](reference-review-verification/ARCHIVES.json)
qualify this inventory-only correction. Only the summary/classification, its
regression assertions and UI rows changed; candidate/publication logic is unchanged.
The465 full-suite and original two-run native workflow remain separate from final
focused qualification; counts are not summed and no full suite/native state
workflow was replayed. All40 available prior checkouts, including original
candidate, are unchanged.

This is a bounded same-agent Linux typed definition/member clone and usage/
enclosed-connector move workflow. Clone covers accepted members and preserves
source unfinished forms without duplicating them onto new identities. Existing
presentation stays exact and is not implicitly cloned or retargeted. Native-core
record cloning/moving, exhaustive opaque-reference meaning, full SysML/NoMagic/
Craft/platform and independent engineering acceptance remain outside this slice.
Clone128-member/move64-usage limits and existing byte/count budgets apply. All56
broad parity specifications stay NOT_RUN; M04 exit remains open.
