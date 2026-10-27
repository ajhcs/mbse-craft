//! Explicit source project sets. No native editor or write capability is passed here.
use eframe::egui;
use mbse_exchange::project_sets::{ProjectSet,ElementKey,LocalIdentity,Recheck};
use std::{collections::BTreeMap,path::PathBuf,sync::{Arc,mpsc::{self,Receiver}}};

enum Completion { Opened(Arc<ProjectSet>),Checked(Recheck) }
#[derive(Default)]
pub struct Browser {
    pub visible:bool,
    path:String,
    source:Option<Arc<ProjectSet>>,
    pending:Option<Receiver<Result<Completion,String>>>,
    checking:bool,
    last_check:Option<Result<Recheck,String>>,
    member:String,
    query:String,
    selected:Option<ElementKey>,
    row_signature:Option<(String,String,String)>,
    rows:Vec<(ElementKey,String)>,
    detail_signature:Option<ElementKey>,
    detail:String,
    detail_truncated:bool,
    note:String,
    error:bool,
    bounds:BTreeMap<String,[f32;4]>,
}
impl Browser {
    pub fn pending(&self)->bool {self.pending.is_some()}
    pub fn open_window(&mut self,path:&str) {
        self.visible=true;
        // Closing/reopening a window retains its exact frozen inspection.
        if self.source.is_none()&&!self.pending(){self.path=path.into();}
    }
    fn start_open(&mut self) {
        if self.pending(){return;}
        let path=PathBuf::from(self.path.trim());
        let(tx,rx)=mpsc::channel();self.pending=Some(rx);self.checking=false;
        self.error=false;self.note="Reading only the manifest and its explicit local files…".into();
        std::thread::spawn(move||{let _=tx.send(ProjectSet::open(&path).map(|s|Completion::Opened(Arc::new(s))));});
    }
    fn start_check(&mut self) {
        if self.pending(){return;}
        let Some(source)=self.source.clone()else{return;};
        let(tx,rx)=mpsc::channel();self.pending=Some(rx);self.checking=true;
        self.error=false;self.note="Checking the original descriptor and original file list; frozen data retained…".into();
        std::thread::spawn(move||{let _=tx.send(source.recheck().map(Completion::Checked));});
    }
    fn poll(&mut self) {
        let done=self.pending.as_ref().and_then(|rx|match rx.try_recv(){Ok(r)=>Some(r),Err(mpsc::TryRecvError::Empty)=>None,Err(mpsc::TryRecvError::Disconnected)=>Some(Err("Source reader disconnected; previous frozen inspection retained.".into()))});
        if let Some(done)=done {
            self.pending=None;
            match done {
                Ok(Completion::Opened(source))=>{self.source=Some(source);self.member.clear();self.query.clear();self.selected=None;self.row_signature=None;self.detail_signature=None;self.detail.clear();self.last_check=None;self.error=false;self.note="Frozen source project set opened. Files have not been checked again since capture.".into();},
                Ok(Completion::Checked(check))=>{self.error=!check.unchanged;self.note=if check.unchanged{"Files matched the frozen capture at this explicit check."}else{"Files differ from the frozen capture. Prior source data and identities are retained."}.into();self.last_check=Some(Ok(check));},
                Err(error)=>{if self.checking{self.last_check=Some(Err(error.clone()));}self.note=error;self.error=true;}
            }
            self.checking=false;
        }
    }
    fn bound(&mut self,key:impl Into<String>,r:&egui::Response){let b=r.interact_rect;self.bounds.insert(key.into(),[b.min.x,b.min.y,b.max.x,b.max.y]);}
    fn refresh_rows(&mut self,source:&ProjectSet,digest:&str) {
        let signature=(digest.into(),self.member.clone(),self.query.clone());
        if self.row_signature.as_ref()==Some(&signature){return;}
        let query=self.query.to_lowercase();
        self.rows=source.summary().members.iter().filter(|m|self.member.is_empty()||m.key==self.member).flat_map(|m|source.keys(&m.key)).filter_map(|key|{
            let label=if let Some(r)=source.native_record(&key){format!("{} · {} #{} · {}",key.member,r.kind.label(),r.id.0,r.name)}else if let Some(n)=source.external_node(&key){format!("{} · {} · {}",key.member,crate::semantic_ui::key_label(&n.key),n.name)}else{return None;};
            if query.is_empty()||label.to_lowercase().contains(&query){Some((key,label))}else{None}
        }).collect();self.row_signature=Some(signature);
    }
    pub fn audit(&self)->serde_json::Value {
        serde_json::json!({"visible":self.visible,"pending":self.pending(),"path_text":self.path,"member_filter":self.member,"query":self.query,"selected":self.selected,"summary":self.source.as_ref().map(|s|s.summary()),"last_explicit_check":self.last_check,"note":self.note,"error":self.error})
    }
    pub fn show(&mut self,ctx:&egui::Context)->BTreeMap<String,[f32;4]> {
        self.poll();self.bounds.clear();if self.pending(){ctx.request_repaint_after(std::time::Duration::from_millis(100));}
        if !self.visible{return BTreeMap::new();}
        let mut visible=self.visible;let source=self.source.clone();let mut open=false;let mut check=false;let mut clear=false;
        egui::Window::new("Source project sets").id(egui::Id::new("source-project-sets")).open(&mut visible).default_size(egui::vec2(1120.,820.)).vscroll(true).show(ctx,|ui|{
            ui.set_max_width(ui.available_width());
            ui.label("Read-only frozen sources · no native admission, model edits or dependency fetching");
            ui.label("Manifest path");let r=ui.add(egui::TextEdit::singleline(&mut self.path).char_limit(4096).desired_width(ui.available_width()).hint_text("/workspace/…/project-set.mbse-projects.json"));self.bound("projects:path",&r);
            ui.horizontal_wrapped(|ui|{
                let r=ui.add_enabled(!self.pending()&&!self.path.trim().is_empty(),egui::Button::new("Open explicit manifest"));self.bound("projects:Open",&r);open=r.clicked();
                let r=ui.add_enabled(!self.pending()&&source.is_some(),egui::Button::new("Recheck original files"));self.bound("projects:Recheck",&r);check=r.clicked();
                let r=ui.add_enabled(!self.pending()&&source.is_some(),egui::Button::new("Clear inspection"));self.bound("projects:Clear",&r);clear=r.clicked();
                let r=ui.add_enabled(source.is_some(),egui::Button::new("Copy inspection JSON"));self.bound("projects:Copy",&r);if r.clicked(){if let Ok(text)=serde_json::to_string_pretty(&self.audit()){ctx.copy_text(text);}}
                let r=ui.button("Close");self.bound("projects:Close",&r);if r.clicked(){self.visible=false;}
            });
            let r=ui.colored_label(if self.error{egui::Color32::DARK_RED}else{egui::Color32::from_rgb(25,65,70)},&self.note);self.bound("projects:status",&r);
            let Some(source)=&source else{return;};let summary=source.summary();
            ui.label(format!("Frozen manifest: {}",summary.manifest_path.display()));
            ui.label(format!("{} members · {} captured bytes · {} additional archive bytes",summary.members.len(),summary.captured_bytes,summary.expanded_bytes));
            ui.label(format!("Source-set SHA-256: {}",summary.source_set_digest));
            ui.label("A matching pin proves captured bytes, not source truth or semantic completeness. Recheck is explicit; a prior result is historical.");
            ui.horizontal_wrapped(|ui|{let r=ui.selectable_label(self.member.is_empty(),"All members");self.bound("projects:all-members",&r);if r.clicked(){self.member.clear();}for m in &summary.members{let r=ui.selectable_label(self.member==m.key,&m.key);self.bound(format!("projects:member:{}",m.key),&r);if r.clicked(){self.member=m.key.clone();}}});
            let r=ui.add(egui::TextEdit::singleline(&mut self.query).char_limit(512).hint_text("Find member, name, kind or local ID"));self.bound("projects:filter",&r);
            self.refresh_rows(source,&summary.source_set_digest);ui.label(format!("{} matching element identities · native list contains top-level records",self.rows.len()));
            egui::ScrollArea::vertical().id_salt("project-set-elements").max_height(220.).show_rows(ui,28.,self.rows.len(),|ui,range|{for i in range{let(key,label)=self.rows[i].clone();let r=ui.add(egui::Button::new(label).selected(self.selected.as_ref()==Some(&key)).truncate().min_size(egui::vec2(ui.available_width(),28.)));self.bound(format!("projects:element:{i}"),&r);if r.clicked(){self.selected=Some(key);}}});
            if let Some(key)=self.selected.clone() {
                ui.separator();ui.strong(format!("Selected member: {}",key.member));ui.label(format!("Primary SHA-256: {}",key.primary_sha256));
                if self.detail_signature.as_ref()!=Some(&key){let value=match &key.identity{LocalIdentity::NativeRecord(_)=>source.native_record(&key).and_then(|r|serde_json::to_value(r).ok()),LocalIdentity::External(_)=>source.external_node(&key).and_then(|n|serde_json::to_value(n).ok())};let text=value.and_then(|v|serde_json::to_string_pretty(&v).ok()).unwrap_or_default();self.detail_truncated=text.chars().count()>16384;self.detail=text.chars().take(16384).collect();self.detail_signature=Some(key);}
                egui::ScrollArea::vertical().id_salt("project-set-detail").max_height(140.).show(ui,|ui|{ui.monospace(&self.detail);});if self.detail_truncated{ui.label("Detail preview limited to 16,384 characters; complete source data remains frozen.");}
            }
            let r=ui.collapsing("Files, pins and interpretation diagnostics",|ui|{for m in &summary.members{ui.strong(format!("{} · {:?}",m.key,m.format));ui.label(format!("{} · {:?} · {}",m.primary.path,m.primary.state,m.primary.actual_sha256.as_deref().unwrap_or("not captured")));ui.label(format!("Expected SHA-256: {}",m.primary.expected_sha256));if let Some(issue)=&m.issue{ui.label(issue);}if let Some(n)=m.native_top_level_records{ui.label(format!("Native document {} · {n} top-level records · complete snapshot retained",m.native_document.as_deref().unwrap_or("unknown")));}if let Some(stage)=&m.external_stage{ui.label(format!("Existing external projection: {stage:?} · {} nodes · {} losses",m.external_nodes.unwrap_or(0),m.external_losses.unwrap_or(0)));}for c in &m.companions{ui.label(format!("Companion {} · {:?} · {:?} · bound declaration checked: {}",c.file.path,c.role,c.file.state,c.bound_declaration_checked));ui.label(format!("Expected {} · observed {}",c.file.expected_sha256,c.file.actual_sha256.as_deref().unwrap_or("not captured")));if let Some(issue)=&c.issue{ui.label(issue);}}ui.separator();}});self.bound("projects:files",&r.header_response);
            let r=ui.collapsing("Last explicit check and limits",|ui|{if let Some(check)=&self.last_check{ui.monospace(serde_json::to_string_pretty(check).unwrap_or_default());}for limit in &summary.limits{ui.label(limit);}});self.bound("projects:limits",&r.header_response);
        });
        self.visible&=visible;
        // The path field never retargets a captured source or its recheck list.
        if open{self.start_open();}else if check{self.start_check();}else if clear{self.source=None;self.selected=None;self.rows.clear();self.row_signature=None;self.detail.clear();self.detail_signature=None;self.last_check=None;self.note="Inspection cleared; source files unchanged.".into();self.error=false;}
        std::mem::take(&mut self.bounds)
    }
}

#[cfg(test)]
#[path="project_set_ui_tests.rs"]
mod tests;
