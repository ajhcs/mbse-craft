# P23 identified requirement hierarchy: bounded evidence

Extends the existing independent mbse-assurance Board on P22 merge
24cb464749e82f28f6199a4a2341ced21bfae421. Native records/calculations, typed data,
profile applications and all saved presentation maps remain exact. Requirements
have stable local IDs plus exact user identifiers such as R-1; parent containment
does not inherit or roll up approval, implementation or verification.

| Acceptance | Recorded evidence |
| --- | --- |
| Identified root/child and duplicate refusal | Actual native R-1 → R-2 → R-3. Duplicate R-2 preserves accepted records, selection, allocator and pending proposal. Existing duplicate edits remain raw drafts. Rename preserves internal parent/evidence endpoints. |
| Cycle refusal and invalid drafts | Native R-2 → R-3 cycle refuses, retaining accepted parent. Later parent `2?`, blank R-3 statement and blank new R-5 statement survive Save/reopen; prior accepted statement and next ID5 remain exact. Model tests additionally cover missing/self/noncanonical parents and deep iterative trees. |
| Reparent/history and independent states | Native moves R-2/R-3 subtree to R-4; actual register Undo/Redo restores exact records. R-1 Approved/Implemented leaves children Draft/UnderReview; verification remains NotStarted. Parent change stales existing evidence through the existing requirement hash. |
| Safe existing-file admission | Actual Open refuses numeric ID alias `01` and forged valid-parent draft; live requirements/drafts/allocation and malformed input bytes remain unchanged. Tests cover duplicate JSON/user IDs, current-state integrity, legacy flat files, unknown metadata and existing evidence histories. |
| Durable independent register | Actual Save, cold Open and fresh-copy Save preserve complete register requirements, drafts, revision and allocator. Cold register history is absent. Native Save and cold second Save equal the complete original Snapshot and every presentation map. |
| Preservation |47 prior available checkouts keep exact heads/status;18 user-removed directories keep registered heads, removed contents unverifiable. Both owned native sessions exit normally with process/auth cleanup. |

Compiler evidence is deliberately source-specific:

- Initial711-file source `6e90c520761c6493185c94ee9a4d22146a7f49232907d437ad95aeaeb07c1fbd`: full workspace504 tests and desktop build.
- Draft-validity guard source `a20e92f13b90c49567964dd37b442da52fd6a0c1d9250309780516d393f3030c`: affected assurance23/UI7 tests and build.
- Final identified-authoring source `9e205f42d5a39f016f0414c7dce1df15ccea919cef41e15424ffcbd8341a65e4`: affected assurance26/UI7 tests and build. Final binary `055d925d86c372f8df4ba5723f4ec8a0602c9353c668cc64a123539754e6e208` drove both native runs.

All earlier case names reconcile, with14 distinct new P23 cases across these slices.
Counts are not summed into a final-source full-workspace claim. No compiler failure.
One native controller KeyError is retained: it expected a nested `document` key in
the views4 output after Save. Corrected top-level map assertions passed before
normal close; authoring was not replayed. Raw receipt remains review-pending;
[same-agent evidence review](verification/NATIVE_REVIEW.json) records the bounded
interpretation. [Archive index](verification/ARCHIVES.json) hashes53 original and
stored artifacts, including losslessly compressed action/audit/build logs.

This is bounded Linux implementation evidence. Hierarchy uses the existing
independent register, not a native requirements classifier or formal statement
language. First256 tree rows are rendered with full depth labels and bounded
indentation; up to10000 requirements persist under existing32MiB limits. No full
NoMagic/SysML/Craft, cross-platform or independent engineering acceptance. M03/M04
integrated exits and all56 broad parity specifications remain unaccepted.
