# P13 follow-up review and corrected acceptance

Reviewed published PR #16 head `091b33b13a4d153736c8d0550a8ebbe17fedeb41`
in isolated `codex/review-p13`, against merged P12. This is a separate review pass
by the implementing agent, not independent review. GitHub had no comments or
review threads when review began.

One actionable finding: eframe calls App::logic before App::ui after admitting
current input. A ready model result could publish before a queued field edit or
Cancel-button release was applied to the current workspace. Completion now defers
while relevant queued input awaits UI handling; the following freshness check
rejects any newer native/view edits, and an explicit Cancel discards the candidate.
Two actual egui-frame regression cases cover ready-result plus typing and ready-result
plus Cancel release. IME/touch/wheel/zoom events also defer, without claiming native
input-method/platform acceptance. The pinned eframe integration ordering was
verified in source. No production sleep, delayed fixture or test hook was added.

The first review fix passed390 tests; final refinement including Cancel release
passed391 distinct workspace tests/nine new P13 cases and built successfully.
Both exact receipts/source closures remain retained, separate from the original389
candidate. Final680-file closure and binary match the frozen manifest; all24
final archive stored/lossless hashes are verified. Final live chooser Cancel,
empty/indeterminate, Save, background failure/success and native/views exact reopen
passed in one normal app/display run with zero action errors. Raw receipts retain
original review-pending status; separate pixel/byte review records scope.

The original failed lock attempt/zero tests and initial native helper error remain.
No prior PASS is relabeled as final evidence. Only runtime changes after initial
acceptance are the scoped publication/input fix and its two regressions. Final
native verification uses the actual newly built binary. No additional actionable
finding remains in chooser guard, lane ownership, worker failure/disconnect,
preservation continuation, native paired-save integration or close protection.

All40 pre-existing worktree heads/statuses remain unchanged, including original
P13 implementation. Same-agent bounded Linux verification is not independent,
OS-picker/platform or engineering acceptance. P13 is ready for the user-authorized
merge; after it,45 of58 roadmap positions remain, starting P14. The integrated
Milestone3 exit and full parity are not inferred from these PR-level checks.
