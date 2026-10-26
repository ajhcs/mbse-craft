# P19 authored defaults, explicit Part overrides and derived previews

Build on P18 PR23 merge239d21d. Accepted typed properties with exact1..1
multiplicity can own a scalar default. A Part usage can own an explicit override
only for a property on its target Component type. Reference usages and mismatched
property contexts refuse. Defaults and overrides occupy separate optional maps;
changing a default never rewrites an override. Clearing an override returns that
Part to inheritance; clearing an unset/default value is an admitted model no-op.
Values are Boolean, i64 Integer, finite decimal Real or bounded literal Text;
type/multiplicity changes that would invalidate stored values refuse atomically.

Per-target durable raw forms retain independent value and preview-expression text.
Selecting an existing form never overwrites its raw work. Apply validates a cloned
candidate; invalid literal/target/kind changes preserve accepted data. Clear and
model Undo/Redo affect authored values while retaining newer raw forms. Default/
override entries refer to existing semantic IDs and allocate no new identity.
Canonical property/Part map keys, bounded forms/counts and aggregate extension byte
budgets apply; old wire values gain no empty optional value fields.

Read-only numeric previews use accepted effective values as binding x in the
existing bounded Stage1 calculator. They are explicitly unitless; no unit/kind
conversion, engineering calculation, solve/simulation or full SysML value semantics
is inferred from property/type names. Boolean/Text previews and integers beyond
exact numeric precision refuse. Invalid expressions/evaluation errors remain
visible and never write authored values. Derived reports carry native snapshot,
model/raw revisions and editor-instance freshness; any edit/reopen makes an older
report outdated. Reports are ephemeral, have no Apply-to-model path, and are not
serialized into the native snapshot. Expression text itself is durable raw work.

Native/views4 Save and cold reopen retain defaults, overrides, every raw form and
all previous semantics/qualified port/route data. UI uses Tools > Defaults and
instance values, CtrlAltY, shows provenance and bounds instance choices to128;
private effective-value audit rows are bounded to512. Current scalar/Part contract
excludes collections, nested instance paths, reference-instance values and units.
All56 broad parity acceptance specifications stay NOT_RUN.
