# P17a port and interface contracts

First child of P17, based on PR19 merge4370e879 in an isolated worktree. P17b
separately introduces typed occurrences and cross-view port reuse; P17 remains
open until that presentation/history/persistence adapter is accepted.

App-owned interface definitions declare one exact Value definition payload and
In, Out or InOut flow direction within a native System. Component-owned ports
refer to one declared interface and optionally an assigned interface. Assignment
requires the same exact payload Value identity, direction and System. Different
interface names/IDs may share this explicit contract; no structural SysML or
commercial-tool compatibility is implied. No connector/execution claim.

Interfaces/ports share native identity allocation and accepted model history.
Raw interface and per-port drafts are separate from accepted data and survive
Undo/Redo, native Save/reopen and detached-owner history. Wrong kinds, missing
IDs, cross-System or incompatible assignment refuse atomically. Unknown optional
data is not promoted. Existing P15/P16 data and native records stay exact.

Acceptance: compatible/different-ID and incompatible payload/direction assignment;
wrong owner/interface/cross-System refusal; no-op/history/identity admission;
invalid raw port/interface forms survive native Save/cold reopen while accepted
contracts and earlier native/property/usage data remain exact. Cross-view reuse is
explicitly P17b, not claimed by this child.
