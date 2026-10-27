//! Source-specific frozen inspection only; no native Editor is available here.
use eframe::egui::{self, Color32};
use mbse_exchange::{v2::*, ExternalDocument};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    time::Duration,
};

enum ResultValue {
    Source(PathBuf, ProjectionSet),
    Text(ExternalDocument),
    Copy(PathBuf),
    Preview(AdmissionPreview),
}
#[derive(Default)]
pub struct SemanticBrowser {
    projects: crate::project_set_ui::Browser,
    project_path: String,
    legacy: crate::exchange_ui::ExchangeBrowser,
    name_editor: crate::source_edit_ui::NameEditor,
    visible: bool,
    source: Option<(PathBuf, Arc<ProjectionSet>)>,
    pending: Option<Receiver<Result<ResultValue, String>>>,
    copying: bool,
    feedback: String,
    error: bool,
    query: String,
    kind: String,
    node_signature: Option<(String, String)>,
    nodes: Vec<usize>,
    selected: Option<ElementKey>,
    role: String,
    provenance: String,
    resolution: String,
    related: bool,
    reference_signature: Option<String>,
    references: Vec<usize>,
    reference: Option<usize>,
    preview: Option<AdmissionPreview>,
    copy_path: String,
    bounds: BTreeMap<String, [f32; 4]>,
}
impl SemanticBrowser {
    pub fn is_visible(&self) -> bool {
        self.visible || self.legacy.is_visible() || self.projects.visible || self.projects.pending()
    }
    pub fn copy_pending(&self) -> bool {
        self.name_editor.write_pending()
            || (self.copying && self.pending.is_some())
            || self.legacy.copy_pending()
    }
    pub fn close_blocked(&self) -> bool {
        self.name_editor.guards_close() || self.copy_pending()
    }
    pub fn source_edit_dirty(&self) -> bool {
        self.name_editor.dirty()
    }
    pub fn source_edit_pending(&self) -> bool {
        self.name_editor.pending()
    }
    pub fn keep_open_for_copy(&mut self) {
        if !self.name_editor.request_close() {
            self.visible = true;
            self.error = false;
            self.feedback = self.name_editor.close_message().into();
        }
        if self.legacy.copy_pending() {
            self.legacy.keep_open_for_copy();
        }
        if self.copying {
            self.visible = true;
            self.feedback="Close canceled while the exact source copy is pending. Wait for acknowledgment, then close again.".into();
        }
    }
    pub fn open(&mut self, path: &str) {
        self.project_path=path.trim().into();
        if path.trim().ends_with(".mbse-projects.json") {
            self.open_projects(path.trim());
            return;
        }
        self.visible = true;
        self.start_open(PathBuf::from(path.trim()), None);
    }
    fn open_projects(&mut self,path:&str) {
        if self.name_editor.blocks_source_change() || self.pending.is_some() || self.legacy.copy_pending() || self.projects.pending() {
            self.visible=true;
            self.feedback="Finish pending source operations or explicitly resolve the retained name proposal before opening a project set.".into();
            return;
        }
        self.projects.open_window(path);
        self.visible=false;
        self.legacy.hide();
    }
    pub fn project_audit(&self)->serde_json::Value { self.projects.audit() }
    fn bound(&mut self, key: impl Into<String>, response: &egui::Response) {
        let r = response.interact_rect;
        self.bounds
            .insert(key.into(), [r.min.x, r.min.y, r.max.x, r.max.y]);
    }
    fn start_open(&mut self, path: PathBuf, expected: Option<String>) {
        if self.name_editor.blocks_source_change() {
            self.feedback="A name proposal is retained for its exact frozen source. Create its new copy or explicitly discard it before opening, replacing or reverifying a source.".into();
            return;
        }
        if self.pending.is_some() || self.legacy.copy_pending() || self.projects.pending() {
            self.feedback = "An external operation is pending; frozen source retained.".into();
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.error = false;
        self.feedback = "Reading bounded source bytes and source-specific semantics…".into();
        std::thread::spawn(move || {
            let result=open(&path).and_then(|set| {
                if expected.as_ref().is_some_and(|digest|digest!=&set.source_set_digest) {
                    return Err("Source set changed. Previous frozen source retained. Open as a new snapshot to inspect changed bytes.".into());
                }
                let bytes=set.primary_bytes();
                let xml=String::from_utf8_lossy(bytes).trim_start_matches('\u{feff}').trim_start().starts_with('<');
                let zip=bytes.starts_with(b"PK\x03\x04")||bytes.starts_with(b"PK\x05\x06")||bytes.starts_with(b"PK\x07\x08");
                if !xml && !zip && path.extension().and_then(|x|x.to_str()).is_some_and(|x|x.eq_ignore_ascii_case("sysml")) {
                    let doc=mbse_exchange::open_external(&path)?;
                    if doc.sha256!=set.primary_sha256 {return Err("Source changed during textual routing; previous frozen source retained.".into());}
                    Ok(ResultValue::Text(doc))
                } else { Ok(ResultValue::Source(path,set)) }
            });
            let _ = tx.send(result);
        });
    }
    fn poll(&mut self) {
        let result = self.pending.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(value) => Some(value),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "External worker disconnected; previous frozen source retained.".into(),
            )),
        });
        if let Some(result) = result {
            self.pending = None;
            self.copying = false;
            match result {
                Ok(ResultValue::Source(path, set)) => {
                    self.selected = set.nodes.first().map(|n| n.key.clone());
                    self.source = Some((path, Arc::new(set)));
                    self.node_signature = None;
                    self.reference_signature = None;
                    self.reference = None;
                    self.preview = None;
                    self.visible = true;
                    self.legacy.hide();
                    self.error = false;
                    self.feedback="Frozen source-specific inspection. Native admission: false. Execution: false.".into();
                }
                Ok(ResultValue::Text(doc)) => {
                    self.legacy.accept_frozen(doc);
                    self.visible = false;
                    self.error = false;
                }
                Ok(ResultValue::Copy(path)) => {
                    self.feedback = format!(
                        "Exact original-byte copy created: {}. Frozen source unchanged.",
                        path.display()
                    );
                    self.error = false;
                }
                Ok(ResultValue::Preview(preview)) => {
                    self.preview = Some(preview);
                    self.feedback =
                        "Admission preview completed; no native model was changed.".into();
                    self.error = false;
                }
                Err(error) => {
                    self.feedback = error;
                    self.error = true;
                }
            }
        }
    }
    fn select(&mut self, key: ElementKey) {
        self.selected = Some(key);
        self.preview = None;
        self.reference_signature = None;
    }
    fn filters(&mut self, set: &ProjectionSet) {
        let signature = (self.query.clone(), self.kind.clone());
        if self.node_signature.as_ref() != Some(&signature) {
            let q = self.query.to_lowercase();
            self.nodes = set
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| {
                    (self.kind.is_empty() || format!("{:?}", n.kind) == self.kind)
                        && (q.is_empty()
                            || format!(
                                "{} {} {} {}",
                                n.name,
                                key_label(&n.key),
                                n.source_type.spelling,
                                n.source_type.namespace
                            )
                            .to_lowercase()
                            .contains(&q))
                })
                .map(|(i, _)| i)
                .collect();
            self.node_signature = Some(signature);
        }
        let signature = format!(
            "{}|{}|{}|{}|{:?}",
            self.role, self.provenance, self.resolution, self.related, self.selected
        );
        if self.reference_signature.as_ref() != Some(&signature) {
            self.references =
                set.references
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| {
                        (self.role.is_empty()
                            || r.feature.to_lowercase().contains(&self.role.to_lowercase()))
                            && (self.provenance.is_empty()
                                || format!("{:?}", r.provenance) == self.provenance)
                            && (self.resolution.is_empty()
                                || format!("{:?}", r.resolution) == self.resolution)
                            && (!self.related
                                || self.selected.as_ref().is_some_and(|k| {
                                    k == &r.source || r.target.as_ref() == Some(k)
                                }))
                    })
                    .map(|(i, _)| i)
                    .collect();
            self.reference_signature = Some(signature);
        }
    }
    fn detail(&mut self, ui: &mut egui::Ui, set: &ProjectionSet, node: &SemanticNode) {
        ui.heading(if node.name.is_empty() {
            "Unnamed source element"
        } else {
            &node.name
        });
        wrapped(
            ui,
            format!(
                "{} · {}",
                human(&format!("{:?}", node.kind)),
                key_label(&node.key)
            ),
        );
        if node.kind == SemanticKind::ReferenceProxy {
            ui.strong("Reference mention; not an owned semantic element");
            wrapped(ui,"The source type names a mentioned classifier. This key identifies the stored proxy/mention, not its referent. No actor/human defaults, property interpretation, expression evaluation or unsupplied resource admission is inferred.");
        } else {
            ui.label("Owned source element in this bounded projection");
        }
        let eligible = set.assets.len() == 1
            && set.stage == ProjectionStage::PartialSemanticProjection
            && node.source_type.namespace == "http://www.polarsys.org/capella/core/ctx/7.0.0"
            && matches!(
                node.source_type.local.as_str(),
                "Mission" | "Capability" | "SystemComponent"
            )
            && matches!(
                node.kind,
                SemanticKind::CapellaMission
                    | SemanticKind::CapellaCapability
                    | SemanticKind::CapellaComponent
            )
            && matches!(node.key.identity, ElementIdentity::Original(_))
            && node
                .attributes
                .iter()
                .any(|a| a.namespace.is_empty() && a.name == "name")
            && !node
                .attributes
                .iter()
                .any(|a| matches!(a.name.as_str(), "href" | "idref"));
        if eligible {
            let response = ui.add_enabled(
                !self.name_editor.blocks_source_change() && self.pending.is_none(),
                egui::Button::new("Propose name change in a new source copy"),
            );
            self.bound("semantic:propose-name", &response);
            if response.clicked() {
                let (path, source) = self.source.as_ref().unwrap();
                self.name_editor.begin(
                    path.clone(),
                    source.clone(),
                    node.key.clone(),
                    node.name.clone(),
                );
            }
        } else {
            ui.label("Source name editing is unsupported for this element/profile. Exact original-byte copy remains available.");
        }
        if let Some(flags) = &node.component_flags {
            wrapped(
                ui,
                format!(
                    "Actor: {}\nHuman: {}",
                    flag(&flags.actor),
                    flag(&flags.human)
                ),
            );
            ui.label("Metamodel defaults describe absent stored attributes; creation policy and engineering validity remain unknown.");
        }
        if let Some(kind) = &node.value_kind {
            wrapped(ui,format!("Lexical value type: {}. Attributes below retain source values and unit/type references; no native quantity is inferred.",human(&format!("{kind:?}"))));
        }
        if let Some(expression) = &node.opaque_expression {
            ui.strong("Opaque expression · unevaluated");
            ui.label(format!(
                "{} ordered bodies · {} ordered languages · evaluated: {}",
                expression.bodies.len(),
                expression.languages.len(),
                expression.evaluated
            ));
            for (i, body) in expression.bodies.iter().enumerate() {
                wrapped(
                    ui,
                    format!(
                        "Body [{}]: {}",
                        i,
                        if body.is_empty() { "[empty]" } else { body }
                    ),
                );
            }
            for (i, language) in expression.languages.iter().enumerate() {
                wrapped(
                    ui,
                    format!(
                        "Language [{}]: {}",
                        i,
                        if language.is_empty() {
                            "[empty]"
                        } else {
                            language
                        }
                    ),
                );
            }
        }
        let response = ui.collapsing("Stored attributes and direct text", |ui| {
            for a in &node.attributes {
                ui.strong(format!("{{{}}}{}", a.namespace, a.name));
                wrapped(ui, &a.value);
            }
            for (i, text) in node.direct_text.iter().enumerate() {
                wrapped(ui, format!("Direct text [{i}]: {text}"));
            }
        });
        self.bound("semantic:attributes", &response.header_response);
        let response = ui.collapsing("Exact advanced provenance", |ui| {
            wrapped(
                ui,
                format!(
                    "Key: {:?}\nKind: {:?}\nQualified source type: {:?}\nLocation: {:?}",
                    node.key, node.kind, node.source_type, node.location
                ),
            );
        });
        self.bound("semantic:advanced-provenance", &response.header_response);
        let response = ui.collapsing("Frozen source span", |ui| {
            source_span(ui, set, &node.location);
        });
        self.bound("semantic:source-span", &response.header_response);
        let response = ui.add_enabled(
            self.pending.is_none(),
            egui::Button::new("Preview selected source admission"),
        );
        self.bound("semantic:admission-preview", &response);
        if response.clicked() {
            let source = self.source.as_ref().unwrap().1.clone();
            let selected = vec![node.key.clone()];
            let (tx, rx) = mpsc::channel();
            self.pending = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(Ok(ResultValue::Preview(
                    source.admission_preview(&selected),
                )));
            });
        }
        if let Some(preview) = &self.preview {
            ui.strong(format!("Native admission: {}", preview.admitted));
            for key in &preview.selected {
                wrapped(ui, format!("Preview selection: {}", key_label(key)));
            }
            for loss in &preview.losses {
                wrapped(
                    ui,
                    format!("{}: {}\nAction: {}", loss.code, loss.detail, loss.action),
                );
            }
            for action in &preview.actions {
                wrapped(ui, action);
            }
            ui.label(format!("Omitted losses: {}", preview.omitted_losses));
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) -> BTreeMap<String, [f32; 4]> {
        self.poll();
        self.name_editor.poll(ctx);
        let mut bounds = self.legacy.show(ctx);
        bounds.extend(self.projects.show(ctx));
        if self.pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if !self.visible {
            return bounds;
        }
        let mut visible = self.visible;
        let source = self.source.clone();
        let mut reopen = None;
        let mut copy = None;
        let mut projects=false;
        let window = egui::Window::new("Source-specific external inspection")
            .id(egui::Id::new("semantic-v2-window"))
            .open(&mut visible)
            .default_size(egui::vec2(1050., 750.))
            .vscroll(true);
        window.show(ctx,|ui|{
            ui.set_max_width(ui.available_width());
            ui.colored_label(if self.error{Color32::DARK_RED}else{Color32::from_rgb(25,65,70)},&self.feedback);
            ui.label("Independent frozen source · Native admission: false · Execution: false. No native model mutation or dependency fetching.");
            ui.label("Explicit source project-set manifest");
            let r=ui.add(egui::TextEdit::singleline(&mut self.project_path).char_limit(4096).desired_width(ui.available_width()).hint_text("/workspace/…/project-set.mbse-projects.json"));self.bound("semantic:project-path",&r);
            let r=ui.add_enabled(self.pending.is_none()&&!self.name_editor.blocks_source_change()&&!self.legacy.copy_pending(),egui::Button::new("Browse source project set"));self.bound("semantic:projects",&r);projects=r.clicked();
            let Some((path,set))=&source else{return;};
            wrapped(ui,format!("Source: {}\nContract: {}\nStage: {}",path.display(),set.profile,human(&format!("{:?}",set.stage))));
            wrapped(ui,format!("Primary SHA256: {}\nSource-set SHA256: {}",set.primary_sha256,set.source_set_digest));
            ui.horizontal_wrapped(|ui|{
                let r=ui.add_enabled(self.pending.is_none(),egui::Button::new("Reverify frozen source set"));self.bound("semantic:reverify",&r);if r.clicked(){reopen=Some((path.clone(),Some(set.source_set_digest.clone())));}
                let r=ui.add_enabled(self.pending.is_none(),egui::Button::new("Open current source as new snapshot"));self.bound("semantic:reopen",&r);if r.clicked(){reopen=Some((path.clone(),None));}
            });
            let response=ui.collapsing("Source versions and independent namespace claims",|ui|{
                for v in &set.source_versions {wrapped(ui,format!("Member: {}\nRoot namespace: {}\nXMI version: {}\nDeclared application version: {}\nRecognized bounded contract: {}",v.member_path,v.root_namespace,v.xmi_version.as_deref().unwrap_or("not declared"),v.declared_application_version.as_deref().unwrap_or("not declared"),v.recognized_contract));for ns in &v.metamodel_namespaces{wrapped(ui,format!("Metamodel namespace: {ns}"));}ui.separator();}
                ui.label("Version claims and recognized namespaces do not establish source validity or engineering acceptance.");
            });
            self.bound("semantic:versions",&response.header_response);
            let response=ui.collapsing("Frozen source manifest",|ui|{for e in &set.manifest{wrapped(ui,format!("{} · {} · {} bytes\n{}",e.logical_path,human(&format!("{:?}",e.role)),e.byte_count,e.sha256));}});
            self.bound("semantic:manifest",&response.header_response);
            ui.separator();
            ui.horizontal_wrapped(|ui|{ui.label("Find name, ID, type or member");let r=ui.text_edit_singleline(&mut self.query);self.bound("semantic:node-filter",&r);
                let kinds:std::collections::BTreeSet<_>=set.nodes.iter().map(|n|format!("{:?}",n.kind)).collect();
                let r=egui::ComboBox::from_id_salt("semantic-kind").selected_text(if self.kind.is_empty(){"All source kinds".into()}else{human(&self.kind)}).show_ui(ui,|ui|{ui.selectable_value(&mut self.kind,String::new(),"All source kinds");for k in kinds{ui.selectable_value(&mut self.kind,k.clone(),human(&k));}}).response;self.bound("semantic:kind-filter",&r);
            });
            self.filters(set);ui.label(format!("{} of {} projected nodes · {} owned elements · {} reference mentions",self.nodes.len(),set.nodes.len(),set.counters.owned_elements,set.counters.reference_proxies));
            // Explicit vertical child layouts prevent inherited horizontal rows from overlapping.
            ui.horizontal_top(|ui|{
                let width=(ui.available_width()*0.36).clamp(180.,360.);
                ui.allocate_ui_with_layout(egui::vec2(width,320.),egui::Layout::top_down(egui::Align::Min),|ui|{
                    ui.set_min_width(width);ui.set_max_width(width);
                    egui::ScrollArea::vertical().id_salt("semantic-nodes").max_height(320.).show_rows(ui,28.,self.nodes.len(),|ui,rows|{
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min),|ui|{ui.set_max_width(width);for row in rows{let index=self.nodes[row];let n=&set.nodes[index];let r=ui.add(egui::Button::new(format!("{} · {} · {}",human(&format!("{:?}",n.kind)),n.name,key_label(&n.key))).selected(self.selected.as_ref()==Some(&n.key)).min_size(egui::vec2(width,28.)).truncate());self.bound(format!("semantic:node:{index}"),&r);if r.clicked(){self.select(n.key.clone());}}});
                    });
                });
                let width=ui.available_width().max(120.);
                ui.allocate_ui_with_layout(egui::vec2(width,320.),egui::Layout::top_down(egui::Align::Min),|ui|{ui.set_max_width(width);egui::ScrollArea::vertical().id_salt("semantic-detail").max_height(320.).show(ui,|ui|{ui.set_max_width(width);if let Some(n)=self.selected.as_ref().and_then(|k|set.nodes.iter().find(|n|&n.key==k)){self.detail(ui,set,n);}});});
            });
            ui.separator();ui.strong("Source references");
            ui.horizontal_wrapped(|ui|{
                ui.label("Feature / role");let r=ui.text_edit_singleline(&mut self.role);self.bound("semantic:role-filter",&r);
                combo(ui,"semantic-provenance","All provenance",&mut self.provenance,&["StoredAttribute","StoredChildReference","ContainmentDerived","SourceContainment","ReferenceMention"],&mut self.bounds);
                combo(ui,"semantic-resolution","All resolutions",&mut self.resolution,&["Resolved","Unresolved","WrongKind","UnsupportedEndpointType","UnsuppliedResource","InvalidContainer","UnsupportedSyntax"],&mut self.bounds);
                let r=ui.checkbox(&mut self.related,"Only selected element endpoints");self.bound("semantic:related-filter",&r);
            });
            self.filters(set);ui.label(format!("{} of {} references",self.references.len(),set.references.len()));
            egui::ScrollArea::vertical().id_salt("semantic-refs").max_height(155.).show_rows(ui,26.,self.references.len(),|ui,rows|{ui.with_layout(egui::Layout::top_down(egui::Align::Min),|ui|{for row in rows{let i=self.references[row];let r=&set.references[i];let response=ui.add(egui::Button::new(format!("{} · {} -> {} · {} · {}",r.feature,key_label(&r.source),r.target.as_ref().map(key_label).unwrap_or_else(||r.raw.clone()),human(&format!("{:?}",r.provenance)),human(&format!("{:?}",r.resolution)))).selected(self.reference==Some(i)).min_size(egui::vec2(0.,26.)).truncate());self.bound(format!("semantic:reference:{i}"),&response);if response.clicked(){self.reference=Some(i);}}});});
            if let Some(r)=self.reference.and_then(|i|set.references.get(i)) {
                wrapped(ui,format!("Feature: {}\nOriginal target spelling: {}\nResolution: {}\nProvenance: {}",r.feature,r.raw,human(&format!("{:?}",r.resolution)),human(&format!("{:?}",r.provenance))));
                ui.horizontal_wrapped(|ui|{
                    let response=ui.button("Navigate source endpoint");self.bound("semantic:navigate-source",&response);if response.clicked(){self.select(r.source.clone());}
                    let response=ui.add_enabled(r.target.is_some(),egui::Button::new("Navigate exact target endpoint"));self.bound("semantic:navigate-target",&response);if response.clicked(){if let Some(key)=&r.target{self.select(key.clone());}}
                });
                ui.collapsing("Exact reference provenance and source span",|ui|{wrapped(ui,format!("{r:?}"));source_span(ui,set,&r.location);});
            }
            let response=ui.collapsing(format!("Unknowns, diagnostics and losses ({} / {})",set.diagnostics.len(),set.losses.len()),|ui|{
                for d in &set.diagnostics{wrapped(ui,format!("{}: {}{}",d.code,d.detail,if d.detail_truncated{" [detail truncated]"}else{""}));}
                for l in &set.losses{wrapped(ui,format!("{}: {}\nAction: {}{}",l.code,l.detail,l.action,if l.detail_truncated{" [detail truncated]"}else{""}));}
                wrapped(ui,format!("Resource counters: {:?}",set.counters));
            });
            self.bound("semantic:diagnostics-losses",&response.header_response);
            ui.separator();
            if let Some((path,digest))=self.name_editor.show(ui){reopen=Some((path,Some(digest)));}
            self.bounds.extend(self.name_editor.take_bounds());
            ui.separator();ui.strong("Exact original-byte copy to a NEW destination");
            let r=ui.add(egui::TextEdit::singleline(&mut self.copy_path).desired_width(ui.available_width()).hint_text("/workspace/…/NEW source file (existing destinations refused)"));self.bound("semantic:copy-path",&r);
            let r=ui.add_enabled(self.pending.is_none()&&!self.name_editor.pending()&&!self.copy_path.trim().is_empty(),egui::Button::new("Create new exact source copy"));self.bound("semantic:fresh-copy",&r);if r.clicked(){copy=Some((set.clone(),PathBuf::from(self.copy_path.trim())));}
            ui.label("Copy retains the original container bytes, including opaque archive members. Inspection reports are not reloadable native models or transformed roundtrip exports.");
        });
        if !visible && (self.name_editor.guards_close() || self.copying) {
            self.keep_open_for_copy();
        } else {
            self.visible = visible;
        }
        if let Some((path, expected)) = reopen {
            self.start_open(path, expected);
        }
        if let Some((set, path)) = copy {
            self.start_copy(set, path);
        }
        if projects { self.open_projects(&self.project_path.clone()); }
        bounds.extend(std::mem::take(&mut self.bounds));
        bounds
    }
    fn start_copy(&mut self, set: Arc<ProjectionSet>, path: PathBuf) {
        if self.pending.is_some() || self.name_editor.pending() {
            return;
        }
        if !path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || path
                .parent()
                .and_then(|p| std::fs::canonicalize(p).ok())
                .is_none_or(|p| !p.starts_with("/workspace"))
        {
            self.error = true;
            self.feedback =
                "Choose an absolute NEW destination with an existing parent on /workspace.".into();
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.copying = true;
        self.error = false;
        self.feedback = "Writing exact original bytes; waiting for acknowledgment…".into();
        std::thread::spawn(move || {
            let result = set
                .copy_original_new(&path)
                .map(|_| ResultValue::Copy(path));
            let _ = tx.send(result);
        });
    }
}
fn wrapped(ui: &mut egui::Ui, text: impl Into<String>) {
    let mut text = text.into();
    if text.len() > 16 * 1024 {
        let mut end = 16 * 1024;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n[Display clipped at 16 KiB; original source remains frozen.]");
    }
    ui.add(egui::Label::new(text).wrap());
}
pub(crate) fn key_label(key: &ElementKey) -> String {
    format!(
        "{} :: {}",
        key.member_path,
        match &key.identity {
            ElementIdentity::Original(id) => id.clone(),
            ElementIdentity::XmlSpan { start, end } => format!("XML bytes {start}..{end}"),
            ElementIdentity::TextSpan {
                start,
                end,
                qualified_name,
            } => format!("{qualified_name} bytes {start}..{end}"),
        }
    )
}
fn flag(value: &FlagValue) -> String {
    match value {
        FlagValue::Explicit(v) => format!("{v} (explicit stored value)"),
        FlagValue::MetamodelDefault(v) => format!("{v} (metamodel default; attribute absent)"),
        FlagValue::UnsupportedLexical(v) => format!("unsupported lexical value: {v}"),
    }
}
fn human(value: &str) -> String {
    let mut out = String::new();
    let mut lower = false;
    for c in value.chars() {
        if c.is_uppercase() && lower {
            out.push(' ');
        }
        out.push(c);
        lower = c.is_lowercase();
    }
    out
}
fn combo(
    ui: &mut egui::Ui,
    id: &str,
    all: &str,
    value: &mut String,
    choices: &[&str],
    bounds: &mut BTreeMap<String, [f32; 4]>,
) {
    let r = egui::ComboBox::from_id_salt(id)
        .selected_text(if value.is_empty() {
            all.into()
        } else {
            human(value)
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(value, String::new(), all);
            for choice in choices {
                ui.selectable_value(value, choice.to_string(), human(choice));
            }
        })
        .response;
    let r = r.interact_rect;
    bounds.insert(
        format!("semantic:{}-filter", id.trim_start_matches("semantic-")),
        [r.min.x, r.min.y, r.max.x, r.max.y],
    );
}
fn source_span(ui: &mut egui::Ui, set: &ProjectionSet, location: &Locator) {
    wrapped(
        ui,
        format!(
            "Member {} · byte span {}..{}",
            location.member_path, location.start, location.end
        ),
    );
    if let Some(asset) = set
        .assets
        .iter()
        .find(|a| a.logical_path == location.member_path)
    {
        if let Some(bytes) = asset.bytes.get(location.start..location.end) {
            let end = bytes.len().min(16 * 1024);
            wrapped(ui, String::from_utf8_lossy(&bytes[..end]).into_owned());
            if end < bytes.len() {
                ui.label("Preview truncated at 16 KiB; exact bytes remain frozen.");
            }
        } else {
            ui.label("Source span unavailable; no substituted source used.");
        }
    } else {
        ui.label("Source member unavailable; no ambient file read.");
    }
}

