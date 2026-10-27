# MBSE-Craft

An independent native Rust systems-modeling workbench built on the attributed Kernspan core and pinned Craft UI/persistence libraries. It supports bounded typed modeling, requirements and validation, behavioral and parametric workflows, source inspection/import, and durable native models and views.

The repository is public and **`main` is the contribution base**. [PR #1](https://github.com/ajhcs/mbse-craft/pull/1) promoted the reviewed implementation with its history retained.

## Current development and contribution

P01–P39 of the [58-position parity roadmap](docs/PARITY_ROADMAP.md) are merged, including split implementation PRs. P40–P58 remain; child PRs can increase the remaining PR count. These bounded implementations do not close the 56 broad acceptance specifications or integrated milestone exits. Full No Magic, SysML, Craft, platform and engineering parity remain open.

Start with [CONTRIBUTING.md](CONTRIBUTING.md). Fork the repository, branch from current `main`, coordinate a small behavior or roadmap position, and open a draft PR. Preserve native meaning, invalid drafts, identity, Undo/Redo and recovery. Keep each change in an isolated worktree and include evidence for its actual scope.

Useful starting points:

- **Independent workflow checks:** create/edit a small model, use two views, exercise invalid drafts and Undo/Redo, save/reopen/recover, and report exact reproduction steps.
- **P40 semantic-subset XMI export:** agree on a small child PR, preserve identity and types/relations, disclose unsupported constructs, and check the result with an independent reader.
- **Craft UI feedback and focused fixes:** identify a concrete keyboard, focus, layout, label or inspector inconsistency with captured behavior and expected results.

See the [branch integration audit](docs/BRANCH_INTEGRATION.md), [capability matrix and acceptance package](docs/parity/README.md), [model contract](docs/MODEL_CONTRACT.md) and [upstream compatibility inventory](docs/UPSTREAM_COMPATIBILITY.md).

## Build on Linux

Recorded qualification uses Rust 1.99.1 and X11/OpenGL. The declared Rust 1.97 minimum and other platforms have not been qualified. Linux needs X11, keyboard and OpenGL libraries. Registry dependencies require access or an existing isolated cache. No provider credentials or service stack is needed.

On BuilderBob, verify the Samsung `/workspace` before creating data and obtain fresh exact-job admission before Cargo/native checks. Choose unused checkout/cache/output paths. On another machine, set `MBSE_RUST_BIN` to the bin directory of your installed Rust toolchain; some current test/native helpers require `/workspace` or BuilderBob-specific facilities.

```bash
git clone https://github.com/ajhcs/mbse-craft.git /workspace/repos/mbse-craft-contributor
cd /workspace/repos/mbse-craft-contributor
mkdir -p /workspace/build/mbse-craft-contributor/{target,tmp,state,config} /workspace/cache/mbse-craft-contributor
export CARGO_HOME=/workspace/cache/mbse-craft-contributor
export CARGO_TARGET_DIR=/workspace/build/mbse-craft-contributor/target
export TMPDIR=/workspace/build/mbse-craft-contributor/tmp
# Set this to your actual installed toolchain bin directory:
export MBSE_RUST_BIN=/workspace/cache/builderbob/global-tools/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin
bash scripts/dev.sh build --locked -p mbse-desktop
bash scripts/dev.sh test --locked --workspace
/workspace/build/mbse-craft-contributor/target/debug/mbse-desktop --state-dir /workspace/build/mbse-craft-contributor/state --config-dir /workspace/build/mbse-craft-contributor/config
```

`scripts/dev.sh` fixes one Cargo job and calls the exact toolchain binaries to avoid wrappers that replace the selected cache. Review added dependencies/build scripts before executing them. Full development and review requirements are in [CONTRIBUTING.md](CONTRIBUTING.md).

## Evidence and limitations

The latest [P39c native import evidence](docs/parity/p39c/README.md) covers explicit member/System selection, pure preparation, disclosure review, checked Apply, one-step Undo/Redo, refusal cases, editing, invalid drafts and exact save/cold reopen. Full 835-case/build evidence belongs to its corrected source; a later display-only source rebuilt and passed cold native checks. It is not a final-source full-suite claim or vendor-created No Magic interoperability qualification.

Each earlier result remains bound to its original source, binary and workflow. The [historical README](docs/history/README-before-main-promotion.md) retains older epoch summaries and captures. [Publication evidence](docs/PUBLIC_READINESS.md) records the retained history, bounded secret scans and publication scope. No release, production acceptance, general language conformance or engineering approval is implied.

## Licenses

Original application code is MIT. Reused source retains its upstream licenses and attribution, including Kernspan, syntax, Craft, egui/winit and fonts. See [LICENSE](LICENSE), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and the pinned provenance manifests. No upstream brand endorsement is claimed.
