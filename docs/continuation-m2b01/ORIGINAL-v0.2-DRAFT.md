# Native views sidecar and scoped history, v0.2 draft

This is an original, unadopted next-feature contract. It refines `ready-contracts/MODEL-VIEW-COMMANDS-v0.1.md` without changing that sealed draft, the frozen six-regression candidate, product source or any native schema. No Rust compilation, GUI, new worker or permission change is authorized here.

Targets: CAT-003/004/006/010/011/014 and CAP-009/010/017/019/022 from the86-feature program; WF-02 plus navigation portions of WF-01/09. This is a useful bounded native multi-view slice, not general UML/SysML, Capella, CATIA or Craft-file parity.

## Source facts that constrain the proposal

Read-only product source pin: `0dfa6f1341b26f955777b15d3f743f4f39c0a9ef`. The current UI candidate is `217c977c` plus preserved core patch `6800a3d1.`; no view implementation is credited to it.

- `crates/mbse-model/src/stage1.rs`: `RecordId(pub u64)` and `DocumentId(pub String)` are existing semantic identities. `Snapshot` owns records, raw/creation drafts, `next_id`, `model_revision` and `raw_version`.
- `Freshness` binds document ID, runtime instance ID, model revision and raw version. `Editor` owns private model/text history; `undo_model`, `redo_model`, `undo_text` and `redo_text` are distinct existing APIs. A sidecar must not invent another authoritative record graph or reach into private history fields.
- The current public `Command` enum has typed add/edit/link/configuration operations and no semantic-delete variant. Directly removing records to simulate a Delete button is outside this slice.
- `stage1_io::IoTicket` binds document, instance, session, request, model/raw revisions, destination and snapshot hash. `WriteJob` retains private immutable encodings. `Session` owns pending I/O and saved/recovery state. Recovery gives the recovered snapshot a distinct document ID.

