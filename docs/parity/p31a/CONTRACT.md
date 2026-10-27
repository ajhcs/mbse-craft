# P31a finite activity control execution

Read accepted activities through the existing typed Store. Explicit start freezes
whole native Snapshot digest and exact document/runtime/model/raw Freshness. A live
Execution is non-deserializable and read-only with respect to Editor; serialized
reports are historical data and cannot resume or mint live authority.

Finite policy v1: one control token, one Initial, Action and Merge visits have at
most one outgoing control flow. Decision literal true wins; otherwise Else wins;
false is disabled. Each step records visited node and exact chosen flow, Final
records completion. No enabled outgoing flow is Deadlock. Cycles are permitted
but budgets1..4096 bound visits; a Final on the last visit still completes. Cancel
terminates before another visit, terminal calls are idempotent, source changes
terminate active execution as StaleSource. Historical outcomes remain inspectable
and expose currentness separately. Partitions are documentary membership only.

Typed pins/object flows refuse explicitly: no action code, payload, fork/join,
concurrency, dynamic expression, solver, external execution or verification engine
is introduced. Missing/unsupported source, fanout, bad budgets/Initial refuse
before live execution. Literal branching, else fallback, false-only deadlock,
cycle budgets, cancellation and stale-source fixtures are specified independently
as static tables before running the implementation. Complete Snapshot/journal/
history preservation is checked. P31b owns native controls/status/trace/cancel and
cold historical boundary. P31 parent remains open until native acceptance/review.
All56 broad specs/integrated exits and independent engineering/full parity remain
unaccepted.
