# CATIA reference crosswalk

Read-only assessment, 2026-10-22. Reference root: `/workspace/repos/builderbob/kernspan/work/references/catia-workflow-20261022` (below, `REF`). MBSE-Craft source inspected at `860fbad0a24926509d359ca1e40c61c7fed0cbd5`; subsequent commits need their own verification. This assessment inspected references and source, and did not run CATIA, evaluate archived expressions, execute model scripts, change Kernspan, or copy proprietary assets into MBSE-Craft.

## Evidence and authorship

`REF/TRANSFER-VERIFICATION.json` records 624 matching payload files/443530323 bytes at 2026-10-22 20:19:43 UTC. This review independently read and SHA-256 checked **16 selected files** against `destination-manifest.json`: all ten model outputs, index.csv, coverage_summary.md, and screenshots0103/0265/0267/0268. It also verified the destination manifest's own hash, below. That verifies the selected transferred bytes; it does not prove the native model runs today or certify all 624 files independently.

`curated-study/index.csv` contains 287 selected states (167 V1,120 V2). Coverage describes two recordings totaling02:14:12.48, sampled and curated rather than a keystroke transcript. Audio was not transcribed; gaps do not establish inactivity. `evidence/independent_qa.md` reports42 independently decoded matching frame comparisons and corrected draft labels; that is the earlier study's QA claim, not a fresh source-video decode here. The recordings themselves were not part of this review.

The session log and impressions are **supplied authored artifacts**, not direct statements attributable to Cole. The impressions explicitly distinguish quantitative execution from descriptive scenario behavior. Recording/index observations, narrative completion claims and this crosswalk's proposals retain separate evidentiary status. Earlier study statements saying no BuilderBob copy describe its original generation stage; the later transfer receipt and intake README document the authorized copy.

## Current native archive versus backup

The current `.mdzip` is230071 bytes; `.bak` is229452 bytes. The log's historical229452-byte saved-file observation therefore matches the backup's size, not the current archive. Both ZIPs were inspected in memory with Python zipfile/XML parsing only. Both contain six UML diagrams, three UAF Capability applications, one EnterpriseMission, one EnterpriseGoal, one EnterpriseObjective, two Organizations and two Concerns. Both main model XMLs contain the same three repaired Built-in Math equations and defaults primary4, backup3, required8, target80. They have different XML/layout/metadata bytes; this review does not equate the two complete archives or assert why they changed.

The current main XML member `com.nomagic.magicdraw.uml_model.model` is99932 bytes, SHA-256 `1a1a05bdab5fc9819c213b22b7423aa462ca958763e598750705bcec7b001433`; the backup member is99332 bytes, SHA-256 `79f89650d3e134819f92d821b0cc62537273db8a35466010e038c10ba8860ef0`. Inspection confirms guarded coverage, target comparison and two English guard bodies. Five CallBehaviorActions have no assigned behavior attribute. Archive contents support saved structure; the recorded simulation states support observed execution separately.

## Workflow comparison

| Reference observation | MBSE-Craft source evidence / consequence |
| --- | --- |
| V1 00:20:10 exposes expression-language selection inside specification dialogs. The supplied impressions describe nested dialogs and perspective-dependent discovery as authoring friction. | `crates/mbse-desktop/src/ui.rs` already exposes editable left controls, typed calculation inputs/expression/output units and click Details popouts. These are implemented source paths, not proof of faster task completion or usability superiority. Preserve discoverable labels and visible invalid drafts in GUI verification. |
| V1 00:32:00 screenshot0103 shows required0, coverage Infinity, targetMet=true; V2 00:05:50 indexed state shows NaN with required8. These are different intermediate failures, not the final saved equation. | `stage1_calc.rs` has finite-number validation and DivisionByZero diagnostics (lines298,320 at assessed source). Require GUI evidence that invalid values show a diagnostic without a fabricated result, and restoring a valid input recomputes. Do not claim every CATIA equation suffers this defect. |
| V2 00:49:10 screenshot0265 shows87.5/true after the reopen sequence;00:50:30 screenshot0267 shows required0/-1/false;00:51:10 screenshot0268 restores87.5/true. The capacity PNG documents that-1 is an invalid-input marker. | CATIA demonstrates a real native quantitative author/edit/test/reopen workflow. MBSE-Craft has save/recovery and calculation source paths, but comparable workflow evidence must come from its actual build. Prefer explicit validity/status to a numeric sentinel; the CATIA model's `coverage >= target` is safe for the documented80 target but-1 is not a universal invalidity guard for arbitrary negative targets. |
| Six exported PNGs directly inspected: strategy, stakeholder/concern, capacity, scenario, context, mission-capability traceability. Reused MCOAE and capabilities have native identities/links; native XML includes Dependencies/Abstractions and non-composite context Associations. | CATIA's strength is a connected model with multiple purpose-specific diagrams and native UAF/SysML/UML tools. MBSE-Craft's typed record canvas and relation ledger do not implement that full taxonomy or view repertoire. Preserve System purpose, neutral Mission accomplishment, typed Objective subject and custom Measure metadata without implying UAF equivalence. |
| The scenario PNG has two swimlanes, five actions and two English-guard branches; the unavailable branch retains unresolved capacity. Native action behavior links are absent. | This is a descriptive operational scenario, not evidence of executed handoff automation. Activity/swimlane editing and native UML behavior execution remain MBSE-Craft gaps; do not replace them with decorative cards and call parity achieved. |
| V2 00:47:10 Welcome,00:47:45 perspective notice,00:48:20 trace view, then repeated calculation form a reopen sequence. Log separately reports successful save/reopen. V1 01:05:20 is an on-screen unresponsive report, not a diagnosed crash. | MBSE-Craft source already separates source/action and save paths, preserves dirty navigation through verified save/recovery, and retains invalid drafts. Its explicit Recovery copy is not automatic recovery. Validate actual failed-save, reopen and close-cancel paths; the earlier reported CATIA interruption is not evidence of a remaining final save failure. |
| V2 00:57:23 indexed export dialog selects six diagrams in PNG; six native PNGs exist in supplied model outputs. | CATIA has convenient native batch diagram export. MBSE-Craft screenshot capture is a verification aid, not an equivalent diagram-export workflow. A future authored diagram export needs explicit scope, metadata and acceptance. |

