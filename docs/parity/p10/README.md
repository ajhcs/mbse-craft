# P10: scoped, persistent shortcut configuration

Draft review: [PR #13](https://github.com/ajhcs/mbse-craft/pull/13).

P09 [PR #12](https://github.com/ajhcs/mbse-craft/pull/12) was reviewed in isolated
`codex/review-p09` with no additional actionable finding, then merged at
`55322e62e1ea60973c71c56503e1c5f0c474e747`. The
[follow-up review](../p09/REVIEW_FOLLOWUP.md) records exact scope and preservation.
P10 builds directly on that merge in isolated `codex/m3-p10-scoped-shortcuts`.

The new Keyboard shortcuts editor validates an entire draft before activation,
shows conflicts against effective defaults, supports Unbound and explicit Default
reset values, and saves through the existing preferences worker. Native fields
retain fixed Undo/Redo keys; configured model and canvas history stay separate.
P09's IDs remain unchanged, with one new editor command bringing the catalog to
22. The [contract](CONTRACT.md) defines grammar, bounds, scopes and limitations.

| Acceptance criterion | Concrete evidence |
| --- | --- |
| Conflicts and invalid drafts stay visible | Native duplicate Search/Save binding shows a conflict and disables Apply without native/active/config mutation; close/reopen retains the invalid draft. Tests cover malformed chords, reserved text keys, default collisions, atomic swaps and bounds |
| Overrides persist | Actual native Apply saves four model/layout overrides, preserving the original comment, recovery values, root unknown key and unknown command entry. A cold app launch loads the exact persisted bytes and displays the configured menu shortcuts |
| Reset persists | Native Default reset writes an explicit marker. Another cold launch restores model Undo's default, keeps other custom bindings and does not dispatch the retired override. An actual store round-trip test also proves no old override resurrection |
| Field Undo stays in the field | Native fixed Ctrl+Z and Ctrl+Shift+Z act on field history despite configured model Undo/Redo. API/controller cases include unbound model Undo, stale/plain-widget scope, editor-owned text and context-dependent menu labels |
| Model/layout histories remain distinct | Actual configured Ctrl+Alt+U / Ctrl+Alt+Shift+U address native model history; Ctrl+Alt+L / Ctrl+Alt+Shift+L undo/redo layout with native snapshot unchanged. Tests reject canvas shortcut routing while editing text |
| Persistence failures are honest | Actual worker test retains pending status and retries the same override generation after an owned write failure. Future-schema API/controller checks allow explicit session-only changes and retain original bytes. Existing stale-generation worker checks remain passing |
| Native meaning and originals survive | Actual native/views Save and cold reopen retain exact snapshot/layout digest, empty runtime history, unedited raw 9e, accepted 95 metre and 2.5 km. All 33 pre-existing worktrees remain unchanged |

The frozen runtime source passed **366 workspace tests**, including **14 new
P10 cases**, followed by desktop build exit 0. Exact suite/name pairs and raw
compiler/build logs are [archived](verification/ARCHIVES.json); see
[TEST_RESULTS.json](verification/TEST_RESULTS.json),
[COMPILER_RECEIPT.json](verification/COMPILER_RECEIPT.json) and [JOB.json](JOB.json).
No compile or Cargo test failed in this P10 execution. Existing warnings remain
in the logs. One Cargo job/test thread, locked/offline, fresh Samsung
workspace/RAM/disk/FD/writer checks and the existing target lease were used.
A sandbox-only DEVICE_ALIAS_MISSING was rechecked in host context before
mutation; the correct Samsung mount/UUID was confirmed. No observer, mount repair
or service was started.

Final runtime source: **676 build-input files**, SHA-256
`d66ca8adc39b0e64c7055c15d07b58236bbc3a2d49bbfdf959b0657f17cfe46c`.
Source commit: `07e92158456767defa783bed3351aedd4311cb2d`.
Executable SHA-256:
`4133267cef9eaf94d2404571983e37a706277f5467cb1444e23bce8d87272a89`.
Cargo.lock remains unchanged:
`475828c2023805c7f168af22aaa116b7f536ed6d09dfb090f3390a6b68c8454a`.
The manifest covers build inputs, not later verification scripts/evidence/docs.
No runtime source changed after this passing job.

## Native execution and config scope

The [seeded native job](NATIVE_JOB_SEEDED.json) completed **three actual app/display
runs**, all with normal app/driver exits 0, exact owned-process cleanup and absent
authentication files. There were **zero action errors**. The
[raw receipt](verification/native/RECEIPT.json) retains its original
`actions_completed_review_pending` status; the separate
[same-agent pixel/byte review](verification/NATIVE_REVIEW.json) records subsequent
review without changing it. Expected command/conflict refusals are not failed
harness actions. Both original fixture hashes stay unchanged.

Each cold native run loads an **exact byte copy of the previous actually saved
settings file** into its new owned writable `run/config` directory. CONFIG_SEED
receipts pin source/destination and digest; this verifies persistence and reset
reload while preserving the private driver's existing read-only-root boundary.
It does **not** claim a restart at the same physical config path. Actual store
reopen/reset and worker tests cover file-backed persistence independently.

An earlier [native run](verification/native-readonly/RECEIPT.json) correctly
reported session-only preferences because its test config path was outside the
owned writable run. It also verified conflict refusal, retained draft and an
explicit session-only override while the outside config remained byte exact.
That app/driver closed normally, with zero action errors; persistence checks were
explicitly deferred, not claimed there. Its original [job](NATIVE_JOB.json),
receipt, screenshots and config bytes remain retained. Verification then added
an optional bounded regular-file config seed **inside the existing writable run**.
No writable bind, AppArmor profile or global permissions were expanded.
[Harness checks](HARNESS_TESTS.json) passed **12 tests**, including six new guards
for exact/private copying, destination scope, symlink/size refusal and no
replacement of an existing config. These are separate from the 366 Cargo tests.
All 93 original/stored/lossless archive hashes are indexed in
[ARCHIVES.json](verification/ARCHIVES.json).

[Preservation](PRESERVATION_AFTER.json) verifies all 33 pre-existing worktree
heads/statuses unchanged, including original P09 and its review checkout. There
are no dependency, external-project/shared-skill, provider, service, signing or
release changes. This is same-agent Linux/software-rendered verification;
independent PR/native review, full platform/input-method/accessibility and domain
engineering acceptance remain pending. Unapplied shortcut draft text is retained
only within the current app session. All 56 prospective full-parity specifications
remain NOT_RUN. P11–P13 and the Milestone 3 exit remain open.

Static package validation passed with local historical Git objects verified:
58 PR positions, 56 capabilities, 56 NOT_RUN specifications and 18 rejected
negative cases. `git diff --check` passed. These are consistency checks, separate
from the actual Cargo/harness/native results above.
