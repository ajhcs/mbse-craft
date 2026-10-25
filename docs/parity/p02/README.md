# P02 readiness review of draft PR 1

## October 25 execution contract update

The operator asked to complete P02 on BuilderBob and questioned the continued
observer assumption. The historical observer was a bounded run whose planned
last sample was October 24; its current Lenovo process state is unknown. Current
project instructions require isolated worktrees and one Cargo job, without a
permanent observer requirement. A newly exposed native send also failed with
`AbsolutePathBuf deserialized without a base path`; coordinator readback stayed
idle ([exact response](admission/route-recovery/NATIVE_SEND_20261025.json)).

[LOCAL_JOB.json](admission/LOCAL_JOB.json) freezes a new bounded local run under
the current task authorization. It keeps the original source, commands, offline
lock, single-job settings and dedicated target. Its
[runner](../../../scripts/run-p02-local.py) checks the actual Samsung mount,
disk reserve, current app-server descriptor headroom and conflicting compiler or
native processes immediately before each child, while holding an exclusive
target lock. It creates no observer or service and claims no original Lenovo
admission. Original held jobs, packets, route failures and plans remain intact.
The following sections retain the original readiness review and historical
gate; actual new outcomes will be recorded separately.

**Decision: retained evidence is consistent; P02 integrated acceptance remains
pending.** Reviewed PR head: `5b907c333d423df60de7d539da2a0900941825aa`.
The review is isolated on `codex/p02-readiness`, stacked on reviewed PR 4 head
`725e28c1c852429449a5f7303ba981db66f775fc`. No PR 1 application source is changed.

## Verified in this review

- All **657 current build inputs** exactly match the retained final source-edit
  manifest, SHA-256
  `fe2c135180c5489306ff46f139c6fad5b5553b72749169bc25edea70b3dfca04`.
- The current lock matches the integrated UI/build receipt. Its exact source,
  binary and native-receipt bindings agree.
- The three compressed logs match both archived and decompressed hashes. They
  retain **50 historical focused core/desktop results** and **six later integrated
  UI results**. The core wrapper's expired-health pause remains recorded; the
  later UI/build receipt remains distinct. These are not a new 56-test run or a
  current full-workspace qualification.
- All **587 retained Craft/egui/winit input files** match their extraction
  manifests. Required license files remain present. Winit's actual manifest and
  `LICENSE` specify Apache-2.0; it is not assumed to use the other components'
  dual-license filenames. This byte check is not a new legal/conformance audit.
- The preserved source-edit executable hashes to
  `4df50cf94f84200b9461fbb64f00f07b36d7a493dec1a5abb90e8e89264a901f`
  without execution. The two retained captures match their recorded identities;
  one was visually inspected as historical source-inspection evidence.
- Static review covered native Session/command/draft boundaries, calculation and
  preservation test targets, close/focus policy, source-name retention and
  private create-new placement. No additional blocking implementation defect was
  established in those reviewed paths. This is not an exhaustive repository or
  security review.

Machine-readable results and exact historical test names are in
[READINESS.json](READINESS.json). The audit reads receipts from the pinned PR 1
Git object, not mutable declarations on the review branch.

## Readiness finding and coverage gap

**P2 — The newest native receipt does not establish P02's blank-author workflow.**
The final source-edit epoch's retained `native-independent.kernspan` contains
only one empty System record. Its receipt and captures demonstrate external
Mission inspection/name-copy and an unchanged independent native save/reopen.
They do not demonstrate authoring the System/Mission/Capability/Objective/Measure
graph, calculations and retained `9e` draft on that final compiled source.

The older alpha has a real five-record blank-authored model and calculation
workflow. Its source hash is `323fe697…`; the paired-fork epoch has its own source,
lock and native workflows. Neither may be relabeled as the newer `fe2c1351…`
integrated P02 workflow. Exact newest build-input equality preserves that newest
epoch's actual evidence; it does not expand the behaviors it exercised.

| P02 criterion | Existing evidence | Review disposition |
| --- | --- | --- |
| Exact candidate/build identity | Current 657-file closure, lock, build log and preserved binary agree | Verified by this read-only audit |
| Relevant model/raw/calculation/preservation regressions | Earlier suites plus limited newer focused waves | New declared P02 focused gate remains not run |
| Blank authoring with typed links and calculations | Actual older alpha workflow | Not yet qualified on the declared integrated P02 candidate |
| Accepted 95 plus raw `9e`, history, failure/Cancel, normal close/reopen | Individual historical workflows across distinct epochs | New integrated M1-AUTHOR acceptance remains not run |
| Current scope/limitations | PR 1 description and latest epoch document bounded source rename/inspection and unqualified platforms | Consistent; no broader product/parity claim |

## Concrete next execution packet

[EXECUTION_PLAN.json](EXECUTION_PLAN.json) specifies the focused regression argv,
offline locked inputs, one Cargo job and a new dedicated `/workspace` target,
followed by the self-contained M1-AUTHOR native workflow. It does not launch or
reserve anything. The model test targets exist in the reviewed baseline.

Fresh original resource-health evidence and an exact-job compiler allocation
were not supplied for this review. The existing execution contract is retained
in the local occurrence-navigation job and launcher: fresh health, current
process identity and explicit target allocation precede new compiler/renderer
launches. Historical 2026-10-23 admissions are expired; this review does not infer
that a hold ended or manufacture/refresh their timestamps. The user authorized
this readiness work; the remaining gap is fresh execution admission, not a new
request for general permission.

After an admitted build, freeze its exact binary and native action script and
obtain separate renderer admission. P02 becomes accepted only when the focused
gate and actual blank-author/failure/Cancel/save/reopen observations pass on the
declared candidate with normal cleanup and the required review. P03–P06 and the
other existing feature drafts remain separate.

## Reproduce the evidence audit

The subsequent execution request and actual held preflight are retained in
[the exact-job admission packet](admission/REQUEST.md). The source check passed,
but no original packet or exact-job allocation was available; zero regression
commands and zero native runs started. P02 remains unaccepted. The guarded
runner is prepared, not product-qualification evidence.

The later [supported dispatch recovery](admission/route-recovery/README.md)
verified the documented CLI queue route to the existing Linux integrator. Direct
delivery to the resource coordinator failed because that rollout is not in the
Linux store; the integrator's completed relay also reports no supported
cross-host route. Fresh original admission remains unavailable.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-p02-readiness.py
```

On the original BuilderBob workspace, optionally add
`--verify-preserved-binary` to rehash the explicitly pinned local executable.
`--output` creates a new report inside the isolated worktree and refuses to
replace an existing report. A successful audit prints acceptance **pending**;
it cannot run Cargo, the app, a renderer, a provider or a service.
