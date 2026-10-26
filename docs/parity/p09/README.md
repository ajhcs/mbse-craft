# P09: stable workspace command catalog and menus

Draft review: [PR #12](https://github.com/ajhcs/mbse-craft/pull/12).

P08 [PR #11](https://github.com/ajhcs/mbse-craft/pull/11) was reviewed without
additional actionable findings in isolated `codex/review-p08`, then merged at
`d7cca1c17002368f58264e7686e8c71a9b73b6dd`. Its
[follow-up review](../p08/REVIEW_FOLLOWUP.md) records the exact scope. This P09
candidate builds directly on that merge in `codex/m3-p09-command-catalog`.

The app now has **21 stable workspace command IDs** with fixed shortcuts and
File/Edit/View/Tools menus. Menu and toolbar actions and matching keyboard
bindings use one availability policy and dispatch boundary. Disabled menu items
show prerequisites; toolbar controls expose the same reason on hover. Existing
paired save, source protections, native/raw/layout history and preservation
flows remain authoritative. See the [contract](CONTRACT.md) and the actual
[native catalog snapshot](CATALOG_SNAPSHOT.json).

| Acceptance criterion | Concrete evidence |
| --- | --- |
| Stable IDs, defaults and menus | Unit test checks 21 unique IDs/default shortcuts and menu groups; actual audit exposes all 21 items. Native File/Edit/View/Tools menus render with default accelerators |
| Common checks and dispatch | Unit/controller cases invoke all 3 surfaces with missing path, busy native write and stale selection, preserving target state and the owning pending ticket. Actual menu/toolbar/keyboard Model Undo/Redo use the recorded IDs and intended scope |
| Disabled commands explain prerequisites | Native View menu displays empty-layout-history reasons. API/controller cases verify missing path, selection, independent history directions, pending write, inspection and preservation-dialog refusal |
| Native field Undo stays scoped | Actual menu captures the focused native-field scope before popup focus changes; menu field Undo and keyboard field Redo/Undo restore the intended raw name. Controller cases refuse stale field selection and leave plain-widget Undo keys unconsumed |
| Exact modifiers prevent aliases | Actual Ctrl+Alt+Z and Ctrl+Alt+Shift+Z undo/redo camera layout with native snapshot unchanged. Tests also distinguish Recovery from Save, ignore real repeat events, and keep histories independently available |
| Equivalent safe saves | Actual native Save via toolbar, menu and keyboard emits paired native/views files. Saved native records agree with expected model history and every unedited draft stays exact. Real pair saves are also tested from each controller surface |
| Navigation and reopen preserve work | Native keyboard and menu New each open the preservation dialog; each Cancel retains exact native/layout. Fresh normal reopen retains exact snapshot/views digest, invalid raw 9e, accepted 95 metre and 2.5 km, with empty runtime history. Catalog Ctrl+Q exits normally |

Final source passed **352 workspace tests**, including **nine new P09 cases**,
then desktop build exited 0. Exact suites/names and commands are in
[TEST_RESULTS.json](verification/TEST_RESULTS.json),
[COMPILER_RECEIPT.json](verification/COMPILER_RECEIPT.json) and
[JOB_INPUT_FIXED.json](JOB_INPUT_FIXED.json). All commands use one Cargo job/test
thread, locked/offline, the existing target lease and fresh Samsung
workspace/RAM/disk/FD/writer checks. A sandbox-only DEVICE_ALIAS_MISSING result
was rechecked in the approved host context and the correct Samsung UUID was
verified before mutation. No mount repair, observer or service was started.

Final source closure: **674 files**, SHA-256
`ef0af52c552a2b741adb3031b56de42afee2ed82b6b420dd67b9f8e767b4232a`.
Executable SHA-256:
`7f2da91ac484bda93d2726c54226a90ee2008ed6e2e7324baf16ec6559620339`.
Cargo.lock remains unchanged:
`475828c2023805c7f168af22aaa116b7f536ed6d09dfb090f3390a6b68c8454a`.
The source manifest covers build inputs, not subsequent evidence/doc files.

Two actual native app/display runs closed normally. The first used normal
window close; the second used the catalog's Ctrl+Q and its existing clean-close
flow. Both drivers exited 0, exact owned processes and authentication files were
absent, and original input hashes stayed unchanged. Frozen
[NATIVE_JOB.json](NATIVE_JOB.json) pins binary/controller/fixture hashes.
The [raw native receipt](verification/native/RECEIPT.json) retains its original
`actions_completed_review_pending` status; the separate
[same-agent pixel/byte review](verification/NATIVE_REVIEW.json) records later
review. All 69 original/stored/losslessly decompressed artifacts are indexed in
[ARCHIVES.json](verification/ARCHIVES.json), including action/event logs, saved
pairs, UI audits and screenshots.

## Retained corrections and claim boundaries

The initial compile failed before tests because the child module declaration
preceded the UI file's module documentation. That [original receipt](failed-compile/RECEIPT.json)
and source/job remain. The corrected compile reached a run with 88 passing tests
and two failing new input fixtures: ordinary shortcut presses lacked key release,
and a synthetic repeat had no preceding held key. Egui derives repeat state
itself. The [failed fixture epoch](failed-input-fixtures/RECEIPT.json) remains;
correcting the fixtures produced the final 352-test/build pass. Runtime handlers
were unchanged by the fixture correction. Counts from epochs are not summed.
Existing compiler warnings remain in the archived logs.

The native session retains **one corrected oracle error**. The first assertion
incorrectly expected Model Undo to rewind every raw draft. Native records had
correctly restored while the latest raw name remained separately retained. The
corrected oracle checks exact restored records, every unedited draft and the
explicit retained raw name. Subsequent field-history checks exercise raw Undo
separately. The original error is preserved and the run is not labeled zero-error.

All **31 pre-existing worktree heads/statuses** remain unchanged, including the
original P08 and its review checkout; see [preservation](PRESERVATION_AFTER.json).
There are no new dependencies or changes to external projects, shared skills,
services, providers, signing or release workflows. Source/pixel review is by the
implementing agent. This is bounded Linux/software-rendered command acceptance;
per-command file/domain behavior is not exhaustively replayed. P10 configured
shortcuts, docking, full keyboard/accessibility/platform coverage and engineering
acceptance remain open. All 56 prospective full-parity specifications remain NOT_RUN.

Static parity validation passed: 58 PR positions, 56 capabilities, 56 NOT_RUN
prospective specifications and 18 rejected negative cases, with local historical
Git objects verified. `git diff --check` passed. These are package checks,
separate from the actual Cargo/native results above.
