# P11 transactional native workspace docking

Build directly on merged P10 PR #13. Use the existing pinned MIT/Apache-licensed
Craft docking model and renderer without modifying upstream or dependencies.
Three stable application panel IDs identify Model browser, Properties and Native
views. They borrow the one live Workspace; no panel owns a cloned native editor.

Acceptance: moving, tabbing, splitting and floating preserve every panel exactly
once, native selection, incomplete raw text, native/views content and histories.
A staged batch validates the complete registry and layout before publication;
stale revisions, exhausted revisions, invalid geometry or forbidden Close refuse
without a partial move. Native writes/occurrence gestures and auxiliary editors
block docking. Visible fields keep their stable panel/body/widget IDs across
moves. Hidden canvas controls leave no stale clickable audit bounds. Escape or
an invalid proposal publishes no layout action. Floating means egui areas inside
this application viewport, not native OS windows. All three panels are nonclosable.

Use stock tab drag/drop and the Panel layout controls, both through the same
transaction handler. Panel geometry is separate from native presentation geometry
and native/model/text Undo; this PR does not persist workspace arrangements.
P12 owns restart/perspective/reset/off-screen/corrupt-workspace handling. Native
model save/recovery and existing shortcut configuration remain authoritative.
Bounded Linux/source/controller/native evidence is not full Craft/platform or
engineering acceptance.
