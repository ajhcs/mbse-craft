# MBSE-Craft parity roadmap

Current execution status: [Milestone 1 results and PR stack](parity/MILESTONE1_RESULTS.md). Six positions are reviewed and merged into `implementation/native-alpha` with bounded evidence; full parity remains open.

Saved from the agreed 2026-10-24 roadmap. Proposed P-numbers are backlog positions, not GitHub PR numbers. The canonical checked data is [roadmap.json](parity/roadmap.json).

Build on the existing implementation and qualified local continuations. The matrix and acceptance package are [documented here](parity/README.md). Full portfolio, language and platform coverage requires the explicit unresolved-scope gates; neither this list nor a shared dependency proves parity.

## Rules for every PR

- Use assigned isolated worktrees and the existing integration owner. Commit only assigned paths; preserve original receipts, branches and dirty work.
- One contract or reviewable user workflow per PR. Separate schema migration, core behavior, adapters and UI whenever independent review requires it.
- Preserve native meaning, accepted values, invalid drafts, stable IDs, separate histories and durable recovery.
- Pin source/build/input identities. Run focused regressions for changed contracts and actual native success/failure/cancel workflows for changed UI. Earlier receipts prove only their original source.
- Review licenses/build scripts before execution; keep caches/output under verified /workspace with one Cargo job. No service, provider, release or signing workflow is authorized by this plan.
- Planning/fixtures/static validation cannot mark a feature accepted. Never sum historical counts into a new qualification claim.

## Ordering and sizing

The dependency graph uses conservative serial integration order. It does not reserve resources or dispatch other writers. P48/P50/P57/P58 require explicit child PRs per locale/font adapter, platform/backend, engine or construct family; they are not single implementation-sized changes.

## Milestone 1: Qualified development baseline

1. **Freeze the parity matrix and acceptance fixtures.** Acceptance: Every capability has a source, current status, owner role, scenario and completion condition; unresolved edition/platform scope stays visible.

2. **Complete readiness review of existing draft PR #1.** Acceptance: The integrated candidate passes relevant regressions and blank-author, calculate, save and reopen; documentation matches limitations.

3. **Complete native acceptance of existing draft PR #2.** Acceptance: One element appears in two views; independent geometry survives reopen; native-success/views-failure retains work; stale acknowledgments cannot save later edits.

4. **Integrate existing draft PR #3 layout recovery.** Acceptance: Layout-only edits recover after the timer; held drags defer recovery; disabled recovery stays inactive; partial failures retain geometry.

5. **Integrate qualified local navigation continuations.** Acceptance: Inspection, rehearsal, usages, search, cameras, System browsing and impact work together without identity, draft or layout regressions.

6. **Finish exact report-row occurrence navigation.** Acceptance: Exact cross-view occurrence selection works; hidden stays hidden; detached preserves native selection; stale and busy requests have no effects; focused and native gates pass.

**Milestone exit:** Create/edit, reuse across two views, search/usages/impact, save/recover/reopen on one reproducible candidate.

## Milestone 2: Presentation recovery

