# P10 scoped shortcut configuration

Build on merged P09 PR #12 at `55322e62e1ea60973c71c56503e1c5f0c474e747`.
P09's stable workspace IDs remain unchanged. One new Tools command opens the
Keyboard shortcuts editor; the catalog now has 22 commands.

Overrides are an additive `shortcut_overrides` string table in the existing
application preferences. They are keyed by stable command ID. Portable syntax
is `Primary+[Alt+][Shift+]Key`; Primary means Ctrl on Linux/Windows and Cmd on
macOS. Supported keys are ASCII A–Z/0–9, F1–F12 and the listed punctuation keys.
No plain typing/navigation key can become a global accelerator. `Unbound`
disables a configured model/workspace binding. `Default` explicitly restores
that command's default; an absent key also uses the default. Blank editor text
writes `Default`. The explicit marker prevents the shared unknown-key-preserving
store from retaining a cleared old override. Unknown command entries stay inert
and are preserved. Configuration has bounded entry/key/value limits.

Acceptance:

1. Editing operates on a retained draft. Malformed chords, reserved text keys
   and duplicate effective bindings (including unmodified defaults) show errors
   and cannot Apply. Apply validates the entire proposed table before changing
   active preferences; multi-command swaps can be applied atomically.
2. Closing the shortcut editor retains unfinished/invalid draft text. Reset
   changes known commands in the draft only, preserves unknown entries and
   requires a valid Apply. No invalid draft changes the native model or active
   bindings. Diagnostics scroll so a long conflict list cannot hide controls.
3. Overrides and reset markers round-trip through the existing asynchronous
   craft-settings worker. Existing recovery values, comments and unknown keys
   remain. Runtime activation is distinct from pending/success/failed file
   persistence. A failed write remains pending and retries the same generation;
   stale acknowledgments cannot save newer settings. Without `--config-dir`, or
   with a future read-only schema, changes are explicitly session-only.
4. Loaded invalid/conflicting overrides remain preserved in the file and editor
   draft, with a visible warning and safe active defaults. No automatic repair
   or rewrite occurs. Newer schemas remain read-only and byte exact.
5. Native fields always retain fixed Primary+Z / Primary+Shift+Z field Undo/Redo,
   even if model Undo is overridden or unbound. Plain text widgets retain their
   Undo keys. Model-history overrides act outside text editing; layout-history
   bindings act on the canvas, separately from native content. Reserved field
   Undo, selection, copy/cut/paste/redo keys cannot be given to global commands.
6. Menus/tooltips/audit display effective bindings, field-specific defaults,
   scope and current prerequisites. Matching keyboard events use those resolved
   chords with exact modifiers, no repeat and the existing dispatch checks.
7. Native UI demonstrates a refused conflict, retained invalid draft, valid
   overrides, independent field/model/layout history, persistence, reset/reopen
   semantics and original native/invalid-raw retention.

Parameterized model/view/camera/copy-mapping controls remain domain controls.
Auxiliary editors retain their own scoped Save/text behavior; the shortcut editor
blocks native file/history shortcuts while it owns editing. No key sequence,
macro, executable script, external provider or service is introduced. Full Craft
shortcut equivalence, platform/input-method/accessibility and engineering
acceptance remain separate from the bounded Linux candidate evidence.
