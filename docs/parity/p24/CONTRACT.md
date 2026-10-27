# P24 typed requirement dependencies

Build on P23 PR29 merge1ae74f4f. Reuse the existing independent assurance Board,
register, allocator, draft/history and atomic Save/load guards. No native record
reclassification or second concept/calculation engine. New trace/test-case maps
omit when empty, preserving legacy flat/hierarchy registers. Old strict readers
refuse new populated fields. Unknown enums/fields, duplicate/aliased keys, missing
local endpoints, shared-ID collisions and invalid Copy graphs refuse atomically.

Each dependency stores an immutable TraceKind and typed client/source endpoint,
plus supplier/target RequirementId. Display labels never select or change meaning.
Derive requires two distinct local requirements. Refine permits a requirement or
native element as source and requirement as target (bounded one-direction subset).
Satisfy permits actual System, ComponentDefinition or Part source, not Measure,
Requirement, evidence or a renamed label. Verify requires a documentary TestCase
specification with name/procedure/expected result. It does not execute a test or
assert proof, adequacy, requirement approval or verification Passed.

Copy points from slave requirement to master requirement. Text must match exactly
on admission; each slave has one master and the Copy graph must be acyclic.
Slave statement edits remain unapplied raw drafts. Valid master statement edits
propagate exact accepted text/revisions to transitive slaves atomically, retaining
their invalid raw drafts. Existing local evidence becomes stale via requirement
revision/hash. Removing Copy releases its slave text. Labels, parent containment,
statuses, source anchors and evidence stay independent; no inheritance/rollup.

Native endpoints store actual document identity, positive local ID and actual
TargetKind from the existing native/type authority. Admission resolves live kind;
projection detaches a missing/wrong-kind/wrong-document endpoint without deleting,
retargeting or promoting the declaration. A cold file retains declared identity,
not authenticated runtime proof. No implicit cross-document ID matching.

Existing register Undo/Redo restores exact accepted traces, cases and raw forms,
with existing audit preservation/non-recycling allocator. At most10000 traces and
10000 documentary test cases under the existing32MiB register bound; labels512
bytes, names512, procedure/expected/raw proposal16384 bytes. Native UI proposals
cap1024 characters and display first128 traces touching selected requirement;
complete register persists. UI exposes explicit Add by type, source prefix,
separate documentary case creation, label editing and removal. Native Save stays
separate. M05 integrated exit and broad parity/independent acceptance remain open.

Reference distinction: [No Magic SysML quick reference](https://docs.nomagic.com/SUP/files/254421775/254421783/1/1757674468472/no-magic-quick-reference-guide_sysml.pdf)
describes client/supplier dependencies and read-only Copy text. This implementation
is an explicit local subset, not SysML stereotype/metaclass, path or full NoMagic
conformance. AC-CAT-018 remains a broad unexecuted specification; bounded evidence
must name actual tested endpoint kinds, history, Save/reopen and unchanged model.