P07 is merged in [PR #10](https://github.com/ajhcs/mbse-craft/pull/10). P08 [live Apply/Undo evidence](parity/p08/README.md) covers the bounded milestone workflow; PR review and milestone engineering acceptance remain open.

7. **Version the presentation-copy format and provenance.** Acceptance: Old files remain readable; future and unknown versions stay preserved; malformed source-to-target mappings refuse without target mutation.

8. **Apply a reviewed presentation copy transaction.** Acceptance: Only selected occurrences copy; skipped mappings remain visible; stale source or target refuses; one layout Undo reverses the copy.

**Milestone exit:** Recover an independent native document, inspect old presentation, map selected occurrences, Apply once, save/reopen, and retain originals.

## Milestone 3: Craft workspace behavior

9. **Introduce a stable command catalog and consistent menus.** Acceptance: Menu, toolbar and keyboard paths use identical command checks and outcomes; disabled commands explain their prerequisites.

10. **Adopt scoped shortcut configuration.** Acceptance: Conflicts are visible; overrides persist; field Undo stays in the field; model and layout histories remain separately addressable.

11. **Adopt transactional panel docking.** Acceptance: Moving, tabbing and floating preserve panel identity, selection and incomplete text; invalid drops lose no panel or model data.

12. **Persist workspaces and recoverable perspectives.** Acceptance: Restart restores arrangements; Reset restores defaults; off-screen panels become reachable; corrupt settings preserve model state.

13. **Standardize file dialogs and long-operation feedback.** Acceptance: Cancel, failure and indeterminate outcomes differ; stale replies cannot replace work; background completion cannot discard edits.

**Milestone exit:** Complete baseline by menu and keyboard; rearrange, restart/reset workspace, and cancel operations without losing drafts.

## Milestone 4: Structural modeling semantics

14. **Introduce a versioned extension schema and migration.** Acceptance: Old documents migrate into a new copy with IDs, opaque fields and drafts intact; newer schemas refuse safely; Stage1 identity is not relabeled.

15. **Add type definitions and typed properties.** Acceptance: Wrong-kind references and invalid multiplicities refuse atomically; pending property text survives save and reopen.

16. **Add parts, references and decomposition.** Acceptance: Containment cycles refuse; moves preserve identity; the browser distinguishes actual containment from System membership.

17. **Add typed ports and interface definitions.** Acceptance: Incompatible assignments refuse; port identities survive cross-view reuse, save and reopen.

18. **Add connectors and conveyed items.** Acceptance: Wrong endpoints and item types refuse; views update consistently; removing a visual edge does not delete the semantic connector.

19. **Separate defaults, instance values and derived results.** Acceptance: Default changes affect inheriting instances only; overrides remain explicit; results never overwrite authored values.

20. **Add explicit clone and move operations.** Acceptance: Preview identifies internal and external reference treatment; moves retain IDs; clones allocate fresh IDs; Cancel and Undo restore state.

21. **Add semantic deletion with impact preview.** Acceptance: Every supported incoming reference is handled; unknown impact prevents an exhaustive verdict; stale previews refuse; Undo restores records and occurrences.

22. **Add exact profile and extension-tag identity.** Acceptance: Namespace/version mismatches stay unknown or refuse; tags round-trip; custom domain records are not silently reclassified.

**Milestone exit:** Author a typed subsystem with two instances, ports and exchange; reuse, override, clone, move and delete with valid references and recovery.

## Milestone 5: Requirements and validation

23. **Add hierarchical requirement authoring.** Acceptance: Duplicate IDs and parent cycles refuse; invalid statements persist as drafts; maturity, implementation and verification remain independent.

24. **Add typed requirement traces.** Acceptance: Derive, refine, satisfy, verify and copy have distinct endpoint rules; relabeling cannot change meaning; traces survive history and reopen.

25. **Add coverage queries and change impact.** Acceptance: Known orphan requirements and missing verification links are found; changed requirements stale affected evidence; unsupported relations are disclosed.

26. **Add named validation suites and issue navigation.** Acceptance: Seeded type, reference and structural faults produce stable expected rule IDs; issue navigation selects the exact element and property.

27. **Add reviewed custom rules and explicit quick fixes.** Acceptance: Unsupported declarative rules refuse; fixes preview changes, reject stale state and Undo atomically; opening projects executes no embedded code.

28. **Add editable relation matrices.** Acceptance: A cell edit uses the authoritative typed relation command; wrong-kind cells are disabled; matrix and diagram agree after reopen.

29. **Add viewpoints, filters and exposed scope.** Acceptance: Filters change presentation only; stakeholder concern and viewpoint scope remain inspectable; switching preserves identity and independent layouts.

**Milestone exit:** Trace a requirement hierarchy to a subsystem and evidence, find a known coverage gap, repair a seeded fault, compare matrix and diagram.

## Milestone 6: Behavior and analysis

30. **Add activity authoring.** Acceptance: Actions, partitions, pins, flows, decisions and guards obey endpoint/ownership rules; invalid guards remain drafts; diagrams round-trip without implied execution.

31. **Execute the first finite activity subset.** Acceptance: Branching fixtures match independently specified traces; deadlock, step-budget exhaustion and cancellation terminate visibly.

32. **Add state-machine authoring.** Acceptance: Invalid targets and ambiguous definitions produce diagnostics; authored states, events, guards and layout survive history and reopen.

33. **Execute the first state-machine subset.** Acceptance: Fixed events yield expected states; conflicting transitions expose policy; cancellation and resource limits preserve the model.

34. **Add sequence and scenario authoring.** Acceptance: Lifelines/messages reference exact existing identities; alternative paths retain guards; removed or incompatible participants produce diagnostics.

35. **Add parametric specifications and bindings.** Acceptance: Binding type/dimension errors are actionable; unbound parameters remain visible; specifications persist independently of execution.

36. **Extend bounded analysis and Boolean assessment.** Acceptance: Independent quantity and threshold fixtures pass; zero denominator, cycle and unit errors stay explicit; assessment cannot set verification Passed automatically.

37. **Add scoped model and trace metrics.** Acceptance: Counts match fixed fixtures; filtered versus global scope is explicit; exports retain model revision and rule version.

**Milestone exit:** Author/run bounded behavior, evaluate dimensioned constraints and Boolean assessment, attach receipt, and demonstrate input-change staleness.

## Milestone 7: Interchange and reusable outputs

38. **Open explicit source project sets.** Acceptance: Duplicate IDs in different members stay distinct; missing or changed companions are visible; offline reopen performs no ambient fetch.

39. **Admit the first editable UML/SysML source subset.** Acceptance: Reviewed constructs map predictably; unsupported semantics cause explicit loss/refusal; original bytes stay untouched.

40. **Export the same semantic subset to XMI.** Acceptance: An independent reader reconstructs expected types and relations; unsupported constructs cannot silently disappear; semantic and layout equality differ.

41. **Add mapped tabular import/export.** Acceptance: Duplicate IDs, invalid types and unresolved references appear before Apply; Cancel changes nothing; supported exports reimport equivalently.

42. **Export authored diagrams.** Acceptance: SVG output retains selected scope, labels, geometry and documented hidden-item policy and opens independently; PNG is a separate adapter if needed.

43. **Generate scoped reports and tables.** Acceptance: IDs/counts match the snapshot; stale or historical evidence stays labeled; failure preserves existing outputs.

44. **Instantiate pinned local libraries.** Acceptance: Original libraries stay read-only; copies have fresh identities; reference-versus-copy behavior and dependency hashes are inspectable.

45. **Preview library updates and preserve attachments.** Acceptance: Conflicts precede mutation; declined updates retain overrides; moved project packages reopen with verified attachment references.

**Milestone exit:** Inspect an explicit source set, admit/export supported constructs, reopen independently, review library update conflicts, generate diagrams/reports.

## Milestone 8: Shared Craft UI experience

46. **Unify themes and control states.** Acceptance: Light, dark and high-contrast fixtures cover enabled, disabled, focused, selected, invalid and pending states; color alone conveys no required meaning.

47. **Normalize inspectors, icons and numeric editing.** Acceptance: Equivalent properties share commit/cancel behavior; Escape cancels only the intended edit; one drag yields one transaction; controls have meaningful accessible labels.

48. **Add font coverage and localization infrastructure.** Acceptance: Non-Latin and RTL fixtures render; translations preserve IDs and shortcuts; missing translations have explicit fallback; exact asset licenses are retained.

49. **Complete keyboard and assistive-technology workflows.** Acceptance: Create, connect, inspect, validate, save and recover without a pointer; focus returns correctly; a real accessibility client operates critical controls.

50. **Qualify layout scaling and interaction performance.** Acceptance: Frozen model tiers meet declared budgets on recorded hardware; target window sizes/scales expose critical controls; dense views retain readable inspection.

**Milestone exit:** Complete the engineering workflow across pinned interaction references, themes, locales, keyboard-only and accessibility checks.

## Milestone 9: Review and collaboration

51. **Add immutable local revisions and semantic diff.** Acceptance: Rename, relation, value and geometry changes classify correctly; comparing revisions modifies neither input.

52. **Add reviewed three-way semantic merge.** Acceptance: Independent edits merge; competing edits remain conflicts; identity/reference integrity holds; Cancel and failed validation preserve the pre-merge document.

53. **Generate immutable stakeholder review packages.** Acceptance: Comments bind exact revision/element IDs; superseded content is identifiable; stakeholder decisions do not mutate verification.

54. **Implement repository protocol and revision storage.** Acceptance: Conditional push/pull protects revisions; interruptions expose no partial revision; incompatible protocols refuse in a local harness.

55. **Add repository permissions and lock semantics.** Acceptance: Unauthorized access is denied; expired/lost locks cannot authorize changes; audit records identify actor, revision and operation.

56. **Add remote branches, merge review and offline reconciliation.** Acceptance: Two clients edit offline, reconnect and review an actual conflict without silent overwrite; reviewed integration retains attribution/history.

**Milestone exit:** Two authenticated clients edit offline, reconnect, review conflicts, merge and publish a fixed-revision stakeholder review in a local harness.

## Milestone 10: Target-specific closure

57. **Add each selected external execution adapter separately.** Acceptance: Each adapter pins inputs/tool versions, requires explicit execution, supports bounded cancellation and matches an independent fixture; unsupported mappings refuse.

58. **Decompose and close standards, platform and edition coverage.** Acceptance: Each missing construct family, diagram, profile, OS path and add-on has an individually reviewable child PR and independent acceptance; open rows block full parity.

**Milestone exit:** Every selected edition/version/platform capability passes its independent oracle; missing coverage blocks parity claims. Production acceptance is separate.

## Final acceptance

Replay pinned end-to-end scenarios against one integrated candidate and independently review outputs. Report feature, interchange, execution, UI and platform results separately. A parity claim names the exact edition/version/platform and requires every selected row to pass. Production/service deployment is a separate action.
