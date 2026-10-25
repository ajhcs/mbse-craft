# Review of draft PR 4

Reviewed head: `ebf977344bd2d52fa2826ce76db140a6fc00cb84`, against
`5b907c333d423df60de7d539da2a0900941825aa`. This is a same-agent follow-up review,
not an independent product or security acceptance review.

## Findings addressed

1. **P2 — Assembly fixture mixed type and instance endpoints.** `PORT-IN` named
   `T-RECORDER` as its owner, but `PORT-OUT` belonged to instance `I-A`. The future
   assembly therefore lacked a Recorder instance for its connector endpoint.
   Added `I-RECORDER`, bound its port to that instance, updated the declared
   instance count and asset digest, and checked instance/type/port/connector
   identity, direction and item compatibility. A rehashed invalid input must
   still be rejected; a digest alone does not establish a valid fixture.

2. **P2 — Declared source expectations could contradict the checked XML.** The
   original checker accepted changing `source_set_oracle.unknown_value` to
   `WRONG`; it checked hard-coded XML facts without comparing the declaration
   future consumers would read. It also accepted a scalar `steps` value or empty
   target scope. Bound the declared oracle to those fixture facts and require
   nonempty typed lists for baseline/scenario metadata. Negative regressions
   reproduce all three previously accepted counterexamples.

3. **P2 — Baseline vectors ambiguously referenced later acceptance prerequisites.**
   M1-AUTHOR required only P02 but used `fixture_ids` for full-capability scenarios
   depending on P14/P15/P19/P29/P35/P36/P47/P58. Renamed the informational links
   to `related_spec_ids`, declared actual input assets separately, documented
   that baseline success does not close those broader capabilities, and reject
   baseline prerequisites outside Milestone 1.

## Verification

`PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-parity.py --self-test
--verify-local-sources` passes with **18 negative cases rejected**, including the
new counterexamples. Whitespace checks and new Markdown links pass. No compiled
application source, prior receipt, native format or existing feature branch is
changed. All acceptance specifications and baseline vectors remain `not_run`.

No additional actionable finding remains in this preparation-package review.
P01's unresolved scope gates and P02–P06's actual native acceptance work remain
open; this review does not convert them into passes.
