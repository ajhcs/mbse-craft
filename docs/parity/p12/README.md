# P12: persisted workspace arrangements and named perspectives

Built on merged P11 PR #14 (`b531d40e66dc436d447abaf29c9b2f3117974018`)
in isolated `codex/m3-p12-workspace-perspectives`. Panel moves now persist through
the existing preferences worker. Panel layout offers named perspectives and Reset.
A bounded versioned envelope validates every owned panel before restoration.

| Acceptance | Concrete evidence |
| --- | --- |
| Restart restores arrangements | Native saved tab arrangement loads exactly in a fresh app from the actual saved config bytes; store reopen test verifies exact floating layout |
| Perspectives are recoverable | Native Save Review, tab elsewhere, cold launch, Restore Review returns its floating arrangement without native snapshot/selection change. Controller tests preserve raw9e and native/views histories; malformed named entry remains stored and refuses without partial restore |
| Reset restores defaults | Native Reset writes explicit empty/default marker, keeps Review, and another fresh launch opens defaults. Actual file-backed store reset/reopen proves no old arrangement resurrection while comments, unknown keys, recovery values and shortcut override remain |
| Off-screen panels become reachable | Native fixture places Properties at x90000/y-90000; startup clamps its header/body into available workspace and saves safe geometry. A shrink test clamps all bounds and keeps exact three-panel registry |
| Corrupt settings preserve model | Native malformed layout opens defaults with visible refusal, selected #1003, unchanged native snapshot/raw9e/accepted95metre/derived2.5km. Entire corrupt config stays byte exact through normal exit; other valid settings still load. API tests refuse unsupported versions, duplicate/missing panels and over-budget payloads |
| Writes/gestures remain protected | Restore, Reset and named Save use existing busy/modal guards; failures retain panel/settings/native state. The existing unsaved-views close guard still works |

The frozen 679-file source closure
`a59ca18fc9a998c629fcf0d94b4a8d0d0ab6b8803172101d11f2d7c1f2979684`
passed **382 distinct workspace tests**, including **eight new P12 cases**, and
desktop build exit 0. Runtime commit `851734f` plus job/docs commit `56ec69a`
are pinned by the [compiler receipt](verification/COMPILER_RECEIPT.json).
Executable SHA-256:
`da3ede6fa3364a2828ad7ff368f3628457c41641313a5a084823cd409ebe4529`.
See [manifest](SOURCE_MANIFEST.json), [job](JOB.json),
[test pairs](verification/TEST_RESULTS.json) and [contract](CONTRACT.md).
No runtime source changed after this passing job.

Four private Linux app/display runs exited normally with exact owned-process and
authentication-file cleanup. [Original native receipt](verification/native/RECEIPT.json)
retains `actions_completed_review_pending`; the [same-agent pixel/byte review](verification/NATIVE_REVIEW.json)
is separate. All 57 artifacts have stored/original/lossless hashes in
[ARCHIVES.json](verification/ARCHIVES.json). Every config seed receipt matches
its actual source bytes; originals remain unchanged. Cold launches copy exact
saved config bytes into each owned writable run/config directory; this does not
claim restart at the same physical config path. Actual store tests cover file-backed
reopen/reset directly.

Two recorded action errors remain. The first assumed a fixed y147 threshold;
the live available workspace starts at y139 before central padding, and the
correct audited geometry proves the panel reachable. The second close request
correctly hit the existing unsaved-views protection. Cancel retained state, then
explicitly opening the saved matching views permitted normal close. No app run
was repeated or protection bypassed, and neither failed action is relabeled.

All **37 pre-existing worktree heads/statuses remain unchanged**. Build/cache/test
outputs stayed on verified Samsung /workspace with a target lock, fresh local
preflight and one Cargo job/thread. The only dependency metadata change enables
already vendored Craft UI serde support and its already locked serde dependency;
no package download, upstream source, shared-skill, provider or service change.
No independent, full-platform/accessibility, domain engineering or full Craft
acceptance is claimed. P13 and the Milestone 3 exit remain open; all 56 prospective
full-parity specifications remain NOT_RUN.
