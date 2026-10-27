# Milestone 1 implementation and verification results

All six Milestone 1 PRs **#4–#9 are reviewed and merged**, in dependency order,
into `implementation/native-alpha`. The requested follow-up review corrected one
acceptance-controller ownership defect. A fresh single-source run passed all
319 workspace tests and the desktop build; six controller regressions pass.
Exact review, merge hashes and preservation results are in
[review-pr4-9](review-pr4-9/REVIEW.md). This remains a bounded Linux development
baseline; domain/engineering and full parity acceptance remain separate.

P02's blocker was resolved through a bounded BuilderBob-local preflight. No
observer/service was started and no original Lenovo admission or release of
another job's hold is claimed.

| Position | Review artifact | Evidence and outcome |
| --- | --- | --- |
| P01 | [PR #4](https://github.com/ajhcs/mbse-craft/pull/4) | [Matrix review](PR4_REVIEW.md): scope/fixture fixes, 18 rejected malformed variants, exact source and metadata checks |
| P02 | [PR #5](https://github.com/ajhcs/mbse-craft/pull/5) | [Execution results](p02/RUN_RESULTS.md): 64 focused tests, build, actual blank authoring, 15 m/s, 2.5 km, accepted 95/raw 9e, save failure/Cancel and normal reopen |
| P03 | [PR #6](https://github.com/ajhcs/mbse-craft/pull/6) | [Qualification](p03/README.md): original 38 tests and actual two-view paired-save/failure/reopen evidence bound to identical source bytes |
| P04 | [PR #7](https://github.com/ajhcs/mbse-craft/pull/7) | [Qualification](p04/README.md): 71-test behavior epoch, ordinary timer/held gesture/failure/retry/recovery; wording successor kept separate |
| P05 | [PR #8](https://github.com/ajhcs/mbse-craft/pull/8) | [Integrated navigation](p05/README.md): 312 workspace tests, separate build remainder, actual scoped/exact search, usages, browser, cameras and stale impact refusal |
| P06 | [PR #9](https://github.com/ajhcs/mbse-craft/pull/9) | [Final candidate](p06/README.md): sealed exact occurrence navigation, hidden/detached and stale checks, final native baseline, source copy, UI reuse and presentation rehearsal |

## Original P06 candidate and evidence boundaries

The application source closure has 669 build inputs, SHA-256
`8b43545d4033edf12c69e86e7d1f32a650c55b867911347542f2ac1d99f30e2b`.
The actual preserved executable has SHA-256
`4b14d96f30d387ab21a1bb736444b062c155a59ad4e6e9170447ef2054c7bf0c`.

The original P06 logs reconcile 319 distinct passing tests across two source epochs, 332
passing executions including 13 repeated assurance tests, and one earlier test
fixture failure corrected to use the required sidecar suffix. This is not a
single clean 319-test run on the final closure. Source/runtime equivalence and
that test-only correction are recorded with the original failure and holds.

Four actual app/driver runs on the final binary exercised exact hidden/detached
navigation, stale refusal, accepted/raw retention, owned-lock partial save,
Cancel/retry, ordinary held/disabled recovery, independent native recovery,
blank authoring/calculations/normal reopen, explicit source inspections and a
name-only fresh XML review copy. A completion session reused the newly authored
Mission in both views, edited its name once, saved both occurrences, and checked
read-only presentation inspection/rehearsal without model/layout/history or
allocator mutation. All four app/driver runs exited normally. Captures, events,
exact pairs, recovery copies, input/output hashes and same-agent reviews are in
[P06's evidence](p06/verification/).

The six [baseline vectors](milestone1.json) are frozen prospective declarations.
Their literal example IDs and all model/controller-only race, pending/busy and
late-acknowledgment assertions are not relabeled as new actual GUI observations.
P05 supplies the dedicated two-System duplicate-name search/usages/browser run;
P06 retains its regressions and adds final-binary impact/occurrence observations.
P03/P04 source-specific independent reviews remain distinct. The requested same-agent follow-up code/evidence review is now complete;
no new external independent native/domain review is claimed.

The original 56 full-capability [acceptance specifications](acceptance.json)
remain NOT_RUN. Milestone 1 is a bounded development baseline; it does not close
those full No Magic capabilities, Craft interaction coverage, accessibility,
platform, performance, engineering acceptance or unresolved edition/scope gates.

## Review verification and merge results

The new review build uses the same 669 application inputs. Its fresh single-source
workspace run passed **319 tests** and its desktop build exited 0. The reviewed
build SHA-256 is
`90dfe7eb3709d7ca57e174bfdbd06942d140f8c21f823e74d1bf3b58abe7f844`.
This new result is separate from the older two-epoch counts above.

Six mocked-driver regressions verify the repaired controller's all-run status,
normal-close proof, FINISH refusal and interrupted cleanup. The initial extra
native smoke was disk-held after one normal run; its partial result and later
mistaken two-run expectation remain retained. The [fresh retry](review-pr4-9/native-retry-02/README.md)
subsequently completed both actual app/driver runs with zero action errors. The
controller reported the second run live after the first closed, refused FINISH
while it was live, then verified both normal closes before completing. The
unchanged 8 GiB reserve passed before both launches. No application source changed.

[MERGES.json](review-pr4-9/MERGES.json) records the exact #4→#9 merge sequence.
The final merge `c61d3379a136c56afc531e80936376922f2ea98a` has exactly the
reviewed tree at `ec0b6c3942472ddec9ae59d898693a103985581f`. Subsequent
ledger/preservation edits are documentation only.

## Preservation and remaining work

[The final preservation check](review-pr4-9/PRESERVATION_AFTER.json) verifies
all 25 pre-existing worktree heads/statuses unchanged. The original feature
PRs #1–#3 and all local branches/checkouts are preserved. Main was not changed;
all six authorized PR merges landed on `implementation/native-alpha`. There was
no deployment, release, service change, branch deletion, reset or force push.

There are **52 remaining roadmap positions, P07–P58, across nine milestones**.
Positions with decomposition gates require additional child PRs, so 52 is not a
fixed final PR count. The next milestone covers presentation recovery: explicit
source inspection, mapping rehearsal and reviewed Apply/Undo/save/reopen.
Live Apply remains deferred. The machine-readable current ledger is
[milestone1-results.json](milestone1-results.json); the original roadmap numbering
and scope stay in [roadmap.json](roadmap.json).
