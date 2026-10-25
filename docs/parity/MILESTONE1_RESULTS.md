# Milestone 1 implementation and verification results

The six Milestone 1 positions are implemented and have bounded Linux verification
evidence. The stacked PRs are ready for review; none is merged or independently
accepted as a combined release. P02's execution blocker is resolved through a new
BuilderBob-local preflight. No observer/service was started and no original
Lenovo admission or release of another job's hold is claimed.

| Position | Review artifact | Evidence and outcome |
| --- | --- | --- |
| P01 | [PR #4](https://github.com/ajhcs/mbse-craft/pull/4) | [Matrix review](PR4_REVIEW.md): scope/fixture fixes, 18 rejected malformed variants, exact source and metadata checks |
| P02 | [PR #5](https://github.com/ajhcs/mbse-craft/pull/5) | [Execution results](p02/RUN_RESULTS.md): 64 focused tests, build, actual blank authoring, 15 m/s, 2.5 km, accepted 95/raw 9e, save failure/Cancel and normal reopen |
| P03 | [PR #6](https://github.com/ajhcs/mbse-craft/pull/6) | [Qualification](p03/README.md): original 38 tests and actual two-view paired-save/failure/reopen evidence bound to identical source bytes |
| P04 | [PR #7](https://github.com/ajhcs/mbse-craft/pull/7) | [Qualification](p04/README.md): 71-test behavior epoch, ordinary timer/held gesture/failure/retry/recovery; wording successor kept separate |
| P05 | [PR #8](https://github.com/ajhcs/mbse-craft/pull/8) | [Integrated navigation](p05/README.md): 312 workspace tests, separate build remainder, actual scoped/exact search, usages, browser, cameras and stale impact refusal |
| P06 | [PR #9](https://github.com/ajhcs/mbse-craft/pull/9) | [Final candidate](p06/README.md): sealed exact occurrence navigation, hidden/detached and stale checks, final native baseline, source copy, UI reuse and presentation rehearsal |

## Final candidate and evidence boundaries

The final source closure has 669 build inputs, SHA-256
`8b43545d4033edf12c69e86e7d1f32a650c55b867911347542f2ac1d99f30e2b`.
The actual preserved executable has SHA-256
`4b14d96f30d387ab21a1bb736444b062c155a59ad4e6e9170447ef2054c7bf0c`.

P06's logs reconcile 319 distinct passing tests across two source epochs, 332
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
P03/P04 source-specific independent reviews remain distinct. Independent review
of the final stack and native evidence is still pending.

The original 56 full-capability [acceptance specifications](acceptance.json)
remain NOT_RUN. Milestone 1 is a bounded development baseline; it does not close
those full No Magic capabilities, Craft interaction coverage, accessibility,
platform, performance, engineering acceptance or unresolved edition/scope gates.

## Preservation and remaining work

[The preservation check](p06/PRESERVATION.json) verifies 20 unrelated original
worktree heads and statuses unchanged. Original feature PRs #1–#3 and their
worktrees remain intact. No GitHub merge, deployment, release or service change
was performed. Review the stack in P01→P06 order; each PR targets its predecessor.

There are **52 remaining roadmap positions, P07–P58, across nine milestones**.
Positions with decomposition gates require additional child PRs, so 52 is not a
fixed final PR count. The next milestone covers presentation recovery: explicit
source inspection, mapping rehearsal and reviewed Apply/Undo/save/reopen.
Live Apply remains deferred. The machine-readable current ledger is
[milestone1-results.json](milestone1-results.json); the original roadmap numbering
and scope stay in [roadmap.json](roadmap.json).
