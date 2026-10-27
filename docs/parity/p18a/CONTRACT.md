# P18a instance-qualified connector semantics

Build on P17b PR21 merge0694708. A Component definition owns directed semantic
connectors between its immediate Part usages. Each endpoint names exact Part usage
and Port definition IDs: the usage must be a Part owned by that Component, and its
target definition must own the selected port. Reusing a type in two Parts therefore
keeps endpoint contexts distinct. Reference usages are not admitted as Parts.

The effective source interface must be Out/InOut and the target In/InOut. Both
payload identities and the conveyed Value-definition ID must agree exactly; no
unit conversion, structural/nominal SysML equivalence, execution or external-tool
compatibility is inferred. Identical qualified endpoints refuse; two different
ports on one Part may connect if directions/type agree. All targets must exist.

Connector identity shares native allocation/model history. Per-connector raw
name/reference/conveyed text is independent and durable through Undo/Redo and
native Save/cold reopen. Apply validates a cloned candidate before publication;
wrong ownership/kind/direction/type keeps accepted connector and raw form intact.
Existing P15/P16/P17 data and port occurrences stay exact. Semantic moves that
would invalidate connector ownership refuse until an explicit reference treatment
workflow exists. Presentation contexts/routes are P18b; P18 remains incomplete.
