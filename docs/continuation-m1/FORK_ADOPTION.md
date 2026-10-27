# Executable paired-fork adoption preparation

Prepared on `lane/fork-m2` from inspected clean M1 integration commit `85dfe0de1536ab036b0ee9b31707530ef1b8a33b`. This commit stages source and an exact patch template; **the active application Cargo.toml/Cargo.lock and M1 native verification epoch are unchanged**. The integrator applies the pair after recording M1 evidence, regenerates one lock, and sends that exact build to the sole verifier. Adoption is the next concrete integration batch, not a permanent disclaimer or an optional-by-omission decision.

## Exact sources, versions and size

| Source | Upstream pin / branch | Staged path | Retained inputs, excluding SOURCE.json |
| --- | --- | --- | --- |
| Storytold egui |767b1cad294120689a514bb4bd68c077e46679b9, craft/0.36.2; upstream base49682f8baa058bf49e011035cfbd6e825f88a5ef |vendor/egui-fork |308 files /5376682 bytes |
| Storytold winit |ffae5198770d7730e718043f035a772b7dbc3b1b, craft/0.30.13 |vendor/winit-fork |203 files /2314728 bytes |

The raw read-only Git checkouts remain outside the repository at `/workspace/worktrees/builderbob/mbse-craft/upstream-egui-fork-review` and `upstream-winit-fork-review`; their source trees exclude Git metadata:9364657 and2764000 bytes respectively (disk usage including Git23M/8.4M). No .git, upstream workflows, examples/demo applications, CI tooling, release scripts or unrelated brand assets are retained. The twelve retained egui-family crates are ecolor, emath, epaint, epaint_default_fonts, egui, egui-winit, egui-wgpu, egui_glow, egui_inspection, egui_extras, egui_kittest and eframe. Optional inspection source is present for dependency closure; the application must not enable its TCP inspection feature.

All egui packages are0.36.2, edition2024, MSRV1.95. Winit is0.30.13, edition2021, MSRV1.70.0. The previously established host compiler1.99.1 exceeds both; the adoption build must record its actual compiler rather than infer continued availability. This is the compatible stable branch, **not** Storytold main0.37.0. There is no expected public egui0.36 type-version migration. The extra eframe ViewportHooks field may affect literal constructors; the application uses NativeOptions defaults and must still compile to establish API compatibility.

## License, assets and source preservation

Egui code retains complete MIT/Apache license texts. Default-font embedding is an actual dependency of the current application, so its required font binaries and complete notices remain in epaint_default_fonts/fonts: Noto OFL1.1; Ubuntu UFL1.0; emoji-icon-font MIT; Hack's MIT/Bitstream Vera/public-domain provenance. The exact font bytes are unchanged and cannot be relicensed merely because application code is MIT. Eframe's compiled default icon data/icon.png is retained because production epi_integration includes it; it is covered by the upstream code distribution, not imported ArtCraft branded artwork. No font-generation Python tool is retained or run. Winit retains Apache2.0 LICENSE and all original source headers/PATCHES.md. Redistribution must carry these notices alongside existing ArtCraft and transitive-dependency notices.

All production Rust source, selected runtime assets, winit/build.rs and license bytes are unchanged. Only extraction manifests differ: egui's containing workspace lists retained members and omits the absent demo dependency; egui_glow/epaint declarations of excluded example/benchmark targets are removed; winit's workspace excludes local dpi and its omitted example declaration is removed. Winit's production dpi dependency already uses the registry in this upstream fork; no Rust code is altered. Every retained input is hashed and its upstream-byte identity recorded in SOURCE.json; no source SHA is claimed for generated custom manifests.

Source inventory hashes:

- vendor/egui-fork/SOURCE.json: `b70754de0176987de02df8eab580718da2ab19d45d6507929cbcbc8a11eb04e2`.
- vendor/winit-fork/SOURCE.json: `ac2f35b17a421c3fcfa57069a83da654d02e1a143a833f16ebc665fbc5e713b9`.

Literal include_str/include_bytes inputs in retained noncomment Rust lines were statically checked to exist. Manifest syntax was parsed with Python tomllib. These checks are not Cargo compilation or behavior validation.

## Build-script audit before execution

No retained egui crate has build.rs or a declared custom build script. Winit's1004-byte build.rs, SHA-256 `eda62c3ebf765708bf1608b51ccf77501b4aa0253214c419f813c6d08d640b1a`, was read in full: it invokes cfg_aliases for platform/feature aliases and prints rerun-if-changed/rustc-check-cfg. It runs no process, performs no network access or downloads, and contains no direct filesystem writes. Its cfg_aliases constraint0.2.1 permits the already-audited cached0.2.2. That cached manifest hash is714317efebb5635660abe816709d2a7e5831cd9496bc169d2f898c4bbcf470c1; src/lib.rs hash fdec633aa893dc52e6aa6e9ff6738e828c18aae0bc8499c1288273cdd62c99f7. Its macro reads Cargo cfg/profile environment and emits cfg declarations. A newly resolved version would need fresh review.

