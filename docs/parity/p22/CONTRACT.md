# P22 exact profiles and literal extension tags

Build on P21b PR27 mergee8475982. A local registered profile is identified by its
exact namespace and version; its stereotype/tag names are exact. Namespace and
version are never normalized, aliased, fetched or inferred. Registering different
contents under an existing exact identity refuses; a distinct version is separate.
Unknown namespace/version/stereotype/target-kind/tag keys or future fields refuse
without accepted mutation. Recursive duplicate JSON keys refuse before admission.

A stereotype declares an explicit set of existing native/typed target kinds and
exact primitive tag keys (Boolean, Integer, finite Real, Text). Every key is
required. Applications bind the exact profile/stereotype identity and current
target kind. Tags are literal metadata, never model references, a second concept
engine or domain behavior; a Valve stereotype cannot reclassify a Measure or turn
custom domain JSON records into native/SysML records. Existing primitive literal
validation is reused; admitted spelling and Unicode Text stay exact. Unsupported
arbitrary domain record kinds refuse. Unknown optional namespaces remain opaque;
required namespaces/future schemas keep their existing safe-refusal rules.

Accepted registry/applications extend the existing versioned typed namespace with
optional empty-omitted fields. Old untagged wire content remains exact; older strict
readers safely refuse unfamiliar fields rather than silently dropping them. Profiles
have at most32 stereotypes/32 tag keys,16 registered exact identities,256 model
applications and existing extension byte budgets. Raw profile/values JSON strings
are bounded32KiB each, other raw strings4096 bytes; malformed/incomplete form is
retained independently. Register/Apply/Remove validate a cloned candidate before
publication via existing native/views freshness and model history. Remove requires
the exact existing application identity and retains newer raw text.

Native and typed record IDs/meaning/calculations/drafts/views are not changed by
metadata. Clone copies applications only for remapped owned typed members and
lists them in its preview; the profile registry is shared unchanged. Move retains
the same target identities. Semantic deletion lists/removes affected applications
alongside member/value treatment, with128-application review cap; model Undo restores
exact metadata and prior views while retaining newer invalid profile form. No hidden
reference semantics are inferred from tag Text/Integer spellings.

Tools > Exact profiles and literal tags (CtrlAltJ) shares modal/write/gesture/history
guards, uses real renderer actions and retains form focus/identity. Acceptance must
cover exact registration/application, namespace/version/target-kind refusals,
immutable profile conflicts, malformed JSON, model Undo/Redo, clone/delete metadata
handling and actual captured native/views4 Save/cold whole-Snapshot equality with
all prior native/typed/raw/view meaning intact. P22 closes a bounded roadmap position;
M04 integrated milestone exit and all56 broad parity specs remain unaccepted unless
separately exercised. No full NoMagic/SysML/Craft/platform/independent engineering claim.
