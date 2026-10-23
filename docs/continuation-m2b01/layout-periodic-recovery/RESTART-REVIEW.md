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
