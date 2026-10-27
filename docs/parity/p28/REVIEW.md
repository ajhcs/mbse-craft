# PR36 isolated same-agent review

Revieweda3e14aa7b5fafcb6d16d86c31e02fe49d8600728 against P27 merge894e00b8
in codex/review-p28. Same implementing agent, not independent review. Provider
comments/threads empty; no CI claim.

F01 fixed: queued row/column callbacks now retain their original report. Dispatch
validates that exact report against current native source and busy state. Previously
an ID-only callback consulted the latest report, allowing a history/source refresh
to replace its guard. Cell edit proofs were already source-bound. New delayed-axis
case refreshes changed native bytes with the same IDs/counters; old selection
refuses and preserves source/selection. Busy current selection refuses, fresh one
succeeds. Existing affected rendered-axis case also passes. Only ui_matrix.rs
changes from original candidate; other726 source hashes remain exact.

Reviewed native graph projection, stable numeric axes/32-by32 bounds, supported
command mapping and explicit read-only/unsupported scope. Eligibility hints do not
authorize edits: existing command validates a clone before sealed proof, dispatch
rechecks exact native instance/Snapshot/raw and uses the joint model/view command.
Mandatory clear/wrong kinds/cross-System/off-page IDs refuse. Subject single-link
replacement/clear and Measure required replacement preserve existing semantics.
Selection alone preserves proof. Reports/edit requests have no deserializer or
public constructor. Role/page changes suppress old cells; changed source hides
old cells until refresh. Catalog/portable shortcut/auxiliary/pending/history guards
and existing native Undo/Redo remain shared authorities.

Original sources retain211 passes before stale31-command audit count; only that
assertion changes,360 exact unexecuted remainder/build pass. Unique571 names,8 new
cases, no successful replay/summed final-source full-workspace claim. Actual native2
show matrix/canvas/ledger agreement, Undo/Redo, subject replace/clear/set, mandatory/
read-only refusal, exact raw154/type extensions/5-map preservation and cold fresh
whole copies. Model82->89 records7 explicit actions; only final accepted subject
differs from original.

Final727-file sourcec657bbfd0d4c0356e75f9091624781b76dfa2aa37ecd2393913651c50465eabc
and binary794e8af3f86094f87d9bbe5e6f1a8f3f805d847074ee175610b45a1803007e80
qualify affected2/build and native1: actual row1002/column1000 selection, current
report and whole Snapshot/5 maps unchanged through owned Save. Normal process/
display/auth cleanup and exact frozen hashes. Unique inventory572,9 new total;
no final-source full-workspace-run claim.

Verified727 final hashes,39 unchanged original and16 review artifacts.62 prior
available checkouts including original candidate retain heads/status;18 user-
removed directories retain registered heads, contents unverifiable. No reset,
recreation/prune/dependency/license/build-script changes. Package/diff checks pass;
no remaining actionable finding. Bounded P28 ready to merge; P29 follows. All56
broad specs/integrated exits/full NoMagic/SysML/Craft/platform/scale/independent
engineering acceptance remain unaccepted.
