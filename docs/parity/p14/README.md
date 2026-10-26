# P14: versioned extension admission and protected migration

Built on merged P13 PR #16 (`5b032f4db6504f3517da54d6f2c691a7d8b2ab81`)
in isolated `codex/m4-p14-extension-schema`. Optional snapshot.extensions carries
app-owned mbse-craft.extensions/1, schema1. Stage1 remains kernspan.stage1/2 with
its original document/record identities. Absence serializes as before. Unknown
optional namespace JSON remains opaque; required unsupported meaning refuses.

| Acceptance | Concrete evidence |
| --- | --- |
| Old documents migrate to a new copy | Actual CLI migrates admitted Stage1/1 and current Stage1/2 synthetic sources. Removing only the added extension header gives the exact original core snapshot, including IDs, allocators, revision counters and all journals |
| Opaque fields and drafts remain intact | Actual current source includes opaque nested JSON, raw9e, accepted95metre, calculation2.5km and a pending creation journal. CLI byte/value comparisons and native Save/cold reopen retain them; API migration tests also preserve full snapshot/journals |
| Newer schemas refuse safely | CLI and API refuse extension schema99 and unsupported required namespace without a copy. Native admission rejects unsupported profiles/versions and legacy typed-key collision. Unknown optional JSON survives store reopen and cannot silently enter restricted SysML export |
| Sources and occupied targets survive | CLI rejects original-source overwrite, existing copy target and repeated migration. All original fixture hashes and occupied output bytes remain unchanged; independent runtime IDs and inherited protected sources are retained |
| Stage1 identity is not relabeled | New copy uses existing Stage1/2 writer; separate extension header is app-owned. Native document/record IDs remain exact. No SysML or full structural meaning is inferred from the header |
| Existing work is preserved | All41 pre-existing worktree heads/statuses remain unchanged. Native runs close normally, exact owned processes/authentication files absent |

The frozen681-file closure
`baaad0a3e9f002c971ec6a3ce57200cf6cd48243d4dea95a1f3645e634ce6c58`
passed **397 distinct workspace tests**, including **six new P14 cases**, and
desktop build exit0. Runtime commit `8fadd1b`, executed checkout `fb14e73`;
[compiler receipt](verification/COMPILER_RECEIPT.json) pins full IDs. Binary SHA-256:
`86801ed83fc1106d1d72be0a59eeece1cc482d240acb1e0af2cb9298a00b8b68`.
See [manifest](SOURCE_MANIFEST.json), [job](JOB.json),
[test pairs](verification/TEST_RESULTS.json) and [contract](CONTRACT.md).
No runtime source changed after that passing job; dependencies/vendor are unchanged.

[CLI request](CLI_JOB.json) pins executable, verifier and immutable fixtures.
Seven actual cases passed: two migrations and five explicit refusals. The
[CLI receipt](verification/cli/RECEIPT.json), stdout/stderr, generated copies and
source provenance are retained. Example from a built candidate:

```sh
mbse-desktop --state-dir /workspace/owned-state \
  --migrate-extensions /workspace/source.kernspan \
  --destination /workspace/new-copy.kernspan
```

The operation is separate/headless and never replaces the active GUI session.
It requires a fresh native destination. This migrates model storage, without
copying/rebinding a pre-existing presentation sidecar; existing sidecars stay
untouched and presentation recovery remains explicit.

Two private Linux native runs opened the actual CLI output, saved an exact
native/views pair, then cold-reopened the same snapshot and exact persisted views
digest with empty detached history. Raw9e/95metre/2.5km remained visible, and audit
reports the separate extension profile/version. [Raw native receipt](verification/native/RECEIPT.json)
retains actions_completed_review_pending; [same-agent pixel/byte review](verification/NATIVE_REVIEW.json)
is separate. Zero native action errors. All43 compiler/CLI/native artifact hashes
are indexed in [ARCHIVES.json](verification/ARCHIVES.json).

The first CLI fixture incorrectly put an opaque field beside Record.fields.
Existing shape admission correctly refused it before any positive case/output.
Bad fixture, request, helper and raw receipt remain in the package; that receipt's
original active status is not a PASS. Corrected fixtures use distinct names/job/
output; see [correction](FIXTURE_CORRECTION.md). Legacy fixtures are explicitly
synthetic derived Stage1/1 examples, not historical customer-file compatibility.
The verifier now records subsequent failures explicitly. No Cargo test replay
was needed for this verification-only correction.

This is a schema/migration foundation. P15–P22 own typed properties, parts, ports,
connectors, values, clone/move/delete and profile/tag semantics. Optional JSON,
even if a later version recognizes its key, remains opaque until an explicit
supported required-namespace admission/migration; it must not be silently promoted.
Full structural modeling, independent/platform/interchange/domain acceptance and
Milestone4 exit remain open; all56 broad specifications remain NOT_RUN.
Build/cache/test output stayed on verified Samsung /workspace under fresh local
preflight, target lock and one Cargo job/thread. No external-project/shared-skill,
service/provider/release change or compatibility relabeling.
