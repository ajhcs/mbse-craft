# P38a acceptance contract

The manifest profile is `mbse-craft.source-project-set/1`, schema version 1. Every member has a unique exact key, an explicit `NativeSaved` or `SemanticV2` primary, a lowercase SHA-256 pin and an optional list of `NativeViews` or `Opaque` companions. Paths are portable relative paths under the manifest directory. Aliases, traversal, URI paths, unknown fields and duplicate keys are refused. Native views require a native member.

Acceptance criteria:

1. Equal local IDs in different members, including identical copied native projects and external XML with the same original IDs, resolve to distinct qualified keys. Old keys cannot resolve against changed primary pins.
2. Whole native snapshots preserve meaning, invalid raw drafts and opaque values. Exact bound companions preserve all eight presentation maps. Inspection cannot mint a runtime checkpoint, inverse proof, target mutation or native admission.
3. Missing, changed, unavailable and incorrectly bound companions remain visible without changing the native primary. Unverified primaries are retained as bytes and never interpreted. Recovery envelopes require the existing restore route.
4. Offline folder copy and reopen preserve the portable digest, identities and native snapshot. No directory discovery, network request, ambient URI resolution or adjacent companion acquisition occurs. Opaque companions are not semantic resolver inputs.
5. Recheck reads only the original descriptor and originally listed files. Changed descriptors cannot introduce new acquisition paths. Frozen data and keys stay unchanged.
6. Unix reads use retained directory descriptors and NOFOLLOW at every ancestor and leaf. Symlinks refuse; swapping a pathname cannot redirect an existing capability. Other platforms explicitly refuse this route.
7. Limits refuse explicitly: 16 members, 64 artifacts, 128 KiB manifest, 64 MiB captured inputs, 64 MiB additional expanded archive assets, 64 MiB serialized interpretation and 100,000 element identities. No silent truncation, identity deduplication or partial success after aggregate overflow.
8. The frozen-source workspace regression and desktop build pass. Existing checkouts and registered heads remain preserved. Native browser acceptance follows in P38b.

Native keys enumerate top-level records; the full native snapshot remains available. External keys and partial/refused stages reuse semantic-set/2. Hash verification is about captured bytes, not source truth, semantic completeness, currentness or engineering acceptance. Encoded limits do not qualify RSS or performance. The broad specifications, integrated exits and full parity claims remain open.
