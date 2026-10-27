# P33a finite state-machine execution

Read accepted flat state machines under the existing typed authority. Explicit
start validates the whole native Snapshot, freezes exact runtime/document/model/
raw freshness and digest, and takes a finite sequence of exact owned Event IDs.
Live Execution is read-only, process-local and not deserializable; reports are
historical data without resume authority.

Policy v1: enabled completion transitions precede the next queued event. None and
literal true guards are equally enabled, false disabled; Else only if none are
enabled. Multiple equally enabled transitions or multiple applicable Else routes
terminate with Conflict and exact sorted transition IDs, with no arbitrary choice
or event consumption. An owned but unhandled queued event is consumed and recorded
as a same-state step. No completion and no queued event is terminal WaitingForEvent,
not a deadlock claim. Reaching Final completes immediately, including on the last
budgeted step. Each step takes at most one transition or consumes one ignored event.
Budgets1..4096 and at most4096 queued events bound work; completion cycles cannot
run without limits. Cancel precedes the next step; stale active source terminates;
terminal history remains unchanged with currentness reported separately.

No dynamic event injection, payloads, hierarchy/orthogonal/concurrency, entry/exit/
action code, arbitrary expressions, external I/O, solver, reachability/liveness or
verification engine. Fixed graph/event/trace tables are specified before executor
implementation. Core tests must preserve entire Snapshot/raw/opaque/history and
prove invalid input, conflicts, finite budgets/cancel and runtime/digest freshness.
P33b owns native controls/status/trace/cancel/cold checks; P33 remains open until
that acceptance and isolated reviews/merges. All56 broad specs/integrated exits and
full NoMagic/SysML/Craft/engineering/platform/scale/production remain unaccepted.
