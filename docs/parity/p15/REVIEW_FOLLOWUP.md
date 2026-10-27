# PR18 review follow-up

Same-agent code review found that P15 retained property drafts durably but kept the
new-definition name and kind choice only in UI memory. The final change stores
name, System scope and value/scalar choice in the native type-draft journal.
Property edits retain this entry; model Undo/Redo merges the current journal, and
native Save/reopen retains unfinished definition work. Adding a matching definition
consumes only its name, counts that raw change, and preserves kind/scope. An
unrelated API addition preserves the pending form.

Two added regressions exercise invalid257-byte names, property edits, atomic Add
refusal, Undo/Redo, actual native file roundtrip, matching-name consumption, raw
revision and repeated identical edits. The first review build refused at compile
time because the System selector borrowed records while publishing widget bounds;
`failed-definition-journal-borrow` retains its zero-test receipt/log/source/job.
The correction collects immutable System rows before rendering.

The original405-test/two-native-run candidate remains in `verification` and its
source/job in `before-definition-journal`. Final evidence is separate in
`final-verification`. Earlier renderer failures/diagnostic remain unchanged.
