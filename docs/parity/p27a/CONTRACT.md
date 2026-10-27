# P27a reviewed declarative requirement rules and atomic fix core

Build on P26 PR33 merge1c49d7ac. Extend existing suite data with optional empty-
omitted custom entries: exact local requirement ID, Identifier or Statement,
NonBlank/StartsWith/Equals literal predicate and optional explicit replacement.
Max32 rules per suite, unique64-byte ASCII IDs,4096-byte control-free literals.
No arbitrary selectors, regex, embedded languages, scripts, services or providers.
Unknown fields/predicates/targets, malformed IDs, bounds and replacements that do
not satisfy their predicate refuse. Original legacy suite bytes stay unchanged
when custom is empty; older strict readers refuse populated custom data.

Custom authoring requires explicit runtime preview and admission of exact suite
proposal/register. Ordinary Save suite cannot silently admit custom rules.
Imported data remains declarative data, not review/execution authority: Open runs
no rule or fix. Explicit Run evaluates accepted requirement text only, uses stable
MBSE.CUSTOM.<id>:requirement<N>:<field> IDs, sealed P26 locations and missing-target
limitations. Existing four-family and report bounds remain. Not full OCL/script
or arbitrary custom-validation compatibility.

A retained issue with explicit literal replacement may prepare a sealed runtime
quick-fix preview. Existing Board identifier/statement authority applies to an
isolated prospective clone: duplicates, blank accepted statements and Copy-slave
changes refuse without modifying live raw/history. Pending invalid target drafts
refuse; unrelated invalid drafts remain exact. Copy-master propagation is included
in the complete accepted/raw before/after list, without rewriting invalid slave
proposals. Run/preview never modify source, lifecycle or evidence decisions.

Apply repeats exact native runtime/snapshot/register guard and admits the prepared
register in one existing Board transaction. Changes/copy propagation Undo/Redo as
one action; original native/views remain unchanged. Source/raw/register changes,
reused counters with different bytes and cold runtime refuse the old proof.
Selection-only changes preserve it. Preview/review have no deserializer/public
constructor; no proof is recovered from disk. Accepted external declarative data
is still manually run, not trusted as an earlier reviewer decision.

P27a is core/data. P27b must add actual native preview/review/Apply/Undo and durable
reopen acceptance before P27 parent completes. No broad standards, native/full
NoMagic/SysML/Craft/platform/independent engineering claim. All56 broad specs and
integrated milestone exits remain unaccepted.
