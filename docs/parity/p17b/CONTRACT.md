# P17b typed port occurrences

Build on P17a PR20 mergec6b3691. Port occurrences are separate presentation records
referencing exact semantic port IDs, never reclassified as Stage1 native records.
They share the views identity allocator, count/byte limits, camera geometry,
layout history and native-bound/unbound-recovery persistence. New views/3 profiles
admit typed occurrences explicitly; older schemas refuse this format.

Two actual views may reuse one port with different occurrence IDs/geometry. Move,
hide, pin and remove affect presentation only. Model Undo detaches occurrences
whose port disappears; exact same-runtime Redo proof may restore them, preserving
geometry and newer raw drafts. Cold restoration retains detached state without
minting runtime proof. Layout Undo never recreates semantic ports. Existing core
views remain exact when no typed presentation exists. Unsupported copy/source
inspection paths refuse typed content explicitly without dropping it.

Acceptance: cross-view reuse/same semantic ID; independent layout operations and
Undo; actual native/views Save/cold reopen; model Undo/Redo versus detached cold
recovery; wrong target/collision/stale/budget/profile refusal; copy refusal retains
original bytes and target; exact P17a raw interface/port and P15/P16 journals/core
values survive. No connector routing, nested port-on-owner glyph, full SysML or
NoMagic/Craft/platform/domain/independent acceptance claim.
