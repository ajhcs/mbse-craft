# P19: authored defaults, Part overrides and read-only derived results

Built on P18 PR23 merge239d21d in isolated `codex/m4-p19-values`.
Tools > Defaults and instance values (Ctrl+Alt+Y) now keeps scalar definition
defaults, explicit Part-instance overrides and read-only results separate.

| Acceptance | Concrete evidence |
| --- | --- |
| Default changes affect inheriting Parts only | Native property1020 on Device1007 gets default12.5 and Part1009 override23. Changing default17 leaves1009 explicitly23 and makes inheritingPart1010 read17 |
| Overrides stay explicit | UI/audit show Default, Override and Unset origins. Clear override returns1009 to17; model Undo/Redo restores the same override while retaining newer value/expression text |
| Results never overwrite authored values | Actual x*2+1 preview47 uses accepted override23 without changing snapshot hash. Newer exact malformed2e+ disables Apply and marks old result outdated. Recompute still uses accepted23, explicitly excluding unfinished text |
| Invalid/context/type edits refuse | Tests cover wrong Part/property context, Reference usages, Boolean/i64/finite Real/Text literals, infinity/overflow/incomplete input, multiplicity changes, stale tickets, canonical maps and large-integer precision refusal |
| Durable recovery and provenance | Actual captured native/views4 Save, cold app and cold second Save retain the entire native snapshot, defaults/overrides/raw forms, prior accepted semantics and all core/qualified-port/connector-route records. Derived receipt is absent on cold app; recomputing47 remains pure |
| Input and ownership preservation | Original native9e draft and2.5km calculation stay exact. All37 available prior checkouts retain heads/status;18 user-removed directories retain registered heads. Original native/views input bytes unchanged |

Exact699-file source
`221998a7de512aaed17a0832a8823c843250470c5352d053554c27607e260d6b`
passed **457 distinct workspace tests**, including **eight new P19 cases**, and
desktop build. The same binary passed two native app runs with zero action errors
and normal app/display cleanup. SHA-256:
`240f0f44d423773dcf010a76713f68859353ee0c28d78de5a3195e9915771166`.

[Contract](CONTRACT.md), [job](JOB.json), [manifest](SOURCE_MANIFEST.json),
[test pairs](verification/TEST_RESULTS.json), [compiler receipt](verification/COMPILER_RECEIPT.json),
[native receipt](verification/native/RECEIPT.json),
[same-agent review](verification/NATIVE_REVIEW.json) and
[43 hash-indexed artifacts](verification/ARCHIVES.json) retain exact evidence.
No failed attempts occurred in this P19 job/session; inherited earlier failures
remain in their original slices. No passed job was replayed.

This is a bounded same-agent Linux scalar/Part workflow. Properties must have
exact1..1 multiplicity; previews are unitless numeric calculations through the
existing bounded calculator. Collections, nested paths, reference-instance values,
unit conversion, solve/simulation and full SysML values are outside the contract.
UI lists128 instance choices and bounds private effective-value audit to512 rows.
No independent engineering/full NoMagic/Craft/platform acceptance is claimed.
All56 broad parity specifications remain NOT_RUN; M04 exit remains open.
