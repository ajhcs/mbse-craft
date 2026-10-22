# MBSE-Craft implementation plan

Integration owner: task 01a1653d-9c16-76a3-ad80-df98ba1e5c94, BuilderBob, builderbob. New isolated repository /workspace/repos/builderbob/mbse-craft. Host storage verified Samsung 990 PRO, writable. Kernspan remains read-only at c80037d2259cd67a8e813cf3bc6c01bb1226a460.

1. Pin all ten ArtCraft repositories and inspect source licenses, notices, dependency versions and scripts. Select actual reusable Rust contracts; do not execute services or signing workflows.
2. Reconcile model/persistence and native UI interfaces before integration. Preserve typed Stage1 relationships, purpose, neutral missions, typed objective subjects, measure metadata, finite calculations and units, invalid drafts, undo/redo, save/reopen/recovery. Retain MIT notices for reused source.
3. Integrate actual craft-libs library calls with pinned dependency identity. Distinguish shared-library integration from document/service compatibility. Keep unsupported integrations explicit.
4. Compile and test in dedicated /workspace builds with one Cargo job. Independently inspect source/build scripts before running. Exercise native GUI on isolated display, error/recovery, calculations, save/reopen. Capture actual frames and source/build hashes.
5. Review exact candidate independently; create focused commits and a draft PR if permitted. No merge/release.

## Acceptance criteria

- Actual imported ArtCraft shared-library calls are exercised by tests/application; crate presence alone fails acceptance.
- System purpose and same-system typed relations survive edits, undo/redo and save/reopen. Cross-system and wrong-kind references fail without accepted-model mutation.
- Valid dimensioned calculations produce independently checked results; invalid/incomplete/non-finite/dimensionally invalid input is retained and visibly rejected.
- Raw drafts and accepted snapshots remain distinct across persistence and recovery. Save failure must not claim Saved. Corrupt/truncated input must fail without destroying the current document.
- Actual Linux GUI demonstrates create/edit/select/calculate/undo/redo/save/reopen/recovery in isolated display. Screenshots bind exact binary/source hashes. Missing GUI capability remains an explicit unpassed gate.
- Dense relationship information has a discoverable visible route; Measure creation copy accurately describes incomplete defaults.
- CATIA reference observations cite verified timestamps/model output, separate inference, and confer no rights to copy proprietary code/assets.

## Initial compatibility matrix

| Surface | Decision / evidence status |
|---|---|
| Kernspan Stage1 | MIT source baseline pinned; public API closure under review |
| craft-libs | MIT OR Apache-2.0; exact crate/API and NOTICE review pending |
| storytold egui | HEAD 0.37 versus craft-ui/consumers 0.36; version reconciliation required |
| craft-ai-integrations | Local newer craft-ai-core differs from craft-libs; not a substitutable type identity |
| artcraft-services | Restrictive custom service license; no vendoring/execution selected |
| common actions | Reuse only reviewed opt-in checks; no signing/release workflows |
| Fonts/i18n/contrib/FigCraft/CADCraft | Source/license review pending; no compatibility assertion |
