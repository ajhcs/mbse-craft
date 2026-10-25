# Review of PRs #4–#9

This is the requested follow-up review by the same executing agent. It is not a
new external, domain-engineering, accessibility or platform acceptance review.
The exact published heads and original stacked bases are in [PR_HEADS.json](PR_HEADS.json).
All fixes are prepared in isolated `work/review-m1-pr4-9`; original checkouts and
feature branches are preserved.

## Finding addressed

**P2 — Native acceptance controller could finish while a later app was active.**
PR #8 introduced the controller and PR #9 extended its preflight. Assigning a new
`n` in the submitted action namespace did not replace `main`'s original local
`n`. Terminal status, FINISH and interrupted cleanup consequently consulted only
the first driver. A closed first run could authorize FINISH while a later run
was live; interrupted cleanup could omit later owned drivers. The prior P06
README disclosed the status limitation, but the reusable controller still needed
a correction. Retained historical per-run receipts show all actual prior runs
closed normally; those receipts and script/job hashes are unchanged.

The controller now registers every owned driver before its readiness handshake,
reports all live runs, requires normal-close/driver-exit/owned-process evidence
for every run before FINISH, and visits all live owned drivers on interrupted
cleanup even if one stop fails. Normal close updates the matching run record,
including out-of-order closes. Six mocked-driver regressions cover later live
runs, missing normal-exit proof, unowned records, multi-run cleanup failure,
out-of-order close and failed readiness. They launch no GUI and are not native
acceptance evidence. [HARNESS_TESTS.log](HARNESS_TESTS.log) retains the results.

The fix is carried by PR #9 so the final stack includes the correction to the
controller introduced in PR #8. No Rust application/build input changes.

## Review coverage

| PR | Contracts examined | Review result |
| --- | --- | --- |
| #4 | Capability/scope declarations, fixture identity/type endpoints, reciprocal scenario dependencies, source oracles, malformed/forged-result checks | Earlier P01 fixes retained; 18 package negative cases pass; no additional finding |
| #5 | Original evidence binding, source/vendor/build pins, frozen job/admission distinction, actual blank-author saved graph and calculation/raw invariants | 657 original source inputs and 587 vendor inputs match; preserved binary rehashed; 41 compiler/native archives verified; no additional finding |
| #6 | Bounded view DTO validation, monotonically allocated IDs/history/runtime origins, native checkpoint, failed/replacement inputs, frozen paired saves and independent acknowledgments | Identity/preservation and stale-ack boundaries retained; no additional finding |
| #7 | Ordinary timer/quiet policy, held/disabled/pending coalescing, native/views worker-start tokens and partial failure/Cancel/retry | Separate status/preservation checks and retry policy retained; no additional finding |
| #8 | Explicit read-only source inspection and mapping rehearsal, scoped search/usages/System browser, camera histories and direct impact | No live Apply or broad/transitive claim; one controller ownership finding fixed in #9 |
| #9 | Sealed row tickets, stale/busy guards, hidden/detached selection, ephemeral-state publication, fixture emitter, test-count/source-epoch reconciliation | Application closure remains unchanged; controller ownership finding fixed |

P05's 48 and P06's 142 archived entries were rehashed, including decompressed
log/event bytes. P02's original read-only audit still reports its original
pending acceptance at that historical point; subsequent actual execution is
recorded separately in RUN_RESULTS.md. Historical pending/failed/held receipts
are not rewritten. No existing GitHub review threads were outstanding at the
checked heads; the earlier PR #5 route-blocker comment is superseded by its
subsequent exact execution comment.

The current dependency notices, retained source/license inventory and pinned
winit build script were inspected before the new offline locked execution.
Application source closure remains 669 files at
`8b43545d4033edf12c69e86e7d1f32a650c55b867911347542f2ac1d99f30e2b`.
The new exact review test/build job is [JOB.json](JOB.json). The fresh final-source run passed all **319 workspace tests** and the desktop
build exited 0. Six controller regressions and 18 package negative checks pass.
[VALIDATION.json](VALIDATION.json) and the exact compressed logs retain these
results. This new single-source result is separate from the earlier P06
319-distinct/two-epoch reconciliation.

The optional extra controller lifecycle smoke completed one actual app/driver
run with normal exits and owned-process/auth cleanup. The second run was
refused by the unchanged 8 GiB workspace reserve guard; available space at the
follow-up check was 7,413,981,184 bytes. It was not launched. A subsequent
expectation that the second run existed was also recorded as a harness error.
The original partial receipt and actions are retained without rewriting them.
The planned two-run smoke is **not completed**; the six mocked regressions
verify the multi-driver refusal/cleanup changes. Older native product evidence
retains its original source/binary pins, and all 669 application inputs match.
No disk cleanup, reserve relaxation or unrelated job mutation was performed.
A subsequent [fresh retry](native-retry-02/README.md) completed the actual two-run
controller smoke with zero action errors. Both normal closes, live-second-run
status and refusal of completion while it remained live were observed. Both
unchanged reserve checks passed. The original partial receipt remains intact.

No actionable finding remains within this code/evidence review scope.

## Completed integration

Merge commits preserved the original feature/source history. #4 merged into
`implementation/native-alpha`; each successor was then retargeted and merged
in #5→#9 order. Every expected head, base merge commit and current mergeability
was checked before merging. [MERGES.json](MERGES.json) records the results.
The final merged tree exactly matches the reviewed ec0b6c3 tree; all 669 build
input hashes still match. [PRESERVATION_AFTER.json](PRESERVATION_AFTER.json)
verifies all 25 pre-existing worktree heads and statuses unchanged. Main and original PRs #1–#3 are outside this merge scope.
No existing branch is reset, force-pushed or deleted. All pre-review worktree
heads/statuses are in [PRESERVATION_BEFORE.json](PRESERVATION_BEFORE.json).

Full No Magic/Craft parity, 56 future full-capability acceptance declarations,
platform/performance/accessibility and engineering acceptance remain open.
