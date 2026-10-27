# P30a finite activity authoring core

Build on P29 PR38 merge7d7247b. Extend existing required type Store/Journal and
Editor::Typed authority; no second model allocator/history/I/O engine. Optional
empty-omitted activities preserve old bytes; strict older readers reject new fields.
Explicit existing extension admission required. No native record reclassification.

Activity owns native System scope, partitions, nodes (Initial/Action/Decision/Merge/
Final), In/Out value-typed pins owned by Actions, and Control/Object flows. All IDs
use the existing shared monotonic native/type allocator and collision/high-water
checks. Partitions/nodes/pins/flows cannot refer across activities; pin value types
must be existing same-System Value definitions. Object flow requires exact payload
identity, Out->In. Initial has no incoming/Final no outgoing control flow. Decision
flows alone require finite literal guards true/false/else; duplicate guard/else
branches refuse. Control cycles and incomplete diagrams are authored data, with
explicit diagnostics, never execution or liveness proof.

Raw flow form retains invalid endpoint/guard text and prior accepted flow. Remove retains journals and refuses remaining references; whole activity removal is
explicit. Apply
validates atomically before allocating or changing accepted data. Native model
Undo/Redo preserves newer raw journals/high-water. Existing save/recovery/cold-open
must preserve activities and raw proposals; malformed/unknown/version/alias/identity/
byte/count/reference failures refuse rather than truncate or retarget.

512 total accepted type/activity elements and existing extension256KiB bound apply.
Activity references prevent invalid type/System deletion by existing final Snapshot
validation; prior deletion impact/refactor/UI do not claim activity authoring/copy/
complete impact. No implicit flow execution, general guard expressions, fork/join,
nested partitions, executable code/provider, full SysML/NoMagic/Craft compatibility.
P30b native creation/editor/diagrams remains required before parent completion.
All56 broad specs/integrated exits/scale/platform/independent engineering acceptance
remain unaccepted. Regression counts and core source/build evidence are distinct
from actual native activity acceptance.
