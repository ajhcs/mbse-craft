# P07 presentation-copy format and provenance

Base: merged `implementation/native-alpha` at
`bfbe0f1ad361739fd14d728d93db3b9527c76b0a`. This implements the version/schema
foundation from P07, building on the existing recovery-copy contract,
read-only inspection, mapping rehearsal, exact identity and paired-save APIs.
**Live Apply and its one-transaction Undo are P08.** No copy action is added here.

## Profiles and compatibility

- `mbse-craft.native-views/1` and `mbse-craft.native-views-recovery/1` retain
  their exact existing keys and strict readers. Their canonical serialization
  stays byte-for-byte unchanged. Even `copy_origin: null` is refused in v1.
- `mbse-craft.native-views/2` and `mbse-craft.native-views-recovery/2` add an
  optional per-view `copy_origin`. Omit it when absent; present provenance must
  be a valid object. New/default workspaces continue using v1. Reading v1 does
  not migrate it. Reading/saving v2 preserves its version and provenance.
- Unknown profiles, unknown fields, duplicate keys and malformed mappings
  refuse. Original files remain unchanged; failed local Open retains bounded
  input bytes and protects its path through the existing IO policy.
- A format change cannot reuse an acknowledged destination of the other
  version. It requires a fresh Save As destination, checked before native Save.
  Existing unowned destinations remain refused. Ordinary same-version owned
  saves retain the existing conflict/backup policy.

Native schema and exact native binding rules are unchanged. Bound v2 still needs
an actually accepted sealed native checkpoint. Unbound v2 recovery grants none.
The declared native binding inside provenance is labeled as a source claim.
It never participates in target native admission or acknowledgment.

## Provenance DTO

One copied view may declare `mbse-craft.presentation-copy-origin/1` with:

- Exact source byte SHA-256, source profile, document ID, sidecar ID and view ID.
- Optional **declared** source native binding, consistent with its source profile.
- A source-occurrence keyed map: source native ID/kind and either an explicit
  mapped target occurrence/native ID, or a nonempty skipped-item reason.
- A source-edge keyed map: both exact source occurrence endpoints and either a
  mapped target edge ID or a nonempty skipped-edge reason.

All presentation and native-reference IDs use canonical positive decimal u64
strings. Numbers, zero, leading zeroes, overflow and unknown fields refuse.
Source view/occurrence/edge IDs are disjoint within their source namespace;
mapped target occurrence/edge IDs cannot alias one another or current views.
When a mapped target row exists, its exact view, native ID/kind and edge endpoints
must match. Mapped edges require both exact source endpoints to be mapped.

Removed target items remain inert historical IDs below the allocator high-water;
provenance does not prevent later layout edits or resurrect removed items. Its
absence/presence does not create runtime origins, pristine-default authority,
native checkpoints, history proof or a fresh mapping plan. Reopen starts empty
runtime histories and a new origin as before.

`CopyOrigin::verify_source` rechecks one explicitly supplied Inspection's actual
bytes, source identities, declared binding, and complete exact source-view row
and edge coverage. It performs no ambient lookup and no target mutation. A hash
or provenance declaration alone is never proof of source truth, semantic
identity or permission to Apply. P08 must independently recheck source/runtime,
explicit target selections, budgets and native relationships before committing.

## Bounds

Existing 8 MiB sidecar, 16-view, 4096-occurrence, 8192-edge and 1024-per-view
bounds remain. Each origin has at most 1024 occurrence mappings and 8192 edge
mappings, with UTF-8 byte limits of 1024 per reason/identity. Combined compact
provenance is bounded at 2 MiB; raw counts/size are checked before typed maps.
The entire pretty-encoded DTO and layout history include provenance in their
existing byte budgets. These are payload limits, not measured RSS/latency claims.

## Concrete acceptance

| P07 criterion | Required observation |
| --- | --- |
| Old files readable | Actual v1 bound/recovery decoder round-trip and unchanged canonical bytes; actual native Open of pinned v1 |
| Version/provenance retained | Actual v2 bound/recovery IO Save/reopen preserves DTO and empty runtime history; native v2 fresh-pair Save/normal reopen |
| Future/unknown preserved | Future profile and unknown provenance-field refusals retain original source bytes/path and exact native/layout state |
| Malformed mappings refuse | Wrong view/kind, duplicate targets, noncanonical IDs, missing/unmapped or aliased edge endpoints and limits refuse atomically |
| Explicit provenance/source check | Exact source byte/identity/coverage recheck passes; changed bytes or claimed source records refuse |
| Existing files protected | Version-changing owned Save refuses until fresh destination; native attempt to reuse an existing v1 destination leaves bytes unchanged |
| Provenance stays inert | No runtime-stamp reuse, inferred native binding, native mutation or allocated IDs/history from inspecting provenance |

The synthetic v2 fixtures are deliberately constructed format inputs for these
checks. They are not output from live Copy/Apply, accepted engineering models or
independent domain acceptance. Original historical evidence and worktrees remain
preserved. The execution report records actual outcomes and limits separately.
