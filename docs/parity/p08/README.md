# P08: live reviewed presentation Apply and Undo

Draft review: [PR #11](https://github.com/ajhcs/mbse-craft/pull/11).

P07 [PR #10](https://github.com/ajhcs/mbse-craft/pull/10) was reviewed in isolated
`codex/review-p07`, with no additional actionable format finding, and merged as
`7244b562cf0e2de6462feeb70f24c69225c829e2`. The final reviewed head was
`7c69915d441cb3a507cf169bae7c46088037b913`; see its
[review record](../p07/REVIEW_FOLLOWUP.md). P08 builds directly on that merge in
isolated `codex/m2-p08-live-copy`.

The inspector now supports explicit mapping, a checked preview and live Apply.
Apply stages one new view with target-owned IDs, complete copy/skip provenance
and one layout history entry. Original views and native records remain intact.
The [transaction contract](CONTRACT.md) defines refusal and persistence behavior.

| Acceptance criterion | Concrete evidence |
| --- | --- |
| Only explicitly selected occurrences copy | API tests include hidden/pinned and detached source, duplicate native targets and exact edge endpoints. Actual native mapping leaves System unselected, skips Capability with a reason and maps Mission, Objective and Measure. Apply creates view 9, occurrences 10–12 and edge 13; next 14 |
| Skips and source remain reviewable | Native preview shows3 mapped/2 skipped and1 proposed/2 skipped edges. Saved provenance includes all 5 source rows and 3 edges, exact source digest and skip reasons. Original synthetic source hash stays unchanged |
| Stale source or target refuses atomically | Native target pan disables stale Apply; a newline appended only to a private source copy makes Apply refuse `VIEWS_SOURCE_CHANGED`. Native/layout/history/IDs remain unchanged. Fresh read/mapping/review succeeds. API tests cover raw/native/layout changes, foreign Plans and reopened inspection |
| One Apply and one Undo | Actual history increases 3→4 once; repeated Apply is disabled. One Layout Undo removes the copy and saved old views/occurrences/edges equal the pre-Apply v1 baseline. v2 and next 14 remain; Redo restores the copied canvas |
| Native meaning and drafts retained | Actual independent recovery creates a different DocumentId, retaining records and drafts. Copy/Undo/Redo/save/reopen keep its native snapshot; raw9e, accepted95metre and calculated2.5km remain visible |
| Save/reopen and originals | Actual owned-v1 pair Save refuses `VIEWS_FORMAT_SAVE_AS_REQUIRED` and both destination hashes stay exact. A fresh v2 pair saves and normally reopens, with copied tab, exact loaded sidecar digest and empty runtime history |
| Failure limits and work preservation | API/controller tests cover capacity, allocator exhaustion, combined provenance budget, title, pending native/views/pair tickets, held gesture and missing-native-dependency Redo. All 29pre-existing worktree heads/statuses remain unchanged |

The **final** source closure passed **343 workspace tests**, including **13 new
P08 cases** (11 integration and 2 controller), followed by desktop build exit 0.
[TEST_RESULTS.json](verification/TEST_RESULTS.json) records343 distinct suite/name
pairs. [JOB_FINAL.json](JOB_FINAL.json) pins one Cargo job/test thread,
locked/offline execution, existing exclusive target lease and fresh Samsung
workspace/RAM/disk/FD/writer preflight. No observer or service was started.

Final source: **672 files**, SHA-256
`b8fc1fc9b70d98d05f5fc3c0f90a19de0b05773d20d96f506623dfb33cba757c`.
Source commit: `fe5f22f4677a302c164a79c362050be47027a0b6`.
Executable SHA-256:
`5af8856bc9a7dbac33108837ea9486871074cdaacb10ef887ea6d397a3240033`.
Cargo.lock is unchanged:
`475828c2023805c7f168af22aaa116b7f536ed6d09dfb090f3390a6b68c8454a`.
Source manifests cover build inputs, not later evidence/documentation files.

The [compiler receipt](verification/COMPILER_RECEIPT.json),
[native receipt](verification/native/RECEIPT.json), actual input events/actions,
saved native/views artifacts, UI audits and screenshots are losslessly
[archived with original and stored hashes](verification/ARCHIVES.json).
Two actual Linux app/display runs exited normally with exact owned-process
cleanup. The fresh reopen selects Primary; the copied tab and loaded sidecar
digest are verified there, and the copied canvas is rendered after Redo in the
first run. The raw receipt retains `actions_completed_review_pending`; the
separate [same-agent pixel/byte review](verification/NATIVE_REVIEW.json) records
subsequent review without changing the receipt. This is bounded execution
coverage for the Milestone 2 workflow, pending PR and engineering acceptance.

## Retained failures and corrections

The first workspace command passed261 tests before a new detached-source test
fixture was correctly refused: it marked an occurrence detached while leaving
touching edges Active. The fixture was corrected to detach its touching edges;
the [original failure](failed-workspace/RECEIPT.json) remains intact. The corrected
source then passed343 tests and build; that [separate epoch](before-audit-fix/RECEIPT.json)
is retained. A final UI audit-order fix exports copy-control bounds after the
inspector renders; all343 tests and build were rerun on that final source.
These epochs are not summed. Existing compiler warnings remain in the logs.

The native session retains **two harness action errors**. A recovery banner
shifted toolbar positions, so the first inspector coordinate hit Zoom out;
explicit 100% restored the camera and subsequent targeting used live bounds.
A fresh-reopen assertion used nonexistent `snapshot_sha256`; it was corrected
to the exported `snapshot_hash` in the same app state. Both errors, corrections,
and the earlier camera history entries remain recorded. A conditional no-op
assertion is not evidence; the following explicit original/recovered/saved
DocumentId comparison supplies that oracle. None of these records is relabeled
as a zero-error run. The stale-source variant is a private 3119-byte copy; the
original 3118-byte source remains exact.

[Preservation](PRESERVATION_AFTER.json) verifies all 29pre-existing worktrees,
including original P07 and its review checkout. Dependencies, external projects,
shared skills, services, provider, signing and release workflows are unchanged.
This is same-agent Linux/software-rendered verification, not independent native
review, domain acceptance or full No Magic/Craft parity. The 56 prospective full
capability acceptance specifications remain NOT_RUN. P09 remains planned.
