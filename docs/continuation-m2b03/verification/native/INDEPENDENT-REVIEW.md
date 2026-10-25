# Independent actual-source and native inspection review

Reviewer periodic_actual_review inspected the actual source/API/UI diff and
identified a live-target shortcut blocker; source fixed it and added real egui
key input regression and non-default target selection/route retention checks.
It later verified the two pinned-egui test-only API/disposal corrections; no
product/model/vendored behavior changed across the83-test qualification chain.

Actual full-frame previews (tool1848x1344 from2200x1600 originals), matching
audits, events and actual source/output bytes for04/06/07/08/10 corroborate the
bounded workflow. Frame04 visibly labels source IDs/bindings as unverified,
foreign source DocumentId and geometry128,79. Real CtrlZ/S events precede06;
target snapshot9af65c44/model1/raw1/view2/active1/selection1 remain unchanged
with no pending write.07/08 retain preview SHAae54f7e after changed-byte and
corrupt-source refusal; whitespace is original+newline, corrupt bytes exactly{.
10 shows intentional final Save after inspector close, correct native binding,
target geometry99,61 and native name Target remains. PNG/audit hashes match
events. Three exact receipts corroborate55+27+1 unique passes; both harness
failures remain preserved. No blocking pixel/audit inconsistency found.

Limits: observed unbound-source inspection/refusal/shortcut flow only, no
copy/apply, v2 migration, source semantic admission, large-file UI performance,
broad accessibility/platform/parity acceptance. Reviewer performed no file,
process, GUI, upload or test execution changes.
