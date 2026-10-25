# P09 workspace command catalog

Build on merged P08 PR #11 at `d7cca1c17002368f58264e7686e8c71a9b73b6dd`.
The app owns a versioned set of 21 stable, label-independent workspace command
IDs, grouped in File, Edit, View and Tools menus. These include native file
operations, model/field history, layout history, search/usages/impact and opening
the existing presentation, external, assurance, library and settings tools.

All catalog menu items, existing toolbar equivalents and fixed keyboard bindings
call the same dispatcher. Availability is calculated by the same policy and
rechecked at activation. Disabled menu items display the prerequisite; toolbar
items expose the same reason on hover. The command audit records IDs, labels,
menus, displayed shortcuts, current availability/reasons and the last invoked
ID/surface/scope/refusal. Hints perform no disk reads and authorize no writes.
Existing handlers remain authoritative for actual file/identity/receipt checks.

Acceptance:

1. IDs and fixed shortcuts are unique; all 21 registered commands have a menu and
   shortcut. File/Edit/View/Tools items show their default shortcuts. Registering
   a command requires an explicit dispatcher handler. Existing toolbar actions
   use the same IDs; Quit uses the menu/shortcut and existing close flow.
2. Model and layout Undo/Redo stay separate. The menu captures the native-field
   scope before opening its popup, so moving focus into the menu cannot turn a
   field Undo into a model Undo. Invalid/stale field selection refuses. Plain
   text widgets retain their own Undo keys; repeats do not retrigger commands.
3. Exact Shift/Alt modifiers distinguish Save from Recovery and model from layout
   histories. Only one workspace shortcut dispatches per frame. Pointer/Tab
   transitions and popups cannot accidentally redirect a model Undo.
4. Missing path/selection/history, native/views/pair writes, held gestures and
   preservation dialogs supply explicit reasons without target mutation.
   Fresh activation rechecks current state, not an earlier menu hint.
5. Save still uses native/views paired receipts, fresh destinations and source
   protections. New/Open/Recover/Import/Quit use existing preservation/close
   flows. Cancel retains native records, geometry and unfinished raw text.
6. Native menu, toolbar and keyboard paths demonstrate matching model-history
   and save behavior; layout shortcuts preserve native content; field Undo keeps
   its field scope. Actual saved pairs reopen with native/raw content retained.

The catalog covers workspace actions. Parameterized model fields, individual
relations, view tabs, camera/geometry controls and source-mapping review/Apply
remain explicit domain controls; they do not acquire global accelerators here.
Auxiliary editors retain their own Save/text-edit scope (including assurance
Ctrl/Cmd+S), and native file/history commands explain that boundary. No command
palette, shortcut overrides/persistence, docking, file dialog, provider, service
or release workflow is introduced. P10 owns configured shortcuts. Full Craft
keyboard/accessibility/platform and engineering acceptance remain separate.
