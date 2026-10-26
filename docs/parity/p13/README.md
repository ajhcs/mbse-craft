# P13: preserving file selection and background model reads

[PR #16](https://github.com/ajhcs/mbse-craft/pull/16) builds on merged P12 PR #15
(`422780873eee22b4170acdbeb5b4d77e3f6bad9b`). Implementation used isolated
`codex/m3-p13-dialogs-progress`; review fixes used `codex/review-p13`, leaving
the original implementation checkout unchanged.

File / Choose model file (Ctrl/Cmd+Alt+D and toolbar) offers an application-owned
chooser with asynchronous directory browsing, native Open/Recover, restricted
Import and native Save. The pinned Craft Outcome type keeps explicit Cancel,
failure and indeterminate results distinct. No OS-picker backend is installed;
this is not an OS-native picker claim. Existing explicit-path commands remain.
All workspace New/Open/Recover/Import reads run on one background lane, retaining
current work until native freshness, selection, views digest, save path and
write/continuation guards approve the result. Cancellation prevents publication
while retaining lane ownership until the reader returns.

| Acceptance | Final evidence |
| --- | --- |
| Cancel, failure and indeterminate differ | Final live chooser Cancel retains native state; actual empty path produces indeterminate; missing-file background Open produces failure. Initial live directory listing/failure evidence remains separately retained |
| Stale replies cannot replace work | Controlled delayed replies after newer raw9e/new view retain exact current snapshot/views. Stale picker Save cannot write/change destination; closed/stale directory replies cannot reopen or populate a new chooser |
| Completion cannot discard queued edits | Review fixed App::logic-before-ui publication: queued text/pointer/IME/touch/wheel/zoom input reaches the current UI first. Two actual egui-frame tests prove newer text and Cancel-button release win over an already-ready candidate |
| Cancellation retains ownership | Controlled cancelled read keeps its lane until the worker returns, rejects second reads, then discards the candidate. Worker failure/disconnect retains current work; current success publishes once |
| Saved native workflow remains exact | Final live chooser Save writes exact native snapshot and paired views; background Open restores it. Explicit views reopen verifies exact persisted file digest, with selected1003/raw9e/accepted95metre/derived2.5km visible |
| Existing work remains intact | All40 pre-existing worktree heads/statuses verified unchanged during review; original fixture unchanged; final app/display exits normally with exact owned-process and authentication cleanup |

Final candidate: **391 distinct workspace tests**, **nine new P13 cases**,
desktop build exit0. Frozen source680-file closure SHA-256:
`04bd96ddfbdc1cbf05cf4917797d649530db8639e860835b8cf24ca5b7366969`.
Runtime review fix `a2560a7` and executed checkout `9bc06ad` are pinned by
[final compiler receipt](final-verification/COMPILER_RECEIPT.json). Executable SHA-256:
`e7a8dea73fbe560bd5cba17369c44076df270b62317b13e9b495690954178c18`.
See [manifest](SOURCE_MANIFEST.json), [job](JOB.json),
[final test pairs](final-verification/TEST_RESULTS.json), [contract](CONTRACT.md)
and [follow-up review](REVIEW_FOLLOWUP.md). No runtime source changed after this
passing job. All outputs stayed on verified Samsung /workspace with fresh local
preflight, target lock and one Cargo job/thread, locked/offline.

Final binary completed one private Linux native run with zero action errors.
[Raw receipt](final-verification/native/RECEIPT.json) retains its original
`actions_completed_review_pending`; [same-agent pixel/byte review](final-verification/NATIVE_REVIEW.json)
is separate. All24 final/intermediate-log artifacts have stored/original/lossless
hashes in [final archive index](final-verification/ARCHIVES.json).

Earlier389-test/native candidate remains in [verification](verification/ARCHIVES.json)
and [before-input-fix](before-input-fix/README.md), with its exact original
manifest/job. Its one empty-fill harness assertion is retained: selecting existing
text and typing nothing did not clear the path; Cancel then actual Backspace
corrected the input without app replay. The390-test intermediate review candidate
has its own [receipt/manifest/job](input-fix-first/RECEIPT.json) and archived logs.
These counts belong to separate candidates and are not summed. The initial
[locked attempt](failed-lock/RECEIPT.json) executed zero tests; Cargo offline
metadata corrected an unintended mbse-catalog lock entry, then refused an uncached
existing cross-platform source. The successful locked Linux jobs downloaded no
package; only the pinned platform dependency was added to desktop.

Queued completion/cancellation races have actual controller/frame scope, without
production timing hooks or a claimed manually timed native race. Listings omit
non-UTF-8 entries and bound visible rows at500. Existing export/auxiliary editors,
OS-native pickers, independent review, full-platform/accessibility, domain and full
Craft acceptance remain outside this bounded slice. The integrated Milestone3
exit remains unaccepted; all56 prospective broad specifications remain NOT_RUN.
