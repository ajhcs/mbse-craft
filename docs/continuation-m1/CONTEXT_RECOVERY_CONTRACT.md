# Reconciled context and recovery contract

Source authority: immutable Kernspan c80037d2259cd67a8e813cf3bc6c01bb1226a460 and MBSE-Craft53133f6. Existing dirty Kernspan files are not inputs. This contract adds bounded editing commands, not a new normative ontology.

- Actor and Organization: existing name and same-System scope only. Unknown source properties remain preserved/read-only. No invented role/purpose or actor relation schema.
- Stakeholder: existing interest text and affiliations map keyed by Organization ID string, value Unknown/Internal/External. Add SetAffiliation with checked Stakeholder/Organization kind and same-System scope; removal deletes only selected authored reference. Existing opaque actors[] is preserved; do not reinterpret it.
- Environment: existing nested context records exact id/type Context/name/scope/kind Description|Assumption|AcceptedConstraint/text. AddContext and SetContextKind validate container/scope/kind; contextName/contextText use retained raw fields and existing undo semantics. Preserve opaque inputs[].
- Expectation: existing text, expressedBy Stakeholder and concernsSystem same scope. Creation requires Stakeholder owner. Preserve existing source/interchange mapping.
- Native unknown fields survive; new commands refuse wrong-kind/cross-System targets atomically without raw journal mutation. Do not globally reject old opaque native data merely because the new editing command cannot author it.
- Existing closed SysML profile maps affiliations/context and refuses nonempty opaque actors/inputs/observations. New fields must roundtrip within the existing supported profile, with explicit semantic-loss refusal elsewhere. No behavior/scenario execution implied.

Recovery discovery: bounded read-only scan of immediate app-owned stage1-session directories, regular nonsymlink recognized Recovery envelopes only; return document identity/revisions/raw generation/hash/count and explicit rejected-file diagnostics. Never auto restore/delete/overwrite originals. Restore remains an independent dirty document with protected source provenance. Metadata sorting does not imply engineer preference or acceptance.

Optional periodic recovery is app-local, off until opted in, with settings under explicit app-owned config directory and recovery under separate data directory per inspected ArtCraft policy. Exact snapshot acknowledgments use existing IoTicket/request/session/model/raw/hash checks. A stale completion cannot claim current recovery or Saved. UI must expose current/failure/pending state, discoverable restart chooser, cancel and source protection. No shared service/scheduler/global config. Relevant craft-settings/quit/docpath APIs remain subject to exact source audit before adoption.

Acceptance: wrong-type/cross-System rejection preserves model+raw; affiliation/context edits UndoRedo/save/reopen and supported SysML roundtrip; opaque unsupported exchange refusal; discovery corrupt/symlink/profile/bounds cases; restart/corrupt-main restore; failure/stale receipt/dirty-close safety; actual Linux authoring and recovery chooser workflows. All previous alpha evidence remains a distinct milestone.
