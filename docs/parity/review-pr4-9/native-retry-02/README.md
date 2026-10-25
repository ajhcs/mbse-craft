# Native controller retry: passed

The previously disk-held two-run controller check completed on the fresh retry.
The unchanged 8 GiB disk reserve and all other local preflight checks passed
before both launches; exact free-byte values are recorded in the original
[receipt](evidence/RECEIPT.json). No reserve relaxation or disk cleanup occurred.

The exact reviewed executable remains SHA-256
`90dfe7eb3709d7ca57e174bfdbd06942d140f8c21f823e74d1bf3b58abe7f844`.
All 669 application source inputs match the previously reviewed closure; no
application/harness code, compiler command or build input changed for this retry.

1. First actual app opened the existing exact native/views pair and closed
   normally; app and driver exits were 0.
2. A separately admitted second actual app opened the same pair. After the first
   driver closed, the controller reported only the second as live. The actual
   FINISH helper refused completion and retained active status while it was live.
3. After the second app/driver closed normally, literal FINISH completed with
   both owned run records verified. Both app/display identity pairs and both
   authentication files were absent; all original input hashes stayed unchanged.

There were **zero action errors**. The executing agent reviewed both captures;
[REVIEW.json](REVIEW.json) records those checks separately from the immutable
receipt's original `actions_completed_review_pending` status. The old partial
attempt and its expectation error remain intact under
[../native-partial](../native-partial/RECEIPT.json).

The [archive manifest](ARCHIVES.json) binds the original/stored/decompressed
bytes, including actual controller actions, XTest events, captures, UI audits,
preflights and per-run identities/exits. [Preservation](PRESERVATION_AFTER.json)
verifies all 26 pre-existing worktree heads/statuses unchanged. This is a
controller lifecycle regression smoke, not new full-feature or engineering
acceptance. No Cargo tests/builds were replayed.
