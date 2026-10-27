# P04 layout recovery qualification

This isolated stack carries existing PR 3 head
`a0079d5232996e9e6350f5bae71138a9b2d276ed` after P03. The original PR 3 branch
and worktree remain unchanged. All **661 current build inputs** match final
source SHA-256 `0735b58512f11edeb07d9fc49b3c14afbda3cb15698bb3a3e1c7b79635565239`.

The actual recovery behavior candidate `933e871` passed **71 focused tests** and
the native timer/failure/reopen workflow. The successor `8937db7` changes exactly
one displayed status string, from “Layout changes awaiting preservation” to
“Layout not saved to bound sidecar”; all other build-input bytes are identical.
That successor has its own passing build and independently reviewed native
label check. These remain two distinct evidence epochs.

| P04 criterion | Retained actual evidence |
| --- | --- |
| Layout-only timer recovery | Ordinary 15-second timer preserved native and unbound layout recovery after a layout-only edit |
| Held drags defer recovery | Real pointer held 35 seconds; no due recovery until release/quiet period |
| Disabled recovery stays inactive | Changed layout retained with no additional recovery across an ordinary interval |
| Failure/Cancel/retry retains geometry | Owned sidecar-lock failure retained live geometry and guarded close; Cancel retained work; verified lock removal followed by ordinary timer retry |
| Save/reopen and independent recovery | Explicit native/bound views reopen restored geometry; native Recover created an independent DocumentId and fresh pair |

[AUDIT.json](AUDIT.json) records the exact current closure, one-string successor
proof, all 71 historical test names, all 18 native capture hash checks and final
binary verification. Six relevant full captures are included. Complete original
receipts, controlled fixtures, action events, native independent review, source
manifests and compressed logs remain under
`docs/continuation-m2b01/layout-periodic-recovery/`.

Both original native epochs record normal cleanup and independent review within
their stated scopes. This audit launched no compiler, renderer or observer and
replayed no successful workflow. It does not turn the 71-test epoch into a new
test run on the wording successor. P05/P06 integration and final combined
Milestone 1 verification remain separate, as do broader platform, accessibility,
performance, engineering and full-parity acceptance. No main merge is included.
