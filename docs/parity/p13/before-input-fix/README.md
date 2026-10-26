# P13: preserving file selection and background model reads

Built on merged P12 PR #15 (`422780873eee22b4170acdbeb5b4d77e3f6bad9b`)
in isolated `codex/m3-p13-dialogs-progress`. File / Choose model file (also
Ctrl/Cmd+Alt+D and toolbar) opens an application-owned chooser with background
directory browsing and native Open/Recover, restricted Import and native Save.
The pinned Craft Outcome type keeps explicit Cancel, failure and indeterminate
results distinct. This host has no installed OS picker backend; no OS-native
picker or full-platform claim is made. Existing explicit-path commands remain.

All workspace New/Open/Recover/Import reads now use one background lane. Current
work stays visible. The result can publish only while the original native
freshness, selection, views digest, save path and write/continuation guards remain
current. Cancellation retains the lane until the reader returns and prevents
publication. Existing frozen native/paired write receipts and protection remain.

| Acceptance | Concrete evidence |
| --- | --- |
| Cancel, failure and indeterminate differ | Native explicit Cancel retains exact native/views state; actual empty path displays distinct indeterminate result; directory failure and missing-file background Open display errors without mutation. Controller tests also cover explicit backend failure and worker disconnect |
| Stale replies cannot replace work | Controlled delayed replies after newer raw9e or a new native view are rejected with exact current snapshot/views retained. Stale picker Save cannot start a write or change the destination. Closed/stale directory replies cannot reopen or populate a new chooser |
| Background completion retains edits | Controlled cancellation keeps the reader lane occupied until its response returns, then discards the candidate. Current success publishes once; read failure retains originals. Actual worker tests cover missing file and New; native Save/Open round-trip retains the original incomplete draft |
| Real file workflow works | Native directory list shows original.kernspan; chooser Save writes the exact original native snapshot and paired views. Chooser Open runs in background, accepts that saved native file, and explicit views reopen verifies its exact file digest/empty history with raw9e, accepted95metre and derived2.5km visible |
| Existing work stays intact | All39 pre-existing worktree heads/statuses unchanged; original fixture unchanged; normal native exit, owned process and authentication-file cleanup |

The corrected 680-file source closure
`54fce092c645209cf13a1508db67d75ef3bba38393bedc0123583e1264e62d63`
passed **389 distinct workspace tests**, including **seven new P13 cases**, and
desktop build exit0. Executed checkout is pinned in the
[compiler receipt](verification/COMPILER_RECEIPT.json); runtime `c7a64f7` plus
corrected lock `8ce95c9` and frozen job `bf0d7ef`. Executable SHA-256:
`0025b031aa9d7994ffac69c558f35060df1e1150f73367771b8c6cc1452de512`.
See [manifest](SOURCE_MANIFEST.json), [job](JOB.json),
[test pairs](verification/TEST_RESULTS.json) and [contract](CONTRACT.md).
No runtime source changed after the successful job. Build/cache/test outputs
stayed on Samsung /workspace with fresh local preflight, target lock and one
Cargo job/thread, using locked/offline inputs.

One private Linux app/display run completed normally. The
[raw native receipt](verification/native/RECEIPT.json) retains
`actions_completed_review_pending`; [same-agent review](verification/NATIVE_REVIEW.json)
is separate. All34 archived original/stored/lossless hashes are in
[ARCHIVES.json](verification/ARCHIVES.json). One harness action error remains:
typing an empty string selected the existing field without deleting it. Existing
preservation protection retained work; Cancel then explicit Backspace produced
an actual empty/indeterminate result. No app replay or product change was needed.

The [first locked attempt](failed-lock/RECEIPT.json) stopped before compilation,
with zero tests. A broad manual lock insertion accidentally listed craft-platform
under mbse-catalog too. Cargo offline metadata corrected that ownership and then
stopped on an uncached existing cross-platform redox_users source; --offline
prevented acquisition. The corrected lock adds only the pinned platform crate to
desktop; no existing package version changed. The old job/manifest/log remain
retained. The successful locked Linux regression/build needed no package download.
Upstream licenses/notices and build scripts were inspected; no vendor edit,
provider/service/release workflow or external-project/shared-skill mutation.

Slow completion and stale/cancel races have controlled actual controller scope;
the native file reads finish too quickly to claim a manually timed race. This
chooser omits non-UTF-8 listing entries and bounds listings at500 rows. Existing
export, auxiliary editors and platform-native pickers remain outside this slice.
Independent, full platform/accessibility, domain engineering and full Craft
acceptance remain pending. P09–P13 each have bounded evidence; no integrated
Milestone3 exit or full-parity acceptance is inferred, and all56 prospective broad
specifications remain NOT_RUN.
