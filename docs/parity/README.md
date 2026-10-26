# Parity matrix and acceptance package

For current execution and PR status, see [Milestone 1 results](MILESTONE1_RESULTS.md), [Milestone 2 results](milestone2-results.json) and [Milestone 3 results](milestone3-results.json). This package introduction describes the original P01 scope; later execution records are separate.

This is the first preparation PR in [the agreed roadmap](../PARITY_ROADMAP.md),
P01 of Milestone 1. It starts the scope/fixture freeze; it does not finish all of
Milestone 1, integrate existing feature branches, or qualify application behavior.

## Read the package

| File | Purpose |
| --- | --- |
| [roadmap.json](roadmap.json) | 58 ordered PR positions, dependencies and ten milestone exit gates |
| [matrix.json](matrix.json) | 40 CAT capability rows, 16 Craft interaction rows and unresolved scope gates |
| [MATRIX.md](MATRIX.md) | Readable index of every capability, current status, PR and acceptance specification |
| [acceptance.json](acceptance.json) | 56 prospective acceptance specifications, input hashes and evidence requirements |
| [milestone1.json](milestone1.json) | Concrete integrated baseline, paired-save/recovery and occurrence-navigation vectors |
| [baseline.json](baseline.json) | Exact implementation/continuation heads, existing draft PRs, assigned paths and target limitations |
| [sources.json](sources.json) | Versioned source identities and limits on what each source establishes |
| [craft-apps.json](craft-apps.json) | Twelve apps in the upstream source audit plus separately researched FigCraft |
| [p02 readiness review](p02/README.md) | Read-only audit of PR 1 evidence, actual coverage gap and prepared unlaunched execution plan |
| [P07 format and provenance](p07/README.md) | Merged P07 format implementation, acceptance results and preservation evidence |
| [P08 live Apply/Undo](p08/README.md) | Merged P08 transaction, native recovery/copy/reopen evidence and preservation |
| [P09 command catalog](p09/README.md) | Merged P09 workspace commands and native acceptance evidence |
| [P10 scoped shortcuts](p10/README.md) | Draft validated overrides, persistence, field/model/canvas scopes and native evidence |
| [fixtures](fixtures) | Original synthetic engineering/UI/source inputs; not production or accepted engineering models |

The historical 86-row handoff supplies the CAT identifiers and feature titles.
Its 46 Capella records remain in their original research package and are not
silently counted as No Magic coverage. The new package does not copy proprietary
models, screenshots, recordings or restricted library code. Existing synthetic
fixture attribution stays with the unchanged source fixture.

## Scope and current status

The proposed No Magic comparison includes Cyber-Systems Engineer/Cameo,
Systems of Systems Architect, Model Analyst and Collaboration Studio. The 2026x
documentation is an initial version target. Exact installed edition/builds and
selected plug-ins have **not** been independently inventoried. Full language,
specialist extension and product-portfolio coverage is still an open scope gate.
There is no assertion of complete SysML, UML, UAF or No Magic compatibility.

Craft comparisons concern relevant shared interactions, not implementation of
every creative engine, file format or tool. The upstream twelve-app source audit
is pinned by Git blob identity. FigCraft is separately sourced. Thirteen app
references are an initial inventory, not an exhaustive count of the evolving
Craft family. Interactive reference builds remain unpinned and unrun.

`partial_bounded` means some custom-domain behavior or earlier source-specific
qualification exists. It does not close the named full capability.
`not_implemented` means no accepted implementation of the complete capability;
it does not deny the presence of a parser, DTO or partial source path.
`unqualified` means the required interaction/target comparison has not passed.
Every new acceptance specification is `not_run`; there are no product acceptance
results or completion percentages in this package.

## What counts as an acceptance fixture

`fixtures/engineering.json` specifies an original synthetic future model and
independently checkable numeric expectations. It is not a native import file.
Its assembly has two Sensor instances and one Recorder instance; connector
ports belong to those instances, while type definitions remain distinct.
`fixtures/ui.json` specifies strings, state combinations and proposed sizes and
budgets. Target hardware, mode, sample counts and thresholds require review
before performance qualification. Desired large-model tiers may exceed current
limits; no limit increase or workload truncation is authorized by this fixture.

`fixtures/source-primary.xmi` and `source-other.xmi` intentionally share a local
ID, contain a cross-member reference, an unresolved member and unknown metadata.
They are minimal original XML fragments, not complete vendor projects. Their
source-set oracle is stored in `milestone1.json`. The checker validates the bytes
and these declared fixture facts, **not** MBSE-Craft's parser or admission.

Milestone 1 vectors are self-contained baseline scenarios with their own input
assets and P02–P06 prerequisites. Their `related_spec_ids` only trace to the
later full-capability scenarios; they do not inherit those scenarios' future
requirements. Passing M1-AUTHOR therefore cannot close general typed property,
normative mission-architecture or schema-migration parity.

The existing `kernspan.stage1.qa-fixture/1` inspection input is referenced by path
and hash, without copying or rewriting it. It is not a serialized Session save.
Future native fixtures must be emitted through the actual Session/checkpoint
protocol; do not hand-author fake save acknowledgments or native/view bindings.

## Execution and evidence contract

For each future scenario, retain exact source, lock/build, binary and input
hashes; OS/backend/tool identity; actual actions and expected/observed outputs;
success/failure/cancel/stale cases; and independent review. Tests, GUI checks,
external-reader results and engineering acceptance are separate results. Store
large/native/private execution artifacts only in assigned `/workspace` output
epochs. Report missing/held/failed work without rewriting earlier receipts.

Milestone 1 ends only after P02–P06's integrated native workflow passes. This PR
does not launch Cargo, a renderer, an external engine, a provider or a service.
Publishing this documentation draft does not authorize merging any existing PR.

## Static package validation

From the isolated checkout, run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-parity.py --self-test
```

The dependency-free checker verifies identifiers, dependency ordering, matrix
coverage, source/app/fixture references, SHA-256 input bindings, concrete fixture
oracles, roadmap rendering and that planned scenarios cannot claim a pass. Its
negative cases exercise forged acceptance, duplicate IDs, dangling references,
forward dependencies, removed scope gates, modified fixture bytes, contradictory
oracles, invalid connector ownership and ambiguous baseline prerequisites. A passing
checker establishes package consistency only. See [VALIDATION.md](VALIDATION.md)
for this PR's actual result and limits.

The optional `--verify-local-sources` flag additionally checks that cited
historical Git objects exist in this workspace. Some continuations remain local
and unpublished; a fresh clone does not need their Git objects to check this
package. The source registry records exact object/content identities and their
availability. Neither mode fetches source or executes upstream code.
