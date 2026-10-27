# P23 hierarchical requirement authoring

Build on P22 PR28 merge24cb4647. Extend the existing mbse-assurance Board and its
independent requirements/evidence register; do not create another requirements or
native concept engine. Requirement.parent is an optional, empty-omitted local
RequirementId. Old flat registers remain exact; strict old readers refuse unfamiliar
parent fields. No native record reclassification or native snapshot embedding.

Create root or child atomically with the existing non-recycling register allocator.
Parent must already exist in this same register. Current and candidate state are
validated around transactions, preventing duplicate/high-water or malformed-ID
state from silently overwriting work. Reader rejects noncanonical numeric map keys
before deserialization can collapse aliases such as1 and01; duplicate JSON keys
and cross-requirement/evidence/audit ID collisions retain existing refusal rules.

Parent projection and cycle validation are iterative, deterministic and bounded by
the existing10000-requirement/32MiB document limits; no recursive depth expansion.
Serialized raw validity cannot claim a parent/statement different from its accepted record; such imports refuse with original bytes preserved. Self, missing and cyclic parent proposals are retained as invalid raw parent drafts
without changing the accepted parent or requirement revision. Empty parent means
root; canonical positive decimal local IDs only. Invalid blank statements remain
raw drafts and keep prior accepted text. Statement admission is bounded nonblank
text, not a new formal requirements language.

Parenting organizes containment only; it does not derive statements, roll up status,
inherit approval, implementation or verification, or execute domain semantics.
Accepted parent changes increment that requirement's revision and participate in
its existing evidence hash/staleness rules. New child starts Draft/UnderReview/
NotStarted independently. Existing register Undo/Redo remains register-edit history
(including explicit raw edits); global/native/text scopes are unchanged. Unknown
source anchors, raw work, evidence and immutable review/decision histories remain
under their existing authority; cold reopen does not manufacture runtime proof.

Native Requirements and evidence editor adds root/child controls, parent draft,
current accepted parent and hierarchy selection. Shows first256 preorder rows with
full depth labels (indent capped16); all records remain persisted. Parent input
is bounded128 UI characters/4096 backend bytes. Independent register Save/replace
uses existing frozen atomic worker and disk-conflict/dirty/close guards. Native
model Save remains separate; native source/typed/profile meaning and views stay
exact while authoring requirements.

Acceptance: cycle/missing/self/duplicate/alias refusal and unchanged failed state;
invalid parent/statement drafts across Save/reopen; deep hierarchy without recursive
failure; exact child IDs/order and statuses; parent-change evidence staleness;
legacy flat/unknown/evidence retention; actual rendered root/child/move/cycle/invalid
statement/status/Undo controls; actual independent register Save and cold reopen
with native snapshot/views unchanged. Same-agent bounded Linux scope; no full
NoMagic/SysML/Craft, trace-family/P24, platform or independent engineering claim.