#[cfg(test)]
mod source_name_tests {
    use super::*;
    #[test]
    fn selecting_another_scoped_node_retains_name_proposal_and_blocks_source_open() {
        let xml=br#"<root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:ctx="http://www.polarsys.org/capella/core/ctx/7.0.0"><ownedMissions xsi:type="ctx:Mission" id="first" name="First"/><ownedMissions xsi:type="ctx:Mission" id="second" name="Second"/></root>"#;
        let set = Arc::new(
            mbse_exchange::v2::inspect_primary_bytes("synthetic.capella", xml.to_vec()).unwrap(),
        );
        let keys: Vec<_> = set
            .nodes
            .iter()
            .filter(|n| n.kind == SemanticKind::CapellaMission)
            .map(|n| n.key.clone())
            .collect();
        assert_eq!(keys.len(), 2);
        let mut browser = SemanticBrowser::default();
        let path = PathBuf::from("/workspace/synthetic.capella");
        browser.source = Some((path.clone(), set.clone()));
        browser.name_editor.begin(
            path,
            set,
            keys[0].clone(),
            "Retained unfinished & name".into(),
        );
        browser.select(keys[1].clone());
        assert_eq!(browser.selected, Some(keys[1].clone()));
        assert_eq!(
            browser.name_editor.retained_binding(),
            Some((keys[0].clone(), "Retained unfinished & name".into()))
        );
        browser.start_open(PathBuf::from("/workspace/never-opened.capella"), None);
        assert!(browser.pending.is_none());
        assert_eq!(
            browser.source.as_ref().unwrap().0,
            PathBuf::from("/workspace/synthetic.capella")
        );
        assert!(browser.close_blocked());
        assert!(!browser.copy_pending());
        browser.open("/workspace/retained-proposal.mbse-projects.json");
        assert!(!browser.projects.visible && !browser.projects.pending());
        assert!(browser.source_edit_dirty());
        browser.visible = false;
        browser.keep_open_for_copy();
        assert!(browser.visible);
        assert!(browser.source_edit_dirty());
        assert_eq!(
            browser.name_editor.retained_binding(),
            Some((keys[0].clone(), "Retained unfinished & name".into()))
        );
    }
}
