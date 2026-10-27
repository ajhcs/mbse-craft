# Recovery-copy proposal reconciliation with qualified09df25d

Status: local source-only contract supplement, not implementation or acceptance. Original9304395 proposal and its eight vectors remain unchanged. No source, schema, scheduler, tests, compiler or renderer changes are introduced. Qualified09df25d/sourcee6cc2113/binary199226d5 and the prepared native gate remain frozen. This supplement has not been published.

## R2: runtime identity in both preview and target

The original step6 checks bytes, native Freshness/snapshot and numeric view revision. Qualified09df25d adds a private runtime Origin to Views, stamps, captures, destination capabilities and acknowledgments. A future copy planner must bind both its source-preview instance and target store to corresponding private process-local origins, alongside exact source bytes/SHA, native freshness and target view revision. Editable wire sidecar_id, equal revision, same labels or equal numeric occurrence IDs cannot replace those capabilities.

If source or target is replaced with a same-ID/revision variant after preview, reject apply before allocating IDs or touching history/state. Freshly parsing the same bytes creates a new runtime preview origin; explicit re-preview/reconfirmation is required. Preserve origins through the planner's own unchanged preview and target history only where that exact store still owns it. Never serialize/recreate an origin as provenance, an Undo capability or evaluator/native checkpoint authority.

## R3: copy is a newer layout edit, not a replacement of a pending save

If policy permits applying the explicitly selected copy while a native/views pair is pending, it is a new target layout revision in the same owning origin. It must not overwrite, cancel, rebind or redirect the frozen pair, its destination, kind, expected prior ownership or native ticket. Its older acknowledgment can record only the older frozen layout and leaves the copied view dirty. Stale native completions still cannot clear a newer capture. Choosing a source file for a read-only copy preview does not call Io::open on the live target, grant local replacement or admit an unpreserved target.

Alternatively a first implementation may disable Copy Apply while preservation is pending and show a clear reason. Settle that interaction explicitly before coding; do not silently alter qualified pairing behavior. Either policy must preserve source bytes and current target work on cancel/refusal.

## Bounded provenance and strict wire decision

Retain explicit per-occurrence mapping, new target IDs, checked exact target edges and inert original provenance. Account source-preview bytes, decoded items, proposed mapping, copied geometry, provenance and skip reports before large clone/allocation, not only the final encoded view. Include skipped/detached items in counters. Define an aggregate preview/payload envelope and its refusal boundary; do not describe it as total process RSS.

Before v2 implementation, settle a concrete deny-unknown-fields DTO and explicit SaveAs/read-only-v1 behavior. Preserve v1 originals, never silently add origin fields to them, and never use source-declared bindings as a new sealed native checkpoint. No automatic migration or general semantic identity reconciliation is adopted by this supplement.

## Layout-only periodic recovery gap to consider first

Direct source anchor: qualified `crates/mbse-desktop/src/ui.rs` periodic-recovery condition around1667–1674 requires `session.is_dirty()` and a stale native recovery. View-only edits can therefore remain outside the periodic trigger while native accepted/raw data are unchanged. Manual recovery and preservation guards exist; this observation is not a claim that view-only periodic recovery is implemented or qualified.

Before broader Copy, define a separate bounded scheduler contract covering native-dirty and layout-only-dirty eligibility, debounce/clock behavior, disabled settings, exact document/runtime-origin/view generation, and native/pair/views-pending suppression. Do not weaken artifact verification or reinterpret a recovery hint as durable authority. A layout-only recovery acknowledgment must not acknowledge a newer edit, another origin or document, or mark native Save current. Recheck actual recovery bytes before destructive navigation. Avoid repeated writes of an unchanged generation.

Acceptance needs deterministic scheduler/controller tests with a controlled clock and actual bounded output files: layout-only edit triggers the eligible recovery exactly once; ineligible/disabled/pending states do not launch; later edit stays dirty after stale acknowledgment; same-ID/revision replacement cannot consume an old acknowledgment; delete/truncate/tamper cannot authorize continuation. A subsequent actual native scheduler workflow is separate. No scheduler code or execution is proposed as a change to the qualified candidate here.

The supplemental vectors define future expected outcomes only, UNRUN. They add no functionality or authority and do not hold a compiler/renderer allocation.
