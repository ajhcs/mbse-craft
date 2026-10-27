# P25 coverage and change impact: bounded evidence

Builds on P24 PR30 merge0c528b7ddb49d6a2662f3d77698ca030c9bbb745. Read-only
coverage uses the existing Board/native/type/calculation/evidence authorities.
New evidence captures accepted supplier/ancestor dependency context, allowing an
upstream edit to stale retained consumer evidence without rewriting the consumer
or original receipt. Existing unknown-reference detection is exposed read-only;
prospective evidence size counting now includes P24 trace/test-case maps.

| Acceptance | Exact evidence |
| --- | --- |
| Known missing links and orphan IDs | Native removes Verify10, creates R-ORPHAN12 and runs background coverage. Requirement2 lacks Satisfy, requirement1 lacks Verify, and12 is the isolated root. Other relation kinds/labels never substitute for Satisfy/Verify; tests cover detached native references and exact declared-kind counts. |
| Explicit uninterpreted impact | Owned fixture adds only register.unknown future.trace/1 to the exact P24 register. Query discloses it without interpreting/dropping it or claiming complete impact. Tests cover native optional data, requirement metadata and128-detail limit with exact total. |
| Real evidence and upstream staleness | Native evaluates calculation1003 and attaches evidence13 to consumer2. Initial query has no freshness/provenance mismatch. Supplier1 statement changes, Copy5 follows, consumer2 whole record and evidence13 stay exact. Refreshed query reports requirement dependency context changed; possible review requirements1/2/3/5 and evidence13 are listed separately. |
| Exact source and navigation | Native query result selects requirement2 without changing accepted records/drafts/native model or query root1. Supplier edit makes old results stale and hides navigation until explicit rerun. Renderer test also rejects delayed results against different accepted bytes with the same revision counter. |
| Save and cold behavior | Actual initial/final register Saves preserve the original consumer/evidence and opaque metadata. Cold exact register reopen restores no query/runtime proof; explicit rerun discloses dependency change, absent evaluator proof and a context-stamp mismatch from the new native instance_id. Whole native Snapshot hash/data remain unchanged. Cold fresh register and native Saves preserve all accepted values/maps. |
| Preservation |51 prior available checkouts retain exact heads/status;18 user-removed directories retain registered heads, removed contents unverifiable. Inputs unchanged; two normal app/display exits, owned process/auth cleanup. |

Final717-file source `5a0ac04348d1cfc69849b4eda3230bfe5ae500a66695ef354dee74af74c0f227`
passed all531 workspace tests and desktop build, including12 new cases. Binary
`503951e240b609bc6a03ff20826eb4734be948aa5ad1277440fa0b4ed2167c1b`
drove both native runs with zero controller errors. All prior case names reconcile.
No compiler/application failure. [38-artifact index](verification/ARCHIVES.json)
verifies original/stored bytes and losslessly compressed logs/actions/audits.
[Same-agent evidence interpretation](verification/NATIVE_REVIEW.json) is separate
from unchanged raw review-pending receipts. No dependency/license/notice/build-script
changes from the reviewed baseline.

Queries inventory local declared links and possible review work; they never prove
satisfaction, execute a new test engine or record verification Passed. At most256
evidence records are assessed, with explicit unexamined total; UI shows first128
requirement/evidence/detached/impact rows. Unknown impact remains unproven, and no
large-model performance qualification is claimed. Legacy dependency-free evidence
omits the new field; old evidence without context cannot become qualified merely
because a dependency appears. Hashes compare supplied state, not source truth or
runtime proof. All56 broad parity specifications, integrated milestone exits and
platform/independent engineering/full NoMagic/SysML/Craft acceptance remain open.
