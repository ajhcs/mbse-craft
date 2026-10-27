# ArtCraft integration assessment

Observed 2026-10-22. This is a source assessment and implementation contract, not proof of runtime or full-suite compatibility. `UPSTREAM_SOURCES.json` records full commit IDs, root license-file Git blob IDs and complete workspace manifests. Upstream may change after these pins.

## Selected integration and architecture

MBSE-Craft retains its own typed systems-engineering model, commands, calculations, document format, SysML subset and UI policy. It uses real upstream code from `storytold/craft-libs` at `b1401e33b105ca2861575ede90f67f4d54449001`: `craft-atomic` for atomic document/recovery writes and `craft-ui` for native egui controls and theme mechanisms. The selected crate source/manifests/tests and notices are unchanged; only their containing workspace manifest is replaced by a minimal extraction workspace. Per-file SHA-256 and modification disclosure are in `vendor/craft-libs/SOURCE.json`.

Use egui/eframe **0.36.2**, with `craft-ui` compiled against the same egui version. Current `storytold/egui` HEAD is **0.37.0** and must not be substituted into this contract: egui public types are version-specific. Rust >=1.95 is required by selected craft-libs metadata; installed compiler suitability and registry dependency availability must be established by the integration build. No compatibility with the Storytold egui fork is claimed by using registry egui.

Concrete shared interfaces:

- `craft_atomic::atomic_write(path, bytes)` stages beside the destination, flushes, then renames. `WriteOptions::default().with_new_file_permissions(NewFilePermissions::Private).atomic_write(...)` is the explicit private recovery policy.
- `craft_ui::theme::WidgetPalette::apply_to` updates widget-state backgrounds, radius and foreground while the app owns its entire visual theme.
- `craft_ui::buttons::IconButton::show` returns egui responses with accessible labels/focus; the app draws original symbols and dispatches model commands.
- Upstream docking, tabs, layout and printing controls are available in the unchanged UI crate but are not established application behavior merely by being linked.

Both selected crates have no `build.rs` or explicit Cargo `build` entry. On Linux craft-atomic uses std only; its rustix dependency is Apple-target-only. craft-ui production dependencies are egui and optional serde. Their manifests and selected source trees were inspected before build execution. Transitive registry crates/build scripts still require the integrator's action-time dependency review; this assessment does not clear unseen downloaded code.

The root application workspace must exclude `vendor/craft-libs` and use path dependencies to its crate directories. The nested workspace supplies inherited package/dependency/lint fields without importing unrelated PDF/color/print/platform components. No upstream scripts, services, AI providers, signing or release workflows are executed by this extraction.

## Ten-repository matrix

All links below use immutable source revisions. License assertions describe actual files, not README badges alone.

