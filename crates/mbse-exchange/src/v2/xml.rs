use super::*;
use std::{borrow::Cow, collections::BTreeMap};
const XMI: &str = "http://www.omg.org/XMI";
const XMI_UML: &str = "http://www.omg.org/spec/XMI/20131001";
const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
const UML: &str = "http://www.omg.org/spec/UML/20131001";
const CTX: &str = "http://www.polarsys.org/capella/core/ctx/7.0.0";
const CORE: &str = "http://www.polarsys.org/capella/core/core/7.0.0";
const CS: &str = "http://www.polarsys.org/capella/core/cs/7.0.0";
const FA: &str = "http://www.polarsys.org/capella/core/fa/7.0.0";
const DV: &str = "http://www.polarsys.org/capella/core/information/datavalue/7.0.0";
const MODELLER: &str = "http://www.polarsys.org/capella/core/modeller/7.0.0";
const SYSML: &str = "http://www.omg.org/spec/SysML/20181001/SysML";
const UAF: &str = "https://www.omg.org/spec/UAF/20211201/UAF";
const UAF_MEASURE: &str = "http://www.omg.org/spec/UAF/20211201/MeasurementsLibrary";
#[derive(Serialize)]
struct TypeView<'a> {
    namespace: &'a str,
    local: &'a str,
    spelling: &'a str,
}
impl TypeView<'_> {
    fn owned(&self) -> QualifiedType {
        QualifiedType {
            namespace: self.namespace.into(),
            local: self.local.into(),
            spelling: self.spelling.into(),
        }
    }
}
#[derive(Serialize)]
struct AttributeView<'a> {
    namespace: &'a str,
    name: &'a str,
    value: &'a str,
}
#[derive(Serialize)]
struct ExpressionView<'a> {
    bodies: Vec<Cow<'a, str>>,
    languages: Vec<Cow<'a, str>>,
    evaluated: bool,
}
#[derive(Serialize)]
struct NodeView<'a> {
    key: &'a ElementKey,
    source_type: TypeView<'a>,
    kind: &'a SemanticKind,
    name: &'a str,
    attributes: Vec<AttributeView<'a>>,
    direct_text: Vec<&'a str>,
    component_flags: Option<&'a ComponentFlags>,
    opaque_expression: Option<ExpressionView<'a>>,
    value_kind: Option<&'a ValueKind>,
    location: &'a Locator,
}
impl NodeView<'_> {
    fn owned(self) -> SemanticNode {
        SemanticNode {
            key: self.key.clone(),
            source_type: self.source_type.owned(),
            kind: self.kind.clone(),
            name: self.name.into(),
            attributes: self
                .attributes
                .into_iter()
                .map(|a| Attribute {
                    namespace: a.namespace.into(),
                    name: a.name.into(),
                    value: a.value.into(),
                })
                .collect(),
            direct_text: self.direct_text.into_iter().map(str::to_owned).collect(),
            component_flags: self.component_flags.cloned(),
            opaque_expression: self.opaque_expression.map(|e| OpaqueExpression {
                bodies: e.bodies.into_iter().map(Cow::into_owned).collect(),
                languages: e.languages.into_iter().map(Cow::into_owned).collect(),
                evaluated: false,
            }),
            value_kind: self.value_kind.cloned(),
            location: self.location.clone(),
        }
    }
}
#[derive(Clone)]
enum Expected {
    Any,
    Exact(&'static str, &'static str),
    UmlClass,
    CapellaInvolved,
    UmlProperty,
    UmlPort,
    InformationExchanger,
    ActivityNode,
}
struct Pending {
    reference: SemanticReference,
    requested: Option<ElementKey>,
    expected: Expected,
}
struct Part {
    version: VersionClaim,
    nodes: Vec<SemanticNode>,
    pending: Vec<Pending>,
    omitted_notices: usize,
    notices: Vec<(&'static str, String, Option<Locator>)>,
}
fn capella(kind: &TypeView<'_>) -> bool {
    matches!(
        kind.namespace,
        CTX | CORE
            | CS
            | FA
            | DV
            | MODELLER
            | "http://www.polarsys.org/capella/core/la/7.0.0"
            | "http://www.polarsys.org/capella/core/pa/7.0.0"
    )
}
fn source_type<'a, 'i>(n: roxmltree::Node<'a, 'i>) -> TypeView<'a> {
    if let Some(a) = n.attributes().find(|a| {
        a.name() == "type" && matches!(a.namespace(), Some(XMI) | Some(XMI_UML) | Some(XSI))
    }) {
        let spelling = a.value();
        if let Some((prefix, local)) = spelling.split_once(':') {
            TypeView {
                namespace: n.lookup_namespace_uri(Some(prefix)).unwrap_or(""),
                local,
                spelling,
            }
        } else {
            TypeView {
                namespace: n.lookup_namespace_uri(None).unwrap_or(""),
                local: spelling,
                spelling,
            }
        }
    } else {
        TypeView {
            namespace: n.tag_name().namespace().unwrap_or(""),
            local: n.tag_name().name(),
            spelling: n.tag_name().name(),
        }
    }
}
fn kind(t: &TypeView<'_>, base: bool) -> SemanticKind {
    use SemanticKind::*;
    match (t.namespace, t.local) {
        (CTX, "Mission") => CapellaMission,
        (CTX, "Capability") => CapellaCapability,
        (CTX, "CapabilityExploitation") => CapellaExploitation,
        (CTX, "MissionInvolvement" | "CapabilityInvolvement") => CapellaInvolvement,
        (CTX, "SystemComponent")
        | ("http://www.polarsys.org/capella/core/la/7.0.0", "LogicalComponent")
        | ("http://www.polarsys.org/capella/core/pa/7.0.0", "PhysicalComponent") => {
            CapellaComponent
        }
        (CORE, "Constraint") => CapellaConstraint,
        (DV, "OpaqueExpression") => CapellaOpaqueExpression,
        (FA, "ComponentPort" | "FunctionInputPort" | "FunctionOutputPort") => CapellaPort,
        (FA, "ComponentExchange" | "FunctionalExchange") => CapellaExchange,
        (FA, "ComponentFunctionalAllocation" | "ComponentExchangeFunctionalExchangeAllocation") => {
            CapellaAllocation
        }
        (CORE, "PropertyValueGroup") => CapellaPropertyGroup,
        (
            CORE,
            "StringPropertyValue"
            | "IntegerPropertyValue"
            | "FloatPropertyValue"
            | "BooleanPropertyValue"
            | "EnumerationPropertyValue",
        ) => CapellaTypedValue,
        (CORE, "EnumerationPropertyType") => CapellaEnumerationType,
        (CORE, "EnumerationPropertyLiteral") => CapellaEnumerationLiteral,
        (CTX, "SystemFunction")
        | ("http://www.polarsys.org/capella/core/la/7.0.0", "LogicalFunction")
        | ("http://www.polarsys.org/capella/core/pa/7.0.0", "PhysicalFunction") => CapellaFunction,
        (MODELLER, "Project" | "SystemEngineering")
        | (
            CTX,
            "SystemAnalysis" | "MissionPkg" | "CapabilityPkg" | "SystemComponentPkg"
            | "SystemFunctionPkg",
        ) => MetamodelHelper,
        (UML, "Model" | "Package") => UmlPackage,
        (UML, "Class" | "Component" | "Interface" | "DataType" | "Enumeration") => UmlClassifier,
        (UML, "Actor") => UmlActor,
        (UML, "Property" | "Parameter") => UmlProperty,
        (UML, "Port") => UmlPort,
        (UML, "Constraint" | "OpaqueExpression") => UmlConstraint,
        (UML, "Activity" | "Action" | "CallBehaviorAction") => UmlActivity,
        (UML, "Association" | "Connector" | "InformationFlow") => UmlExchange,
        (UML, "Message") => UmlMessage,
        (SYSML, _) if base => SysmlProfileApplication,
        (UAF | UAF_MEASURE, _) if base => UafProfileApplication,
        (_, _) if base => UnknownProfileApplication,
        _ => UnknownClassifier,
    }
}
fn flag(n: roxmltree::Node<'_, '_>, name: &str) -> FlagValue {
    match n.attribute(name) {
        None => FlagValue::MetamodelDefault(false),
        Some("true") => FlagValue::Explicit(true),
        Some("false") => FlagValue::Explicit(false),
        Some(raw) => FlagValue::UnsupportedLexical(raw.into()),
    }
}
fn id<'a, 'i>(n: roxmltree::Node<'a, 'i>) -> Option<&'a str> {
    n.attributes()
        .find(|a| {
            a.name() == "id"
                && (a.namespace().is_none() || matches!(a.namespace(), Some(XMI) | Some(XMI_UML)))
        })
        .map(|a| a.value())
}
fn input_preflight(
    text: &str,
    doc: &roxmltree::Document<'_>,
    c: &mut Counters,
) -> Result<(), String> {
    if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
        return Err("V2_XML_DTD_ENTITY_REFUSED".into());
    }
    for n in doc.descendants().filter(|n| n.is_element()) {
        if n.ancestors().count() > MAX_DEPTH {
            return Err("V2_DEPTH_LIMIT".into());
        }
        if n.tag_name().name().len() > MAX_VALUE_BYTES {
            return Err("V2_XML_NAME_VALUE_LIMIT".into());
        }
        // roxmltree excludes xmlns declarations from attributes(). Bound every
        // effective scope, counting new/shadowed bindings once rather than
        // repeatedly charging inherited bindings against aggregate bytes.
        if n.namespaces().len() > MAX_ATTRIBUTES_PER_NODE {
            return Err("V2_NAMESPACE_COUNT_LIMIT".into());
        }
        for ns in n.namespaces() {
            if ns.uri().len() > MAX_VALUE_BYTES
                || ns.name().is_some_and(|v| v.len() > MAX_VALUE_BYTES)
            {
                return Err("V2_NAMESPACE_VALUE_LIMIT".into());
            }
            let inherited = n
                .parent_element()
                .and_then(|p| p.lookup_namespace_uri(ns.name()));
            if inherited != Some(ns.uri()) {
                c.attribute_bytes = c
                    .attribute_bytes
                    .checked_add(ns.uri().len() + ns.name().map_or(0, str::len))
                    .filter(|v| *v <= MAX_ATTRIBUTE_BYTES)
                    .ok_or("V2_ATTRIBUTE_BUDGET")?;
            }
        }
        if n.attributes().len() > MAX_ATTRIBUTES_PER_NODE {
            return Err("V2_ATTRIBUTES_PER_NODE_LIMIT".into());
        }
        for a in n.attributes() {
            if a.value().len() > MAX_VALUE_BYTES
                || a.name().len() > MAX_VALUE_BYTES
                || a.namespace().is_some_and(|s| s.len() > MAX_VALUE_BYTES)
            {
                return Err("V2_ATTRIBUTE_VALUE_LIMIT".into());
            }
            c.attributes = c.attributes.checked_add(1).ok_or("V2_ATTRIBUTE_BUDGET")?;
            c.attribute_bytes = c
                .attribute_bytes
                .checked_add(a.value().len() + a.name().len() + a.namespace().map_or(0, str::len))
                .filter(|v| *v <= MAX_ATTRIBUTE_BYTES)
                .ok_or("V2_ATTRIBUTE_BUDGET")?;
        }
        let bytes = n
            .children()
            .filter_map(|n| n.text().filter(|_| n.is_text()))
            .try_fold(0usize, |sum, t| sum.checked_add(t.len()))
            .ok_or("V2_TEXT_BUDGET")?;
        if bytes > MAX_TEXT_PER_NODE {
            return Err("V2_TEXT_PER_NODE_LIMIT".into());
        }
        c.direct_text_bytes = c
            .direct_text_bytes
            .checked_add(bytes)
            .filter(|v| *v <= MAX_TEXT_BYTES)
            .ok_or("V2_TEXT_BUDGET")?;
    }
    Ok(())
}
fn leaf_text<'a>(n: roxmltree::Node<'a, '_>) -> Result<Cow<'a, str>, String> {
    if n.children().any(|n| n.is_element()) {
        return Err("V2_OPAQUE_BODY_COMPLEX_XML_UNSUPPORTED".into());
    }
    let texts: Vec<_> = n
        .children()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect();
    match texts.as_slice() {
        [] => Ok(Cow::Borrowed("")),
        [one] => Ok(Cow::Borrowed(one)),
        _ => Ok(Cow::Owned(texts.concat())),
    }
}
fn parse(
    asset: &SourceAsset,
    counters: &mut Counters,
    budget: &mut Budget,
) -> Result<Part, String> {
    let text = std::str::from_utf8(&asset.bytes).map_err(|_| "V2_UTF8_ONLY")?;
    if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
        return Err("V2_XML_DTD_ENTITY_REFUSED".into());
    }
    let clean = text.trim_start_matches('\u{feff}');
    if let Some(header) = clean
        .strip_prefix("<?xml")
        .and_then(|h| h.split_once("?>").map(|v| v.0))
    {
        if let Some(value) = header.split_once("encoding").map(|v| v.1.trim_start()) {
            let value = value
                .strip_prefix('=')
                .map(str::trim_start)
                .ok_or("V2_XML_ENCODING")?;
            let quote = value.chars().next().ok_or("V2_XML_ENCODING")?;
            let encoding = value
                .get(1..)
                .and_then(|s| s.split(quote).next())
                .ok_or("V2_XML_ENCODING")?;
            if !encoding.eq_ignore_ascii_case("UTF-8") {
                return Err("V2_UTF8_ONLY".into());
            }
        }
    }
    let doc = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: (MAX_NODES * 4) as u32,
        },
    )
    .map_err(|e| format!("V2_XML_PARSE: {e}"))?;
    input_preflight(text, &doc, counters)?;
    let root = doc.root_element();
    let uris: Vec<_> = root.namespaces().map(|n| n.uri().to_owned()).collect();
    let xmi_version = root
        .attributes()
        .find(|a| a.name() == "version" && matches!(a.namespace(), Some(XMI) | Some(XMI_UML)))
        .map(|a| a.value().to_owned());
    let app = doc
        .descendants()
        .filter(|n| n.is_comment())
        .filter_map(|n| n.text())
        .find_map(|t| {
            t.trim()
                .strip_prefix("Capella_Version_")
                .map(|s| s.chars().take(64).collect::<String>())
        });
    let recognized = if root.tag_name().namespace() == Some(MODELLER) {
        root.tag_name().name() == "Project"
            && xmi_version.as_deref() == Some("2.0")
            && app
                .as_deref()
                .map_or(true, |v| matches!(v, "7.0.0" | "7.1.0"))
    } else {
        uris.iter().any(|u| u == UML)
    };
    let version = VersionClaim {
        member_path: asset.logical_path.clone(),
        root_namespace: root.tag_name().namespace().unwrap_or("").into(),
        metamodel_namespaces: uris,
        xmi_version,
        declared_application_version: app,
        recognized_contract: recognized,
    };
    budget.admit(&version)?;
    let mut part = Part {
        version,
        nodes: vec![],
        pending: vec![],
        notices: vec![],
        omitted_notices: 0,
    };
    let mut keys = BTreeMap::new();
    let mut original_ids = BTreeSet::new();
    for n in doc.descendants().filter(|n| n.is_element()) {
        let t = source_type(n);
        let original = id(n);
        let typed = n.attributes().any(|a| {
            a.name() == "type" && matches!(a.namespace(), Some(XSI) | Some(XMI) | Some(XMI_UML))
        });
        let base = n.attributes().any(|a| {
            matches!(
                a.name(),
                "base_Class"
                    | "base_Property"
                    | "base_Port"
                    | "base_Activity"
                    | "base_Constraint"
                    | "base_Actor"
                    | "base_InformationFlow"
                    | "base_Connector"
                    | "base_Package"
            )
        });
        if original.is_none() && !typed && !base && n != root {
            continue;
        }
        if part.nodes.len() + counters.nodes >= MAX_NODES {
            return Err("V2_NODE_LIMIT".into());
        }
        if let Some(id) = original {
            if id.is_empty() || !original_ids.insert(id) {
                return Err("V2_DUPLICATE_OR_EMPTY_MEMBER_ID".into());
            }
        }
        let key = ElementKey {
            asset_sha256: asset.sha256.clone(),
            member_path: asset.logical_path.clone(),
            identity: original
                .map(|s| ElementIdentity::Original(s.into()))
                .unwrap_or(ElementIdentity::XmlSpan {
                    start: n.range().start,
                    end: n.range().end,
                }),
        };
        let semantic = kind(&t, base);
        let flags = (semantic == SemanticKind::CapellaComponent).then(|| ComponentFlags {
            actor: flag(n, "actor"),
            human: flag(n, "human"),
        });
        let expression = if semantic == SemanticKind::CapellaOpaqueExpression {
            let mut bodies = vec![];
            let mut languages = vec![];
            for child in n.children().filter(|n| n.is_element()) {
                match (child.tag_name().namespace(), child.tag_name().name()) {
                    (None, "bodies") => bodies.push(leaf_text(child)?),
                    (None, "languages") => languages.push(leaf_text(child)?),
                    _ => (),
                }
            }
            if bodies
                .iter()
                .chain(languages.iter())
                .map(|v| v.len())
                .sum::<usize>()
                > MAX_TEXT_PER_NODE
            {
                return Err("V2_EXPRESSION_TEXT_PER_NODE_LIMIT".into());
            }
            if bodies.len() != languages.len() && part.notices.len() >= MAX_DIAGNOSTICS - 1 {
                part.omitted_notices = part.omitted_notices.saturating_add(1);
            } else if bodies.len() != languages.len() {
                part.notices.push((
                    "OPAQUE_BODY_LANGUAGE_CARDINALITY",
                    "Ordered bodies/languages differ; no pairing, evaluation or truth is inferred"
                        .into(),
                    Some(Locator {
                        member_path: asset.logical_path.clone(),
                        start: n.range().start,
                        end: n.range().end,
                    }),
                ));
            }
            Some(ExpressionView {
                bodies,
                languages,
                evaluated: false,
            })
        } else {
            None
        };
        let value_kind = if semantic == SemanticKind::CapellaTypedValue {
            match t.local {
                "IntegerPropertyValue" => Some(ValueKind::Integer),
                "FloatPropertyValue" => Some(ValueKind::Float),
                "BooleanPropertyValue" => Some(ValueKind::Boolean),
                "EnumerationPropertyValue" => Some(ValueKind::Enumeration),
                _ => Some(ValueKind::String),
            }
        } else {
            None
        };
        let location = Locator {
            member_path: asset.logical_path.clone(),
            start: n.range().start,
            end: n.range().end,
        };
        let view = NodeView {
            key: &key,
            source_type: t,
            kind: &semantic,
            name: n.attribute("name").unwrap_or(""),
            attributes: n
                .attributes()
                .map(|a| AttributeView {
                    namespace: a.namespace().unwrap_or(""),
                    name: a.name(),
                    value: a.value(),
                })
                .collect(),
            direct_text: n
                .children()
                .filter(|n| n.is_text())
                .filter_map(|n| n.text())
                .collect(),
            component_flags: flags.as_ref(),
            opaque_expression: expression,
            value_kind: value_kind.as_ref(),
            location: &location,
        };
        budget.admit(&view)?;
        let node = view.owned();
        keys.insert(
            n.id().get(),
            (
                node.key.clone(),
                node.source_type.clone(),
                node.kind.clone(),
            ),
        );
        part.nodes.push(node);
    }
    for n in doc.descendants().filter(|n| n.is_element()) {
        let Some((source, ty, semantic)) = keys.get(&n.id().get()) else {
            continue;
        };
        let location = Locator {
            member_path: asset.logical_path.clone(),
            start: n.range().start,
            end: n.range().end,
        };
        if let Some((parent, _, _)) = n.ancestors().skip(1).find_map(|n| keys.get(&n.id().get())) {
            pending(
                &mut part.pending,
                source,
                Some(parent.clone()),
                "",
                "xml.container",
                ty,
                ReferenceProvenance::SourceContainment,
                Expected::Any,
                &location,
                budget,
            )?;
        }
        if n.tag_name().namespace().is_none() && n.tag_name().name() == "ownedSpecification" {
            if let Some((parent_key, _, parent_kind)) =
                n.parent_element().and_then(|p| keys.get(&p.id().get()))
            {
                if *parent_kind == SemanticKind::CapellaConstraint {
                    pending(
                        &mut part.pending,
                        parent_key,
                        Some(source.clone()),
                        "",
                        "ownedSpecification",
                        &declaring(
                            "http://www.polarsys.org/capella/common/core/7.0.0",
                            "AbstractConstraint",
                        ),
                        ReferenceProvenance::SourceContainment,
                        Expected::Exact(DV, "OpaqueExpression"),
                        &location,
                        budget,
                    )?;
                }
            }
        }
        for a in n.attributes().filter(|a| a.namespace().is_none()) {
            if let Some((declaring, expected)) = role(ty, semantic, a.name()) {
                for raw in a.value().split_whitespace() {
                    let requested = if let Some(id) = raw.strip_prefix('#') {
                        Some(ElementKey {
                            asset_sha256: asset.sha256.clone(),
                            member_path: asset.logical_path.clone(),
                            identity: ElementIdentity::Original(id.into()),
                        })
                    } else if raw.contains(['#', '/', ':']) {
                        None
                    } else {
                        Some(ElementKey {
                            asset_sha256: asset.sha256.clone(),
                            member_path: asset.logical_path.clone(),
                            identity: ElementIdentity::Original(raw.into()),
                        })
                    };
                    pending(
                        &mut part.pending,
                        source,
                        requested,
                        raw,
                        a.name(),
                        &declaring,
                        ReferenceProvenance::StoredAttribute,
                        expected.clone(),
                        &location,
                        budget,
                    )?;
                }
            }
        }
        if matches!(
            semantic,
            SemanticKind::CapellaExploitation | SemanticKind::CapellaInvolvement
        ) {
            let expected = if *semantic == SemanticKind::CapellaExploitation
                || ty.local == "MissionInvolvement"
            {
                Expected::Exact(CTX, "Mission")
            } else {
                Expected::Exact(CTX, "Capability")
            };
            let parent = n
                .parent_element()
                .and_then(|p| keys.get(&p.id().get()))
                .map(|v| v.0.clone());
            pending(
                &mut part.pending,
                source,
                parent,
                "",
                "container-derived.involver",
                ty,
                ReferenceProvenance::ContainmentDerived,
                expected,
                &location,
                budget,
            )?;
        }
        for child in n.children().filter(|n| n.is_element()) {
            for a in child
                .attributes()
                .filter(|a| matches!(a.name(), "href" | "idref"))
            {
                let raw = a.value();
                let local_id = raw
                    .strip_prefix('#')
                    .or_else(|| (!raw.contains(['#', '/', ':'])).then_some(raw));
                let requested = local_id.map(|id| ElementKey {
                    asset_sha256: asset.sha256.clone(),
                    member_path: asset.logical_path.clone(),
                    identity: ElementIdentity::Original(id.into()),
                });
                let feature = if let Some(ns) = child.tag_name().namespace() {
                    format!("{{{ns}}}{}", child.tag_name().name())
                } else {
                    child.tag_name().name().to_owned()
                };
                pending(
                    &mut part.pending,
                    source,
                    requested,
                    raw,
                    &feature,
                    ty,
                    ReferenceProvenance::StoredChildReference,
                    Expected::Any,
                    &location,
                    budget,
                )?;
            }
        }
    }
    Ok(part)
}
fn declaring(namespace: &str, local: &str) -> QualifiedType {
    QualifiedType {
        namespace: namespace.into(),
        local: local.into(),
        spelling: local.into(),
    }
}
fn role(
    ty: &QualifiedType,
    kind: &SemanticKind,
    feature: &str,
) -> Option<(QualifiedType, Expected)> {
    use SemanticKind::*;
    match (kind, feature) {
        (CapellaExploitation, "capability") => Some((
            declaring(CTX, "CapabilityExploitation"),
            Expected::Exact(CTX, "Capability"),
        )),
        (CapellaInvolvement, "involved") => {
            Some((declaring(CORE, "Involvement"), Expected::CapellaInvolved))
        }
        (CapellaConstraint, "constrainedElements") => Some((
            declaring(
                "http://www.polarsys.org/capella/common/core/7.0.0",
                "AbstractConstraint",
            ),
            Expected::Any,
        )),
        (CapellaExchange, "source" | "target") => Some((
            declaring(
                if ty.local == "FunctionalExchange" {
                    "http://www.polarsys.org/capella/common/activity/7.0.0"
                } else {
                    "http://www.polarsys.org/capella/common/core/7.0.0"
                },
                if ty.local == "FunctionalExchange" {
                    "ActivityEdge"
                } else {
                    "AbstractInformationFlow"
                },
            ),
            if ty.local == "FunctionalExchange" {
                Expected::ActivityNode
            } else {
                Expected::InformationExchanger
            },
        )),
        (CapellaAllocation, "sourceElement" | "targetElement") => Some((
            declaring(
                "http://www.polarsys.org/capella/common/core/7.0.0",
                "AbstractTrace",
            ),
            Expected::Any,
        )),
        (CapellaTypedValue, "type") if ty.local == "EnumerationPropertyValue" => Some((
            declaring(CORE, "EnumerationPropertyValue"),
            Expected::Exact(CORE, "EnumerationPropertyType"),
        )),
        (CapellaTypedValue, "value") if ty.local == "EnumerationPropertyValue" => Some((
            declaring(CORE, "EnumerationPropertyValue"),
            Expected::Exact(CORE, "EnumerationPropertyLiteral"),
        )),
        (
            SysmlProfileApplication | UafProfileApplication | UnknownProfileApplication,
            "base_Class",
        ) => Some((ty.clone(), Expected::UmlClass)),
        (
            SysmlProfileApplication | UafProfileApplication | UnknownProfileApplication,
            "base_Property",
        ) => Some((ty.clone(), Expected::UmlProperty)),
        (
            SysmlProfileApplication | UafProfileApplication | UnknownProfileApplication,
            "base_Port",
        ) => Some((ty.clone(), Expected::UmlPort)),
        (SysmlProfileApplication | UafProfileApplication | UnknownProfileApplication, f)
            if f.starts_with("base_") =>
        {
            Some((ty.clone(), Expected::Any))
        }
        (UmlProperty | UmlPort, "type") | (UmlExchange, "source" | "target" | "memberEnd") => {
            Some((ty.clone(), Expected::Any))
        }
        _ if ty.namespace == CORE
            && matches!(
                feature,
                "appliedPropertyValues"
                    | "appliedPropertyValueGroups"
                    | "involvedElements"
                    | "status"
                    | "features"
            ) =>
        {
            Some((declaring(CORE, "CapellaElement"), Expected::Any))
        }
        _ if capella(&TypeView {
            namespace: &ty.namespace,
            local: &ty.local,
            spelling: &ty.spelling,
        }) && matches!(
            feature,
            "appliedPropertyValues" | "appliedPropertyValueGroups"
        ) =>
        {
            Some((declaring(CORE, "CapellaElement"), Expected::Any))
        }
        _ => None,
    }
}
#[allow(clippy::too_many_arguments)]
fn pending(
    out: &mut Vec<Pending>,
    source: &ElementKey,
    requested: Option<ElementKey>,
    raw: &str,
    feature: &str,
    declaring: &QualifiedType,
    provenance: ReferenceProvenance,
    expected: Expected,
    location: &Locator,
    budget: &mut Budget,
) -> Result<(), String> {
    if out.len() >= MAX_REFERENCES {
        return Err("V2_REFERENCE_LIMIT".into());
    }
    #[derive(Serialize)]
    struct View<'a> {
        source: &'a ElementKey,
        target: Option<&'a ElementKey>,
        raw: &'a str,
        feature: &'a str,
        declaring_type: &'a QualifiedType,
        provenance: &'a ReferenceProvenance,
        resolution: ReferenceResolution,
        location: &'a Locator,
    }
    let view = View {
        source,
        target: requested.as_ref(),
        raw,
        feature,
        declaring_type: declaring,
        provenance: &provenance,
        resolution: ReferenceResolution::UnsupportedEndpointType,
        location,
    };
    budget.admit(&view)?;
    out.push(Pending {
        reference: SemanticReference {
            source: source.clone(),
            target: None,
            raw: raw.into(),
            feature: feature.into(),
            declaring_type: declaring.clone(),
            provenance,
            resolution: ReferenceResolution::Unresolved,
            location: location.clone(),
        },
        requested,
        expected,
    });
    Ok(())
}
fn compatible(expected: &Expected, node: &SemanticNode) -> ReferenceResolution {
    let valid = match expected {
        Expected::Any => true,
        Expected::Exact(ns, local) => {
            node.source_type.namespace == *ns && node.source_type.local == *local
        }
        Expected::UmlClass => {
            node.source_type.namespace == UML && node.source_type.local == "Class"
        }
        Expected::CapellaInvolved => matches!(
            (
                node.source_type.namespace.as_str(),
                node.source_type.local.as_str()
            ),
            (CTX, "SystemComponent" | "SystemFunction")
                | (FA, "AbstractFunction" | "FunctionalExchange")
        ),
        Expected::UmlProperty => {
            node.source_type.namespace == UML && node.source_type.local == "Property"
        }
        Expected::UmlPort => node.source_type.namespace == UML && node.source_type.local == "Port",
        Expected::InformationExchanger => {
            node.source_type.namespace == FA && node.source_type.local == "ComponentPort"
        }
        Expected::ActivityNode => {
            node.source_type.namespace == FA
                && matches!(
                    node.source_type.local.as_str(),
                    "FunctionInputPort" | "FunctionOutputPort"
                )
        }
    };
    if valid {
        ReferenceResolution::Resolved
    } else {
        // A classifier outside this bounded subtype table is a coverage gap.
        // Report wrong-kind only for explicit disjoint cases supported by the audit.
        let wrong_kind = match expected {
            Expected::UmlClass => {
                node.source_type.namespace == UML
                    && matches!(
                        node.source_type.local.as_str(),
                        "Property" | "Port" | "Package" | "Model" | "Interface"
                    )
            }
            Expected::UmlProperty | Expected::UmlPort => {
                node.source_type.namespace == UML
                    && matches!(
                        node.source_type.local.as_str(),
                        "Class" | "Package" | "Model" | "Interface"
                    )
            }
            Expected::CapellaInvolved => {
                node.source_type.namespace == CTX
                    && matches!(node.source_type.local.as_str(), "Mission" | "Capability")
            }
            Expected::Exact(ns, local) => {
                *ns == CTX
                    && matches!(*local, "Mission" | "Capability")
                    && node.source_type.namespace == CTX
                    && matches!(
                        node.source_type.local.as_str(),
                        "SystemComponent" | "Mission" | "Capability"
                    )
                    && node.source_type.local != *local
            }
            _ => false,
        };
        if wrong_kind {
            ReferenceResolution::WrongKind
        } else {
            ReferenceResolution::UnsupportedEndpointType
        }
    }
}
pub(super) fn project(
    report: &mut ProjectionSet,
    semantic: &[usize],
    budget: &mut Budget,
) -> Result<(), String> {
    let mut pending_refs = vec![];
    for index in semantic {
        let part = parse(&report.assets[*index], &mut report.counters, budget)?;
        if part.omitted_notices > 0 {
            report.counters.omitted_diagnostics = report
                .counters
                .omitted_diagnostics
                .saturating_add(part.omitted_notices);
            report.diagnostic(
                "DIAGNOSTICS_TRUNCATED",
                "Additional per-expression notices omitted; see counters.omitted_diagnostics",
                None,
            );
        }
        for (code, detail, location) in part.notices {
            report.diagnostic(code, &detail, location);
        }
        if !part.version.recognized_contract {
            report.diagnostic("SOURCE_VERSION_OR_NAMESPACE_UNQUALIFIED","Source namespace/version does not match the selected qualified family; exact types remain separately visible",None);
        }
        report.source_versions.push(part.version);
        report.counters.nodes += part.nodes.len();
        report.nodes.extend(part.nodes);
        pending_refs.extend(part.pending);
        if pending_refs.len() > MAX_REFERENCES {
            return Err("V2_REFERENCE_LIMIT".into());
        }
    }
    let index: BTreeMap<_, _> = report
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.key.clone(), i))
        .collect();
    for mut item in pending_refs {
        let resolution = if let Some(key) = &item.requested {
            if let Some(index) = index.get(key) {
                compatible(&item.expected, &report.nodes[*index])
            } else {
                ReferenceResolution::Unresolved
            }
        } else if item.reference.provenance == ReferenceProvenance::ContainmentDerived {
            ReferenceResolution::InvalidContainer
        } else {
            ReferenceResolution::UnsuppliedResource
        };
        if resolution == ReferenceResolution::Resolved {
            item.reference.target = item.requested;
        } else {
            report.loss(match resolution{ReferenceResolution::UnsupportedEndpointType=>"ENDPOINT_TYPE_COVERAGE_UNSUPPORTED",ReferenceResolution::InvalidContainer=>"SOURCE_CONTAINER_TYPE_INVALID",ReferenceResolution::WrongKind=>"SOURCE_ENDPOINT_KIND_INVALID",ReferenceResolution::UnsuppliedResource=>"UNSUPPLIED_RESOURCE_REFERENCE",_=>"ENGINEERING_REFERENCE_UNRESOLVED"},Some(&item.reference.source),&format!("Role {} is {:?}; raw spelling remains in reference and source bytes",item.reference.feature,resolution),"Inspect the exact member-scoped endpoint/type; supply resources explicitly or reduce selected scope");
        }
        item.reference.resolution = resolution;
        report.references.push(item.reference);
    }
    for i in 0..report.nodes.len() {
        let node = &report.nodes[i];
        let note = match node.kind {
            SemanticKind::UnknownClassifier => Some(("UNSUPPORTED_METAMODEL_CASE", "Qualified type is not implemented; this is an adapter coverage gap, not automatically an unknown engineering decision")),
            SemanticKind::UnknownProfileApplication => Some(("UNKNOWN_PROFILE_LAYER_RETAINED", "Original profile application and base links remain distinct; no meaning is flattened into a UML or native type")),
            SemanticKind::CapellaOpaqueExpression if node.opaque_expression.as_ref().is_some_and(|e| e.bodies.is_empty() || e.bodies.iter().any(String::is_empty)) => Some(("ENGINEERING_EXPRESSION_EMPTY", "Empty source expression is retained unevaluated; no true/satisfied result is inferred")),
            _ => None,
        };
        if let Some((code, detail)) = note {
            // Move the key out briefly, avoiding an unbounded all-node key clone.
            let key = std::mem::replace(
                &mut report.nodes[i].key,
                ElementKey {
                    asset_sha256: String::new(),
                    member_path: String::new(),
                    identity: ElementIdentity::XmlSpan { start: 0, end: 0 },
                },
            );
            report.loss(
                code,
                Some(&key),
                detail,
                "Inspect the source slice; whole native admission remains refused",
            );
            report.nodes[i].key = key;
        }
    }
    report.diagnostic("SOURCE_SEMANTICS_UNEVALUATED","Type/role interpretation is partial. Ecore defaults are labelled separately from persisted attributes; creation-service initialization, semantic validity, behavior and engineering acceptance are not evaluated",None);
    Ok(())
}
