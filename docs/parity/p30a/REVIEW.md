# PR39 isolated same-agent review

Reviewed candidatec1dc9901a5de79567f2240ca2e3d86b19be13c5d against P29 merge7d7247b
in codex/review-p30a. Same implementing agent, not independent review. Provider
comments/threads empty; no CI claim. One actionable finding F01 fixed.

F01: legacy deletion Summary could mark its supported reference inventory complete
while new activity pin/System references were outside that inventory. Final Snapshot
validation already refused invalid deletions, but the completeness flag was false
confidence. New test initially failed compilation by using a private summary field;
that zero-test attempt is retained. Public accessor correction reproduces the
incorrect flag exactly. Activities and retained raw flow proposals now add bounded
unknown impact entries, mark completeness false and refuse legacy deletion plans.
Raw-form count includes the proposal. Explicit ClearFlowForm discards only that
proposal; accepted activities remain unchanged and keep impact unsupported. Once
activities are explicitly removed and raw proposal explicitly discarded, existing
supported deletion works. Test covers both accepted and raw-only impact atomically.

Final732-file source7c25c23af756c13acca5ccdf9099a0709f506eb635c642bf608b96c34c5146f4
passes19 affected deletion cases and desktop build. Original core slices218/251/126
remain595 distinct successes; one new review case yields596. No unrelated successful
replay or summed final-source full-workspace pass. Original seven artifacts plus
seven review artifacts retain jobs/logs/failures/binary hashes. Reviewed all source
hashes, original stored/lossless artifacts and final binary bytes. Native activity
editor/diagram acceptance has not run.

Reviewed shared allocator/global identity and raw high-water/collision checks,
canonical nested maps/strict fields, aggregate type/activity counts and extension
byte bound. Same-activity node/partition/pin/flow ownership, native System and
same-System Value pin payload identity, direction, finite Decision guards and
branch rules delegate existing atomic Editor transactions. Model Undo/Redo retains
newer raw proposal and allocator; save/cold Open preserves whole Snapshot, recovery
Restore changes document identity explicitly while every other field stays exact.
Cycles/incomplete authoring data and diagnostics do not imply execution/liveness.

68 prior available checkouts including original candidate retain heads/status;
18 removed directories retain registered heads, contents unverifiable. No reset,
clean/recreation/prune or upstream/license/build-script changes. Package/diff checks
pass. Core ready to merge. P30b must preserve unfinished forms across native
selection and add independent diagrams/save/cold acceptance; parent remains open.
No general guards/fork/join/nested partitions/activity refactor/complete impact/
execution/full SysML/NoMagic/Craft/platform/scale/independent engineering claim.
All56 broad specs/integrated milestone exits remain unaccepted.
