# P08 reviewed presentation-copy transaction

P08 builds on P07 merged at `7244b562cf0e2de6462feeb70f24c69225c829e2`.
It copies presentation into one new view of the current native document. It
never imports native records or grants authenticity to source declarations.

1. Inspect an explicit local source. Every occurrence begins **Unselected**.
   Map each desired occurrence to an existing, matching-kind native identity or
   skip it with a nonempty reason. An unselected row stays skipped on Apply.
   Source declarations, labels, detached state and old bindings never auto-map.
2. Check the title and mapping preview. A private runtime review token binds
   the exact Plan instance/generation, source inspection, target stamp and title.
   Changing a choice/title requires another review. Reopening the same bytes
   does not adopt old runtime authority. Preview allocates no target IDs.
3. Apply rechecks source bytes, target native/raw/layout identity and revision,
   kinds, capacity, IDs, encoded budgets and native authored relationships.
   Native/views/pair work or a held gesture blocks the UI operation. Refusal
   publishes no partial view, allocator change, history entry or selection change.
4. A successful Apply adds one named v2 view, using target-owned fresh monotonic
   view/occurrence/edge IDs. Only mapped rows copy geometry, visibility and pins;
   source camera is retained. An edge copies only when both exact source
   occurrences are mapped and the target native relationship actually exists.
   All source rows/edges receive mapped or skipped provenance with reasons.
   Existing views, records, accepted values and invalid raw drafts stay intact.
5. The whole copy is one Layout Undo entry. Undo restores prior presentation
   content, retains v2 format and retires allocated IDs. Redo checks current
   native dependencies before popping history; it cannot resurrect deleted or
   undone native records. Runtime history is not restored from saved files.
6. A destination frozen for v1 cannot authorize v2 output after Apply. An owned
   v1 pair requires a fresh Save As before native output begins. A fresh v2 pair
   Save and normal reopen retain exact native content and inert copy provenance.

Acceptance and execution scope are recorded in [README.md](README.md). The
constructed old-presentation fixture is synthetic QA source, not a model save
acknowledgment or accepted engineering content. Native inputs/saves are emitted
through the actual Session/checkpoint path. Original inputs remain unchanged.
