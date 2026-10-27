# P34a bounded scenario and sequence authoring core

Extend the existing native typed authority and keyed draft journal; no second
participant, concept, operation or execution registry. A Scenario belongs to an
existing native System. Lifelines reference exact existing native records with
expected RecordKind, Component definitions, or Part/Reference usages with expected
UsageKind in that System. These are documentary participants, not inferred runtime
instances. New bindings require a current exact compatible target. Later removed,
wrong-kind or out-of-scope targets retain authored identities and produce exact
participant/message diagnostics; they never silently rebind to matching names.

Messages reference exact owned source/target Lifeline IDs, have explicit positive
order and synchronous/asynchronous/reply documentary kind. Replies reference an
exact prior synchronous call with reversed endpoints in the same sequence scope.
Main messages and Alternative fragments share unique order positions; each
Alternative has ordered operands with retained nonempty guard text and ordered
messages. One flat alternative level only. Guards are documentary expressions,
never evaluated or reduced to Boolean truth. Missing operands/messages and
duplicate guards/participant aliases produce diagnostics without claiming path
exclusivity, completeness, timing, execution, causality or verification.

All model changes go through explicit keyed Scenario/Lifeline/Message/Alternative/
Operand forms and Apply. Invalid edits retain raw proposals without accepted
changes. Selected Discard changes only that proposal. Removing an in-use lifeline,
call, operand or alternative refuses; root removal retains raw. Existing model
Undo/Redo preserves newer raw and allocator highwater. Strict schemas/canonical
IDs, family128 drafts/4096-byte raw fields, shared512 accepted objects, native ID
collision and existing encoded extension budgets apply. Original type/activity/
state/native/raw/opaque data and paired persistence/recovery remain authoritative.
Legacy deletion explicitly holds unsupported scenario/proposal impact rather than
claiming a complete reference inventory. Refactor retains scenario references as
external/uninterpreted and makes no exhaustive global verdict.

Core acceptance covers exact identities/foreign scope/order/reply/alternative
rules; invalid distinct proposals and history; removed/wrong-kind participant
diagnostics without accepted overwrite; shared budgets/collisions/wire/legacy
roundtrips; full preexisting Snapshot/opaque/native sidecar/recovery preservation.
P34b owns independent presentation data, P34c actual native authoring/rendering/
selection/history/save/cold acceptance; parent P34 stays open until review/merges.
No hierarchical fragments, operation or signal signature registry, executable
scenario, concurrency/time constraints or normative full UML/SysML/NoMagic/Craft/
independent engineering/platform/scale/production acceptance. All56 broad specs
and integrated exits remain open.
