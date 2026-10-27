# PR25 isolated review and correction

Reviewed candidate5ffefa66eecac34ea96d683008ddf8cac919da2a in `codex/review-p20`.
Same implementing Codex agent; no independent engineering acceptance claimed.

Found and fixed: move preview placed source/target Part references between selected
usages and enclosed connectors into the external-target list. Their actual IDs and
candidate/publication were correct, but the preview classification was misleading.
Added a retained-internal inventory and UI rows; strengthened the connected-move
regression to require internal Part1009 and external Port1017. Original inventory
claims are corrected by final evidence, not retroactively attributed to old binary.

Final702-file closure b353f555 passed seven model regressions, one actual renderer
Prepare/Cancel/stale/Apply test and build. One new native run from the actual saved
clone/move pair shows four internal retained Part references, external port/type
references, unchanged Preview/Cancel snapshot and unchanged accepted model/views
through Save. Source changes are confined to reference-summary metadata, the
assertions and UI list. Candidate construction, native publication, allocator,
model history and IO logic are unchanged from original465-test source. Original
full suite/two-run clone/move/cold state checks remain distinct; counts not summed.

Verified all702 source hashes, both preserved binaries and45 original plus19 new
indexed artifacts including lossless raw logs/events. Screenshots show corrected
internal rows; native audit asserts ports stay external. All owned processes exited
normally, original inputs/auth cleanup verified. Provider comments/threads empty;
no CI PASS inferred. No actionable findings remain after correction.

All40 available prior checkouts, including original implementation candidate,
retain exact heads/status;18 user-removed directories retain registered heads,
removed contents unverifiable. No recreation/pruning. Unknown native-field/opaque
reference semantics remain explicitly outside exhaustive verdict; typed scope and
all56 broad NOT_RUN parity specifications remain unchanged. M04 exit stays open.
