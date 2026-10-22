//! Versioned, source-specific semantic inspection. No native editor/admission API.
mod xml;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read, Write},
    path::Path,
};
pub const PROFILE: &str = "mbse-craft.external.semantic-set/2";
pub const MAX_PRIMARY: usize = 32 * 1024 * 1024;
pub const MAX_ADDITIONAL: usize = 16 * 1024 * 1024;
pub const MAX_SET_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ASSETS: usize = 256;
pub const MAX_NODES: usize = 100_000;
pub const MAX_REFERENCES: usize = 200_000;
pub const MAX_DEPTH: usize = 96;
pub const MAX_ATTRIBUTES_PER_NODE: usize = 128;
pub const MAX_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_TEXT_PER_NODE: usize = 1024 * 1024;
pub const MAX_ATTRIBUTE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_REPORT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_DIAGNOSTICS: usize = 512;
pub const MAX_LOSSES: usize = 512;
const MAX_AUXILIARY_BYTES: usize = 1024 * 1024;
#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetRole {
    Primary,
    SemanticMember,
    OpaqueMember,
    Companion,
    Dependency,
}
#[derive(Clone, Debug, Serialize)]
pub struct SourceAsset {
    pub logical_path: String,
    pub role: AssetRole,
    pub sha256: String,
    pub byte_count: usize,
    #[serde(skip_serializing)]
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementIdentity {
    Original(String),
    XmlSpan {
        start: usize,
        end: usize,
    },
    TextSpan {
        start: usize,
        end: usize,
        qualified_name: String,
    },
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElementKey {
    pub asset_sha256: String,
    pub member_path: String,
    pub identity: ElementIdentity,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct QualifiedType {
    pub namespace: String,
    pub local: String,
    pub spelling: String,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum SemanticKind {
    CapellaMission,
    CapellaCapability,
    CapellaExploitation,
    CapellaInvolvement,
    CapellaComponent,
    CapellaConstraint,
    CapellaOpaqueExpression,
    CapellaPort,
    CapellaExchange,
    CapellaAllocation,
    CapellaPropertyGroup,
    CapellaTypedValue,
    CapellaEnumerationType,
    CapellaEnumerationLiteral,
    CapellaFunction,
    UmlClassifier,
    UmlActor,
    UmlProperty,
    UmlPort,
    UmlConstraint,
    UmlActivity,
    UmlExchange,
    UmlMessage,
    UmlPackage,
    SysmlProfileApplication,
    UafProfileApplication,
    UnknownProfileApplication,
    MetamodelHelper,
    UnknownClassifier,
}
#[derive(Clone, Debug, Serialize)]
pub struct Attribute {
    pub namespace: String,
    pub name: String,
    pub value: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Locator {
    pub member_path: String,
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum FlagValue {
    Explicit(bool),
    MetamodelDefault(bool),
    UnsupportedLexical(String),
}
#[derive(Clone, Debug, Serialize)]
pub struct ComponentFlags {
    pub actor: FlagValue,
    pub human: FlagValue,
}
#[derive(Clone, Debug, Serialize)]
pub struct OpaqueExpression {
    pub bodies: Vec<String>,
    pub languages: Vec<String>,
    pub evaluated: bool,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum ValueKind {
    String,
    Integer,
    Float,
    Boolean,
    Enumeration,
}
#[derive(Clone, Debug, Serialize)]
pub struct SemanticNode {
    pub key: ElementKey,
    pub source_type: QualifiedType,
    pub kind: SemanticKind,
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub direct_text: Vec<String>,
    pub component_flags: Option<ComponentFlags>,
    pub opaque_expression: Option<OpaqueExpression>,
    pub value_kind: Option<ValueKind>,
    pub location: Locator,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum ReferenceProvenance {
    StoredAttribute,
    StoredChildReference,
    ContainmentDerived,
    SourceContainment,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum ReferenceResolution {
    Resolved,
    Unresolved,
    WrongKind,
    UnsupportedEndpointType,
    UnsuppliedResource,
    InvalidContainer,
}
#[derive(Clone, Debug, Serialize)]
pub struct SemanticReference {
    pub source: ElementKey,
    pub target: Option<ElementKey>,
    pub raw: String,
    pub feature: String,
    pub declaring_type: QualifiedType,
    pub provenance: ReferenceProvenance,
    pub resolution: ReferenceResolution,
    pub location: Locator,
}
#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub detail: String,
    pub detail_truncated: bool,
    pub location: Option<Locator>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Loss {
    pub code: String,
    pub element: Option<ElementKey>,
    pub detail: String,
    pub detail_truncated: bool,
    pub action: String,
}
#[derive(Clone, Debug, Serialize, Default)]
pub struct Counters {
    pub asset_bytes: usize,
    pub nodes: usize,
    pub references: usize,
    pub attributes: usize,
    pub attribute_bytes: usize,
    pub direct_text_bytes: usize,
    pub serialized_report_bytes: usize,
    pub omitted_diagnostics: usize,
    pub omitted_losses: usize,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum ProjectionStage {
    SourceRetained,
    PartialSemanticProjection,
    SourceAdmissionRefused,
    InterpretationRefused,
    OutputBudgetRefused,
}
#[derive(Clone, Debug, Serialize)]
pub struct VersionClaim {
    pub member_path: String,
    pub root_namespace: String,
    pub metamodel_namespaces: Vec<String>,
    pub xmi_version: Option<String>,
    pub declared_application_version: Option<String>,
    pub recognized_contract: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct ManifestEntry {
    pub logical_path: String,
    pub sha256: String,
    pub byte_count: usize,
    pub role: AssetRole,
}
#[derive(Clone, Debug, Serialize)]
pub struct ProjectionSet {
    pub profile: String,
    pub primary_sha256: String,
    pub source_set_digest: String,
    pub manifest: Vec<ManifestEntry>,
    pub assets: Vec<SourceAsset>,
    pub stage: ProjectionStage,
    pub source_versions: Vec<VersionClaim>,
    pub nodes: Vec<SemanticNode>,
    pub references: Vec<SemanticReference>,
    pub diagnostics: Vec<Diagnostic>,
    pub losses: Vec<Loss>,
    pub counters: Counters,
    #[serde(skip_serializing)]
    auxiliary_bytes: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct AdmissionPreview {
    pub profile: String,
    pub source_set_digest: String,
    pub selected: Vec<ElementKey>,
    pub admitted: bool,
    pub losses: Vec<Loss>,
    pub omitted_losses: usize,
    pub actions: Vec<String>,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn logical_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.starts_with('/')
        && !value.contains(['\\', ':', '\0'])
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}
fn digest_manifest(primary: &str, entries: &[ManifestEntry]) -> Result<String, String> {
    let canonical: Vec<_> = entries
        .iter()
        .map(|e| (&e.logical_path, &e.sha256, e.byte_count, &e.role))
        .collect();
    serde_json::to_vec(&("mbse-craft.source-set-digest/2", primary, canonical))
        .map(|v| hash(&v))
        .map_err(|e| e.to_string())
}
fn clipped(value: &str) -> (String, bool) {
    let mut n = value.len().min(1024);
    while !value.is_char_boundary(n) {
        n -= 1;
    }
    (value[..n].into(), n < value.len())
}
struct SizeWriter {
    used: usize,
    limit: usize,
}
impl Write for SizeWriter {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.used = self
            .used
            .checked_add(b.len())
            .filter(|n| *n <= self.limit)
            .ok_or_else(|| std::io::Error::other("V2_REPORT_BUDGET"))?;
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn serialized_size(value: &impl Serialize, limit: usize) -> Result<usize, String> {
    let mut writer = SizeWriter { used: 0, limit };
    serde_json::to_writer(&mut writer, value).map_err(|e| e.to_string())?;
    Ok(writer.used)
}
pub(super) struct Budget {
    used: usize,
}
impl Budget {
    fn admit(&mut self, value: &impl Serialize) -> Result<(), String> {
        let remaining = MAX_REPORT_BYTES
            .checked_sub(self.used)
            .ok_or("V2_REPORT_BUDGET")?;
        let n = serialized_size(value, remaining)?;
        self.used = self
            .used
            .checked_add(n + 1)
            .filter(|n| *n <= MAX_REPORT_BYTES)
            .ok_or("V2_REPORT_BUDGET")?;
        Ok(())
    }
}
impl ProjectionSet {
    fn new(primary: SourceAsset) -> Self {
        Self {
            profile: PROFILE.into(),
            primary_sha256: primary.sha256.clone(),
            source_set_digest: String::new(),
            manifest: vec![],
            assets: vec![primary],
            stage: ProjectionStage::SourceRetained,
            source_versions: vec![],
            nodes: vec![],
            references: vec![],
            diagnostics: vec![],
            losses: vec![],
            counters: Counters::default(),
            auxiliary_bytes: 0,
        }
    }
    fn freeze_manifest(&mut self) -> Result<(), String> {
        self.manifest = self
            .assets
            .iter()
            .map(|a| ManifestEntry {
                logical_path: a.logical_path.clone(),
                sha256: a.sha256.clone(),
                byte_count: a.byte_count,
                role: a.role.clone(),
            })
            .collect();
        self.manifest
            .sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
        if self
            .manifest
            .windows(2)
            .any(|w| w[0].logical_path == w[1].logical_path)
        {
            return Err("V2_LOGICAL_PATH_AMBIGUITY".into());
        }
        self.source_set_digest = digest_manifest(&self.primary_sha256, &self.manifest)?;
        self.counters.asset_bytes = self.assets.iter().map(|a| a.byte_count).sum();
        Ok(())
    }
    fn diagnostic(&mut self, code: &str, detail: &str, location: Option<Locator>) {
        #[derive(Serialize)]
        struct View<'a> {
            code: &'a str,
            detail: &'a str,
            detail_truncated: bool,
            location: &'a Option<Locator>,
        }
        let (detail, detail_truncated) = clipped(detail);
        let view = View {
            code,
            detail: &detail,
            detail_truncated,
            location: &location,
        };
        let size = serialized_size(&view, MAX_AUXILIARY_BYTES).unwrap_or(MAX_AUXILIARY_BYTES + 1);
        if self.diagnostics.len() >= MAX_DIAGNOSTICS - 1
            || self.auxiliary_bytes.saturating_add(size + 1) > MAX_AUXILIARY_BYTES
        {
            self.counters.omitted_diagnostics = self.counters.omitted_diagnostics.saturating_add(1);
            if !self
                .diagnostics
                .iter()
                .any(|d| d.code == "DIAGNOSTICS_TRUNCATED")
            {
                self.diagnostics.push(Diagnostic {
                    code: "DIAGNOSTICS_TRUNCATED".into(),
                    detail: "Additional diagnostics omitted; see counters.omitted_diagnostics"
                        .into(),
                    detail_truncated: false,
                    location: None,
                });
            }
            return;
        }
        self.auxiliary_bytes += size + 1;
        self.diagnostics.push(Diagnostic {
            code: code.into(),
            detail,
            detail_truncated,
            location,
        });
    }
    fn loss(&mut self, code: &str, element: Option<&ElementKey>, detail: &str, action: &str) {
        #[derive(Serialize)]
        struct View<'a> {
            code: &'a str,
            element: Option<&'a ElementKey>,
            detail: &'a str,
            detail_truncated: bool,
            action: &'a str,
        }
        let (detail, detail_truncated) = clipped(detail);
        let view = View {
            code,
            element,
            detail: &detail,
            detail_truncated,
            action,
        };
        let size = serialized_size(&view, MAX_AUXILIARY_BYTES).unwrap_or(MAX_AUXILIARY_BYTES + 1);
        if self.losses.len() >= MAX_LOSSES - 1
            || self.auxiliary_bytes.saturating_add(size + 1) > MAX_AUXILIARY_BYTES
        {
            self.counters.omitted_losses = self.counters.omitted_losses.saturating_add(1);
            if !self.losses.iter().any(|d| d.code == "LOSSES_TRUNCATED") {
                self.losses.push(Loss {
                    code: "LOSSES_TRUNCATED".into(),
                    element: None,
                    detail: "Additional losses omitted; see counters.omitted_losses".into(),
                    detail_truncated: false,
                    action: "Inspect original source; native admission remains refused".into(),
                });
            }
            return;
        }
        self.auxiliary_bytes += size + 1;
        self.losses.push(Loss {
            code: code.into(),
            element: element.cloned(),
            detail,
            detail_truncated,
            action: action.into(),
        });
    }
    pub fn primary_bytes(&self) -> &[u8] {
        &self.assets[0].bytes
    }
    pub fn copy_original_new(&self, path: &Path) -> Result<(), String> {
        if hash(self.primary_bytes()) != self.primary_sha256 {
            return Err("V2_PRIMARY_HASH_CHANGED".into());
        }
        craft_atomic::WriteOptions::default()
            .with_new_file_permissions(craft_atomic::NewFilePermissions::Private)
            .write_new(path, self.primary_bytes())
            .map_err(|e| e.to_string())
    }
    pub fn admission_preview(&self, selected: &[ElementKey]) -> AdmissionPreview {
        // Bound the borrowed selection before building indexes or cloning identifiers.
        if selected.len() > MAX_NODES
            || serialized_size(&selected, MAX_REPORT_BYTES - 2 * MAX_AUXILIARY_BYTES).is_err()
        {
            return AdmissionPreview { profile: "mbse-craft.source-admission-preview/2".into(), source_set_digest: self.source_set_digest.clone(), selected: vec![], admitted: false,
                losses: vec![Loss {code: "SELECTION_OUTPUT_BUDGET_REFUSED".into(), element: None, detail: "Selection cannot be represented within the preview budget; no partial selection is admitted".into(), detail_truncated: false, action: "Reduce the explicit source selection".into()}], omitted_losses: selected.len(), actions: vec!["Reduce the explicit source selection".into()] };
        }
        let mut losses = vec![];
        let mut loss_bytes = 0usize;
        let mut omitted = 0usize;
        let chosen: BTreeSet<_> = selected.iter().collect();
        let node_index: BTreeMap<_, _> = self.nodes.iter().map(|n| (&n.key, n)).collect();
        let mut push = |code: &str, key: Option<&ElementKey>, detail: &str, action: &str| {
            #[derive(Serialize)]
            struct View<'a> {
                code: &'a str,
                element: Option<&'a ElementKey>,
                detail: &'a str,
                detail_truncated: bool,
                action: &'a str,
            }
            let (detail, detail_truncated) = clipped(detail);
            let size = serialized_size(
                &View {
                    code,
                    element: key,
                    detail: &detail,
                    detail_truncated,
                    action,
                },
                MAX_AUXILIARY_BYTES,
            )
            .unwrap_or(MAX_AUXILIARY_BYTES + 1);
            if losses.len() >= MAX_LOSSES - 1
                || loss_bytes.saturating_add(size + 1) > MAX_AUXILIARY_BYTES
            {
                omitted += 1;
                return;
            }
            loss_bytes += size + 1;
            losses.push(Loss {
                code: code.into(),
                element: key.cloned(),
                detail,
                detail_truncated,
                action: action.into(),
            });
        };
        if selected.is_empty() {
            push(
                "NO_EXPLICIT_SELECTION",
                None,
                "No source element selected; no first-ID fallback is permitted",
                "Select exact scoped source elements",
            );
        }
        for key in selected {
            match node_index.get(key) {
                None => push(
                    "STALE_OR_UNKNOWN_SCOPED_KEY",
                    Some(key),
                    "Selection is absent from this exact source set",
                    "Refresh source inspection and select again",
                ),
                Some(node) => {
                    let(code,detail)=match node.kind{SemanticKind::CapellaConstraint|SemanticKind::CapellaOpaqueExpression=>("UNEVALUATED_CONSTRAINT","Source constraint bodies/languages remain unevaluated; no native requirement/calculation conversion exists"),SemanticKind::CapellaTypedValue=>("TYPED_PROPERTY_NOT_MEASURE","Source typed property values are not native Measures or calculation outcomes"),SemanticKind::UnknownClassifier|SemanticKind::UnknownProfileApplication=>("UNSUPPORTED_SOURCE_TYPE","This exact source namespace/classifier/profile has no qualified native mapping"),_=>("EDITABLE_SOURCE_SLICE_CONTRACT_PENDING","Source meaning is inspected; reversible source-specific editable admission is not yet defined")};
                    push(code,Some(key),detail,"Keep source slice, or define and review an explicit editable subset; no native mutation");
                }
            }
        }
        for r in self
            .references
            .iter()
            .filter(|r| chosen.contains(&r.source))
        {
            if r.resolution != ReferenceResolution::Resolved
                || r.target.as_ref().is_some_and(|t| !chosen.contains(t))
            {
                push("SELECTED_CLOSURE_INCOMPLETE",Some(&r.source),"Required source endpoint is unresolved, wrongly typed or outside the selected closure","Inspect the named source role and select/resolve its exact scoped endpoint");
            }
        }
        if self.stage != ProjectionStage::PartialSemanticProjection {
            push(
                "PROJECTION_STAGE_REFUSED",
                None,
                "Source interpretation is incomplete/refused",
                "Inspect stage diagnostics before attempting admission",
            );
        }
        if omitted > 0 {
            losses.push(Loss {
                code: "LOSSES_TRUNCATED".into(),
                element: None,
                detail: format!("{omitted} additional admission losses omitted"),
                detail_truncated: false,
                action: "Reduce selected scope; admission remains refused".into(),
            });
        }
        AdmissionPreview {
            profile: "mbse-craft.source-admission-preview/2".into(),
            source_set_digest: self.source_set_digest.clone(),
            selected: selected.to_vec(),
            admitted: false,
            losses,
            omitted_losses: omitted,
            actions: vec![
                "Inspect source-specific slice".into(),
                "Choose a smaller closed selection".into(),
                "Export original bytes as a new copy".into(),
            ],
        }
    }
}
/// Only an explicitly supplied primary is read. No adjacent-file discovery or URI fetch.
pub fn open(primary: &Path) -> Result<ProjectionSet, String> {
    let bytes = mbse_model::local_input::read(primary, MAX_PRIMARY, "V2_PRIMARY")?;
    let name = primary
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("V2_PRIMARY_NAME")?;
    if !logical_path(name) {
        return Err("V2_PRIMARY_NAME".into());
    }
    // Content determines the parser. A suffix is only a UI hint and cannot
    // turn text into XML or prevent recognizable XML/archive inspection.
    let zip_content = [
        b"PK\x03\x04".as_slice(),
        b"PK\x05\x06".as_slice(),
        b"PK\x07\x08".as_slice(),
    ]
    .iter()
    .any(|signature| bytes.starts_with(signature));
    let xml_content = std::str::from_utf8(&bytes).ok().is_some_and(|text| {
        text.trim_start_matches('\u{feff}')
            .trim_start_matches([' ', '\t', '\r', '\n'])
            .starts_with('<')
    });
    let primary = SourceAsset {
        logical_path: format!("primary/{name}"),
        role: AssetRole::Primary,
        sha256: hash(&bytes),
        byte_count: bytes.len(),
        bytes,
    };
    let mut report = ProjectionSet::new(primary);
    let mut semantic = vec![0usize];
    if zip_content {
        match archive_assets(&report.assets[0].bytes) {
            Ok((assets, core)) => {
                semantic = vec![core + 1];
                report.assets.extend(assets);
            }
            Err(error) => {
                report.stage = ProjectionStage::SourceAdmissionRefused;
                report.diagnostic("ARCHIVE_ADMISSION_REFUSED", &error, None);
                semantic.clear();
            }
        }
    } else if !xml_content {
        report.stage = ProjectionStage::InterpretationRefused;
        report.loss("FORMAT_ADAPTER_UNAVAILABLE",None,"V2 currently interprets source-specific XML/MDZIP only; exact original bytes are retained","Use original-byte inspection/copy; no native admission");
        semantic.clear();
    }
    report.freeze_manifest()?;
    let used = serialized_size(&report, MAX_REPORT_BYTES)?
        .checked_add(MAX_AUXILIARY_BYTES + 4096)
        .ok_or("V2_REPORT_BUDGET")?;
    let mut budget = Budget { used };
    if !semantic.is_empty() {
        match xml::project(&mut report, &semantic, &mut budget) {
            Ok(()) => report.stage = ProjectionStage::PartialSemanticProjection,
            Err(error) => {
                report.stage = if error.contains("BUDGET") || error.contains("LIMIT") {
                    ProjectionStage::OutputBudgetRefused
                } else {
                    ProjectionStage::InterpretationRefused
                };
                report.nodes.clear();
                report.references.clear();
                report.diagnostic("SEMANTIC_PROJECTION_REFUSED", &error, None);
                report.loss(
                    "WHOLE_INTERPRETATION_REFUSED",
                    None,
                    &error,
                    "Inspect original bytes; no typed graph or native admission is claimed",
                );
            }
        }
    }
    report.counters.nodes = report.nodes.len();
    report.counters.references = report.references.len();
    match serialized_size(&report, MAX_REPORT_BYTES) {
        Ok(mut size) => {
            for _ in 0..3 {
                report.counters.serialized_report_bytes = size;
                let next = serialized_size(&report, MAX_REPORT_BYTES)?;
                if next == size {
                    break;
                }
                size = next;
            }
        }
        Err(error) => {
            report.nodes.clear();
            report.references.clear();
            report.source_versions.clear();
            report.stage = ProjectionStage::OutputBudgetRefused;
            report.diagnostics.clear();
            report.losses.clear();
            report.counters.nodes = 0;
            report.counters.references = 0;
            report.diagnostic("REPORT_BUDGET_REFUSED", &error, None);
            report.counters.serialized_report_bytes = serialized_size(&report, MAX_REPORT_BYTES)?;
        }
    }
    for _ in 0..16 {
        let size = serialized_size(&report, MAX_REPORT_BYTES)?;
        if size == report.counters.serialized_report_bytes {
            break;
        }
        report.counters.serialized_report_bytes = size;
    }
    Ok(report)
}
fn archive_assets(bytes: &[u8]) -> Result<(Vec<SourceAsset>, usize), String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if archive.len() + 1 > MAX_ASSETS {
        return Err("V2_MEMBER_LIMIT".into());
    }
    let mut total = bytes.len();
    let mut names = BTreeSet::new();
    let mut assets = vec![];
    let mut cores = vec![];
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|e| e.to_string())?;
        let raw = file.name();
        let canonical = raw.strip_suffix('/').unwrap_or(raw);
        if !logical_path(canonical) || !names.insert(canonical.to_owned()) {
            return Err("V2_UNSAFE_OR_AMBIGUOUS_MEMBER_PATH".into());
        }
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("V2_SYMLINK_MEMBER".into());
        }
        let size = usize::try_from(file.size()).map_err(|_| "V2_MEMBER_SIZE_LIMIT")?;
        if size > MAX_ADDITIONAL || file.size() > file.compressed_size().max(1).saturating_mul(200)
        {
            return Err("V2_MEMBER_SIZE_OR_RATIO_LIMIT".into());
        }
        total = total
            .checked_add(size)
            .filter(|n| *n <= MAX_SET_BYTES)
            .ok_or("V2_SET_BYTES_LIMIT")?;
        let name = canonical.to_owned();
        let mut bytes = vec![];
        file.by_ref()
            .take(MAX_ADDITIONAL as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() != size {
            return Err("V2_DECLARED_MEMBER_SIZE_MISMATCH".into());
        }
        let semantic = name == "com.nomagic.magicdraw.uml_model.model";
        if semantic {
            cores.push(assets.len());
        }
        assets.push(SourceAsset {
            logical_path: format!("members/{name}"),
            role: if semantic {
                AssetRole::SemanticMember
            } else {
                AssetRole::OpaqueMember
            },
            sha256: hash(&bytes),
            byte_count: size,
            bytes,
        });
    }
    if cores.len() != 1 {
        return Err("V2_EXACT_MDZIP_CORE_REQUIRED".into());
    }
    Ok((assets, cores[0]))
}
