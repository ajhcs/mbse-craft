# P07 pre-publication review

Same executing agent; independent review remains pending. Reviewed the exact
final 671-file source closure used by the 330-test run, build and native session.

Checked v1 strictness and byte-stable serialization, bounded/duplicate-rejecting
v2 decode, source namespace/identity and UTF-8 reason bounds, exact mapped
endpoints and retired historical references, preserved outer native binding,
version-aware Save As before native write, and profile preservation through
capture/recovery/save/reopen. Source provenance never participates in sealed
checkpoint or runtime-origin creation. Existing tests cover replacement,
worker ownership, stale/busy navigation and histories on the new source.

No new dependency, build-script or external source change is present. The
retained notices/inventory and pinned winit build script were read before
execution. The initial audit-macro compile failure is corrected and retained;
no actionable finding remains in this bounded same-agent review.

P08 must recheck a fresh source and target, validate explicit choices and budget
before one atomic layout transaction, allocate fresh target IDs and preserve
originals. P07's public provenance DTO and explicit source-check helper do not
provide an Apply capability or prove semantic identity. A future copy Undo must
preserve the selected output format policy; wire provenance never restores
runtime history by itself.
