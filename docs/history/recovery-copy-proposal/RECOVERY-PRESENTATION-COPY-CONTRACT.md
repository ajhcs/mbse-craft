# Proposed explicit presentation copy after native recovery

Status: source-only next contract, not implemented or accepted. No compilation, tests, renderer, upload, service or worker is authorized by this file. This proposal continues the recovery/reconciliation boundary explicitly deferred in the original native-views v0.2 contract and M2B-01 adoption. It does not change the frozen replacement-guard source `fdae8dfa94bf5bef18944243612a04cbf3047fda` or its held retest candidate `32cd156f34d9496acf931b5543f4b40c42c8e240`.

## Source evidence and bounded goal

At that source, `Session::restore` creates a new native DocumentId. `Views::restore` validates the original view document ID, and `Io::open` requires preservation before replacement. Those refusals remain correct: an old sidecar cannot automatically become the recovered document's view authority. `Views::apply(Layout::Reuse)` already resolves a chosen native RecordId/kind and uses the target's monotonic presentation allocator. `Views::dispatch` remains the sole desktop native command/history path. Semantic deletion, source-model admission and general clone/refactor remain outside the proposal.

Offer one explicit operation: inspect one old view read-only, choose existing target native records, and copy its geometry into one newly created target view. Never replace current target views or native records. Source bytes remain unchanged. This is presentation copying, not proof that old and recovered semantic records are equivalent.

## Proposed transaction

1. Acquire one explicit regular local source file through the bounded strict reader. Admit only a supported native-views/recovery profile and current item/byte bounds; future/unknown/corrupt input remains read-only. Retain exact source bytes and SHA. No path hint, directory scan, network lookup or automatic native-source load.
2. Inspect source occurrences, visibility, geometry, expected kind and detached reasons. Source document IDs and any declared native binding are source claims, not a newly minted checkpoint. An unbound recovery supplies no native artifact authority.
3. Require explicit per-occurrence selection of an existing target RecordId of the expected kind. Mapping defaults to unselected. Equal numbers, names, types or retained recovery IDs do not establish a match. Names are display text only. The operator can skip an occurrence; every skip has a visible reason. No native record creation or semantic-property copy.
4. Stage a new named view with freshly allocated target presentation IDs. Even identical source/target IDs cannot be reused as durable references. The preview uses provisional labels; it does not reserve IDs or consume allocator high-water. Existing target views stay intact. Copy finite geometry/visibility/pin values only. A previously detached source item may become a new active occurrence only through the explicit target selection; it does not reactivate the old item or acquire its history proof.
5. Propose an edge only when both exact source occurrence endpoints are mapped and the mapped native relationship exists in the authoritative target graph under one of the three supported stable keys. Derive the target graph once. No retarget to another occurrence, relationship fabrication, unsupported routing or implicit classifier conversion. Show skipped edges and reasons.
6. Recheck exact source bytes, target native Freshness/snapshot and target view revision immediately before commit. Reject the whole operation on stale source/target, incompatible kind, invalid endpoint or budget failure. No history pop, partial view insertion, native mutation or allocator rewind occurs. Normal target layout history owns the successful one-view addition and can undo it as one action; native model/text history remains separate.
7. Preserve source-origin metadata and explicit occurrence mappings with the copied view. Reuse normal native-first/views-second preservation. No native artifact digest is fabricated before actual native acknowledgment. Pending/failed saves and local Open use the replacement guard; presentation-copy provenance must never create a pristine-default capability.

## Proposed wire boundary for review

The current v1 decoder denies unknown fields. Therefore source-origin metadata is **not** silently added to v1. A future versioned profile, provisionally `mbse-craft.native-views/2-draft`, would carry one bounded origin record on the new view: exact source file SHA, source profile/document/view identities, explicit source-occurrence→new-target-occurrence/native-record mapping, and skipped-item reasons. A declared source native binding is labeled as declared; only an independently loaded sealed native checkpoint could verify it. This proposal does not add a parallel semantic graph.

Before implementation, review the exact v2 DTO and a deliberate SaveAs/read-only-v1 policy. Preserve source v1 files; no automatic migration, rewrite or inplace upgrade is proposed. Unknown future files continue to refuse. Native schema2 is unchanged. Schema/migration review remains a separate gate, not authorization inferred from this draft.

## Bounds, Undo and acceptance boundary

Copy at most one source view and at most1024 occurrences in a transaction. Target limits remain16 views,4096 total occurrences,8192 edges,1024 occurrences/view,1024 UTF8 bytes/title,8MiB encoded sidecar and existing finite geometry/camera ranges. Detached and provenance/mapping/skipped-item payload must be accounted in encoded/history budgets. Fail before large typed allocations and before cloning staged history; no RSS or measured latency claim.

Layout Undo removes exactly the newly copied view; Redo revalidates every mapped target record/kind/key. Missing targets refuse without native resurrection. Presentation IDs remain retired across Undo/Redo and reopen. Reopen restores provenance as inert data, never live Undo, evaluator proof or native checkpoint authority. Cancel and refused planning/apply leave both source and target state/history untouched.

The attached vectors are proposed expected outcomes, UNRUN. Implementation, test execution, actual native use and engineering acceptance are distinct future gates. The P1 replacement-guard candidate remains frozen in its own worktree while this proposal is reviewed.
