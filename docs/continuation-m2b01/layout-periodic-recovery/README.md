# Layout-only periodic recovery candidate

Status: 47 focused tests passed on source cfe7b4b306d673d72b36d008537e55f980877363.
No native desktop binary build or GUI has run for this epoch. The preparation
record below describes the state before execution; verification/ contains exact
frozen job, source manifest, admission, allocation, launcher, raw receipt and
losslessly compressed logs.

The previous timer required a dirty native session, so a layout-only change
could remain without a periodic recovery copy. The scheduler now observes the
private presentation/native stamp, including native values/drafts, layout
revision and runtime origin. It waits for 750 ms without a stamp change and the
configured interval. Disabled recovery and pending navigation suppress requests.
Native writes, paired captures, and views writes defer a due request without
replacing or canceling any ticket. A failed attempt is throttled by the interval.
Settings changes reset the interval; opening a document starts a fresh clock.

Actual writing still uses the existing prepare_native / Session.begin_recovery /
capture_native / exact native acknowledgment / frozen views destination chain.
Layout receipt hints only schedule work. Close and navigation still verify
actual artifacts. R1 replacement preservation, R2 private origin, and R3 frozen
request/destination code are unchanged. Native and layout failures/status remain
independent; writing an older layout never acknowledges newer edits.

Nine new controller tests use controlled Instants and actual isolated synthetic
files, without GUI events, sleeping, or worker threads. They cover clean-native
layout-only recovery, both dirty, native edits during recovery, layout edits
during recovery, a busy manual pair, views worker start failure, a late foreign
destination leaf, disabled/navigation/origin observation, and native failure.
The existing thirty model/native-views tests and eight controller tests remain
in scope. New test authoring is not a test pass.

Static checks: rustfmt parsed the four changed Rust files; formatting applied
only to the new module and controller tests; git diff --check passed. Dependencies,
lockfile, model core, native views/IO implementations and vendored sources are
unchanged. Execution still needs fresh job-bound health and an action allocation
that confirms this existing MBSE target has no concurrent writer.

Proposed focused commands, in this worktree with scripts/dev.sh environment:

```sh
bash scripts/dev.sh test --locked --offline -p mbse-model native_views -- --test-threads=1
bash scripts/dev.sh test --locked --offline -p mbse-desktop views_ui::tests -- --test-threads=1
```

Reuse CARGO_HOME=/workspace/cache/builderbob/mbse-craft-m1,
CARGO_TARGET_DIR=/workspace/build/builderbob/mbse-craft/semantic-v2/target,
TMPDIR=/workspace/build/builderbob/mbse-craft/semantic-v2/tmp, exact Rust1.99.1,
one Cargo job and one test thread. No duplicate target is created speculatively.
Previous measured times were 10.9 s model and 23.1 s desktop tests; this source
has nine additional cases and has not been timed. Peak RSS is unknown.

Qualified source09df25d/source e6cc2113/binary199226d5 and its worktree remain
unchanged. Its actual two-view run produced 35 original PNGs and ended
2026-10-23T13:02:15.463393Z; exact six-identity cleanup and removed Xauthority
were recorded at13:02:39.017830Z. Renderer released; owned jobs=[]; display ID
was not retained. Parent independently checked four lossless crop hashes and
pixels, confirming only their narrow label/partial-save/corrected-refusal
claims. Full-window, accessibility, edge and performance acceptance remain open.
Parent reported its local review path m2b-01/native-pixel-review/REVIEW.md;
that file was not read from this host. Original receipt remains immutable; the
parent report is in native-gate/INDEPENDENT-CROP-REVIEW.json beside the run.
No image upload or renderer rerun occurred.

Observed Ok(()), Ok(true), VIEWS_PINNED and missing-target error notes are
recorded for later wording work. Recovered-layout-copy/v2 migration, semantic
edge native workflows, Fit, live delayed/foreign acknowledgment timing and
broader product qualification are outside this candidate.

## Focused test execution

The exact frozen job07b4c238 ran at2026-10-23T13:23:55.395479Z through
13:24:30.210983Z. Both commands returned0: thirty native_views model/IO tests
and seventeen views_ui controller tests, including all nine new periodic cases.
The model Cargo PID1021089/start13618628 ran10.859s; desktop Cargo
PID1022344/start13619715 ran23.925s. Each launch independently validated the
original fresh job-bound health. The desktop launch at13:24:06.275549Z preceded
the original packet expiry13:24:18.927461Z and watchdog-age cutoff. The packet
was not extended. One Cargo job and one test thread were used. No automatic
retry, source repair, second writer, GUI, or monitoring helper was launched.

Receipt SHA2566b3939ecc7120d8d9d9020bef74023556f8e9eb0d91e10690bad34c45028e767.
All661 source-closure files and Cargo.lock were unchanged through execution.
Both exact Cargo identities are now absent and owned_jobs=[]; test allocation
released. Existing target apparent bytes5805647217, allocated5695596KiB. Peak
RSS was not measured. Observer/coordinator evidence must establish any actual
overlap and concurrency result; this test receipt does not establish either.

Existing unused patch/unused import/unused variable/deprecation warnings remain;
no warning-only cleanup was mixed into this source. This bounded controller
qualification is not native timer UI, accessibility, performance, platform or
engineering acceptance. The qualified09df25d/199226d5 native epoch remains
separate and unchanged.
