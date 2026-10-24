# Supported dispatch recovery result

**A supported Linux dispatch route works; fresh P02 resource admission is still
unavailable.** This supersedes the narrower earlier conclusion that this chat
had no dispatch route at all. The original held preflight and STATUS.json remain
unchanged as historical evidence.

Official OpenAI documentation and the installed Codex CLI both expose
[`codex queue`](https://learn.chatgpt.com/docs/cli/reference), which queues a
message for an existing session by UUID or exact name. The existing managed
server's read-only version query succeeded in host context: CLI and app server
are both 0.164.0. No daemon start, restart, update or configuration change ran.

Two exact requests were issued through that documented command with an explicit
existing Unix-socket endpoint:

| Target | Actual result |
| --- | --- |
| Existing resource coordinator `01a165b9-9828-768d-a69d-bcd556fb1032` | Queue exit 1: no rollout found in the Linux store; no delivery or resource-coordinator turn started |
| Existing MBSE integrator `01a1653d-9c16-76a3-ad80-df98ba1e5c94` | Queue exit 0; message `01a16fcb-901d-7f72-9d17-387c03220a73`; a new turn was observed active, then completed |

The existing integrator received an admission-only relay request, explicitly
excluding feature development, product writes, agents, builds and GUI launches.
Its completed response reports no supported send/enqueue/handoff route to the
resource coordinator in its own catalog. It delivered no relay and obtained no
fresh packet or exclusive target allocation. Its unrun occurrence job and
receipts were reported untouched. This is an actual existing-owner response,
not an assumption from an idle status.

The resource coordinator is backed by the durable/Lenovo environment. The
working Linux queue does not make its rollout available locally. The host's
existing SSH aliases include no Lenovo route. Plugin searches returned no
suitable dispatch provider; connected document sessions were empty and the
browser had only an about:blank page. No unrelated host, new pairing, new
observer, raw task-mutation client or permission change was attempted.

## Exact retained evidence

- [Direct coordinator queue receipt](queue-resource-coordinator.json)
- [Existing integrator queue receipt](queue-existing-integrator.json)
- [Exact completed relay response](RELAY_RESULT.json)
- [Direct request text](request-resource-coordinator.txt)
- [Admission-only relay request](request-existing-integrator.txt)
- [Structured route result](RESULT.json)
- [Frozen compiler request](../REQUEST.md)

The required next change is on the existing client/producer route: continue the
existing **Repair BuilderBob handle exhaustion** conversation on its original
connected host, or restore supported cross-host dispatch provisioning. Supply
genuine fresh original observer/watchdog evidence and exact-job allocation for
job `f74c46e5522493e28e59494b2514ee686f49757d2017ceb41054ea472206a750`.
General execution permission is already granted. A local descriptor sample or
an old packet cannot substitute for that original admission contract.

P02 remains incomplete: zero regression commands, builds and native runs have
started. The guarded runner will still refuse without actual fresh admission.
