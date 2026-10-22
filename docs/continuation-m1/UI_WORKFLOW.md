# M1 connected native workflows

This describes the continuation source, not a replacement for the frozen alpha acceptance receipt. Actual native verification is recorded separately with exact source and binary hashes. The M1 UI uses the reconciled [context/recovery contract](CONTEXT_RECOVERY_CONTRACT.md), the audited Craft settings and frame-policy APIs, and the existing typed Stage1 commands. It does not implement a new normative Actor ontology or general CATIA/Capella file interchange.

## Context editing through shared identities

Select a Stakeholder in the navigator or canvas. Interest remains an independently journaled text field. Affiliations lists only Organizations of the same System, with their stable IDs and eligible count. Each organization offers **No affiliation**, **Unknown**, **Internal**, or **External**. No affiliation removes that selected reference; Unknown is an authored affiliation value. The atomic core command checks both endpoint kinds and System scope. Select that Organization to see reverse affiliated Stakeholders; choosing one returns to the same record instead of making a copy.

Actor and Organization retain their actual authored name and System scope. Actor roles, purposes, scenario allocations and interactions are not inferred from names. Existing opaque actor data is preserved and remains outside the editing contract.

Select an Environment. Each existing nested Context retains its own stable ID, name, text, System scope and kind: Description, Assumption or AcceptedConstraint. Edit name/text through the same raw-journal field path used elsewhere, and select kind through its typed command. **Add context** opens a retained creation draft; dismissing it preserves the draft, while Create allocates a real nested identity. Classifying a statement AcceptedConstraint does not record an independent engineering approval. Existing opaque external-input data is preserved; the UI does not invent an external connection.

Affiliation and context changes participate in model Undo/Redo, native save/reopen and the explicitly supported custom SysML profile. Existing unsupported opaque payload remains preserved natively; an exchange refusal remains a refusal rather than permission to silently omit semantics.

## Focus and history

Model fields register their actual egui widget IDs against complete RawKeys, including nested context item IDs. Ctrl+Z / Ctrl+Shift+Z use core field history only for the exact focused model text widget. Ordinary source/save path and creation text keep their own TextEdit history; they do not dispatch model Undo. A pointer or Tab transition frame lets the widgets resolve the new focus before any global history dispatch. Nontext canvas selection routes model history. Popups and auxiliary dialogs keep their own interaction context. Feedback distinguishes field-text history from model history. These policies have focused pure tests; actual native focus checks remain separate evidence.

## Explicit recovery inventory

**Review recovery copies** reads the bounded model-layer inventory for the owned app data directory. Recognized Recovery envelopes show path, System names, record count, model/raw generations and snapshot hash. Rejected-file diagnostics and scan truncation remain visible. The UI neither selects nor restores a preferred document automatically.

Select a recognized snapshot and choose **Restore independent copy**. If current work is dirty or a snapshot write is pending, the existing preservation prompt remains in force. Restore protects its source, opens an independent dirty document, and leaves the native save destination empty. Corrupt main files and valid recovery copies remain separate artifacts. Refreshing or dismissing the chooser creates no accepted model changes.

## App-local optional periodic recovery

Periodic recovery is **off by default**. Open **Recovery settings** to opt in and choose an interval from 15 seconds to one hour. After the interval and at least 750 ms of editing quiet, a dirty snapshot lacking current recovery is frozen and sent to a write worker. It writes a recovery copy only; it does not autosave the native source file or fabricate a Save As destination. A current recovery is not rewritten while unchanged.

Start with `--state-dir /workspace/<owned-data>`. To persist the opt-in policy, also pass `--config-dir /workspace/<separate-owned-config>`. The explicit config directory must be absolute, owned consistently with the data directory, remain within /workspace, and not overlap the data directory. Without it, settings use Craft's Memory location for the session. The application never selects Craft's Auto fallback. No global service, scheduler, network connection or system settings are changed.

Actual `craft-settings` owns loading, schema behavior, unknown-key preservation and atomic preference storage. Preference updates use the shared 500 ms quiet debounce and failure/backoff policy through actual `SettingsStore::tick` on a local worker. Pending remains visible until that generation is durable; failures remain visible while the worker retries. Shutdown attempts at most one final flush rather than an unbounded retry loop. Invalid or newer-schema/read-only configuration reports remain visible; read-only changes apply to this session without a durable-settings claim.

Model saves and recovery also use frozen worker jobs. Only the acknowledged current snapshot can release pending navigation or show current recovery. If editing advances while an older snapshot is written, the live work remains unsaved and the UI requests another preservation step. **Cancel** cancels navigation while keeping the current document; already executing snapshot bytes may still finish. A failed write leaves the document present and reports the error.

## Native close and frame policy

The preservation decision runs in `eframe::App::logic`, including while the native window is hidden or minimized. Dirty work or pending snapshot writes cancel close and show the preservation prompt. The permit for a subsequent close is bound to the exact acknowledged snapshot digest. The actual `craft-quit` hook runs last; it is not treated as an unsaved-work policy itself.

At startup, actual `craft-window::Choice::probe(Prefix::new("MBSE_CRAFT"))` selects decorations. X11 normally retains the desktop frame. The process-local `MBSE_CRAFT_TITLEBAR=app` override asks for an app frame; its supplied Layout controls button order. The app provides actual minimize, maximize/restore and safe-close actions, with header dragging and double-click maximize. No inactive imitation window buttons are drawn. The geometry receipt records the chosen policy and override. Wayland/compositor behavior requires its own qualification; an Xvfb run cannot establish it.

## External library browser boundary

**Browse library folder** uses the source/action path and opens the integrator-owned external catalog browser. This reads a bounded external index and presents metadata and unresolved dependencies. Opening the browser does not mutate the current native model or require a preservation prompt. A catalog entry is not an admitted native projection, a compatible executable model, or proof that its dependencies resolve. Broader external projection has a separate contract and acceptance gate.

## Current limits

Actor-role authoring, opaque external inputs, generic scenario execution, free diagram placement and broad CATIA/Capella taxonomy are not implemented by these context editors. Recovery remains local and bounded; no power-loss durability guarantee is added. Preference storage retains upstream last-writer behavior, not a new multi-process merge protocol. Native keyboard, minimized-close, restart, failure, stale-write and frame-action checks must be assessed from the independent verifier's exact candidate receipts.
