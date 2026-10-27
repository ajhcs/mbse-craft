# P32b independent state-machine diagram data

Build on P32a PR44. Views profile7/recovery7 adds optional state_machine_diagrams
under existing Document/ViewId/camera/layout history/allocator/I/O. Each diagram
binds source document and accepted Machine, with stable state occurrence IDs,
expected Initial/State/Final kind, geometry/visibility/pinning/presence. No semantic
state/event/transition copy or second model. Transitions project exact current
source/target/Event/guard only with unique active visible state endpoints; absent
or ambiguous endpoints are counted. Event identities remain semantic data/ledger.

Only state occurrences inhabit each state-machine view; mixed native/port/edge/
activity payloads refuse. Shared4096 occurrences/1024per-view/16views/8MiB encoding
and existing history bounds apply. Strict canonical decimal wire IDs, duplicate/
foreign source/global presentation ID/geometry/future profile refusals preserve
source. Existing profiles1..6 refuse the new field; profile7 survives native ports,
connectors, viewpoints, activity creation, clear/close, capture/save/recovery/cold.
Older maps remain exact except intentional layout additions/protocol metadata.

Native model removal/kind change detaches original occurrence identities. Only
observed exact inverse same-runtime model history reactivates; cold serialized
state cannot mint inverse authority. Source selection and layout stamps remain
exact; layout history preserves native semantic/raw data. Unsupported legacy
source inspection/copy/mapping refuses before partial inventory, preserving raw
inputs. Copy-origin collision checks include state-machine occurrence IDs.

P32c owns actual native authoring/diagnostics/diagram rendering/interaction/save/
cold acceptance. P32 parent remains open; P33 execution not inferred. All56 broad
specs/integrated exits and independent engineering/full NoMagic/SysML/Craft/
platform/scale/production acceptance remain open. No external project/dependency/
license/build-script changes; one admitted Cargo job/test thread.
