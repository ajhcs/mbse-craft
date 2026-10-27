# P13 file selection and long-operation feedback

Build on merged P12. Reuse the pinned Craft Outcome type: Selected, explicit
Cancelled, Failed and Indeterminate. This host has no installed OS picker backend;
the supported chooser is application-owned, with directory browsing, explicit
paths, native Open/Recover, restricted Import and native Save. No OS-native picker
or full platform parity claim. Existing explicit-path commands remain available.
Directory reads are asynchronous, bounded to500 entries, preserve exact UTF-8
paths, and publish only into the still-open matching directory. Non-UTF-8 paths
are not listed; an unrepresentable path is not silently rewritten for selection.

All workspace New/Open/Recover/Import reads now run on one background lane. A
candidate publishes only if original native freshness, selection, views digest,
save destination and native/write/continuation guards remain current. Newer edits
or views reject the candidate; they are never discarded. Cancellation prevents
publication while retaining the lane until the reader returns, rather than
pretending OS I/O was interrupted. Failure/disconnected worker preserves current
work. Close and conflicting commands retain the parent while a chooser/read is
active. File outcomes and progress remain visible; native/paired writes retain
existing frozen-snapshot receipt and retry behavior. Existing export and auxiliary
editors retain their established contracts and are outside this chooser slice.

Acceptance combines actual worker reads and directory/native UI selection,
Cancel, indeterminate empty selection, missing-file failure, Save/reopen, and
controlled delayed replies covering stale drafts/views, cancellation and worker
disconnect. No artificial sleep/delay or test hook is added to production code.
Source fixtures, exact failed attempts, receipts, raw input events and screenshots
remain retained. These are bounded same-agent Linux checks, not broad engineering
acceptance or a Milestone3 exit proof by themselves.
