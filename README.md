# MBSE-Craft

An independent native Rust systems-modeling workbench. It reuses the attributed MIT Kernspan Stage1 core and integrates pinned ArtCraft persistence, UI controls, settings, quit, document-path and window-frame contracts. It has its own native shell and preserves the bounded Kernspan storage and exchange profiles; it is not a full ArtCraft application or general SysML implementation.

## Build on Linux

Rust 1.97 or newer is required (the recorded BuilderBob toolchain is 1.99.1). The native GUI uses eframe/egui 0.36.2 with X11/OpenGL. Linux needs X11, keyboard and OpenGL runtime libraries; private test display uses an already installed Xvfb. No MySQL, Redis, Node, provider credentials or ArtCraft service is needed for this offline integration.

Keep source, cache, temporary state and build output under Samsung-backed `/workspace` on BuilderBob. With this repository checked out under `/workspace/repos/builderbob/mbse-craft`:

```bash
cd /workspace/repos/builderbob/mbse-craft
mkdir -p /workspace/build/builderbob/mbse-craft/{target,tmp,state} /workspace/cache/builderbob/mbse-craft
export CARGO_HOME=/workspace/cache/builderbob/mbse-craft
export CARGO_TARGET_DIR=/workspace/build/builderbob/mbse-craft/target
export TMPDIR=/workspace/build/builderbob/mbse-craft/tmp
export MBSE_RUST_BIN=/workspace/cache/builderbob/global-tools/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin
bash scripts/dev.sh build --locked -p mbse-desktop
bash scripts/dev.sh test --locked --workspace
/workspace/build/builderbob/mbse-craft/target/debug/mbse-desktop --state-dir /workspace/build/builderbob/mbse-craft/state
```

BuilderBob’s generic Cargo/Rust wrappers override CARGO_HOME; this script uses the exact installed toolchain binaries to preserve isolation. Set `MBSE_RUST_BIN` to your installed toolchain directory on another machine.

The first dependency resolution requires existing registry access or an available isolated cache. Review dependency/build scripts before execution in restricted environments. `scripts/dev.sh` fixes one Cargo build job and refuses absent explicit output/cache/temp paths. No upstream release, signing or provider workflow is run.

## Modeling workflow

Start with a System, edit its name/purpose in the left controls, and add typed records. Missions describe intended accomplishment; capabilities reference same-System missions; objectives choose a typed System/Mission/Capability subject; Measures belong to explicit Objectives. Select cards in the canvas to edit, or open Details. The relation ledger lists every projected typed link, including links whose canvas captions would be crowded.

Accepted model values and incomplete raw drafts remain separate. Save/reopen/recovery preserve both. Native save status acknowledges the frozen model/raw generation. Recovery opens an independent dirty draft. Native `.kernspan` schema2 is retained for the shared bounded core; the separate application name does not imply a new general interchange format.

Calculations use the inherited bounded scalar/ratio/count/length/duration/speed evaluator and named typed inputs. Results are derived values, separate from observations and engineering acceptance. No MATLAB, Wolfram, external execution, equation solver or general SysML simulation is provided. UI workflow details and current verification findings are recorded separately.

## Compatibility and provenance

- [Implementation plan and acceptance criteria](docs/IMPLEMENTATION_PLAN.md)
- [Ten-repository source/license and compatibility matrix](docs/UPSTREAM_COMPATIBILITY.md)
- [Pinned upstream manifests](docs/UPSTREAM_SOURCES.json)
- [Model and persistence contract](docs/MODEL_CONTRACT.md)
- [Loaded guidance provenance](docs/GUIDANCE_PROVENANCE.json)
- [Kernspan source provenance](licenses/kernspan/PROVENANCE.json)
- [ArtCraft extraction hashes](vendor/craft-libs/SOURCE.json)