## Scoped next improvements (proposals, not accepted product decisions)

1. Verify the existing left inspector, typed calculation editing, relation ledger, Details popouts, Undo/Redo and preservation prompts against actual native GUI workflows. These source improvements already exist; screenshots and independent workflow outcomes belong to the verifier.
2. Keep complete relation labels/IDs available in the ledger at high density. The current fixed-grid canvas is not a general layout/router; propose focused relation highlighting, stable selection and legible zoom before claiming CATIA-style free diagram composition.
3. Expose calculation validity next to inputs/results and distinguish accepted values from pending drafts. Require zero-denominator, unit mismatch, invalid text and restored-input workflows; never present-1 as a legitimate computed percentage by borrowing the reference convention.
4. Preserve stakeholder interests separately from external operational actors and environment. Richer Actor/Environment/Organization property editors, purpose-specific views and scenario semantics remain gaps documented in `docs/UI_WORKFLOW.md`.
5. Retain the reference's strengths as future requirements candidates: element reuse across views, explicit stakeholder/concern links, native activity guards, editable layout and batch export. No broad CATIA replacement or superiority conclusion follows from this compact session.

## Selected SHA-256 anchors

All paths are relative to `REF`; source timestamps above are recording-relative, not wall-clock times.

| Path | SHA-256 |
| --- | --- |
| destination-manifest.json |9f0f7db923a99ec24335be0470c63fd6dd856231341d398cfcba48ee07df6c6c |
| curated-study/index.csv |87f759c6129c34849855e78db83d7d9c74a5e639ec62835138e6b4d2c52ecc0e |
| model-outputs/MCOAE Incident Coordination.mdzip |7beaf024aee883c5a1117b6e55bd4e2898c26d4ddbfc308dcd08fd3839c07f8a |
| model-outputs/MCOAE Incident Coordination.mdzip.bak |dba0267f9149a0d63cf7729bfb1af25f88d6497e136455e18651756171e7c94c |
| model-outputs/MCOAE-session-log.txt |1719091b36d796717cadcdd14877eb251be4a3f11fa1ec7fd26f4341518e1e0a |
| model-outputs/MCOAE-impressions.txt |177e262bb87dc8dbc76eff3bf19e29944d4bdc54123dced69e2e73b4f3c7d86d |
| curated-study/screenshots/0103_V1_00-32-00-000_infinity-result.png |a94a3f24c950b3704e6209066b1c8ff41c61e13ce1f67e9ef0ac343872448678 |
| curated-study/screenshots/0265_V2_00-49-10-000_simulation-87-5-after-reopen.png |40243eeebce2733be4378b3622a4fcf23c5131aca5fc9936d18f82c234679422 |
| curated-study/screenshots/0267_V2_00-50-30-000_zero-required-invalid-after-reopen.png |ac6504d2647a322a0c8a24f9ebd2a373440815472068490f350fd7b368486ff9 |
| curated-study/screenshots/0268_V2_00-51-10-000_required-restored-after-reopen.png |48b270d7fc04f15451e6766581ccacc64e89343d1ed36358b62a3daabada2728 |

References remain private and read-only. Watermarked screenshots, exported diagrams and archived model bytes stay outside the MBSE-Craft repository; this independently authored textual comparison grants no redistribution rights to proprietary software or artwork.

## Reconciliation with independent Kernspan comparison r02

Read-only comparison package `/workspace/repos/builderbob/kernspan/work/stage1-completion-20261022/intake/CATIA-WORKFLOW-COMPARISON-r02.zip` was inspected;45260 bytes, SHA-256`4254d326052a6b9f50cdf301baf99a97954af181370e81636a10d541cf564a58`. Its `COMPARISON.md` agrees on distinct current/backup CATIA model versions, sampled recording limitations, richer CATIA views and narrower typed native workflow. It reports independent screenshot checking by its analyst; MBSE-Craft does not relabel that as its own287-frame audit.

Count rejection for negative/fractional/greater-than-2^53-1 input already exists in the shared Kernspan evaluator. Corresponding MBSE-Craft checks are inherited regression evidence, not new differentiation. Similar87.5 arithmetic under different targets/fixtures does not establish parity or superiority. CATIA Boolean target assessment and native activity/diagram family remain explicit gaps; local numeric results and preserved drafts are not substitutes for those semantics.
