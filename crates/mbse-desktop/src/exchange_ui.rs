//! Frozen external projection browser. It never writes an Editor or native Snapshot.
use eframe::egui::{self, Color32, RichText};
use mbse_exchange::{ExternalDocument, ExternalNode, ExternalReference, NodeKind, SourceLocation};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct NodeKey {
    file: String,
    id: String,
}
impl NodeKey {
    fn of(node: &ExternalNode) -> Self {
        Self {
            file: node.location.file.clone(),
            id: node.source_id.clone(),
        }
    }
}
enum WorkerValue {
    Document(ExternalDocument),
    Copy(PathBuf),
}
#[derive(Default)]
pub struct ExchangeBrowser {
    visible: bool,
    document: Option<Arc<ExternalDocument>>,
    pending: Option<Receiver<Result<WorkerValue, String>>>,
    feedback: String,
    error: bool,
    query: String,
    last_query: String,
    kind: Option<NodeKind>,
    last_kind: Option<NodeKind>,
    filtered: Vec<usize>,
    selected: Option<NodeKey>,
    reference: Option<usize>,
    reference_filter: bool,
    copy_path: String,
    bounds: BTreeMap<String, [f32; 4]>,
}
impl ExchangeBrowser {
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    pub fn open(&mut self, path: &str) {
        self.visible = true;
        self.start_open(PathBuf::from(path.trim()), None);
    }
    fn bound(&mut self, key: impl Into<String>, response: &egui::Response) {
        let r = response.interact_rect;
        self.bounds
            .insert(key.into(), [r.min.x, r.min.y, r.max.x, r.max.y]);
    }
    fn start_open(&mut self, path: PathBuf, expected: Option<String>) {
        if self.pending.is_some() {
            self.feedback="An external read or copy is already pending; the existing frozen preview remains available.".into();
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.error = false;
        self.feedback="Reading a bounded external projection. Native model and prior frozen preview remain unchanged.".into();
        std::thread::spawn(move || {
            let result=mbse_exchange::open_external(&path).and_then(|doc|{
            if expected.as_ref().is_some_and(|hash|hash!=&doc.sha256){Err("Source bytes changed. Previous frozen projection retained; use Open current source as new snapshot to inspect new bytes.".into())}else{Ok(WorkerValue::Document(doc))}});
            let _ = tx.send(result);
        });
    }
    fn start_copy(&mut self, doc: Arc<ExternalDocument>, path: PathBuf) {
        if self.pending.is_some() {
            return;
        }
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
        if !path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || parent
                .and_then(|p| std::fs::canonicalize(p).ok())
                .is_none_or(|p| !p.starts_with("/workspace"))
        {
            self.feedback =
                "Choose an absolute new destination with an existing parent on /workspace.".into();
            self.error = true;
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.feedback="Writing an exact original-byte copy to a new destination; no transformed roundtrip is claimed.".into();
        self.error = false;
        std::thread::spawn(move || {
            let result = mbse_exchange::fresh_copy(&doc, &path).map(|_| WorkerValue::Copy(path));
            let _ = tx.send(result);
        });
    }
    fn poll(&mut self) {
        let result = self.pending.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "External worker disconnected; previous frozen document retained.".into(),
            )),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            self.pending = None;
            match result {
                Ok(WorkerValue::Document(doc)) => {
                    self.selected = doc.nodes.first().map(NodeKey::of);
                    self.filtered = (0..doc.nodes.len()).collect();
                    self.query.clear();
                    self.last_query.clear();
                    self.kind = None;
                    self.last_kind = None;
                    self.reference = None;
                    self.document = Some(Arc::new(doc));
                    self.feedback="Frozen typed external projection opened. Native admission: false. Execution: false.".into();
                    self.error = false;
                }
                Ok(WorkerValue::Copy(path)) => {
                    self.feedback = format!(
                        "Exact original-byte copy created: {}. The frozen source was not modified.",
                        path.display()
                    );
                    self.error = false;
                }
                Err(error) => {
                    self.feedback = error;
                    self.error = true;
                }
            }
        }
    }
    fn filter(&mut self, doc: &ExternalDocument) {
        if self.query == self.last_query && self.kind == self.last_kind {
            return;
        }
        let query = self.query.to_lowercase();
        self.filtered = doc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                self.kind.as_ref().is_none_or(|kind| kind == &node.kind)
                    && (query.is_empty()
                        || node.source_id.to_lowercase().contains(&query)
                        || node.name.to_lowercase().contains(&query)
                        || node.original_type.to_lowercase().contains(&query)
                        || node.location.file.to_lowercase().contains(&query))
            })
            .map(|(index, _)| index)
            .collect();
        self.last_query = self.query.clone();
        self.last_kind = self.kind.clone();
    }
    fn node_details(&mut self, ui: &mut egui::Ui, doc: &ExternalDocument, node: &ExternalNode) {
        ui.heading(if node.name.is_empty() {
            "Unnamed source element"
        } else {
            &node.name
        });
        ui.label(format!(
            "Projection kind: {:?} · original type: {}",
            node.kind, node.original_type
        ));
        ui.monospace(format!(
            "Source identity: ({}, {})",
            node.location.file, node.source_id
        ));
        ui.label(format!(
            "Resolved type namespace: {}",
            if node.namespace.is_empty() {
                "Unspecified"
            } else {
                &node.namespace
            }
        ));
        if node.kind == NodeKind::Unknown {
            ui.colored_label(
                Color32::DARK_RED,
                "Unknown source type preserved. No supported modeling meaning is inferred.",
            );
        }
        ui.label("Source attributes and scalar text (including authored unit/quantity/classification values):");
        egui::ScrollArea::vertical().id_salt("external-properties").max_height(145.).show(ui,|ui|{egui::Grid::new("external-attribute-grid").striped(true).show(ui,|ui|{
            for (key,value) in node.properties.iter().take(128){ui.monospace(key);let text=bounded_text(value.as_bytes(),8192);ui.label(text);ui.end_row();}
        });if node.properties.len()>128{ui.label("Property table limited to128 entries; all original properties and bytes remain retained.");}});
        self.source_preview(ui, doc, &node.location);
    }
    fn source_preview(
        &mut self,
        ui: &mut egui::Ui,
        doc: &ExternalDocument,
        location: &SourceLocation,
    ) {
        ui.collapsing(format!("Frozen source span · {} bytes {}..{}",location.file,location.start,location.end),|ui|{
            let files:Vec<_>=doc.files.iter().filter(|file|file.name==location.file).collect();
            if files.len()!=1{ui.colored_label(Color32::DARK_RED,"Source member identity is absent or ambiguous; no first-member fallback is used.");return;}
            let file=files[0];ui.monospace(format!("Member SHA-256: {}",file.sha256));
            if let Some(bytes)=file.bytes.get(location.start..location.end){let mut text=bounded_text(bytes,32*1024);egui::ScrollArea::both().id_salt(("external-span",&location.file,location.start)).max_height(180.).show(ui,|ui|{ui.add(egui::TextEdit::multiline(&mut text).code_editor().interactive(false).desired_width(650.));});}
            else{ui.colored_label(Color32::DARK_RED,"Recorded source span is outside retained bytes; preview refused.");}
        });
    }
    fn endpoint(
        &mut self,
        ui: &mut egui::Ui,
        doc: &ExternalDocument,
        reference: &ExternalReference,
        target: bool,
    ) {
        let id = if target {
            &reference.target
        } else {
            &reference.source
        };
        ui.label(format!(
            "{}: {id}",
            if target { "Target" } else { "Source" }
        ));
        let candidates = endpoint_candidates(doc, reference, target);
        if candidates.is_empty() {
            ui.label("No projected endpoint available. The original reference remains unresolved here; no dependency is fetched.");
        }
        if candidates.len() > 1 {
            ui.colored_label(Color32::DARK_RED,"Ambiguous identity across members. Choose a concrete (file, ID) to inspect; navigation does not resolve the reference.");
        }
        for key in candidates {
            let label = format!("Inspect [{}] {}", key.file, key.id);
            let response = ui.button(label);
            self.bound(
                format!(
                    "exchange:endpoint:{}:{}:{}",
                    if target { "target" } else { "source" },
                    key.file,
                    key.id
                ),
                &response,
            );
            if response.clicked() {
                self.selected = Some(key);
            }
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) -> BTreeMap<String, [f32; 4]> {
        self.bounds.clear();
        self.poll();
        if self.pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if !self.visible {
            return BTreeMap::new();
        }
        let mut visible = true;
        let mut reopen = None;
        let mut reverify = None;
        let mut copy = None;
        egui::Window::new("External typed projection · frozen source").id(egui::Id::new("external-typed-projection")).open(&mut visible).default_pos([135.,105.]).default_size([1170.,800.]).vscroll(true).show(ctx,|ui|{
            ui.colored_label(if self.error{Color32::DARK_RED}else{Color32::DARK_GREEN},&self.feedback);
            ui.label(RichText::new("Read-only external projection. Native admission: false · Execution: false · Engineering acceptance: not established").strong());
            let Some(doc)=self.document.clone() else{ui.label("Use the source/action path to choose a local .mdzip, .capella, .aird, .xmi, .uml, .xml or .sysml file, then Open external projection. Unsupported or malformed input leaves any previous preview intact.");return;};
            ui.label(format!("Source: {}",doc.source_path.display()));ui.label(format!("Format: {:?} · Projection contract: {}",doc.format,doc.contract));ui.monospace(format!("Frozen source SHA-256: {}",doc.sha256));
            ui.horizontal_wrapped(|ui|{
                ui.label(format!("{} members · {} nodes · {} references · {} diagnostics",doc.files.len(),doc.nodes.len(),doc.references.len(),doc.diagnostics.len()));
                let response=ui.add_enabled(self.pending.is_none(),egui::Button::new("Reverify frozen source"));self.bound("exchange:Reverify",&response);if response.clicked(){reverify=Some((doc.source_path.clone(),doc.sha256.clone()));}
                let response=ui.add_enabled(self.pending.is_none(),egui::Button::new("Open current source as new snapshot"));self.bound("exchange:Open current",&response);if response.clicked(){reopen=Some(doc.source_path.clone());}
            });
            ui.collapsing("Source contracts, namespaces and versions",|ui|{for source in &doc.source_evidence{
                ui.label(format!("{} · recognized contract: {}",source.file,source.recognized_contract));ui.monospace(format!("Root namespace: {}",source.root_namespace));
                for (key,value) in &source.version_attributes{ui.label(format!("Source version {key}: {value}"));}if source.version_attributes.is_empty(){ui.label("No source version attribute was supplied; none is inferred.");}
                for (prefix,namespace) in &source.namespaces{ui.label(format!("Namespace {prefix}: {namespace}"));}
            }});
            ui.collapsing(format!("Diagnostics and loss boundaries ({})",doc.diagnostics.len()),|ui|{egui::ScrollArea::vertical().id_salt("external-diagnostics").max_height(200.).show(ui,|ui|{for diagnostic in &doc.diagnostics{ui.label(format!("{}{}: {}",diagnostic.code,diagnostic.file.as_ref().map(|f|format!(" [{f}]")).unwrap_or_default(),diagnostic.message));}});});
            ui.separator();ui.horizontal_wrapped(|ui|{ui.label("Filter name, ID, original type or member");let response=ui.text_edit_singleline(&mut self.query);self.bound("exchange:filter",&response);
                let combo=egui::ComboBox::from_id_salt("external-kind-filter").selected_text(self.kind.as_ref().map(|k|format!("{k:?}")).unwrap_or("All kinds".into())).show_ui(ui,|ui|{ui.selectable_value(&mut self.kind,None,"All kinds");for kind in [NodeKind::Package,NodeKind::Classifier,NodeKind::Actor,NodeKind::Property,NodeKind::Port,NodeKind::Constraint,NodeKind::Activity,NodeKind::Message,NodeKind::Exchange,NodeKind::Requirement,NodeKind::Stereotype,NodeKind::Unknown]{let label=format!("{kind:?}");ui.selectable_value(&mut self.kind,Some(kind),label);}});self.bound("exchange:kind-filter",&combo.response);
            });self.filter(&doc);
            ui.label(format!("{} matching nodes. Selection uses (member, source ID), never an unqualified first-ID lookup.",self.filtered.len()));
            ui.horizontal_top(|ui|{
                egui::ScrollArea::vertical().id_salt("external-node-list").max_width(360.).max_height(280.).show_rows(ui,34.,self.filtered.len(),|ui,rows|{ui.set_width(350.);for row in rows{let node=&doc.nodes[self.filtered[row]];let key=NodeKey::of(node);let response=ui.add_sized([345.,30.],egui::Button::selectable(self.selected.as_ref()==Some(&key),format!("{:?} · {} · {}",node.kind,if node.name.is_empty(){"(unnamed)"}else{&node.name},node.source_id)).truncate());self.bound(format!("exchange:node:{}:{}",key.file,key.id),&response);if response.on_hover_text(format!("Member: {}\nID: {}\nOriginal type: {}",key.file,key.id,node.original_type)).clicked(){self.selected=Some(key);}}});
                ui.vertical(|ui|{let selected:Vec<_>=self.selected.as_ref().map(|key|doc.nodes.iter().filter(|node|node.location.file==key.file&&node.source_id==key.id).collect()).unwrap_or_default();if selected.len()==1{self.node_details(ui,&doc,selected[0]);}else{ui.label("Choose a unique (member, source ID) to inspect; ambiguous identities are not collapsed.");}});
            });
            ui.separator();ui.horizontal(|ui|{ui.label("Source references");let response=ui.checkbox(&mut self.reference_filter,"Only references mentioning selected ID");self.bound("exchange:reference-filter",&response);});
            let references:Vec<_>=doc.references.iter().enumerate().filter(|(_,r)|!self.reference_filter||self.selected.as_ref().is_some_and(|key|r.source==key.id||r.target==key.id)).map(|(index,_)|index).collect();
            egui::ScrollArea::vertical().id_salt("external-reference-list").max_height(135.).show_rows(ui,24.,references.len(),|ui,rows|{for row in rows{let index=references[row];let r=&doc.references[index];let response=ui.selectable_label(self.reference==Some(index),format!("[{}] {} -- {} -> {} · {:?}",r.location.file,r.source,r.role,r.target,r.resolution));self.bound(format!("exchange:reference:{index}"),&response);if response.clicked(){self.reference=Some(index);}}});
            if let Some(reference)=self.reference.and_then(|index|doc.references.get(index)){
                ui.label(format!("Role: {} · Core resolution: {:?} · resolved flag: {}",reference.role,reference.resolution,reference.resolved));if let Some(original)=&reference.original_target{ui.label(format!("Original target spelling: {original}"));}if let Some(scope)=&reference.lexical_scope{ui.label(format!("Lexical scope: {scope}"));}
                ui.horizontal_top(|ui|{ui.vertical(|ui|self.endpoint(ui,&doc,reference,false));ui.separator();ui.vertical(|ui|self.endpoint(ui,&doc,reference,true));});self.source_preview(ui,&doc,&reference.location);
            }
            ui.separator();ui.label("Exact original-byte fresh copy (not a transformed semantic export)");let response=ui.add(egui::TextEdit::singleline(&mut self.copy_path).hint_text("/workspace/…/NEW-file with the source format extension").desired_width(ui.available_width()));self.bound("exchange:copy-path",&response);
            let response=ui.add_enabled(self.pending.is_none()&&!self.copy_path.trim().is_empty(),egui::Button::new("Create new exact source copy"));self.bound("exchange:Fresh copy",&response);if response.clicked(){copy=Some((doc.clone(),PathBuf::from(self.copy_path.trim())));}
            ui.label("No source model is rewritten, no formula or behavior is executed, and no reference dependency is fetched. Duplicate source IDs refused by the projection core remain explicit open errors.");
        });
        self.visible = visible;
        if let Some((path, hash)) = reverify {
            self.start_open(path, Some(hash));
        } else if let Some(path) = reopen {
            self.start_open(path, None);
        } else if let Some((doc, path)) = copy {
            self.start_copy(doc, path);
        }
        std::mem::take(&mut self.bounds)
    }
}
fn bounded_text(bytes: &[u8], limit: usize) -> String {
    let mut text = String::from_utf8_lossy(&bytes[..bytes.len().min(limit)]).into_owned();
    if bytes.len() > limit {
        text.push_str("\n[Preview truncated; original bytes remain retained.]");
    }
    text
}
fn endpoint_candidates(
    doc: &ExternalDocument,
    r: &ExternalReference,
    target: bool,
) -> Vec<NodeKey> {
    let raw = if target {
        r.target.as_str()
    } else {
        r.source.as_str()
    };
    let (file, id, explicit) = if let Some((file, id)) = raw.split_once('#') {
        (
            if file.is_empty() {
                r.location.file.as_str()
            } else {
                file
            },
            id,
            true,
        )
    } else {
        (r.location.file.as_str(), raw, false)
    };
    let local: Vec<_> = doc
        .nodes
        .iter()
        .filter(|n| n.location.file == file && n.source_id == id)
        .map(NodeKey::of)
        .collect();
    if !local.is_empty() || explicit {
        return local;
    }
    doc.nodes
        .iter()
        .filter(|n| n.source_id == id)
        .map(NodeKey::of)
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use mbse_exchange::{ExternalFormat, ReferenceResolution};
    fn node(file: &str, id: &str) -> ExternalNode {
        ExternalNode {
            source_id: id.into(),
            kind: NodeKind::Classifier,
            original_type: "uml:Class".into(),
            namespace: "http://www.eclipse.org/uml2/5.0.0/UML".into(),
            name: "same label".into(),
            properties: BTreeMap::new(),
            location: SourceLocation {
                file: file.into(),
                start: 0,
                end: 1,
            },
        }
    }
    #[test]
    fn member_identity_never_silently_chooses_first_global_id() {
        let doc = ExternalDocument {
            contract: "test".into(),
            source_evidence: vec![],
            format: ExternalFormat::Mdzip,
            source_path: PathBuf::from("source.mdzip"),
            sha256: String::new(),
            original_bytes: vec![],
            files: vec![],
            nodes: vec![node("a.uml", "same"), node("b.uml", "same")],
            references: vec![],
            diagnostics: vec![],
        };
        let r = ExternalReference {
            source: "source".into(),
            target: "same".into(),
            role: "type".into(),
            resolved: false,
            resolution: ReferenceResolution::Ambiguous,
            location: SourceLocation {
                file: "other.uml".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(endpoint_candidates(&doc, &r, true).len(), 2);
        let mut explicit = r.clone();
        explicit.target = "b.uml#same".into();
        assert_eq!(
            endpoint_candidates(&doc, &explicit, true),
            vec![NodeKey {
                file: "b.uml".into(),
                id: "same".into()
            }]
        );
        explicit.target = "missing.uml#same".into();
        assert!(endpoint_candidates(&doc, &explicit, true).is_empty());
    }
}
