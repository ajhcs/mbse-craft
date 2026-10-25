# Admission acquisition and guarded preflight

Recorded 2026-10-24 for the new direct request to obtain admission and execute
P02. This is a pre-execution hold, not a failed product test or an approval
denial. No historical admission bytes or timestamps were reused.

The frozen job SHA-256 is
`f74c46e5522493e28e59494b2514ee686f49757d2017ceb41054ea472206a750`.
Its separately pinned runner SHA-256 is
`97aed460478c3d215f2aaf2daf8984b0c59ee060b2d38dae3ba37f1c7d9f8647`.
The job, status and actual preflight bind the same exact bytes.

The guarded preflight command was:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/run-p02-regressions.py \
  --record docs/parity/p02/admission/PREFLIGHT.json
```

Exit code **2**: source/lock/runner bindings verified, followed by
`No original exact-job admission packet available`. The receipt contains no
commands or owned jobs. The proposed dedicated Cargo target does not exist.
Zero regression commands, builds and native runs started.

The Samsung `/workspace` mount was verified in host context. Supported task
listing succeeded after correcting an initial client-side limit from 100 to its
maximum of 50. The original MBSE owner and existing resource/coordination chats
were read successfully. The actual catalog has list/read/wait but no supported
send/handoff control; no acquisition request could be dispatched. The resource
coordinator's retained 2026-10-23 report is not a fresh observer/watchdog packet.
An unsupported CLI/service route was not substituted.

Static checks verified runner syntax, exact job/runner/status/receipt bindings,
new Markdown links and whitespace. Twenty other pre-existing worktree heads and
statuses matched the preservation snapshot. No existing source, receipt, target,
observer, service, permission or infrastructure setting changed.

The user has been asked for a working supported coordinator route. General
execution permission is already authorized. P02 remains incomplete until real
admission, focused regressions, build and separately admitted native acceptance
are completed. The original read-only readiness report remains unchanged.