The pair's remaining target/transitive graph must be compared to the audited dependency inventory during lock resolution. This audit does not authorize automatically executing unseen new dependency build scripts. Keep existing isolated /workspace Cargo cache/target locations and one Cargo job. No Cargo commands or builds were run by this source-preparation lane.

## Apply the pair after M1 evidence is frozen

1. Cherry-pick this staged source commit into a new integration epoch after the current M1 native verifier finishes. Preserve old binary, source commit, lock hash, screenshots and evidence files.
2. Merge `docs/continuation-m1/PAIRED_PATCH.toml` into the **application workspace root** manifest. It patches every retained egui-family registry name and winit to exact local extracted paths. Do not patch just eframe or egui-winit. Root excludes must cover the nested vendor workspaces and crate directories as needed for Cargo workspace discovery; the vendored independent manifests supply their inherited fields.
3. Resolve a new lock with the existing isolated cache. Inspect every newly resolved source/build script before the first build. Preserve application renderer/display features for the first paired comparison; do not turn on Wayland/wgpu/inspection just by changing source identity.
4. Generate full `cargo metadata --offline --locked --format-version 1` plus `cargo tree --offline --locked --duplicates` and reverse trees for egui, eframe and winit. For each reachable egui-family package, assert one package identity, version0.36.2 and canonical manifest path under vendor/egui-fork. Assert one reachable winit0.30.13 under vendor/winit-fork and one compatible registry dpi identity. Reject residual registry/git copies in the reachable graph. Unused optional patch warnings do not establish duplicate active dependencies; record them accurately.
5. Build/test once with the reviewed root lock and pinned host toolchain, one job, owned /workspace target directory. Run focused existing M1 model/lifecycle/recovery tests and fork-specific static/unit checks justified by changes. The sole verifier then runs the actual paired native binary; do not claim prior M1 screenshots validate its new bytes.

## Patch applicability and actual native acceptance

The exact upstream PATCHES.md files are retained. Egui changes: E1 e7e37ad adds per-viewport hooks for native tablet inputs; E2/E3 b02321d bounds oversized wgpu surfaces and returns initial configuration/renderer errors; E4 c01acc2 routes Wayland clipboard through winit. Winit W15 exposes the Clipboard API E4 references: the pair is mandatory for Wayland+clipboard compilation/behavior. Winit also carries X11 W5 (replacement IME-context focus) and W11 (server-DPI priority), and other platform-specific changes. Registry/native behavior is not assumed equivalent because the app currently uses glow/X11.

| Batch | Executable acceptance / evidence | Remaining target-specific gate |
| --- | --- | --- |
| F1 identity/build | New root source/lock/compiler/dependency/build hashes; one egui family+winit identity; actual binary starts on owned authenticated display | Compilation alone does not prove UI behavior. |
| F2 X11 focus/edit | Use actual XTest input: focus/re-focus a model field after switching selection/popout, type/commit text, Undo/Redo and save/reopen; inspect saved bytes. Include replacement-focus path if the test environment exposes it. | Basic Latin input is not proof of W5 IME composition. No configured IME means named IME acceptance gap, not a claimed fix validation. |
| F3 X11 DPI | Separate owned X servers at96 and144 DPI; preserve resolution/physical size evidence; record pixels_per_point and actual screenshots/control positions; verify usable selection/edit/close at both scales. | Two virtual server DPIs do not establish mixed physical monitors, desktop XSETTINGS overrides or all HiDPI configurations. |
| F4 lifecycle | Normal/custom app frame close, minimize/restore, hidden dirty close, Cancel and durable save/recovery continuation all use real paired binary; no stale permit may discard work. | Other platform native termination/menu behavior requires its own host. |
| F5 Wayland | Compile feature graph with paired W15+E4 when separately selected; real compositor clipboard copy/paste and file drops, pending clipboard lifetime, seat removal and second data-device behavior | No running approved compositor means honest named compositor gate; Xvfb cannot close it. |
| F6 wgpu/tablet | E2/E3 initial-surface/error-fallback and oversize tests when wgpu selected; E1 per-viewport pen route requires actual tablet/native events | Current glow/mouse-only X11 verification does not exercise these paths. |

F1–F4 are the concrete next adoption epoch. F5–F6 remain capability-specific acceptance boundaries after the pair is linked, not reasons to leave compatible sources permanently unused. Do not administer displays, install IME/compositor services, change security/network settings or claim other-platform acceptance to satisfy a test. Existing task permissions allow the integrator to apply these prepared reversible source changes and perform owned Linux verification; no merge, release, paid/provider or unrelated infrastructure action is included.
