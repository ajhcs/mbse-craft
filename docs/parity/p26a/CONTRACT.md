# P26a named local validation core

Build on P25 PR31 merge885dbb21. P26 is split into core P26a and native authoring/
exact-field navigation P26b. This PR adds core APIs and fixtures only; P26/AC-CAT-028
native action and parent completion remain open until P26b passes.

The existing assurance Board stores optional, empty-omitted validation_suites.
Named suites contain a set of four known RuleFamily enums: PropertyDraft,
PartContainment, CalculationUnits, RequirementDrafts. Built-in Local working drafts
is immutable. Custom names are exact/case-sensitive, nonblank/trimmed, no controls,
up to128 UTF8 bytes; at most32 named suites, nonempty rule sets. Unknown rule
enums/fields refuse. Names/data cannot load code, scripts, external providers or
services. Invalid name/rules forms remain raw drafts under existing independent
Save/history/dirty guards. No additional model or calculation engine.

Run delegates to existing native snapshot validation, type property/usage previews,
Part containment admission, native unit/quantity evaluation and accepted requirement
draft flags. Stable rule IDs and exact typed field targets are original local
contracts, not normative SysML/NoMagic rule equivalence. Supported property errors
route to name/value type/lower/upper fields; unroutable errors are disclosed.
Part cycles route to the changed owner when target is unchanged, otherwise target.
QuantityKindMismatch/UnitMismatch for a calculation route to its expression field
within the existing Measure. Parent/identifier/statement requirement drafts route
to exact existing fields. Raw/accepted state is never repaired or promoted.

Reports are runtime-only, immutable through the public API and cannot be
deserialized as authority. Navigation proofs have no public constructor/deserializer.
Native document/instance/revision/raw/snapshot hash and whole register digest seal
each report/location; checks repeat at location resolution. Reused IDs/counters,
different runtime, native/raw/register/suite changes refuse; selection alone does
not change source. Stable issue IDs survive equivalent reruns, while proof does not
survive cold runtime replacement. At most256 issues and128 limitation details are
retained, with total/unreported counts explicit.

Scope is four working-draft rule families on an admitted native/register pair.
Ports/interfaces/connectors/profile/value/refactor/deletion drafts, arbitrary
external schemas and other calculation diagnostics remain outside this scope and
are disclosed. No broad clean-model, engineering, verification Passed, safe-to-delete
or full native/SysML/NoMagic/Craft verdict. Named suite data persists under existing
atomic register Save; native model Save/views are separate. Dependency context and
prospective evidence byte accounting retain current suite map semantics.

Core acceptance: seeded dangling property type, invalid Part containment, unit
incompatibility and requirement drafts have exact stable rule IDs/targets; existing
validators independently report the same faults. Run/resolve preserve accepted
model/raw/register/history. Named scope/Save/cold/Undo and invalid/unknown/limit
refusals preserve source. Stale runtime/hash/reused-ID locations refuse. No native
interaction or independent engineering acceptance is claimed in P26a.
