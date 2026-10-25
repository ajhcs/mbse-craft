# P03 persistent native views qualification

P03's bounded baseline criteria are covered by the existing actual native run
and focused tests on identical source bytes. This isolated branch carries PR 2
head `557ee47e0bb7237a5064c8e51718506431f8f825` after completed P02; the existing
PR 2 branch and worktree are unchanged.

All **660** current build inputs match source SHA-256
`e6cc2113f5283a9b4a86c837a782a12028f1733c83f13cfb2cc70809b18691f4`.
The retained compiler epoch passed **30 model/IO and 8 controller tests** and its
scoped build; the later actual native receipt uses executable SHA-256
`199226d504ccd1cc066b30bb29e8a8a64f0fb0eeaf2dd63c47b8668bc492a9b4`.
The earlier PR description's “No GUI ran” describes its compiler receipt and
omits the later native run; these distinct records are reconciled here.

| P03 criterion | Verified evidence |
| --- | --- |
| One semantic element in two views | Actual shared Mission 1000, Primary occurrence 5 and Review occurrence 6; native labels agree |
| Independent geometry and reopen | Actual drag, hide/layout Undo and explicit paired save/reopen preserve occurrence IDs and separate geometry |
| Native success/views failure retains work | Owned cooperative-lock fixture: changed native bytes saved, previous sidecar bytes retained, live geometry retained and close guarded; verified lock removal and retry |
| Stale acknowledgments cannot save later edits | Exact retained model/IO/controller tests, including stale views/recovery/native and foreign-origin cases; timing remains controller-only |

[AUDIT.json](AUDIT.json) binds every source file, all 35 original native PNG
hashes, five native artifact-receipt hashes and the original independent crop
review's exact receipt hash. Original receipts, selected full captures and
controlled-fixture results are included here. The historical test/build logs,
names, held partial wave and build-only continuation remain in
`docs/continuation-m2b01/verification-09df25d/`.

The original native run ended normally; app/driver/server exits were 0, six
recorded owned identities were absent and Xauthority was removed. No compiler
or native workflow was replayed for this audit. The old receipt's pending
review status remains intact beside its subsequently received independent crop
review, which covers narrow pixel claims only.

Explicit semantic-edge native workflows, live delayed/foreign acknowledgment
timing, platform, accessibility, performance and engineering acceptance remain
outside this baseline. P04–P06 and the final combined Milestone 1 candidate still
need their own qualification. No merge to main or alteration of another owner's
work is included.
