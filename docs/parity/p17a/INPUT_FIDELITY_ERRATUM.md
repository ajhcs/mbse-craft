# Native input fidelity correction

Artifact review found that the old native driver did not encode trailing ? with a
named shifted keysym. P16 target1007? and P17a assignment1013?/payload1005? requests
actually produced1007,1013,1005. Original captured audits/saved files prove this.
The acceptance helpers compared later state to what the audit returned without
asserting exact requested text, so those specific invalid-draft native claims
were unsupported. Prior raw receipts, input events, pixels and original/stored
hashes remain unchanged. Existing pure/native-IO invalid-text tests still pass;
actual cycle/wrong-kind/direction refusals and identity/move checks remain valid.

The driver now prevalidates admitted printable ASCII, uses named shifted keysyms
for punctuation, and refuses unsupported text before emitting any key sequence.
Four input-contract tests cover ?/#, all95 printable ASCII, uppercase/path symbols
and refusal of control/non-ASCII input. A fresh exact job executes the missing
invalid-form scenarios with immediate exact text assertions before any refusal,
history or Save/reopen claim. It does not relabel the earlier receipts as PASS.

P16's original invalid native usage-text claim is qualified by this correction in
a reviewed descendant binary; the original P16 closure is not retroactively
claimed to have received the missing input. P17a remains unpublished until the
corrective execution and exact byte/pixel review are complete.
