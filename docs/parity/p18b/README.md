# P18b: qualified port occurrences and connector routes

Built on P18a PR22 merge8fec6d8 in isolated `codex/m4-p18b-connector-routes`.
Each port card can now name an exact Part usage, and connector routes bind exact
qualified port occurrences. Native views/4 and recovery/4 retain these contexts
without reinterpreting core record kinds or existing unqualified port cards.

| Acceptance | Concrete evidence |
| --- | --- |
| Same connector in two views | Native connector1018 routes from Part1009/Port1017 to Part1010/Port1015 as edge13 between occurrences11/12 in view1 and edge16 between14/15 in view2. Existing unqualified9/10 remain exact |
| Wrong contexts refuse | API tests reject reference usage, wrong port type, unqualified, reversed, cross-view, duplicate and stale routes atomically; UI offers only exact matching qualified pairs |
| Presentation removal preserves meaning | Native Remove route17 leaves semantic connector1019. Layout Undo restores17. Remove endpoint15 removes dependent16/17 together; Layout Undo restores the same maps. Other-view geometry and model hash remain exact |
| Model history preserves unfinished work | Native Undo of newly authored connector1019 detaches only route17; Redo restores17 with newer exact malformed1005? text intact. API cold recovery never regains runtime reattachment proof |
| Current geometry drives drawing | Real pointer gestures move only endpoint15 across retained checkpoints, finally to495,375. Route IDs, accepted semantics and view1 remain unchanged. Endpoint hide keeps route records; route Hide/Layout Undo also preserves semantics |
| Readable conveyed captions | Actual final screenshots and private probes show complete captions clear of cards, parallel captions and the visible canvas clip in both views and after a new pointer move |
| Actual native/views persistence | Captured native/views4 Save and two-run cold app checks retain exact native snapshot, all forms, core occurrences, qualified contexts and route IDs. Cold second Save equals the complete snapshot. Final renderer Save retains this state with only intended endpoint geometry changed |
| Compatibility/preservation | Old views/3 fixture reserializes exactly. Global presentation IDs, schema/profile/context, count and byte budgets reject malformed inputs. All35 available prior checkouts unchanged;18 user-removed directories retain registered heads |

The696-file schema/model/history/UI closure
`34eccea2ac1af8501753905105232037b9e2b936e84420cd193b405894a9fadb`
passed **448 distinct workspace tests**, including **eight new cases**, and build.
Native screenshot/pointer review then found caption overlap and viewport clipping.
Only `ui.rs` drawing order and `ui_connector_routes.rs` caption placement/probes/
regression changed afterward; all model/schema/history/IO bytes remain identical.
Final source `c4244cacfe42b001d02f48352582df1c164424e099f682be6dd05e335c484177`
passed **two focused renderer tests** and build; the added long/parallel/tight-gap/
clipped-viewport regression is a ninth distinct P18b test. Counts are not summed.
Final binary SHA-256:
`b53ce4b8bb82839570396339a2a929fc089ac0a176a0ef77f4195a204cfa8706`.

[Jobs](JOB.json), [manifest](SOURCE_MANIFEST.json),
[test pairs](verification/TEST_RESULTS.json), [source differences](verification/SOURCE_DIFFERENCES.json),
[same-agent review](verification/NATIVE_REVIEW.json) and
[101 hash-indexed artifacts](verification/ARCHIVES.json) preserve exact evidence.
The initial native run qualifies authoring/history/removal; the two-run captured
cold session qualifies persistence; the final one-run session qualifies viewport
captions/pointer/Save. Each used fresh admission and an actual saved checkpoint.
All owned app/display processes exited normally. The intermediate post-drag caption
assertion remains failed with one action error. Three zero-test compiler refusals,
the original label issue and the later viewport finding remain separate; no raw
receipt was relabeled and no passed whole suite was replayed.

[Contract](CONTRACT.md): immediate Part contexts and straight directed routes only.
UI lists at most96 qualified reuse and128 route/caption choices. Dense scenes with
no free visible caption interval may need card movement or panning; no complete
routing/layout guarantee, bend editing, nested paths, root delegation, execution
or full SysML equivalence is claimed. Same-agent synthetic Linux checks establish
this bounded slice; full NoMagic/Craft/platform/domain and independent engineering
acceptance remain open. All56 broad acceptance specifications stay NOT_RUN.
