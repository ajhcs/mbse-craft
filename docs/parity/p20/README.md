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

Exact702-file source
`e867f19811518d02f8b9359bc0b5336f1fb01f21311f753013785f68f224b613`
passed **465 distinct workspace tests**, including **eight new P20 cases**, and
desktop build. The same binary passed two native app runs with zero action errors
and normal app/display cleanup. Binary SHA-256:
`202a0149e1842aa365d7892e41406220a0e4b52b828c68cbff077d275764a1f3`.

[Contract](CONTRACT.md), [job](JOB.json), [manifest](SOURCE_MANIFEST.json),
[test pairs](verification/TEST_RESULTS.json), [compiler receipt](verification/COMPILER_RECEIPT.json),
[native receipt](verification/native/RECEIPT.json),
[same-agent review](verification/NATIVE_REVIEW.json) and
[45 hash-indexed artifacts](verification/ARCHIVES.json) retain exact evidence.
This P20 compiler/native job had no failed attempts; prior slices' failures remain
unchanged. No passed job was replayed.

This is a bounded same-agent Linux typed definition/member clone and usage/
enclosed-connector move workflow. Clone covers accepted members and preserves
source unfinished forms without duplicating them onto new identities. Existing
presentation stays exact and is not implicitly cloned or retargeted. Native-core
record cloning/moving, exhaustive opaque-reference meaning, full SysML/NoMagic/
Craft/platform and independent engineering acceptance remain outside this slice.
Clone128-member/move64-usage limits and existing byte/count budgets apply. All56
broad parity specifications stay NOT_RUN; M04 exit remains open.