This candidate patches the complete egui0.36.2 family to Storytold craft/0.36.2 pin767b1cad and winit0.30.13 pinffae5198. The resolved lock has one identity per family crate; actual paired build/native qualification is a separate pending epoch. The frozen M1 binaries use the prior registry graph. Storytold main0.37 is not used. AI/service/plugin/launcher/CAD/font/catalog protocol compatibility is unqualified. The restricted `artcraft-services` workspace, unlicensed shared workflows and trademark assets are not bundled. `kernspan.stage1.sysml/1` exchanges only the closed custom domain profile, with atomic refusal of unsupported semantics; syntax success is not SysML conformance.

## Frozen alpha acceptance

Independent Linux verification passed 123 tests, including actual Craft streaming/rename fault preservation and corrupted-main recovery. Native workflows passed blank-model creation with typed links and a 2500 m to 2.5 km calculation, save/normal-close/relaunch; fixture-based invalid-draft preservation, typed relation edits, save failure/cancel safety, independent recovery and graceful close after keeping recovery. These are distinct fixture and blank-authoring checks, not general usability acceptance.

Exact build-input SHA-256: `323fe697051fe40a508dcab6d8d3979898a0da8c34a4a2007aa1ec30ee3ac6d7`. Linux debug executable SHA-256: `ddefaaaa73675b5e1aacd8aee6288cfdf513049c0e8cad88b15be894a7e4b587`. Build used Rust 1.99.1, one Cargo job, offline locked registry inputs, private Xvfb and software OpenGL. Recompute inputs with `python3 scripts/source-manifest.py`.

The draft PR is not a release. Windows native behavior, AT-SPI accessibility, physical FPS/input latency, engineering acceptance and full ecosystem compatibility remain unqualified. The canvas uses a responsive grid with basic crossing routes, explicit scope-link visibility and a complete relation ledger; this does not establish collision-free dense layouts. The continuation adds actor/environment/organization context editors and default-off periodic recovery; its evidence is recorded separately below. Generic CAD and service/AI integrations remain outside the implemented scope. See `docs/UI_WORKFLOW.md` and the independent acceptance artifacts under `docs/verification/`.

[Independent acceptance report](docs/verification/FINAL_ACCEPTANCE.md) · [machine-readable receipt](docs/verification/FINAL_ACCEPTANCE.json) · [exact source manifest](docs/verification/SOURCE_MANIFEST.json) · [blank-authored native model](docs/verification/models/blank-authored.kernspan)

![Actual blank-authored model reopened in the verified Linux binary](docs/verification/screenshots/blank-authored-reopened.png)

## Continuation implementation and qualification

The continuation adds typed context and affiliations, bounded recovery discovery, actual persistent preference policy and hidden-window close guards. The frozen external library browser indexes retained source without changing the active native model. Bounded MDZIP/UML, Capella and SysML text projections expose typed IDs, properties, reference endpoints, source spans, unknown semantics and original bytes. A partial projection is not native editable admission or execution; AIRD layout fidelity remains unsupported.

The independent requirements/evidence window authors a separate JSON sidecar with retained raw drafts, Undo/Redo and distinct maturity, implementation and verification states. Native evaluator receipts retain selected output kind/unit and the complete global result. Adequacy and verification require explicit review. Reopening retains historical decisions and clears live evaluator authority; current stamps alone cannot authenticate execution. Local external artifacts remain explicitly attributed inert evidence.

Frozen continuation qualification includes168-test native assurance/recovery and169-test resource epochs, plus earlier actual context/recovery, preferences failure/retry, future-schema read-only settings, hidden dirty close and readable library-preview workflows. Source and binary identities differ from the frozen alpha. Receipts live under [continuation verification](docs/continuation-m1/verification/). The projection and requirements workflows have scoped native receipts indexed in docs/continuation-m1/FREEZE_INDEX.json. Later audit-merge/draft corrections passed26focused tests. The activated paired Storytold egui/winit candidate still requires its own build and native regression checks. These open gates prevent a full interoperability or finished-product claim.

## Licenses

Original app code is MIT. Reused Kernspan core retains its MIT copyright; the pinned syntax crate is Apache-2.0; Craft libraries are MIT OR Apache-2.0 with complete NOTICE/ATTRIBUTION retained. See [third-party notices](THIRD_PARTY_NOTICES.md). No upstream brand endorsement is claimed.
