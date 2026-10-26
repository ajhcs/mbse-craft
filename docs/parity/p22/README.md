# P22 exact profiles and literal tags: bounded evidence

Build on P21b PR27 mergee8475982d3586a15b47425c4ba34dffb5c4a5afd. The final
709-file source60490bd237df79089654450f3b09fe60bc95a3207b21bceb421df8c27fc71f6f
built binaryee7f1dbcc4a0b904f31b338d7bc6b95191923a2f5d75fa4221c2a1bb74fc597f.
It passed the failed-catalog/unexecuted regression remainder, desktop build and two
native runs. Earlier initial sourceb4b5dcdd4969c1d2b955091d03a221fa17740a2c22af5c5c852c986b6446c7ab
passed139 cases, including the new rendered profile test, before one stale catalog
count assertion failed. Only catalog test counts and the deletion-cap message
changed before the remainder. Counts stay separated; no summed final-source
full-workspace-run claim.

| Acceptance | Exact evidence |
| --- | --- |
| Exact profile identity and immutable contents | Native registered urn:mbse-craft:qa version1.0 Valve stereotype. Version1.0.0, uppercase URN namespace, wrong Mission target and changed same-key label type disable mutation and preserve accepted metadata. Model cases also reject unknown stereotype case, future fields, duplicate JSON keys and custom target kind CustomValve. |
| Literal metadata cannot reclassify a domain/native record | Native labels applied to Measure1003, ComponentDefinition1021 and Part1023 with their actual kinds. Complete original native record map remains exact. Model tests cover Boolean/Integer/finite Real/Text, exact numeric spelling/Unicode, missing/extra keys, invalid values and tampered target kinds. Metadata does not execute domain behavior or resolve external URIs. |
| Independent raw form and model history | Native Undo/Redo removes/restores last application while retaining raw namespaceurn:unknown, target1023? and incomplete valuesJSON. Wrong form cannot Apply. Cold retains exact invalid form and applications without model history. |
| Clone/move/delete preserve applications and identities | Actual native Clone1021 allocates1027..1032, copying only applications for owned target1021/1023 to1027/1029. Shared registry and original3 applications exact. Native Delete lists/removes both copied applications with six fresh records; Undo restores whole clone Store and Redo removes it while preserving newer profile/deletion1027? forms. Same-ID move retains application targets without retargeting. |
| Captured native/views4 and cold reopen | Actual saved profile-tags pair and cold second Save equal full Snapshot and all four presentation maps. Original native5 records,9e draft,2.5km definition, typed members/defaults/overrides and prior forms intact, apart from explicitly edited refactor and new profile/deletion forms. Allocator remains1033 after clone/delete. |
| Preservation and cleanup |45 prior available checkouts keep exact heads/status.18 user-removed historical directories retain registered heads; removed contents unverifiable. Two normal app/display exits; input hashes, owned process absence and auth cleanup checked. |

[Contract](CONTRACT.md), [source manifest](SOURCE_MANIFEST.json),
[initial failed job](attempts/01/JOB.json),
[failed/unexecuted remainder](REMAINDER_JOB.json),
[last unexecuted syntax job](JOB.json),
[test inventory](verification/TEST_RESULTS.json) and
[native review](verification/NATIVE_REVIEW.json) identify the bounded slices:

- Initial139 successful tests and one failed catalog-count case retained unchanged.
- Corrected catalog1 and unexecuted model/exchange344 passed; assurance/catalog doc
  groups and desktop build passed. Ten new model cases are in this group.
- Ten remaining syntax checks passed after exact prior-inventory reconciliation.
  No earlier successful group replayed. All prior case names plus11 new names are
  accounted for; this is an inventory, not a single summed final-source run.
- Two native runs, zero controller errors, with actions/UI audits/pixels/checkpoint
  pairs in the [47-artifact hash index](verification/ARCHIVES.json).

No dependencies, vendored licenses/notices/build scripts changed; pinned baseline
review applies. One Cargo job/test thread, locked/offline, fresh job-specific host
preflight and target lock. Reopen used that same frozen target scope consistently.
Same-agent implementation/evidence review only. Optional empty fields preserve old
untagged wire values; pre-P22 strict typed readers refuse unfamiliar fields rather
than silently interpreting them. Unknown optional namespaces remain opaque and
unsupported required/future schemas refuse. Local namespace/version/tag/target
identity and bounded primitive metadata are qualified; full profile standards,
NoMagic/Craft/platform, custom domain execution and independent engineering
acceptance are not. M04 integrated exit and all56 broad parity specs remain NOT_RUN.
