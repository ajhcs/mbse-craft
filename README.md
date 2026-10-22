# MBSE-Craft

An independent native Rust systems-modeling workbench. It reuses the attributed MIT Kernspan Stage1 core and integrates pinned ArtCraft `craft-atomic` persistence and `craft-ui` controls. It has its own native shell and preserves the bounded Kernspan storage and exchange profiles; it is not a full ArtCraft application or general SysML implementation.

## Build on Linux

Rust 1.97 or newer is required (the recorded BuilderBob toolchain is 1.99.1). The native GUI uses eframe/egui 0.36.2 with X11/OpenGL. Linux needs X11, keyboard and OpenGL runtime libraries; private test display uses an already installed Xvfb. No MySQL, Redis, Node, provider credentials or ArtCraft service is needed for this offline integration.

Keep source, cache, temporary state and build output under Samsung-backed `/workspace` on BuilderBob. With this repository checked out under `/workspace/repos/builderbob/mbse-craft`:

```bash
cd /workspace/repos/builderbob/mbse-craft
mkdir -p /workspace/build/builderbob/mbse-craft/{target,tmp,state} /workspace/cache/builderbob/mbse-craft
export CARGO_HOME=/workspace/cache/builderbob/mbse-craft
export CARGO_TARGET_DIR=/workspace/build/builderbob/mbse-craft/target
export TMPDIR=/workspace/build/builderbob/mbse-craft/tmp
bash scripts/dev.sh build --locked -p mbse-desktop
bash scripts/dev.sh test --locked --workspace
/workspace/build/builderbob/mbse-craft/target/debug/mbse-desktop --state-dir /workspace/build/builderbob/mbse-craft/state
```

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

Storytold egui HEAD0.37 is not used; registry egui0.36.2 supplies the native type family. AI/service/plugin/launcher/CAD/font/catalog protocol compatibility is unqualified. The restricted `artcraft-services` workspace, unlicensed shared workflows and trademark assets are not bundled. `kernspan.stage1.sysml/1` exchanges only the closed custom domain profile, with atomic refusal of unsupported semantics; syntax success is not SysML conformance.

## Acceptance status

Implementation and independent Linux verification are in progress. The draft PR is not a release. Test/build receipts and actual native screenshots will identify their precise source and executable hashes. Windows native behavior, AT-SPI accessibility, physical FPS/input latency, engineering acceptance and full ecosystem compatibility remain unqualified.

## Licenses

Original app code is MIT. Reused Kernspan core retains its MIT copyright; the pinned syntax crate is Apache-2.0; Craft libraries are MIT OR Apache-2.0 with complete NOTICE/ATTRIBUTION retained. See [third-party notices](THIRD_PARTY_NOTICES.md). No upstream brand endorsement is claimed.
