# Contributing to MBSE-Craft

MBSE-Craft is an experimental native Rust workbench. Start with the [roadmap](docs/PARITY_ROADMAP.md), [current development status](README.md#current-development-and-contribution) and the evidence for the behavior you intend to change. Implementation positions and broad product acceptance have different completion gates. Full SysML, No Magic, Craft, platform and engineering parity remain open.

## Choose the development base

PR #1 is merged and the repository is public. Clone or fork current `main`; it contains the reviewed work through P39. Retained implementation/review branches and older lane branches are history and provenance references.

```bash
git clone https://github.com/ajhcs/mbse-craft.git /workspace/repos/mbse-craft-contributor
cd /workspace/repos/mbse-craft-contributor
git worktree add /workspace/worktrees/mbse-craft-my-change -b codex/my-change HEAD
```

Choose unused paths. On BuilderBob, verify the Samsung `/workspace` with `bob-admin workspace-status` before creating working data. Keep the canonical checkout and other worktrees untouched. Follow [AGENTS.md](AGENTS.md); one integrator owns the integration branch. Use current remote `main` as the base of each new contribution.

For an external contribution, fork the repository, make a focused branch from the current integration base and open a draft PR. Coordinate a roadmap position or a small defect first so two people do not replace the same implementation. No separate model, calculation or persistence authority should be introduced to duplicate existing contracts.

## Build and verify

Recorded qualification uses Linux X11/OpenGL, Rust 1.99.1 and one Cargo job. Cargo metadata declares Rust 1.97; that minimum and other operating systems have not been qualified. Install an appropriate Rust toolchain and the X11, keyboard and OpenGL libraries before building. An initial registry resolution needs network access or a populated isolated cache; vendored forks do not include every registry dependency.

Use the [README build example](README.md#build-on-linux), selecting dedicated `/workspace` source/cache/target/temp/state/config paths and setting `MBSE_RUST_BIN` to the **bin directory of your actual installed toolchain**. `scripts/dev.sh` calls those binaries directly and fixes one Cargo job. A typical changed workspace regression is:

```bash
bash scripts/dev.sh test --locked --workspace
bash scripts/dev.sh build --locked -p mbse-desktop
```

On the shared BuilderBob host, obtain fresh exact-job admission using the existing job/native scripts and retain their receipt before compiling or starting a native test. Do not run concurrent Cargo/native jobs around a hold. An old PASS is not fresh admission. No provider keys, MySQL, Redis, Node, release, signing or service workflow is needed.

Some saved test helpers require `/workspace`, and the private native acceptance harness is configured for BuilderBob. These are current constraints, not a portable setup guarantee. Report a host-specific failure with the command and environment; do not change fixtures or claim a skipped check passed.

## Keep changes reviewable

Use one PR per behavior family, with separate schema/core/adapter/UI changes where they need separate review. Preserve accepted native meaning, invalid raw drafts, identity, Undo/Redo and durable recovery. Never reset or clean someone else's checkout. Keep generated build/cache output outside tracked source. Preserve failed attempts and run the failed or unexecuted remainder rather than replaying passed work without cause.

Describe the trigger, resulting behavior and exact evidence in each PR. Record source/build/input identities and relevant tests. A changed native UI needs actual success, failure, cancel and stale-state checks where applicable; controller tests or screenshots alone do not establish a rendered workflow. Distinguish same-agent review from independent review. Documentation-only changes need accurate links and scope, not a new Cargo/native epoch.

Use original synthetic fixtures or explicitly authorized sources. Do not add credentials, private colleague/customer data, proprietary vendor models or restricted upstream assets. Preserve [third-party notices](THIRD_PARTY_NOTICES.md), upstream licenses, provenance and pinned manifests. Review added dependencies and build scripts before execution. Original contributions use the repository's MIT license; retained upstream material keeps its own terms.

## Finding a first task

P40 is the next planned position. Read its acceptance criteria and dependencies before proposing its implementation. Smaller useful contributions include a reproducible defect in an existing native workflow, a focused failure/cancel regression, or a documentation correction backed by current receipts. Do not mark the broad acceptance matrix complete from a bounded test result.
