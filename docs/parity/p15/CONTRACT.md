# P15 app-owned type definitions and typed properties

Build on the migrated P14 native snapshot. Required namespaces mbse-craft.types/1
and mbse-craft.type-drafts/1 have explicit profile/version/shape admission. Optional
opaque entries using those same keys are never silently reinterpreted or replaced.
Component definitions own properties; value definitions declare Boolean, Integer,
Real or Text. References use exact typed IDs, with owner/type-kind and same-System
checks. Definitions and properties share the native identity high-water allocator;
core/nested/retained journal collisions and noncanonical serialized keys refuse.
Multiplicity is a canonical unsigned32-bit lower/upper or unbounded upper *, with
lower <= upper. No assignment/default/derived-value, inheritance or SysML claim.

Types and properties opens from Tools/menu/keyboard. Property forms are durable
raw journals, independent of accepted property data. Invalid/unfinished type-ID,
name and multiplicity text is saved/reopened intact. Apply stages and validates
atomically; wrong-kind reference or invalid multiplicity leaves accepted state
and the retained raw form unchanged. Model history now includes accepted extension
state alongside Stage1 records, retaining newer property forms and monotonic IDs
through Undo/Redo. Native text and views histories retain their existing ownership.

Unapplied definition-name UI text is retained in-session; Add commits a definition.
Property form text has durable native journal scope. Opening an unmigrated document
requires the explicit P14 new-copy migration workflow before authoring types.
Unknown required namespaces and future schema refuse; nonempty extensions retain
the explicit exchange refusal until an adapter exists. Synthetic bounded Linux
checks do not assert broad NoMagic, full Craft/platform or engineering acceptance.

Acceptance: native/controller wrong-kind and multiplicity refusal, durable invalid
property text, same model-history Undo/Redo without losing newer forms, exact-ID
native/views Save and cold reopen, preserved native raw9e and accepted values,
opaque namespace collision refusal and unchanged pre-existing worktrees.
