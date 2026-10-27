# P06 exact occurrence navigation and final Milestone 1 verification

P06 carries the existing independently source-reviewed `7cb7876` occurrence
navigation candidate into the isolated P02–P05 stack. It adds a checked public
fixture emitter and corrects one previously unrun test's destination suffix.
Original feature branches, the old unrun job and all prior receipts are intact.

The final **669-file** source closure is
`8b43545d4033edf12c69e86e7d1f32a650c55b867911347542f2ac1d99f30e2b`.
The preserved executable is SHA-256
`4b14d96f30d387ab21a1bb736444b062c155a59ad4e6e9170447ef2054c7bf0c`.

## Test failure, correction and exact count

The first full-workspace run passed 78 tests, then failed the new busy-state
test before its assertions: `pending-views.json` violates the real sidecar API's
`.mbse-views.json` requirement. Only that test filename was corrected; production
validation and runtime navigation are unchanged. The original failed receipt
and log are retained.

A separately frozen remainder passed the corrected test, still-unexecuted
packages, desktop build and actual-API fixture emission. Its package exclusion
was incomplete: 13 already-passing assurance tests accidentally ran again. They
are retained and explicitly deduplicated. The record covers **319 distinct
passing tests, 332 passing executions and one earlier resolved failure** across
two source epochs. It does **not** claim a single clean 319-test run on the final
closure. Exact suites, names, outcomes and repeats are in
[TEST_RECONCILIATION.json](verification/TEST_RECONCILIATION.json).

Initial zero-child resource holds remain distinct. The local check was refined
after identifying the conflicting Cargo process as a separate Kernspan target
with ample available memory: unrelated compilers are recorded, while MBSE/target
writers, descriptor headroom, memory/disk reserve and the exclusive M1 target
lease remain checked. No other process, hold, observer or service was changed.

## Actual final-binary native observations

The new synthetic fixture includes Active Primary occurrence 7, hidden Active
Review occurrence 10 and prior-kind Detached Review occurrence 12 for exact
native ID 1002. The emitter validates it through the real decoder and actual
native/views Save/acknowledgment protocols. It is deliberately constructed QA
input, not accepted engineering content or in-product authoring evidence.

The actual final-binary session observed:

- Exact report-row navigation to Primary, hidden Review without showing it, and
  Detached Review while retaining a separately selected Measure and raw `9e`.
  Navigation preserved native snapshot, layout revision/digest, history/allocator
  counters and cameras. Raw/layout changes disabled old Go actions; Refresh
  retained the original report target and enabled exact navigation.
- Controlled owned-lock native-success/views-failure, unchanged prior sidecar
  bytes, retained geometry/raw state, close/Cancel, exact lock removal and retry.
- Real held pointer across 35 seconds without recovery, ordinary 15-second
  recovery after release/quiet, disabled recovery producing no new copy, paired
  normal close/reopen, and independent native recovery refusing the old views
  binding before preserving a fresh pair.
- Actual New blank native document, five typed records/links, field/model history,
  **15 m/s** and **2.5 km**, accepted 95/raw `9e`, paired Save and normal reopen
  with the exact snapshot retained.
- Independent source-primary/source-other XML inspections with distinct scoped
  identities and explicit unsupplied references. Native snapshot stayed intact.
  A Capella 7 Mission-only proposal canceled app close; a fresh private copy
  changed exactly the one original name span and reparsed. An existing output
  was not acknowledged or overwritten; its dirty proposal was explicitly
  discarded. All original input hashes stayed unchanged.

The separate final-binary [completion session](verification/native-completion/RECEIPT.json)
then reused the actual blank-authored Mission 1000 as Primary occurrence 5 and
Review occurrence 6. One name edit appeared in both views; five native records,
the Measure and raw draft were retained, and the new pair was saved. Read-only
presentation inspection and explicit Target 1/1000 mapping produced a two-mapped
preview without changing native/layout hashes, history or the allocator. Source
recheck cleared that preview. Live Apply remains deferred to P09. The retained
audit exposes choice kinds only; one overly specific harness assertion was
corrected in place, with exact target IDs checked in menu actions and pixels.
The [completion review](verification/NATIVE_COMPLETION_REVIEW.json) records this.

All four app/driver runs exited 0, exact app/display identities were absent and
authentication files were removed. The executing agent reviewed the listed full
captures; independent PR/native review remains separate. Busy/native-pending,
views-pending, pair-pending, runtime/stamp and other race refusals retain their
explicit model/controller test scope. No new actual button keyboard-focus,
platform/accessibility/performance, engineering or full-parity guarantee is made.

[verification](verification/) includes the actual native events/actions, all
captures/audits, authored/reopened/recovered pairs, source copy, original failed
and held receipts, logs and exact hashes. The original native receipt remains
`review_pending`; the subsequent [review](verification/NATIVE_REVIEW.json)
records pixel/byte checks and two corrected harness expectations without
rewriting history. Terminal `app_running` diagnostics after replacing the first
driver are stale; the three actual PID/exit/cleanup records are authoritative.

Twenty original unrelated worktree heads/statuses were verified unchanged in
[PRESERVATION.json](PRESERVATION.json). No main/feature branch was overwritten or
merged on GitHub. The final Milestone 1 ledger records review readiness and
remaining scope limits separately from these execution results.
