# Layout-only periodic recovery candidate

Status: source-only candidate. No Cargo command or GUI has run for this epoch.

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
