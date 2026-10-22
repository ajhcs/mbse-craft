# Pinned Capella inherited name attribute audit

Read-only primary-source follow-up for the separately owned narrow source-edit route. Repository `eclipse-capella/capella`, tag v7.1.0, commit `3153b930a4284f6c82952658f1e8d8d6d8c8f340`; exact Ecore namespaces remain 7.0.0. Facts below are extracted metadata; no Java/Ecore files or private model assets copied. EPL-2.0 license/notice are already recorded in PRIMARY_METAMODEL_AUDIT.json. This confirms inheritance only, not source instance validity, application initializer policy or engineering acceptance.

| Exact classifier | Audited inheritance path to name |
|---|---|
| ctx7 Mission | ContextArchitecture.Mission → CapellaCore.NamedElement → ModellingCore.AbstractNamedElement |
| ctx7 Capability | ContextArchitecture.Capability → Interaction.AbstractCapability → CapellaCore.Structure → Namespace → NamedElement → ModellingCore.AbstractNamedElement |
| ctx7 SystemComponent | ContextArchitecture.SystemComponent → CompositeStructure.Component → Block → CapellaCore.ModellingBlock → Type → Namespace → NamedElement → ModellingCore.AbstractNamedElement |

A second SystemComponent path is Component → CapellaCore.Classifier → GeneralizableElement → Type → Namespace → NamedElement. The existing ContextArchitecture declarations are in PRIMARY_METAMODEL_AUDIT.json. No class-specific name override appears along these paths. `ModellingCore.AbstractNamedElement` owns `name`, an EAttribute of EString. Its declaration has no additional serialized default, lower-bound or derived flag; Ecore omission semantics must remain distinct from app initializer behavior.

Primary paths under the pinned repository:

| Path | SHA256 | Bytes |
|---|---|---:|
| core/plugins/org.polarsys.capella.core.data.gen/model/CapellaCore.ecore | d00364ca5a4b8c843cbe68c49d466ed5b8db176b3fc39effa7a1d89e7e292451 | 128560 |
| core/plugins/org.polarsys.capella.core.data.gen/model/Interaction.ecore | ef20e5cba1429ee3f00e9e285cc45114f2e47020bd98f166098e5389030c4bd2 | 191263 |
| core/plugins/org.polarsys.capella.core.data.gen/model/CompositeStructure.ecore | 881e329b226682bb2f9704f8c472e2cabc80955b4abbb8a56d6ca91b07c0d7f2 | 183111 |
| common/plugins/org.polarsys.capella.common.data.core.gen/model/ModellingCore.ecore | fd547944b0d6096c378c62e66e887beccfc0be7024d0c15203d3f1782ca61c20 | 71300 |

Each primary URL is `https://raw.githubusercontent.com/eclipse-capella/capella/3153b930a4284f6c82952658f1e8d8d6d8c8f340/` followed by the exact path above. This supports the initial exact class table for editing an existing unqualified `name` attribute on an owned direct-XML element under an explicitly versioned edit contract. It does not authorize proxy editing, absent-attribute insertion, generic XML/DOM serialization, archive rewriting or wider metamodel intake. Those operations require separate preservation rules and tests.
