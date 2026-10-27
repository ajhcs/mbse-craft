# P29a versioned documentary viewpoints and native-card filter core

Build on P28 PR36 merge5fa7c05e. Existing views gain optional accepted viewpoint
and retained raw form. Explicit views/recovery profile5 supports these fields;
legacy1..4 reject even null viewpoint fields rather than silently upgrading.
Legacy data without fields stays unchanged. Save/capture/recovery/layout history,
port/connector actions and reviewed view copy preserve profile5 and metadata.
Older strict readers refuse5. Native model meaning, identities and raw data remain
under existing model authority; this creates no normative SysML Viewpoint/Concern.

Accepted scope records exact source document, label, enabled flag, optional typed
Stakeholder/Concern(native Expectation)/System IDs, native RecordKind set and
case-insensitive literal accepted-name substring. Empty kind set means all native
kinds. Reference wire IDs are canonical positive decimal strings; source/name/
substring bounds and unknown fields/enums refuse. Raw ID/name strings up to4096
bytes each remain proposals, including invalid values. Applying rejects invalid
IDs, missing/wrong-kind/cross-System context or concern not expressed by the declared
stakeholder/for declared System. Refusal retains prior accepted scope and raw form.

All edits are existing guarded Layout actions with independent layout Undo/Redo;
no Editor command, source mutation or calculated/engineering verdict. Per-view
metadata does not change camera/geometry/manual visibility/occurrence identity.
Projection filters native record cards only. Typed port/connector rendering and
complete model/ledger retain existing scope; manually hidden stays hidden. P29b
must wire actual rendering and native UI before parent completion.

Stored foreign/missing/wrong-kind context is retained and disclosed, with filtering
inactive (all otherwise visible native cards eligible). No ID retargeting or
implicit model migration. Reviewed view copy retains exact source document and
raw metadata; foreign target requires explicit form source rebind and fresh Apply.
Clear removes accepted scope while preserving the form. References are documentary,
do not prevent native deletion and do not claim complete deletion impact.

Core tests use existing illustrative synthetic inspection data, not engineering
acceptance. Native source/layout/recovery/copy bounds remain explicit, no RSS/scale
claim. All56 broad specs/integrated exits/full NoMagic/SysML/Craft/platform/independent
engineering acceptance remain unaccepted; P29b native workflow remains required.
