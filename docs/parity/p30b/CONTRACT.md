# P30b durable activity forms and independent diagram core

Build on P30a PR39 mergeaf4b55c. Existing type Journal gains bounded keyed forms for
activity, partition, node, pin and flow creation/editing. All unfinished proposals
survive switching and model history/save/cold-open, independently of accepted data.
Migrate the older single flow proposal exactly into its stable slot when the new
API is used; refuse conflicting/aliased/unknown forms. Explicit Apply validates all
fields atomically and uses the existing allocator/Editor history. Explicit discard
removes only the selected raw proposal. New drafts cannot claim existing foreign
identities; detached old forms/high-water remain retained. Legacy deletion discloses
unsupported activity/forms impact and refuses a complete plan.

Existing views document/history/allocator gains profile6 activity diagrams, each
bound to source document/activity and existing ViewId/camera. Node/partition/pin
occurrences have stable presentation IDs, expected kinds, geometry/manual visibility/
pinning/presence. Multiple diagrams reuse semantics with independent layout/camera.
Flow rendering is derived from exact current flow endpoints; no second flow store.
Model removal/type change detaches; observed inverse model history alone can
reactivate, and cold data cannot forge that proof. Layout history never edits the
native model. Existing count/geometry/byte/history bounds apply jointly.

Profiles1..5 without activity fields stay exact; older readers reject6. Native/views
save/recovery/cold-open, typed port/connector/viewpoint actions retain6 and metadata.
Source inspection and cross-document reviewed activity diagram copy/mapping through
the older native-only UI are unsupported and refuse
without partial publication. No implicit execution, engineering verdict, general
SysML guard language or full NoMagic/SysML/Craft compatibility. P30c actual native
editor/rendering/selection/save/cold evidence is required before parent completion.
All56 broad specs/integrated exits/scale/platform/independent engineering acceptance
remain unaccepted. Core evidence is separate from rendered native acceptance.
