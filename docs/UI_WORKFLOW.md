# Native workflow contract

MBSE-Craft's desktop is a new egui/eframe shell over the bounded MIT Kernspan Stage1 model, calculation, graph and persistence contracts. It uses pinned ArtCraft `craft-ui` **IconButton** controls for accessible Undo/Redo and **WidgetPalette** styling. Model persistence uses the integrator's separately documented Craft filesystem adapter. This does not imply compatibility with every ArtCraft application, backend, file type, or service.

## Connected editing path

Start `mbse-desktop --state-dir /workspace/<owned-state-directory>`. A new document contains an unnamed System. Name it and edit Purpose in the left controls; select a canvas card or navigator row to move the same selection. Add Mission and Capability records, select a Capability, and check explicitly chosen same-System missions. Add an Objective and explicitly select its typed System/Mission/Capability subject. With that Objective selected, **Add Measure** opens the named creation panel. Creating it makes the model's required Objective relation. No subject or measure owner is inferred from canvas proximity.

Add Measure appears consistently in the creator and left actions. It is disabled until an Objective is selected. Add Expectation similarly requires a selected Stakeholder. Creation text is journaled in the native document and retained when the panel is dismissed. Select the same owner and reopen Add to resume that draft.

Choose the Measure quantity/unit before entering a criterion. Current accepted core measure pairs are dimensionless/percent, dimensionless/one and length/metre; calculations have their separately typed unit table. Invalid or incomplete text remains visible with its validity and previous accepted criterion. Model revision and raw journal version are shown separately. Ctrl+Z / Ctrl+Shift+Z dispatch the core field-text history while an editable model field is active and model history otherwise; accessible ArtCraft Undo/Redo controls use the same dispatch.

Add calculation creates an actual typed input `x` and expression `x`, with the Measure's unit. Edit input values and expression through the raw journal. Add scalar, length, duration or count inputs; their generated names are shown and can be used in the expression. Compatible unit choices physically convert accepted input quantities through the model API. Select another output to create a real calculation dependency. Output units can be selected independently; incompatible algebra produces diagnostics rather than a fabricated value. Derived values remain separate from criteria and observations. Bare expression literals are dimensionless; bracketed physical-unit literals are unsupported.

The primary canvas shows selectable typed records and authored relation arrows. **Details** opens the actual record properties in a click popout, with an action returning to editable left controls. The persistent relation ledger lists every typed graph edge with complete endpoint labels and IDs, including calculation dependencies. Selecting a ledger row opens the typed relation and endpoint selection actions. The ledger is deliberately authoritative for discovering relation text at high density; the initial canvas has a simple fixed grid rather than a general collision-free layout.

## Persistence and error behavior

The source/action path (Open, Import SysML, Recover, Export SysML copy) is separate from the **Save native model to** path. Pending navigation captures its requested target path. Saving current work during that prompt cannot silently reinterpret the requested source as a save destination. Existing model saves retain core byte-conflict checks. The UI reports success only after the operation result, consumed receipt and current save/recovery status agree.

New/Open/Import/Recover and closing a dirty document present a preservation prompt. Continue requires a successful verified save or a current durable recovery copy. Cancel or a failed operation leaves the current session present. A close permit is bound to the exact preserved snapshot digest. Recovered and imported documents are independent drafts with no inherited native save destination. The same source/action path supports the restricted, honestly labeled SysML import/export subset; it is not a conformance verdict.

To recover after a damaged main file, use the path of a previously acknowledged recovery copy in the source/action path and choose Recover. The corrupt source is not repaired or overwritten by this action. Save the restored draft to a fresh native destination.

## Native verification surface

Optional CLI flags: `--open FILE`, `--fixture JSON`, `--recover FILE`, `--select ID`, `--capture NEW.png`. Input flags are mutually exclusive. Fixture launch is explicitly illustrative. `--capture` requests an actual eframe native screenshot and refuses an existing output path. It does not substitute for exercising GUI controls.

The owned state directory contains `ui-geometry.json`: a non-semantic automation receipt with actual rendered control rectangles, current pixel scale, snapshot hash/revisions, selection, save status, and feedback. It exposes no model-writing endpoint. Native verification can target the recorded bounds through XTest on a private display and then observe updated geometry and durable native files. Geometry and screenshots alone do not prove accessibility, usability or engineering acceptance; executable workflow evidence is recorded separately by the verifier.

## Current limits

No production services, AI providers, external execution, hosted model calls or full SysML conformance are implied. Record creation is typed and bounded. Actor/Environment/Organization records currently expose names and preserved source properties; richer context editors remain a gap. Grid positioning is presentation-only; free placement, routing optimization and generic diagram editing are not implemented. Undo history is session-local, while raw drafts and accepted values persist. Automatic recovery is not enabled by this shell; the explicit Recovery copy action and preservation prompt create durable copies.
