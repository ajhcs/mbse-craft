# P16 parts, references and decomposition

Build on P15 merge19cb651 in isolated codex/m4-p16-parts-references. App-owned
Component definitions own Part or Reference usages targeting Component definitions
in the same System. Parts form an acyclic decomposition graph; reference targets
may be reused and do not imply containment. System membership remains explicit
and separate. Usage IDs share native high-water allocation and remain stable on
owner changes. No implicit instance configuration, port, exchange or SysML claim.

The type store and draft journal receive optional additive fields under their
existing admitted schema; previous P15 documents remain byte-preserving on load.
Older readers reject populated unknown fields rather than discarding them. Raw
name/owner/target/kind text survives history and native persistence. Apply previews
and validates atomically; wrong-kind/cross-System/cycle refusal retains accepted
state and the raw form. Browser labels show membership separately from owned
Part containment and non-containing Reference rows.

Acceptance: direct/indirect cycle and wrong-kind refusal; identity-preserving
move with Undo/Redo; reference reuse without containment; invalid raw form Save/
cold reopen; actual browser rows/labels distinguish membership and containment;
original native records/drafts/values and typed P15 data remain unchanged.
