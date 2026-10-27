# P07: versioned presentation-copy format and provenance

Merged [PR #10](https://github.com/ajhcs/mbse-craft/pull/10) at `7244b562cf0e2de6462feeb70f24c69225c829e2`; [requested review](REVIEW_FOLLOWUP.md) found no additional actionable P07 issue. The execution record below remains source-specific; [P08](../p08/README.md) adds live Apply/Undo.

P07 builds on the merged Milestone 1 baseline at
`bfbe0f1ad361739fd14d728d93db3b9527c76b0a`, in isolated branch
`codex/m2-p07-copy-format`. It adds strict v2 presentation/recovery profiles,
bounded inert copy provenance, exact source/mapping checks and a fresh Save As
boundary between format versions. Existing v1 reads and canonical serialization
are unchanged. The native inspector displays recorded origin as **unverified
source data**. Live Copy/Apply and its one-transaction Undo are implemented separately in **P08**.

The [format contract and acceptance criteria](CONTRACT.md) explain the fields,
bounds and historical-reference policy. Constructed QA v2 input is not output
from a live copy action or accepted engineering content.

## Concrete acceptance evidence

| Criterion | Actual result |
| --- | --- |
| Old v1 files readable without migration | Bound/recovery API round-trips retain exact canonical bytes; actual native v1 Open succeeds and all original files remain unchanged |
| v2 provenance survives preservation | Bound/recovery API Save/reopen retains provenance and empty runtime history; actual fresh native/v2 pair Save and normal reopen retain exact layout bytes, provenance and native snapshot |
| Future versions/unknown fields preserved | Actual decoder/IO tests retain/protect refused bytes; native future-version/unknown-field Open refuses while native/layout/history/allocator stay exact |
| Malformed mappings refuse atomically | Wrong target view/kind, duplicate target IDs, invalid decimal IDs, unmapped/aliased edge endpoints, reason and payload limits refuse; actual native malformed-mapping Open retains work |
| Source truth checked explicitly | Actual source hash/identity/row/edge coverage and changed-byte tests; declared source binding cannot authorize an incorrect target binding |
| Existing files preserved | Actual owned version-change guard requires fresh Save As; native existing-v1 destination refuses before native output, then fresh v2 pair saves successfully |
| Provenance stays inert | Native inspection leaves target state exact; API tests retain historical removed IDs, separate layout history and reject foreign runtime stamps |

The full workspace passed **330 tests**, including **11 new P07 tests**, in one
run on the final source closure. Desktop build exited 0. Exact suites/names are
in [TEST_RESULTS.json](verification/TEST_RESULTS.json); the original compiler
receipt and lossless logs are [archived](verification/ARCHIVES.json).

Final source: **671 files**, SHA-256
`8ecb8f9c36f2b976acf9aaba98235db4371acf0f10685e20e78447977320ba8c`.
Cargo.lock is unchanged, SHA-256
`475828c2023805c7f168af22aaa116b7f536ed6d09dfb090f3390a6b68c8454a`.
Actual executable SHA-256:
`b1e8b7f07d5dcf1ad5d9d636510c772d175704a8b498e6696b0e3988f925862d`.
[JOB_FIXED.json](JOB_FIXED.json) pins the exact guarded offline/locked commands,
source manifest, runner/preflight hashes, one Cargo job and one test thread.
The existing exclusive target lease and fresh Samsung workspace/RAM/disk/FD
checks were reused; target/cache output remains under `/workspace`.

## Actual native session

Two actual app/driver runs completed normally with **zero action errors**.
The first read legacy v1, tried future/unknown/malformed files, opened v2,
inspected its provenance, refused an existing v1 destination, then saved a fresh
v2 pair. The second normally reopened that pair with native snapshot unchanged,
accepted 95/raw 9e retained, exact saved presentation digest and empty history.
Both exact app/display identity pairs and authentication files were absent
at completion. All six original input-file hashes remained unchanged.

[NATIVE_JOB.json](NATIVE_JOB.json) pins the executable, controller/driver and
fixture bytes. The [native receipt](verification/native/RECEIPT.json) retains
its original `actions_completed_review_pending` status; the subsequent
[same-agent review](verification/NATIVE_REVIEW.json) records pixel/byte checks
without changing that receipt. Owned-version conversion, v2 recovery and
maximum-budget/edge-endpoint cases have API-test scope. The native existing-v1
save refusal used `VIEWS_EXISTING_UNOWNED_TARGET`, not the distinct owned-version
Save As guard. The fixture was loaded; this makes no new blank-authoring claim.
One coordinate click missed the source-view combo without changing data; 07b
shows the actual menu and 08 the provenance. No action assertion failed.

## Earlier compile failure and preservation

The original compile hit the existing UI audit JSON macro expansion limit after
two new fields were added. No tests ran and no desktop build completed in that
attempt. The correction inserts those audit fields outside the large macro;
it does not raise the crate limit. The original [failed receipt/log](failed-compile/RECEIPT.json),
[JOB.json](JOB.json) and source manifest remain intact beside the corrected
single-source run. No passing test command was replayed.

[Preservation](PRESERVATION_AFTER.json) verifies all **27 pre-existing worktree
heads/statuses** unchanged. Main, the merged integration branch and existing
feature checkouts are preserved. Only this isolated P07 branch is published.
Dependencies, external projects, shared skills, services and release workflows
were not changed.

P07 supplies bounded Linux format/native evidence and is merged. P08 now records
its separate Apply/Undo execution; independent native review, engineering
acceptance and full No Magic/Craft parity remain separate.
