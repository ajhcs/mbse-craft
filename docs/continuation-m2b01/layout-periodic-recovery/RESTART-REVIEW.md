# Restart continuation: actual-source gap closure

Candidate changes after the preserved e50b1eb evidence epoch:

- Active multi-frame layout drags suppress scheduler requests; release/committed
  revision must then settle for750ms. Native and persisted-layout stamp changes
  reset quiet observation, including history/runtime replacement.
- Cheap preference, navigation, busy-pair, quiet and interval checks precede
  bounded disk verification. The last check advances even for a verified no-op,
  so unchanged ticks do not reread acknowledged recovery each frame.
- Due-time eligibility verifies actual saved/recovered layout bytes and native
  recovery. Native-current/layout-stale remains eligible. Existing valid Save
  pair or existing valid recovery pair is a no-op. Layout hints alone never
  establish preserved state.
- Display/audit cache explicitly says verified at timer check or awaiting check.
  Native Save status, paired wait, views pending and views failure remain separate.
  Destructive navigation/close still verify actual bytes immediately.
- Native workers start with fallible Builder and sealed matching start token;
  start refusal releases only that request/pair. Exact native-to-views validation
  failure also releases its owning pair while unrelated/stale tickets retain
  newer captures. Live geometry and input remain retained.

Conservative artifact conflict contract: native recovery uses an owned fixed
reservation with its acknowledged hash. If that native leaf is removed/tampered,
a due attempt reports conflict and retains work/evidence. It does NOT silently
repair/overwrite the artifact. Tests restoring controlled fixture bytes prove
retry after fixture restoration, not autonomous repair. New layout recovery
leaves do not overwrite damaged earlier sidecars or any Save target.

Independent actual-source review identified the five mechanisms before changes
and reviewed their fixes. New tests cover held gesture/release, bound and recovery
sidecar removal/truncation/whitespace tamper, check cadence/native loss, native-only
edits, owning pair cancellation/retry, stale failed-start tokens and native worker
refusal. These new tests are authored; execution status must come from the new
candidate gate, not historical47-test/build evidence. Real multi-frame pointer
input, ordinary15s scheduled recovery, separate receipts/bytes/geometry and
status/continued editing/close behavior require a new exact-binary native run.

No screenshot upload work, old evidence changes, service/security operations,
external parity/admission or recovered-layout-copy/v2 implementation is included.

## Executed qualification for933e871

All71 focused tests plus build passed, receiptc3950adfb5e7ac9aa049054f37eb91da4061fb9bd8f19ea98e47c377a759a49a.
Binaryf5cb158b4e4811df712b978a4bd1c8a36e4e0840869800be45e98b0a4333dc3d
completed actual native timer workflow15:36:34.990356Z–15:44:17.881812Z.
Held pointer, normal15s recovery, disabled timer, controlled partial failure,
ordinary retry, cancel-close, bound reopen and independent native Recover were
observed.18 original PNGs remain local; hashes and receipts are retained here.
Independent review corroborated narrow claims with the two documented timing/
wording limitations in native/INDEPENDENT-REVIEW.md. All six identities absent,
Xauthority removed, owned_jobs=[]. No upload/publication/merge.

The separate UI wording correction changes only Layout changes awaiting
preservation to Layout not saved to bound sidecar, because a current recovery
is independent of a stale bound Save. These71-test/native results belong to
933e871; the label-only successor requires its own build/UI evidence.

## Label successor qualified separately

Wording-only source8937db779410a4c0fe81d99b907ead77f070d508 built successfully,
receipt aa6cb4c25f741e80789b5e1931772a46590ff22759915a4b261e318eb1309a93.
Binary998b2b4b81e3f2fcc109e4bd43b482671ea234d6b6172e7ad994a950a3a7d731
completed a separate ordinary15s timer/actual-status visual check and normal
Save/close. Native receipt3cbdf61e3a2007dfa13e09745019daa4e0eedf27c58b605fad81c94e2feb40e5;
three original PNGs remain local. Independent review confirms the corrected
label alongside actual verified recovery and stale bound Save. The71tests and
full held-gesture/failure/retry/reopen workflow were not replayed or relabelled.
All six new identities absent/auth removed/owned_jobs=[]; renderer released.
Exact packet expiry holds and prior admission evidence remain retained.

Source publication is still stopped: original and once-permitted non-force
retry were both rejected before execution. No branch or new PR published;
PR2 draft/head557ee47e remains unchanged, no alternate route or merge.

The next isolated feature is recovery-layout-preview at the retained57ba816
contract steps1–2, related to CAT002/010/011 and CAP017/019 in the historical
86-feature research index. It is bounded read-only inspection, not geometry
copy/apply, native semantic admission or v2 migration.
