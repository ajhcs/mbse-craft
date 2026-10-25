# P02 BuilderBob execution results

The focused regression/build gate and actual blank-author M1-AUTHOR workflow
passed on October 25, 2026. These results close the execution gap identified by
the readiness review. This PR is ready for review; no merge or independent
review is implied.

## Exact candidate and execution

- Reviewed PR 1 head: `5b907c333d423df60de7d539da2a0900941825aa`.
- All 657 build inputs remain at source SHA-256
  `fe2c135180c5489306ff46f139c6fad5b5553b72749169bc25edea70b3dfca04`.
- Cargo.lock SHA-256:
  `475828c2023805c7f168af22aaa116b7f536ed6d09dfb090f3390a6b68c8454a`.
- Regression checkout: `afb02da4f3963e14435625bd255848ea9fb6fb28`.
- Actual preserved debug executable: 390975040 bytes, SHA-256
  `75bdb7b6df792d1426259d70039ec021afd942d8667a3c11e74d6b90f277a458`.
- Native input script after the harness repair: SHA-256
  `7c4cd7d30d60f7f379246470304bfbefb580b0a4a6122afa205297535e1c686d`.

The [new local compiler job](admission/LOCAL_JOB.json) ran once, with one Cargo
job and one test thread in the new dedicated P02 target. It used the previously
prepared exact offline/locked commands. The local preflight checked current
Samsung workspace identity, disk and app-server descriptor reserve, conflicting
compiler/native processes and an exclusive target lock immediately before each
child. No observer or service was started. This is explicitly **not** an original
Lenovo observer/watchdog admission claim. The operator's current instruction and
clarification superseded carrying that old bounded observer into this new run.
Original held jobs, plans and route failures remain intact; no other job hold
or allocation is released by this record.

## Observed results

| Check | Actual result |
| --- | --- |
| Five declared model integration targets | 62 tests passed; exit 0 |
| Desktop `ui::policies` | 2 tests passed; exit 0 |
| Offline desktop build | Exit 0; exact executable preserved |
| Blank native start | No `--fixture` or `--open`; five records authored through actual controls |
| Typed graph | System purpose, Capability→Mission, typed Objective→Capability and Measure→Objective saved with exact IDs 1/1000–1003 |
| Calculation | Captured UI displays 120 m / 8 s = **15 m/s**, then 2500 m = **2.5 km**; saved definitions independently checked |
| Histories | Model Undo/Redo and focused field Undo/Redo exercised; captured field restoration 95→9e leaves accepted criterion 95 |
| Controlled save failure | Owned missing-parent destination refused with STAGE1_PARENT; current snapshot unchanged |
| Cancel | Dirty close followed by Cancel retains the exact snapshot in the open document |
| Normal close/reopen | Both app exits 0; reopened snapshot exactly equals authored snapshot, including accepted 95 and raw 9e |
| Cleanup | Both app/display identity pairs absent after completion; driver exits 0; authentication files absent |

The authored save's exact file SHA-256 is
`c200117a6d8e48ccd0491d313063fd6d4723af38b1f295f5eea992a413e61626`.
Its canonical saved snapshot SHA-256 is
`b06fece2e6bc277470a6c9bfe931f258ba14eb12f3d7696c30324e455708f0c3`.

## Exact retained evidence

[verification-local-20261025](verification-local-20261025/) contains the full
compiler receipt, three losslessly compressed logs with every test name and
raw/archive hashes, both native attempt receipts, exact input events, captured
pixels/audits, authored/reopened models and a separate same-agent pixel review.
The executable remains under the receipt's exact `/workspace/build` path.

The first native attempt is **partial**, with no normal-exit claim: the driver
scrolled the sidebar instead of the clipped output-unit popup. Its original
receipt, events and captures are retained. The second frozen job corrected only
the popup scroll target and completed the workflow; the app source and binary
were unchanged. No passing Cargo command was replayed. The second receipt's
original `pixels_review_pending` status is preserved; the subsequent
[pixel review](verification-local-20261025/PIXEL_REVIEW.json) records the actual
inspection, without rewriting that receipt.

Existing compiler warnings remain in the exact logs. These checks establish the
bounded Linux software-rendered workflow. Independent review, broader platform,
accessibility, performance, engineering acceptance and full No Magic/Craft
parity remain separate. P03–P06 and the other Milestone 1 vectors are not marked
passed by P02. The original declarative fixture package remains its frozen
specification; this execution record supplies the new observations.
