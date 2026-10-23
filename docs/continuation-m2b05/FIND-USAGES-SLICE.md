# Native Find usages

Status: implementation in progress, UNRUN. Isolatedfeature/native-find-usages from clean qualifiedmapping7308d2b. Existingintegrationowner01a1653d-9c16-76a3-ad80-df98ba1e5c94 is solewriter; completedrecoverya007/preview3584430/mapping7308d2b stayfrozen. HostSamsungworkspace freshlyverifiedrw. No compiler/renderer launched for this slice.

M2B CAT004/CAP010 acceptance: select existingnative record, explicitlyFindusages from the existing authoredstage1_graph, inspect exactreferencingnativeID/kind/role and calculationbindingnames, then navigate to an existingreferencingrecord. NativeScope graphparent→child normalizes to childreferencingparent; CalculationInput producer→consumer normalizes to consumerusesproducer. ContributesTo/Subject/ForObjective preserveowner→referencedtarget. No generic arrow-direction assumption or name/number matching.

UsageReport is nonClone/nonDeserialize and pinsfull targetViewsStamp for private runtimeidentity and nativefreshness/hash. Layoutchanges conservativelystale the report but geometry neverderivessemantics. Every navigationbyreportindex rechecks the stamp. Only acceptednative relations represented by the5existinggraphroles are reported; opaque/unknown fields, drafts and unresolvedcalculationreferences are not interpreted as usages. No newsemanticgraph, importauthority, model/history mutation, migration or savewire. Target selection is a separate ephemeral operation, not an edit.

Fourmodeltests authored UNRUN coverroledirection, calculationconsumer names, stale runtime/raw/layout and selectionwithoutmutation. Independentactualsourcereview, focusedtests/build and actualnativeinteraction remain required; publication is not authorized by this file.
