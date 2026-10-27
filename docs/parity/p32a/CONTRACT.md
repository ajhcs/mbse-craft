# P32a flat state-machine authority and keyed proposals

Build on P31b PR43. Existing type Store/Journal/Editor/allocator/history/native I/O
remain authoritative. Add optional strict state_machines/state_machine_forms fields;
empty fields omit exactly for old bytes. Old readers refuse unknown nonempty fields.
One native System owns each flat Machine. States Initial/State/Final, named owned
Event identities, and owned source/target/event transitions with existing literal
true/false/Else guard type. All accepted IDs share native global allocation,512
accepted type/activity/state-machine element cap and256KiB extension cap.

No hierarchy/orthogonal regions/entry-exit actions/payloads/execution is inferred.
Accepted wrong/foreign targets/events, incoming Initial, outgoing Final, Initial
event/guard, multiple Initial and duplicate/colliding IDs refuse atomically.
Incomplete authored machines remain diagnostic. Competing unconditional/true or
Else transitions and duplicate event labels expose exact identities in diagnostics;
no selection policy is invented. P33 will own finite execution policy.

Machine/State/Event/Transition proposals have canonical family/owner/id slots,
128 family proposals/4096bytes per raw string, strict field/schema admission,
detached-ID protection and high-water reservation. Apply migrates creation slot
to accepted-ID slot inside the existing atomic candidate; selected Discard affects
only raw text. Invalid targets/guards stay raw, newer proposals survive model
Undo/Redo/removal and native save/cold/recovery. In-use removals refuse. Legacy
deletion explicitly holds unknown state-machine/proposal impact, never claiming
complete inventory or global deletion authority.

P32b owns independent diagram data, P32c native authoring/diagnostics/layout/
history/save/cold acceptance. P32 parent remains open. All56 broad specs/integrated
exits and independent engineering/full NoMagic/SysML/Craft/platform/scale/production
acceptance remain open. No external project, dependency/license/build-script changes.
