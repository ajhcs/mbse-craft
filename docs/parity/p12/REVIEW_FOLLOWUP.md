# P12 follow-up review

Reviewed PR #15 published head `943f15327dd65b7dc9da7288847b75d1f436b21d`
in isolated `codex/review-p12`, against merged P11
`b531d40e66dc436d447abaf29c9b2f3117974018`. Separate review pass by the
implementing Codex agent; no independent reviewer claim.

No additional actionable finding remains. Reviewed the byte/version/registry
validation before layout publication, defaults on malformed data without implicit
rewrite, generation-owned preferences persistence, named perspective budget and
reset marker, existing busy/write/gesture protection, off-screen startup/resize
clamping and focus retention. The only Cargo change enables the existing vendored
serde feature and existing locked dependency; upstream source is unchanged.

Reverified all679 source hashes,57 archive stored/lossless hashes,382 distinct
workspace test pairs including8 P12 cases, successful desktop build, and4 normal
native exits with cleanup. Pixel/byte review confirms off-screen Properties
recovery, restored tabs, named perspective restore, cold Reset, corrupt config
byte retention and native raw9e/accepted95metre/derived2.5km. Both recorded harness
errors remain, with the corrected geometry oracle and an actual preserved
unsaved-views close guard. No passing test/native workflow was replayed.

GitHub had no comments/review threads. No CI success is inferred from that.
Acceptance verifies37 original worktrees unchanged; this review inventories38,
including the implementation checkout, without editing or advancing them.
P12 is ready for the user-authorized dependency-order merge. There are47
remaining positions including P12, then46 starting P13. Full parity and the
Milestone3 exit remain open; the56 broad acceptance specifications remain NOT_RUN.
