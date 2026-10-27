# P18a: Part-qualified connectors and conveyed types

Built on P17b merge0694708 in isolated `codex/m4-p18a-connectors`.
Component definitions now own directed connectors whose endpoints each name an
exact Part usage and a Port on its target type. Two Parts using one Device type
remain distinct endpoint contexts. Tools > Connectors and conveyed items opens
with Ctrl+Alt+K on the qualified Linux path.

| Acceptance | Evidence |
| --- | --- |
| Exact endpoints and conveyed type | Native connector1018 owned by Component1004 connects Part1009/Port1017 to Part1010/Port1015 and conveys Value1005. Both Parts use Device1007; target uses its compatible assigned Interface1013, source declared Out Interface1014 |
| Invalid edits refuse atomically | Native reference usage1011, input port1015 as source and Component1007 as conveyed type disable Apply and retain accepted connector/revision. Exact malformed1005? text is asserted immediately. API tests additionally cover wrong owner/type, same endpoint, noncanonical IDs and distinct Real-value identity |
| History retains newer work | Native model Undo removes connector1018 while retaining newer raw1005?; Redo restores exact accepted connector/endpoints and both port occurrences. API regression covers stale tickets, repeated Apply no-op and monotonic allocation |
| Durable native/views workflow | Actual captured native/views3 Save and cold application reopen retain full native snapshot, every prior definition/property/usage/interface/port form, accepted semantics and original core/typed occurrence maps. A second cold Save equals the first snapshot exactly |
| Existing ownership stays valid | Moving a connected Part out of its owning Component refuses atomically; explicit model Undo of connector allows the move with the same Part ID. No automatic deletion or reference reinterpretation |
| Legacy/preservation | Old optional stores/journals serialize without empty connector fields. All33 available preexisting checkouts retain exact heads/status. User confirmed removing18 historical directories; registered heads retained, removed contents unverifiable |

The693-file semantic/UI source closure
`2eedba64db84bcb8252ce274f74e96cf6bf10a55316f7672c8dabd55b585b599`
passed **440 distinct workspace tests**, including **eight new cases**, and build.
Native testing found Ctrl+Alt+C did not dispatch this workspace command. The final
source `32f26058ffb52aef944db0eac662b0c0935a50978add32ae871f78b560d1f244`
differs only by the default shortcut C to K and passed **nine focused command
catalog/routing tests** and desktop build. Counts are not summed. Final binary:
`aa29b05b4dc538d13d33f4d495d39337bafa0fa41bedbc780d6acbf4cd43a416`.

[Exact jobs](JOB.json), [manifest](SOURCE_MANIFEST.json),
[test pairs](verification/TEST_RESULTS.json), [native receipt](verification/native/RECEIPT.json),
[same-agent review](verification/NATIVE_REVIEW.json) and
[65 hash-indexed artifacts](verification/ARCHIVES.json) retain the evidence.
The final native session has two normal app/display exits and zero action errors.
The initial native run retains its shortcut assertion failure and normal cleanup;
its actual saved output-port checkpoint feeds only the unexecuted remainder.
Two earlier zero-test compiler refusals remain under `failed-initial-compile` and
`failed-test-compile`. No failure receipt was relabeled or erased.

[Contract](CONTRACT.md): immediate owned Parts, exact payload identity and directed
Out/InOut to In/InOut exchange only. Reference endpoints, root delegation, unit
conversion, execution and full SysML equivalence remain outside this slice.
P18b qualified port occurrences and visual connector routing are still pending;
P18 remains incomplete. Same-agent Linux checks do not establish independent
engineering, platform, full NoMagic or Craft UI acceptance. All56 broad parity
acceptance specifications remain NOT_RUN.
