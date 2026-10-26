# P14 versioned extension admission and protected migration

Add optional `snapshot.extensions` with app-owned profile mbse-craft.extensions/1,
version1. Stage1 profile remains kernspan.stage1/2; no SysML/Stage1 identity is
renamed. Absence serializes exactly as before. Namespace keys/payloads are bounded;
unknown optional namespaces remain opaque JSON. Unsupported required namespaces,
future profile/schema, invalid/duplicate structural data and legacy typed-key
collisions refuse. P15+ register actual typed namespace interpreters; an admitted
empty header does not claim structural modeling semantics.

Headless `--migrate-extensions SOURCE.kernspan --destination NEW.kernspan` with
an owned --state-dir reads an admitted saved/recovery source and creates a protected
independent runtime. A fresh copy retains native document/record IDs, opaque record
fields, drafts, creation journals, allocators and core revision counters; only the
extension header is added. Legacy Stage1/1 uses the current Stage1/2 storage writer,
without relabeling into another modeling standard. Source and inherited provenance
paths remain protected; an existing/alias/source target refuses. Normal native
write receipt/checkpoint verifies the actual new copy. Active GUI work is untouched.

GUI opens and saves the migrated native file using existing protected IO and P13
background-read guards; audit exposes the separate extension header. Nonempty
opaque extension namespaces refuse restricted SysML export until an explicit
extension exchange adapter exists. No silent semantic-compatibility claim.

Acceptance: actual old/current source migration, exact core/journal comparison,
source/occupied-target preservation, unsupported future/required schema refusals,
opaque optional round-trip, actual CLI copy/refusals and GUI open/save/reopen of the
migrated artifact. Same-agent bounded Linux evidence only; full structural feature,
platform/interchange/domain acceptance and the Milestone4 exit remain open.
