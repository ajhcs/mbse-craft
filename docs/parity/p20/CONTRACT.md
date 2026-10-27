# P20 accepted-model clone/move with explicit reference preview

Build on P19 PR24 merge3c628b2. This bounded typed workflow handles Component or
Value definitions and owned accepted properties, ports, usages and connectors.
Definition System membership is not containment. Part target definitions,
interfaces and conveyed/value types remain shared external targets unless already
inside the admitted owned set. Clone allocates fresh global semantic IDs and remaps
internal references; copied owned property defaults and owned-usage overrides
retain literals. Incoming references keep their original source targets. Original
accepted members, opaque source bytes and all unfinished forms remain exact.
Source raw forms are retained, not duplicated onto cloned identities. Existing
presentation records are retained and are not implicitly cloned or retargeted.

Move takes up to64 explicit Part/Reference usage IDs and a Component owner. Usage
and enclosed-connector IDs stay exact. If both connector endpoint Parts move, the
connector owner moves atomically; a connector crossing the move boundary blocks
Apply. No implicit deletion/retargeting treats that boundary. Same-System, exact
type/context and Part-DAG validation apply. Defaults/overrides keep their reference
identities; qualified source views remain consistent with exact semantic endpoints.

Durable raw form holds mode, source/name/selection/owner text and explicit
keep-external-target choice. Preview enumerates remapped internal references,
shared external targets, retained incoming targets, fresh-ID mapping and blocked
connector boundaries. Preview allocates candidate IDs only; Cancel changes neither
live allocator nor accepted data and retains the raw form. Native fields outside
this typed scope and opaque namespaces are listed as retained/uninterpreted; no
exhaustive global impact verdict or native-core record clone/move is claimed.

Plan/candidate authority is runtime-only, private and not deserializable. Native
Apply checks exact snapshot, model/raw revisions, allocator and editor instance;
any intervening edit/reopen refuses an older plan. No public supplied candidate or
cached preview can write. Apply is one cloned validation/publication transaction;
model Undo/Redo restores accepted state while keeping newer raw forms and monotonic
allocation. Previews never gain recovery authority. Clone is capped at128 owned
members; existing semantic/extension byte/count budgets remain active.

Tools > Clone and move with reference preview (CtrlAltB) shares the command/modal/
history guards. Native/views4 Save/cold reopen retain all cloned/moved identities,
prior meaning, raw work and source occurrence/route records. All56 broad parity
specifications remain NOT_RUN; this same-agent Linux scope does not establish full
SysML/NoMagic/Craft/platform or independent engineering acceptance.
