//! Original bounded external projections. ExternalDocument is never a native model.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
    path::{Path, PathBuf},
};
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_EXPANDED: u64 = 64 * 1024 * 1024;
pub const MAX_MEMBERS: usize = 256;
pub const MAX_NODES: usize = 100_000;
pub const MAX_REFERENCES: usize = 200_000;
pub const MAX_DEPTH: usize = 96;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExternalFormat {
    UmlXmi,
    Mdzip,
    Capella,
    SysmlText,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeKind {
    Package,
    Classifier,
    Actor,
    Property,
    Port,
    Constraint,
    Activity,
    Message,
    Exchange,
    Requirement,
    Stereotype,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceFile {
    pub name: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalNode {
    pub source_id: String,
    pub kind: NodeKind,
    pub original_type: String,
    pub namespace: String,
    pub name: String,
    pub properties: BTreeMap<String, String>,
    pub location: SourceLocation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalReference {
    pub source: String,
    pub target: String,
    pub role: String,
    pub resolved: bool,
    pub location: SourceLocation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub file: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalDocument {
    pub contract: String,
    pub format: ExternalFormat,
    pub source_path: PathBuf,
    pub sha256: String,
    pub original_bytes: Vec<u8>,
    pub files: Vec<SourceFile>,
    pub nodes: Vec<ExternalNode>,
    pub references: Vec<ExternalReference>,
    pub diagnostics: Vec<Diagnostic>,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
impl ExternalDocument {
    fn diagnostic(&mut self, code: &str, message: impl Into<String>, file: Option<&str>) {
        self.diagnostics.push(Diagnostic {
            code: code.into(),
            message: message.into(),
            file: file.map(str::to_owned),
        });
    }
}
pub fn open_external(path: &Path) -> Result<ExternalDocument, String> {
    let bytes = mbse_model::local_input::read(path, MAX_BYTES, "EXTERNAL")?;
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let format = match extension.as_str() {
        "mdzip" => ExternalFormat::Mdzip,
        "capella" | "aird" => ExternalFormat::Capella,
        "xmi" | "uml" | "xml" => ExternalFormat::UmlXmi,
        "sysml" => ExternalFormat::SysmlText,
        _ => return Err("EXTERNAL_UNSUPPORTED_EXTENSION".into()),
    };
    let contract = match format {
        ExternalFormat::Mdzip => "mdzip",
        ExternalFormat::Capella => "capella-7",
        ExternalFormat::UmlXmi => "uml-xmi",
        ExternalFormat::SysmlText => "sysml-text",
    };
    let mut doc = ExternalDocument {
        contract: format!("mbse-craft.external.{contract}/1"),
        format,
        source_path: path.to_owned(),
        sha256: hash(&bytes),
        original_bytes: bytes.clone(),
        files: vec![],
        nodes: vec![],
        references: vec![],
        diagnostics: vec![],
    };
    if doc.format == ExternalFormat::Mdzip {
        project_archive(&mut doc, &bytes)?;
    } else {
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or("EXTERNAL_FILENAME")?;
        doc.files.push(SourceFile {
            name: name.into(),
            sha256: hash(&bytes),
            bytes: bytes.clone(),
        });
        if doc.format == ExternalFormat::SysmlText {
            project_text(&mut doc, name, &bytes)?;
        } else {
            project_xml(&mut doc, name, &bytes)?;
        }
    }
    let ids: BTreeSet<_> = doc.nodes.iter().map(|n| n.source_id.clone()).collect();
    if ids.len() != doc.nodes.len() {
        return Err("EXTERNAL_DUPLICATE_ID_AMBIGUITY".into());
    }
    for reference in &mut doc.references {
        reference.resolved = ids.contains(&reference.target);
    }
    let unresolved = doc.references.iter().filter(|r| !r.resolved).count();
    if unresolved > 0 {
        doc.diagnostic("UNRESOLVED_REFERENCES",format!("{unresolved} references lack an exact local ID target; external resources are never fetched"),None);
    }
    doc.diagnostic("PARTIAL_SEMANTICS","Projection preserves original source types/attributes; formulas, behaviors, guards, units, profiles and diagram layouts are not executed or validated",None);
    Ok(doc)
}
/// Exact original-byte copy only; never overwrites or claims transformed roundtrip.
pub fn fresh_copy(doc: &ExternalDocument, path: &Path) -> Result<(), String> {
    if hash(&doc.original_bytes) != doc.sha256 {
        return Err("EXTERNAL_ORIGINAL_HASH_CHANGED".into());
    }
    craft_atomic::WriteOptions::default()
        .with_new_file_permissions(craft_atomic::NewFilePermissions::Private)
        .write_new(path, &doc.original_bytes)
        .map_err(|error| error.to_string())
}
fn safe_member(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains(['\\', ':', '\0'])
        && !name.split('/').any(|p| p == ".." || p == ".")
}
fn project_archive(doc: &mut ExternalDocument, bytes: &[u8]) -> Result<(), String> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("EXTERNAL_ZIP: {e}"))?;
    if archive.len() > MAX_MEMBERS {
        return Err("EXTERNAL_MEMBER_LIMIT".into());
    }
    let mut names = BTreeSet::new();
    let mut expanded = 0u64;
    let mut candidates = vec![];
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|e| format!("EXTERNAL_ZIP_MEMBER: {e}"))?;
        let name = file.name().to_owned();
        if !safe_member(&name) || !names.insert(name.clone()) {
            return Err("EXTERNAL_UNSAFE_OR_DUPLICATE_MEMBER".into());
        }
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("EXTERNAL_SYMLINK_MEMBER".into());
        }
        expanded = expanded
            .checked_add(file.size())
            .ok_or("EXTERNAL_EXPANDED_LIMIT")?;
        if expanded > MAX_EXPANDED
            || file.size() > MAX_BYTES as u64
            || file.size() > file.compressed_size().max(1).saturating_mul(200)
        {
            return Err("EXTERNAL_ZIP_SIZE_OR_RATIO_LIMIT".into());
        }
        let mut content = vec![];
        file.by_ref()
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut content)
            .map_err(|e| e.to_string())?;
        if content.len() > MAX_BYTES {
            return Err("EXTERNAL_MEMBER_LIMIT".into());
        }
        if content.len() as u64 != file.size() {
            return Err("EXTERNAL_ZIP_DECLARED_SIZE_MISMATCH".into());
        }
        if name == "com.nomagic.magicdraw.uml_model.model"
            || name.ends_with(".uml")
            || name.ends_with(".xmi")
        {
            candidates.push((name.clone(), content.clone()));
        }
        doc.files.push(SourceFile {
            name,
            sha256: hash(&content),
            bytes: content,
        });
    }
    // Native MagicDraw core has priority; alternative multiple candidates are ambiguous.
    let cores: Vec<_> = candidates
        .iter()
        .filter(|(n, _)| n == "com.nomagic.magicdraw.uml_model.model")
        .collect();
    let selected = if cores.len() == 1 {
        cores[0]
    } else if candidates.len() == 1 {
        &candidates[0]
    } else {
        return Err("EXTERNAL_AMBIGUOUS_OR_MISSING_MDZIP_CORE".into());
    };
    project_xml(doc, &selected.0, &selected.1)?;
    doc.diagnostic(
        "ARCHIVE_LAYOUT_UNPROJECTED",
        "All members retained byte-exact; only selected UML/XMI core is semantically projected",
        None,
    );
    Ok(())
}
fn classify(ty: &str, overlay: bool) -> NodeKind {
    if overlay {
        return NodeKind::Stereotype;
    }
    match ty.rsplit(':').next().unwrap_or(ty) {
        "Package" | "Model" | "Project" | "SystemEngineering" => NodeKind::Package,
        "Class" | "Component" | "Interface" | "DataType" | "Enumeration" | "LogicalComponent"
        | "PhysicalComponent" | "SystemComponent" => NodeKind::Classifier,
        "Actor" => NodeKind::Actor,
        "Property" | "Attribute" => NodeKind::Property,
        "Port" | "FlowPort" | "ComponentPort" | "FunctionInputPort" | "FunctionOutputPort" => {
            NodeKind::Port
        }
        "Constraint" | "OpaqueExpression" => NodeKind::Constraint,
        "Activity" | "Action" | "CallBehaviorAction" | "LogicalFunction" | "PhysicalFunction"
        | "SystemFunction" => NodeKind::Activity,
        "Message" => NodeKind::Message,
        "Connector" | "InformationFlow" | "Association" | "ComponentExchange"
        | "FunctionalExchange" => NodeKind::Exchange,
        "Requirement" => NodeKind::Requirement,
        _ => NodeKind::Unknown,
    }
}
fn project_xml(doc: &mut ExternalDocument, name: &str, bytes: &[u8]) -> Result<(), String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "EXTERNAL_UTF8_ONLY")?;
    if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
        return Err("EXTERNAL_XML_DTD_ENTITY_REFUSED".into());
    }
    if let Some(header) = text
        .strip_prefix("<?xml")
        .and_then(|s| s.split_once("?>").map(|v| v.0))
    {
        if let Some(tail) = header.split_once("encoding").map(|v| v.1.trim_start()) {
            let value = tail
                .strip_prefix('=')
                .map(str::trim_start)
                .ok_or("EXTERNAL_XML_ENCODING")?;
            let quote = value.chars().next().ok_or("EXTERNAL_XML_ENCODING")?;
            let encoding = value[1..]
                .split(quote)
                .next()
                .ok_or("EXTERNAL_XML_ENCODING")?;
            if !encoding.eq_ignore_ascii_case("UTF-8") {
                return Err("EXTERNAL_UTF8_ONLY".into());
            }
        }
    }
    let options = roxmltree::ParsingOptions {
        allow_dtd: false,
        nodes_limit: (MAX_NODES * 4) as u32,
        ..Default::default()
    };
    let xml = roxmltree::Document::parse_with_options(text, options)
        .map_err(|e| format!("EXTERNAL_XML: {e}"))?;
    let root = xml.root_element();
    doc.diagnostic(
        "SOURCE_NAMESPACES",
        root.namespaces()
            .map(|ns| format!("{}={}", ns.name().unwrap_or(""), ns.uri()))
            .collect::<Vec<_>>()
            .join("; "),
        Some(name),
    );
    doc.diagnostic(
        "SOURCE_VERSION",
        format!(
            "root={} attrs={:?}",
            root.tag_name().name(),
            root.attributes()
                .map(|a| (a.name(), a.value()))
                .collect::<Vec<_>>()
        ),
        Some(name),
    );
    let mut node_ids = BTreeMap::new();
    for n in xml.descendants().filter(|n| n.is_element()) {
        if n.ancestors().count() > MAX_DEPTH {
            return Err("EXTERNAL_DEPTH_LIMIT".into());
        }
        let id = n
            .attributes()
            .find(|a| {
                a.name() == "id"
                    && (a.namespace().is_none() || a.namespace().is_some_and(|v| v.contains("XMI")))
            })
            .map(|a| a.value().to_owned());
        if let Some(id) = id {
            node_ids.insert(n.id(), id);
        }
    }
    for n in xml.descendants().filter(|n| n.is_element()) {
        let Some(id) = node_ids.get(&n.id()).cloned() else {
            continue;
        };
        if doc.nodes.len() >= MAX_NODES {
            return Err("EXTERNAL_NODE_LIMIT".into());
        }
        let ty = n
            .attributes()
            .find(|a| a.name() == "type" && a.namespace().is_some())
            .map(|a| a.value())
            .unwrap_or(n.tag_name().name());
        let namespace = ty
            .split_once(':')
            .and_then(|(p, _)| n.lookup_namespace_uri(Some(p)))
            .or(n.tag_name().namespace())
            .unwrap_or("");
        let overlay = n.attributes().any(|a| a.name().starts_with("base_"));
        let mut properties: BTreeMap<String, String> = n
            .attributes()
            .map(|a| {
                (
                    format!(
                        "{}{}",
                        a.namespace()
                            .map(|ns| format!("{{{ns}}}"))
                            .unwrap_or_default(),
                        a.name()
                    ),
                    a.value().into(),
                )
            })
            .collect();
        for child in n
            .children()
            .filter(|c| c.is_element() && c.children().all(|v| !v.is_element()))
        {
            if let Some(value) = child.text() {
                properties.insert(format!("child:{}", child.tag_name().name()), value.into());
            }
        }
        let location = SourceLocation {
            file: name.into(),
            start: n.range().start,
            end: n.range().end,
        };
        doc.nodes.push(ExternalNode {
            source_id: id.clone(),
            kind: if overlay
                || namespace.contains("/UML/20131001")
                || namespace.contains("/capella/")
            {
                classify(ty, overlay)
            } else {
                NodeKind::Unknown
            },
            original_type: ty.into(),
            namespace: namespace.into(),
            name: n.attribute("name").unwrap_or("").into(),
            properties,
            location: location.clone(),
        });
        if let Some(parent) = n.ancestors().skip(1).find_map(|p| node_ids.get(&p.id())) {
            doc.references.push(ExternalReference {
                source: id.clone(),
                target: parent.clone(),
                role: "containment.owner".into(),
                resolved: false,
                location: location.clone(),
            });
        }
        for a in n.attributes() {
            let role = a.name();
            if a.namespace().is_none()
                && (matches!(
                    role,
                    "type"
                        | "source"
                        | "target"
                        | "client"
                        | "supplier"
                        | "general"
                        | "specific"
                        | "association"
                        | "memberEnd"
                        | "ownedEnd"
                        | "sendEvent"
                        | "receiveEvent"
                        | "covered"
                        | "abstractType"
                        | "involved"
                        | "sourcePort"
                        | "targetPort"
                ) || role.starts_with("base_"))
            {
                for target in a.value().split_whitespace() {
                    doc.references.push(ExternalReference {
                        source: id.clone(),
                        target: target.trim_start_matches('#').into(),
                        role: role.into(),
                        resolved: false,
                        location: location.clone(),
                    });
                }
            }
        }
        for child in n.children().filter(|c| c.is_element()) {
            for a in child
                .attributes()
                .filter(|a| matches!(a.name(), "href" | "idref"))
            {
                doc.references.push(ExternalReference {
                    source: id.clone(),
                    target: a.value().strip_prefix('#').unwrap_or(a.value()).into(),
                    role: child.tag_name().name().into(),
                    resolved: false,
                    location: SourceLocation {
                        file: name.into(),
                        start: child.range().start,
                        end: child.range().end,
                    },
                });
            }
        }
        if doc.references.len() > MAX_REFERENCES {
            return Err("EXTERNAL_REFERENCE_LIMIT".into());
        }
    }
    if doc.format == ExternalFormat::Capella {
        doc.diagnostic("CAPELLA_AIRD_EXTERNAL","No sidecar is loaded automatically; AIRD diagrams/styles and cross-resource hrefs are retained but unresolved",Some(name));
    }
    if doc.nodes.is_empty() {
        doc.diagnostic(
            "NO_IDENTIFIED_NODES",
            "XML parsed but no supported source identity attributes were found",
            Some(name),
        );
    }
    Ok(())
}
fn project_text(doc: &mut ExternalDocument, name: &str, bytes: &[u8]) -> Result<(), String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "EXTERNAL_UTF8_ONLY")?;
    // Existing preflight enforces token/depth bounds before recursive syntax parsing.
    mbse_model::boundary::preflight(name, bytes)?;
    let parsed = sysmlv2_syntax::parser::parse_source(text);
    for d in &parsed.diagnostics {
        doc.diagnostic("TEXT_PARSE_DIAGNOSTIC", format!("{d:?}"), Some(name));
    }
    walk_members(doc, name, text, &parsed.unit.members, "", 0)?;
    doc.diagnostic("TEXT_PARTIAL_RESOLUTION","Pinned syntax AST declarations and explicit typing/specialization targets only; imports, expressions, implicit relationships and behavioral execution are unresolved. Span anchors are source-revision identities, not persistent universal IDs",Some(name));
    Ok(())
}
fn walk_members(
    doc: &mut ExternalDocument,
    file: &str,
    text: &str,
    members: &[sysmlv2_syntax::ast::Member],
    scope: &str,
    depth: usize,
) -> Result<(), String> {
    use sysmlv2_syntax::ast::{FeatureSpecialization, MemberKind, TargetRef};
    if depth > MAX_DEPTH {
        return Err("EXTERNAL_DEPTH_LIMIT".into());
    }
    for member in members {
        if doc.nodes.len() >= MAX_NODES {
            return Err("EXTERNAL_NODE_LIMIT".into());
        }
        let mut targets: Vec<(String, TargetRef)> = vec![];
        let (ident, ty, body) = match &member.kind {
            MemberKind::Package(p) => (&p.id, "Package".to_owned(), p.body.as_deref()),
            MemberKind::Definition(d) => {
                for target in &d.specializes {
                    targets.push(("specializes".into(), target.clone()));
                }
                (&d.id, format!("{:?}Definition", d.kind), d.body.as_deref())
            }
            MemberKind::Usage(u)
            | MemberKind::Actor(u)
            | MemberKind::Stakeholder(u)
            | MemberKind::Subject(u)
            | MemberKind::Objective(u) => {
                for s in &u.declaration.specializations {
                    match s {
                        FeatureSpecialization::TypedBy(types) => {
                            for t in types {
                                targets.push(("typedBy".into(), t.target.clone()));
                            }
                        }
                        FeatureSpecialization::Subsets(ts) => {
                            for t in ts {
                                targets.push(("subsets".into(), t.clone()));
                            }
                        }
                        FeatureSpecialization::Redefines(ts) => {
                            for t in ts {
                                targets.push(("redefines".into(), t.clone()));
                            }
                        }
                        FeatureSpecialization::References(t) => {
                            targets.push(("references".into(), t.clone()))
                        }
                        FeatureSpecialization::Crosses(t) => {
                            targets.push(("crosses".into(), t.clone()))
                        }
                    }
                }
                (
                    &u.declaration.id,
                    format!("{:?}Usage", u.kind),
                    u.body.as_deref(),
                )
            }
            _ => {
                doc.diagnostic(
                    "TEXT_UNPROJECTED_MEMBER",
                    format!(
                        "Source span {}..{} remains source-only",
                        member.span.start, member.span.end
                    ),
                    Some(file),
                );
                continue;
            }
        };
        let name = ident
            .name
            .as_ref()
            .or(ident.short_name.as_ref())
            .map(|n| n.value.clone())
            .unwrap_or_default();
        let qualified = if scope.is_empty() {
            name.clone()
        } else {
            format!("{scope}::{name}")
        };
        let source_id = format!(
            "{}:{}:{}:{}",
            doc.sha256, qualified, member.span.start, member.span.end
        );
        let location = SourceLocation {
            file: file.into(),
            start: member.span.start as usize,
            end: member.span.end as usize,
        };
        let kind = if ty == "Package" {
            NodeKind::Package
        } else if ty.starts_with("Port") {
            NodeKind::Port
        } else if ty.starts_with("Attribute") {
            NodeKind::Property
        } else if ty.starts_with("Constraint") || ty.starts_with("Calc") {
            NodeKind::Constraint
        } else if ty.starts_with("Requirement") {
            NodeKind::Requirement
        } else if ty.starts_with("Action") {
            NodeKind::Activity
        } else if ty.starts_with("Connection") || ty.starts_with("Flow") {
            NodeKind::Exchange
        } else if ty.starts_with("Part") || ty.starts_with("Item") {
            NodeKind::Classifier
        } else {
            NodeKind::Unknown
        };
        let properties = BTreeMap::from([
            ("qualifiedName".into(), qualified.clone()),
            (
                "sourceText".into(),
                text.get(location.start..location.end)
                    .ok_or("EXTERNAL_BAD_TEXT_SPAN")?
                    .into(),
            ),
        ]);
        doc.nodes.push(ExternalNode {
            source_id: source_id.clone(),
            kind,
            original_type: ty,
            namespace: "SysMLv2Syntax@821221767c3c56cb1ebe7da22666197a47c9c645".into(),
            name,
            properties,
            location: location.clone(),
        });
        for (role, target) in targets {
            let values = match target {
                TargetRef::Name(n) => vec![n.to_display_string()],
                TargetRef::Chain(chain) => chain.iter().map(|n| n.to_display_string()).collect(),
            };
            for target in values {
                doc.references.push(ExternalReference {
                    source: source_id.clone(),
                    target,
                    role: role.clone(),
                    resolved: false,
                    location: location.clone(),
                });
            }
        }
        if doc.references.len() > MAX_REFERENCES {
            return Err("EXTERNAL_REFERENCE_LIMIT".into());
        }
        if let Some(body) = body {
            walk_members(doc, file, text, body, &qualified, depth + 1)?;
        }
    }
    Ok(())
}
