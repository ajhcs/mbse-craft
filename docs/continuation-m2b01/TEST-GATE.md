# M2B-01 bounded validation gate

Completed for exact source `5bebe92`:23 model tests,5 desktop controller tests and desktop build passed. [Actual receipt](verification-5bebe92/README.md). The commands and admission below describe that executed bounded gate; they do not authorize a new launch. The parent allocated the next compiler wave after SysON released it. The launch used a fresh independent root health packet bound to the exact job SHA; each command launch was within its120-second sample lifetime. Kernspan retains the renderer: no GUI, screenshot job, upload, alternate worker or agent context belongs to this gate.

Prepared job: `/workspace/build/builderbob/mbse-craft/native-views-m2b01/JOB.json`, SHA256 `3d0247c9897855797cf1f17bed064363d061e8e015e052bedc429278ecb999af`. Its source manifest covers660 files and hashes to `20f9b9aaf933bd1b75c42c7ba885806b3a76972bb6a55878d373e60b73ddbe3f`; Cargo.lock SHA256 is `475828c2023805c7f168af22aaa116b7f536ed6d09dfb090f3390a6b68c8454a`. Any source repair creates a new attempt manifest and preserves the failed attempt/logs.

Use the assigned worktree, not the sealed integration-m1 checkout. Existing reviewed offline toolchain/cache paths:

```bash
export CARGO_HOME=/workspace/cache/builderbob/mbse-craft-m1
export CARGO_TARGET_DIR=/workspace/build/builderbob/mbse-craft/semantic-v2/target
export TMPDIR=/workspace/build/builderbob/mbse-craft/semantic-v2/tmp
export MBSE_RUST_BIN=/workspace/cache/builderbob/global-tools/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin
export CARGO_BUILD_JOBS=1 CARGO_TERM_COLOR=never RUST_TEST_THREADS=1
bash scripts/dev.sh test --locked --offline -p mbse-model native_views -- --test-threads=1
bash scripts/dev.sh test --locked --offline -p mbse-desktop views_ui::tests -- --test-threads=1
bash scripts/dev.sh build --locked --offline -p mbse-desktop
```

Run serially; stop the batch on a failed command. Preserve per-command argv, start/end UTC, exit status, exact stdout/stderr logs and SHA256. Verify the source/lock manifest before and after; preserve the resulting binary separately with its SHA before releasing the compiler. Ordinary scoped type/test failures may be repaired and meaningfully rerun within the allocation, with every source/attempt hash retained. Health warnings/denial/EMFILE/stale health stop new launches; no mitigation or security/permission changes are authorized.

After compilation, an actual two-view workflow still needs a separately allocated exact binary/renderer. It must exercise shared native-name edit, independent move/hide/remove, Layout Undo/Redo, explicit edge endpoint restoration, native/views partial preservation, save/reopen high-water, native Undo detach/Redo and recovery mismatch. Unit/controller tests do not prove GUI behavior, external admission, performance, interoperability or engineering acceptance. Earlier frozen receipts remain unchanged and cannot qualify this source.