These facts were inspected; this draft has not been compiled against those interfaces. Source anchors: [model](https://github.com/ajhcs/mbse-craft/blob/0dfa6f1341b26f955777b15d3f743f4f39c0a9ef/crates/mbse-model/src/stage1.rs), [persistence](https://github.com/ajhcs/mbse-craft/blob/0dfa6f1341b26f955777b15d3f743f4f39c0a9ef/crates/mbse-model/src/stage1_io.rs), [model contract](https://github.com/ajhcs/mbse-craft/blob/0dfa6f1341b26f955777b15d3f743f4f39c0a9ef/docs/MODEL_CONTRACT.md).

## Proposed first slice and explicit exclusions

Provide two named views of the same native document; containment browser/search/find usages; reuse existing elements in a view; move/pin/fit; hide or remove a presentation occurrence; edit native fields through existing typed commands. Display names derive from native records. No copied semantic property store or label-based identity.

Semantic delete, automatic graph layout, generalized clone/refactor, broader classifier families, external source occurrences and companion/native admission remain separate later contracts. The current relation ledger must stay available. Unsupported routing persistence is disclosed rather than losing a relationship from the model.

The primary canvas cannot be closed accidentally. Optional Craft docking/tabs are mechanisms: `DockArea::show` emits actions; a validated layout adapter applies panel changes. They do not own model commands or confer sibling application file compatibility.

## Concrete proposed decisions

| ID | Proposed choice | Meaning and limit |
| --- | --- | --- |
| V2-D01 | Add a separate `mbse-craft.native-views/0.2-draft` sidecar | Native `kernspan.stage1/2` stays unchanged. The new profile is not supported until implementation/migration/tests qualify it. |
| V2-D02 | Bind the loaded native DocumentId, exact checkpoint artifact SHA256 and existing snapshot hash | Doc ID alone is insufficient after changed bytes. Runtime instance/revisions guard commands; they are not portable authority. No filename-based rebinding or ambient model search. |
| V2-D03 | One monotonic presentation allocator, with typed View/Occurrence/Edge keys | Sidecar IDs are canonical decimal u64 strings; this is a sidecar wire choice, not a change to native RecordId serialization. Presentation IDs begin at1; native RecordId retains its existing range and must resolve through the core. No label, sort index or geometry is identity. |
| V2-D04 | Persist current view state and allocator high-water, not runtime undo/run capabilities | Undo/Redo stacks and evaluator proof are cleared on reopen. A new creation cannot reuse a retired presentation ID. Preview IDs are explicitly provisional and never durable references. |
| V2-D05 | Separate, explicitly labeled model/text and layout histories for the first slice | Preserve existing core histories. Canvas layout Undo cannot undo a native field. Field focus retains text Undo. Menu labels explain scope. A later unified timeline needs its own core history adapter. |
| V2-D06 | Every undoable view action validates target identity at application time | Undo of an occurrence removal may restore it only if its native document/record/kind remains available. If not, refuse atomically and explain which model step must be resolved. Never resurrect a native record or retarget geometry by name. |
| V2-D07 | Save native first, then views, with independent acknowledgments | Do not claim a multi-file atomic save. A native success plus sidecar failure is `native saved / views unsaved`; preserve view memory/recovery. Reopen mismatches leave the sidecar read-only/unbound for explicit reconciliation. |
| V2-D08 | Core-affecting actions stage on an Editor clone and validate view consequences before committing | Use public freshness/command APIs. No I/O lock/reservation/Session clone or direct record mutation. App commit rechecks current freshness; stale candidates refuse. Exact implementation needs adapter review. |

All choices above are proposals. They are not user acceptance, product functionality, a new migration decision or a permission grant.

## Proposed wire responsibilities

Top-level sidecar: profile/version; sidecar identity; `native_binding`; monotonic `next_presentation_id`; views; occurrences; edges; inert detached presentation records; app view revision. Strictly decode known fields. A future/unknown profile stays preserved read-only; do not rewrite it as the current version. Unknown external model data remains in the native/source stores, not copied into this sidecar.

`native_binding` contains opaque native DocumentId, artifact and snapshot digests, and informational path hint. A path hint never causes automatic I/O. Admission uses an already explicitly loaded Session whose identity and exact bytes match; a mismatch does not become a native model error. Recovery/new-document IDs require deliberate view-copy/rebinding review.

Before the first native save, views may exist in memory with an explicit `Unsaved` binding. Ordinary sidecar save first requires an explicit native save and its acknowledgment. A separately labeled view-recovery record may retain unsaved geometry for inspection, but must not masquerade as a bound sidecar or mint a native artifact digest. The production bound sidecar uses exact acknowledged native artifact bytes, not a convenient serialization of an arbitrary Snapshot.

An occurrence contains ViewId, native RecordId, expected RecordKind, finite position/size, visibility and pin state. Its semantic name, units and relationship fields are read from the core. Notes/caption overrides are excluded initially. Two occurrences for one record in one view require an explicit per-view policy; duplicate labels are ordinary presentation.

An edge occurrence references explicit source/target OccurrenceIds plus a checked semantic relation key. Initially persist routing for three stable closed-profile relationships: Capability `contributesTo` Mission; Objective typed `subject`; Measure `forObjective`. The key uses owner ID, typed role and target ID/kind, never an array offset. The current derive-graph/ledger remains authoritative for every other displayed relationship; mark its route as derived/unpersisted until a stable key contract exists. Removing an occurrence cannot silently attach its edge to a second occurrence of the same record.

View-only name/selection/camera/visibility/layout changes increment only the view revision and relevant dirty state. Native accepted/raw input stamps remain unchanged. Semantic quantity/reference/name operations use existing command freshness and invalidate whatever the current evaluator actually binds; do not silently weaken evaluator policy.

## Interleaved history and missing targets

The first implementation must intercept all app model commands and core Undo/Redo through one dispatcher, while leaving the core authoritative. Stage candidate Editor state and proposed view consequences together; validate before exposing either state. `Editor` cloning is a candidate staging mechanism only, not a second long-lived model store or a claim of bounded native history memory.

If a core Undo removes an added target, associated active occurrences/edges become **inactive detached presentations**, with exact IDs/geometry and a reason. They cannot participate in active navigation/edges or imply native records still exist. View ID high-water includes detached records. Keep this state visible in the view ledger rather than silently deleting the user's layout.

Only a matching observed native-history transition within the same runtime instance may reactivate detached occurrences automatically. Same display name or an arbitrary record with the same number is insufficient. After reopen, model history is not restored: explicit operator reconciliation/copy may create a new occurrence using retained geometry; no live proof or old undo capability is rehydrated. A native-file mismatch never receives automatic reconciliation.

View Undo revalidates its native identity and counterpart edge references. If the target is absent or changed incompatibly, it refuses with an actionable diagnostic and leaves all stores unchanged. Model/text and layout redo branches are scoped and labeled; an unrelated layout action does not discard a valid native model redo branch. A semantic change that invalidates a pending layout result rejects that result rather than applying stale geometry.

## Save, reopen and recovery state machine

1. Freeze native request and view candidate with their own generations. View worker tickets bind native document/runtime/request identity, expected native snapshot binding, view revision and destination. They cannot acknowledge newer view edits.
2. Let the existing Session complete its native save. On native failure/conflict, do not write a newly bound views file; retain all drafts and normal retry/cancel behavior.
3. After successful native acknowledgment, bind the views candidate to exact acknowledged native bytes/snapshot. Persist the view sidecar with the selected Craft storage mechanism and app-owned destination/conflict policy. No new general persistence engine.
4. On view failure, show separate saved states and preserve recovery. Do not roll back a successful native save secretly or acknowledge an entire project checkpoint.
5. On reopen, validate native binding before activating views. Missing/corrupt/future/mismatched sidecars remain original-preserved and unbound/read-only. Open the native model safely with its default primary canvas; do not synthesize proof of the missing layout.
6. Recovery is an independent dirty document. Its changed native DocumentId forbids automatic admission of the old views sidecar. Offer a reviewed presentation-copy operation with explicit provenance later; keep the current recovery intact.

Crash points before/after each acknowledgment and stale acknowledgments after a new edit are required vectors. This first slice guarantees explicit partial states, not cross-file atomicity, filesystem CAS or power-loss safety.

Required future adapter, **not an existing API**: a read-only durable-checkpoint binding obtained only after the owning Session accepts its actual write result. It must expose exact document/snapshot/artifact identity and acknowledged generation without leaking mutable I/O ownership. A caller-supplied/deserialized IoTicket alone is insufficient. Runtime staging must recheck existing Freshness immediately before commit. These interface additions require source review in the future feature branch; they are not patched into the frozen candidate.

## Provisional resource envelope for review

Proposal only:16 views,4096 total occurrences,8192 edge occurrences,1024 occurrences per view,1024 UTF8 bytes per view title,8MiB encoded sidecar, finite coordinates within +/-1,000,000 diagram units, zoom0.05-20. Detached presentations share those budgets. These values are starting admission limits for review, not measured scalability or accepted performance requirements.

Input/output/aggregate item counters must be enforced before large allocation/clone or serialization; strict JSON rejects duplicate keys and nonfinite numbers. Bound new view history by both retained entries and accounted bytes; current native history has not thereby become bounded. No claim of total process RSS, accessibility, collision-free layout or physical latency follows.

## Implementation handoff and closure

After the frozen UI test/walkthrough milestone, assign a separate future worktree/branch and concrete exclusive paths for a Rust views module plus desktop adapter. The sole integrator admits it; do not place these draft changes in the pending `217c977c` candidate. No new worker context is created now.

The next source milestone is the sidecar DTO/validation and transaction adapter design, followed by focused core tests and a real two-view save/reopen/Undo/missing-target workflow on an allocated exact binary. Preserve original native storage, accepted/raw drafts, evidence history and the primary canvas. `VIEW-VECTORS.json` defines proposed expected outcomes; none are executed product tests.
