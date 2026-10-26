# P18b qualified port occurrences and semantic connector routes

Build on P18a PR22 merge8fec6d8. Optional usage context on typed Port occurrences
names an exact Part usage whose target Component owns the port. Existing unqualified
port occurrences remain unchanged; they cannot be connector endpoints. Reuse of
one type in two Parts does not collapse contexts. Reference usages refuse.

Connector presentation edges reference one accepted semantic connector and two
exact qualified port occurrences in the same view. The source/target Part/Port
pairs must exactly match the accepted connector. Stale, reversed, cross-view,
wrong-kind, ordinary/unqualified and duplicate routes refuse atomically. All
presentation IDs share the existing global allocator and collision/count/byte
budgets. Profile views/4 and recovery/4 explicitly carry this contract; v1/v2/v3
wire values gain no empty optional fields.

Moving an endpoint updates the rendered straight directed route from its current
geometry. Hiding an endpoint hides its route without deleting it. Removing a
visual route leaves its semantic connector. Removing an endpoint occurrence
removes dependent visual routes in one layout transaction; Layout Undo restores
both. Other views keep separate geometry and route IDs for the same connector.
Model Undo detaches removed semantic connectors/endpoints and exact same-runtime
Redo proof restores their original presentation IDs; cold recovery never gains
that runtime authority. Invalid newer connector text stays in the native journal.
Actual captured native/views Save and cold reopen preserve all qualified context,
route, legacy occurrence, semantic and unfinished-form identities.

UI uses explicit qualified reuse/route buttons and labeled port cards, with at
most96 reuse and128 route choices shown. API supports admitted contexts beyond
those UI lists. Routes are straight arrows with conveyed Value labels; no bend
editing, root delegation, nested path expansion, simulation or full SysML/domain/
NoMagic/Craft UI equivalence is claimed. Typed source-copy inspection continues to
refuse explicitly rather than dropping typed content. Broad56 specifications
remain NOT_RUN; this is a bounded Linux workflow slice.

Conveyed captions use free vertical intervals inside the actual visible canvas
clip and avoid card/prior-caption bounds. At most128 captions are drawn. A fully
crowded viewport may need card movement or panning; automatic complete graph
layout/readability is outside this bounded slice. Private per-frame caption probes
are neither saved presentation data nor acceptance authority on their own.
