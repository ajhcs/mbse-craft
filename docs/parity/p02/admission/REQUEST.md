# Exact-job admission request

The current user explicitly authorizes fresh resource admission and P02's
prepared regression/blank-author checks. This request is for the compiler phase
only: one Cargo job, one test thread and a new dedicated target.

- Frozen job: [JOB.json](JOB.json)
- Job SHA-256: `f74c46e5522493e28e59494b2514ee686f49757d2017ceb41054ea472206a750`
- Requesting owner: `01a16f82-07a1-7cd3-bb10-3ef7b8e83905`
- Existing integration owner: `01a1653d-9c16-76a3-ad80-df98ba1e5c94`
- Reviewed PR 1 head: `5b907c333d423df60de7d539da2a0900941825aa`
- Build-input SHA-256: `fe2c135180c5489306ff46f139c6fad5b5553b72749169bc25edea70b3dfca04`
- Exact target: `/workspace/build/builderbob/mbse-craft/p02-readiness/target`
- Commands: the three exact argument arrays in JOB.json. No renderer, service,
  provider, new observer or additional agent context is requested.

The existing resource-admission coordinator must supply a genuinely fresh
`builderbob.job-bound-health/1` packet bound to that job hash, plus explicit
compiler/target allocation and independently checked current app-server identity.
Original observer/watchdog/heartbeat freshness must be at most 120 seconds at
each new child launch. HOLD, UNKNOWN, identity mismatch or expiry refuses launch.

Use a supported route to deliver original packet bytes and corresponding
allocation. Do not copy historical packets, change timestamps, assume a hold
ended, start a replacement monitor or use a raw task mutation client. Neither
this request nor the runner grants or authenticates the origin of admission.

After delivery and ordinary action review, preflight with:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run-p02-regressions.py \
  --admission work/p02-admission/ADMISSION.json \
  --allocation work/p02-admission/ALLOCATION.json \
  --record work/p02-preflight-new.json
```

The explicit `--execute` switch additionally requires a clean candidate, exact
allocation and the recorded disk reserve. Source, lock, runner and job bindings
are checked before each command. A job that starts a child is spent: preserve
results and separately freeze only an unexecuted remainder if later held. Do not
rerun successful commands merely to refresh status. Partial/failed/held records
remain separate. There is no background watcher or automatic retry.

Freeze the actual binary and native action script after the admitted build and
obtain separate renderer admission. GUI execution is outside this compiler job.

## Actual acquisition result

[STATUS.json](STATUS.json) records the tool-catalog/coordinator discovery.
[PREFLIGHT.json](PREFLIGHT.json) is the actual guarded preflight: source bindings
verified, then held before any compiler/test/GUI child because no original
exact-job packet is available. This is capability unavailability, not an action
approval denial or a product-test failure.

The working catalog provides list/read/wait controls but no supported
`send_message_to_thread` or `handoff_thread`. Existing coordinators can be read
but cannot be dispatched from this chat. A request for a working coordinator
route is pending with the user. General execution permission is already given.
No infrastructure, security setting, observer or existing job has changed.
