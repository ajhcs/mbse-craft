# P11: transactional panel docking

Built on merged P10 PR #13 (`0dd5051b8532a58b9f0e9ac01ab5df3cf6727968`)
in isolated `codex/m3-p11-transactional-docking`. The pinned Craft renderer now
hosts Model browser, Properties and Native views. Stable panel bodies borrow the
one live Workspace. Every tab, float, split and resize action goes through a
staged layout transaction; all three panels remain nonclosable.

| Acceptance | Exact evidence |
| --- | --- |
| Moves preserve identity and selection | Native Float, real pointer tab drop, tab switches and split keep selected #1003, native snapshot and views history unchanged. All three body IDs return exactly after split |
| Incomplete text survives | Screenshots show raw `9e`, accepted 95 metre and calculated 2.5 km before/after moves and cold reopen. Controller test verifies raw buffers, native/views documents and histories |
| Invalid drops lose no work | Actual Escape cancels a pending pointer drag with exact layout/revision unchanged. Tests reject invalid target geometry, invalid mixed batch, forbidden Close, missing panel registry, stale/exhausted revisions and busy writes/gestures |
| Focus and audit are safe | Actual renderer test retains exact native-field widget ID and restores focus after the floating area measuring pass. New user pointer/Tab input cancels restoration. Hidden canvas drops cached toolbar audit bounds |
| Native persistence stays exact | Baseline/final saved native snapshots are equal; baseline/final sidecar JSON is entirely equal. A second app opens final native plus saved views, checks exact file digest and empty runtime history, and shows the original draft |
| Existing work is preserved | All 35 pre-existing worktree heads/statuses remain unchanged; no vendor, dependency, shared-skill or service change |

The corrected source passed **374 distinct workspace tests**, including **eight
new P11 cases**, and desktop build exit 0. Source commit
`9d7d3c76964b8e98723bad8354e035d58c562f2f`, 678-file closure SHA-256
`ef17e95b5f5f2e25ef0ac03bb9cdaf432adda9f866369b22d46629dfbb4f3c7a`;
executable SHA-256
`e17f9d4fe4d8024fe706eb4c039e682dd51fd3d3e962b0a5d88a7d9bf2b6d6e4`.
See [source manifest](SOURCE_MANIFEST_FOCUS.json), [job](JOB_FOCUS.json),
[test pairs](verification/TEST_RESULTS.json) and
[compiler receipt](verification/COMPILER_RECEIPT.json). No runtime source changed
after that passing job. Build/cache/test output stayed on verified Samsung
/workspace, with one Cargo job/thread, a target lock and fresh local preflight.

Two private Linux native app/display runs exited normally; exact owned processes
and authentication files were removed. The [raw native receipt](verification/native/RECEIPT.json)
retains `actions_completed_review_pending`; [same-agent review](verification/NATIVE_REVIEW.json)
is separate. All 37 archived artifacts have stored/original hashes in
[ARCHIVES.json](verification/ARCHIVES.json). Original fixture bytes are unchanged.

One recorded acceptance assertion incorrectly compared the reopened file digest
with the pre-save in-memory digest. It is retained as an action error. The
correct comparison then proved the reopened digest equals the actual final
sidecar bytes; baseline and final sidecars are entirely equal. No product change
or repeated native run was needed. The first regression exposed a real focus
handoff defect; its [failed receipt/log](failed-focus/RECEIPT.json) are retained,
and the bounded focus ticket fixes it without stealing subsequent user input.
An initial [prelaunch request](NATIVE_PRELAUNCH.md) refused stale historical
script hashes before any application launch; the corrected request hashes the
current unchanged merged scripts.

Floating panels are egui areas inside this viewport. Individual tabs dock by
pointer or explicit Panel layout controls. A floating group's grip moves the
group; whole-group docking on release is not implemented. P12 owns persisted
arrangements, reset and off-screen/corrupt-workspace recovery. P11 starts with
default panel arrangement on a fresh launch. These are bounded same-agent Linux
checks, not independent, full platform/accessibility, full Craft or engineering
acceptance. All 56 prospective full-parity specifications remain NOT_RUN.