| Repository / pinned revision | Architecture and version | License and integration status |
| --- | --- | --- |
| [craft-common-actions](https://github.com/storytold/craft-common-actions/tree/ea8c0f57afc5ae8a8724f9605d4de8c21cee2694) | Composite GitHub actions and reusable packaging/release workflows. Includes shell/PowerShell, signing, downloads, VMs, Python and packaging tools. No Rust app API. | No root license file in inspected tree. Reference only; do not copy or execute its workflows based on public visibility. No releases/signing authorized. |
| [craft-ai-integrations](https://github.com/storytold/craft-ai-integrations/tree/97dc9962515a4dfca576e2f2f4fa34582dfe8a7b) | Rust runtime adapters, connection store, consent/jobs, provider clients and egui controls. Workspace0.1.0, edition2024, base MSRV1.90; egui widgets need1.95. Its local core copy has additions beyond craft-libs; dependency switch gated by CORE-SYNC. | MIT OR Apache-2.0 code, actual dual license texts. Selected backend/model assets can have additional conditions, e.g. Candle engine Apache-only NOTICE. No AI code or models needed or adopted; hosted providers not runtime-validated upstream. |
| [figcraft](https://github.com/storytold/figcraft/tree/80f916326fb94f3a88cb24e64f396f564a0ebc06) |0.10.0, Rust1.95, egui/eframe0.36. Native/web design app with persistent document/command engine, CLI/MCP, format/render, collaboration and metered wasm plugin interpreter. | MIT OR Apache-2.0 with NOTICE/ATTRIBUTION. Brand assets excluded under trademark terms. Optional font licenses separately retained. Architecture reference; no `.figcraft`, plugin, MCP or collaboration compatibility claimed. |
| [craft-libs](https://github.com/storytold/craft-libs/tree/b1401e33b105ca2861575ede90f67f4d54449001) |0.1.0, edition2024, Rust1.95. Small shared crates for atomic storage, UI, settings, paths, windows, localization and optional media/PDF/printing. | MIT OR Apache-2.0 with actual NOTICE and per-asset ATTRIBUTION. **Selected atomic+UI integration**; no bundled artwork, font, PDF or ICC assets in extraction. |
| [artcraft-services](https://github.com/storytold/artcraft-services/tree/e57ee3d63ac478a24149384bf1473ef97aa067f8) | Rust Actix/Tokio backend, shared HTTP clients/types, asynchronous provider workers; React/TypeScript Nx/Vite frontend, separate Next.js site. Backend needs MySQL/SQLx, Redis, Elasticsearch and S3/R2 plus applicable credentials. | **Custom ArtCraft License (WIP), not MIT/Apache.** Allows private personal use; restricts commercial sale, competing businesses/products and removal of community/donation/paid-service links; trademark promotion permission separate. Source reviewed only. No code copied or executed. No service/provider API use needed. |
| [egui fork](https://github.com/storytold/egui/tree/bf5ac0dac137af9a4861611f3d2a77ad531f50e3) |0.37.0, Rust1.95, edition2024, eframe/winit/wgpu and rendering workspaces. | MIT OR Apache-2.0 actual license texts; embedded font assets keep separate notices. HEAD differs from craft-ui's0.36 public-type contract. Not adopted: use pinned registry0.36.2 and state this boundary. |
| [craft-contrib](https://github.com/storytold/craft-contrib/tree/2e87f7de01fb1638354b8fcdf141605450103dd3) | Contributor guidance and RFC decision process; no implementation library. | No root license file in inspected tree. Read-only reference, not copied project policy or runtime dependency. |
| [craft-fonts](https://github.com/storytold/craft-fonts/tree/43a994403242b287ab4c78499b7ff7d943ea6dba) |0.1.0, Rust1.90 code plus optional external font build input via CRAFT_FONTS_DIR. Manifest records source/asset hashes. | Code MIT OR Apache-2.0; font files individually OFL1.1 with attribution/license files. Not bundled initially; shipping any font must retain its exact license/attribution. Registry egui default font obligations remain separate. |
| [craft-i18n](https://github.com/storytold/craft-i18n/tree/554df2c4d8345de6b084adeabca8ff8117f910e3) |0.1.0/Rust1.90 catalog/test workspace. Optional TSV catalogs via CRAFT_I18N_DIR; lookup lives in craft-libs. App owns English keys/context/overrides. | Code/translations MIT OR Apache-2.0; contributor provenance in ATTRIBUTION. No automatic translation or catalog compatibility claimed; not bundled. |
| [cadcraft](https://github.com/storytold/cadcraft/tree/8f988f630dd63ce5d9c997f5ef28a09dd1bdfcde) |0.11.0, Rust1.90, egui0.36. Native/web CAD engine, constraints, DXF/DWG, command API, CLI/MCP. | MIT OR Apache-2.0, NOTICE/ATTRIBUTION; DWG dependency acadrust MPL2.0, brand assets separately restricted. No CAD engine/fonts copied; no DWG/DXF/constraint/MCP compatibility claimed. |

## Distribution and identity boundaries

Selected craft-libs MIT/Apache license texts, complete NOTICE and ATTRIBUTION are retained both beside the extracted code and in `licenses/artcraft/`. Preserve these with source and binary distributions; identify modifications in the extraction manifest. Full NOTICE references excluded components for completeness, without claiming those components are shipped. The UI source provenance includes PhotoCraft, VectorCraft, PdfCraft, FilmCraft, EffectCraft and DesignCraft contributors; those notices remain intact.

MBSE-Craft is independent: no ArtCraft logo, name-as-endorsement, mascot, branded assets or upstream release packaging is imported. MIT/Apache code permission does not confer trademark permission. If later copying fonts, PDF backends, catalogs or full-app engines, re-inspect their precise file/asset notices rather than assuming this selection's obligations cover them. Review transitive Cargo.lock dependency notices before binary distribution.

## Executable acceptance contract

The integrator owns actual test execution and records exact commands, outcomes, build/source hashes and genuine GUI screenshots. A passed source assessment alone does not satisfy these criteria:

1. `cargo tree` proves one egui0.36.2 type family and actual craft-atomic/craft-ui path dependencies; `cargo build --locked` succeeds from documented /workspace paths after dependency review.
2. Actual document/recovery save paths call craft-atomic, with explicit private recovery creation; actual UI calls a shared craft-ui control/theme mechanism. Dead dependency entries do not count.
3. Save/reopen retains all typed model IDs, relation fields, units, calculations and custom measure/MOE/MOP metadata. Unknown schema, invalid model and malformed input return actionable errors and retain the current model/file.
4. Injected atomic staging/rename failures retain old project bytes and surface a save error. Recovery data stays available until an explicit user choice; a failed save never marks the model clean.
5. Isolated Linux GUI workflows exercise editing, typed relationships, invalid drafts, calculations/unit errors, undo/redo, save/reopen and recovery, with no interference with Kernspan's display or files.
6. The README reports supported SysML subset, independent file format, unimplemented integrations and source/build evidence. It does not claim full ArtCraft suite, SysMLv2, source-editor or service compatibility.

Atomicity limits: directory sync is best effort, physical power-loss durability is unproven, abnormal termination may leave craft_tmp staging files, and atomic replacement does not preserve owner/group/ACLs/xattrs/hardlinks. The app owns orphan recovery and conflict policy. craft-settings, if later adopted, is last-writer-wins rather than a multi-process locking protocol and writes config files; use explicit /workspace settings paths in tests.
