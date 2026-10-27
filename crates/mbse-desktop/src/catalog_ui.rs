//! External frozen library preview; never mutates a native model.
use eframe::egui;
use mbse_catalog::Catalog;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};
#[derive(Default)]
pub struct CatalogBrowser {
    visible: bool,
    catalog: Option<Catalog>,
    pending: Option<Receiver<Result<Catalog, String>>>,
    feedback: String,
    selected: Option<String>,
}
impl CatalogBrowser {
    pub fn open(&mut self, path: &str) {
        self.visible = true;
        self.start(PathBuf::from(path.trim()), None)
    }
    fn start(&mut self, path: PathBuf, expected: Option<String>) {
        if self.pending.is_some() {
            self.feedback = "A bounded catalog admission is already pending.".into();
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.feedback = "Reading external library; active model is unchanged.".into();
        std::thread::spawn(move || {
            let result=mbse_catalog::open(&path).and_then(|c|{if expected.as_ref().is_some_and(|h|h!=&c.source_sha256){Err("Source changed since frozen preview; existing preview retained. Reopen explicitly to admit new bytes.".into())}else{Ok(c)}});
            let _ = tx.send(result);
        });
    }
    pub fn show(&mut self, ctx: &egui::Context) {
        let result = self.pending.as_ref().and_then(|r| match r.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "Catalog worker disconnected; previous frozen preview retained.".into(),
            )),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            self.pending = None;
            match result {
                Ok(c) => {
                    self.selected = c.symbols.first().map(|s| s.path.clone());
                    self.feedback="Frozen external library admitted for read-only indexing. No native semantic model was imported.".into();
                    self.catalog = Some(c)
                }
                Err(e) => self.feedback = e,
            }
        }
        if self.pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if !self.visible {
            return;
        }
        let mut visible = true;
        let mut reverify = None;
        egui::Window::new("External library catalog — frozen indexed source").id(egui::Id::new("external-catalog")).open(&mut visible).default_size([1000.,780.]).show(ctx,|ui|{
   ui.label(&self.feedback);
   ui.label("Package inventory is separate from typed external projection and native semantic admission. No evaluator or resolver is run.");
   if let Some(c)=&self.catalog{
    ui.separator();ui.heading(format!("{} {}",c.name,c.version));ui.label(format!("Publisher: {} · Declared license: {}",c.publisher,c.declared_license));ui.monospace(format!("Metamodel: {}\nFrozen SHA-256: {}",c.metamodel,c.source_sha256));
    ui.horizontal(|ui|{ui.label(format!("{} files · {} exported namespace entries",c.files.len(),c.symbols.len()));if ui.add_enabled(self.pending.is_none(),egui::Button::new("Reverify current source bytes")).clicked(){reverify=Some((c.root.clone(),c.source_sha256.clone()));}});
    for d in &c.diagnostics{ui.label(d);}
    ui.collapsing("Retained dependency metadata — sources unresolved",|ui|{ui.monospace(serde_json::to_string_pretty(c.project.value.get("usage").unwrap_or(&serde_json::Value::Null)).unwrap_or_default());});
    ui.collapsing("Original metadata and unknown fields",|ui|{ui.label("These exact bytes are retained; typed JSON is never rewritten.");egui::ScrollArea::vertical().max_height(180.).show(ui,|ui|{ui.monospace(String::from_utf8_lossy(&c.project.bytes));ui.monospace(String::from_utf8_lossy(&c.metadata.bytes));});});
    ui.separator();ui.horizontal(|ui|{egui::ScrollArea::vertical().id_salt("catalog-symbols").max_height(300.).max_width(330.).show(ui,|ui|{ui.set_width(300.);ui.vertical(|ui|{for s in &c.symbols{if ui.selectable_label(self.selected.as_deref()==Some(&s.path),format!("{}\n{}",s.name,s.path)).clicked(){self.selected=Some(s.path.clone());}}});});
     ui.vertical(|ui|{if let Some(file)=self.selected.as_deref().and_then(|p|c.source(p)){ui.monospace(format!("{}\n{}",file.path,file.sha256));let limit=file.bytes.len().min(128*1024);let mut text=String::from_utf8_lossy(&file.bytes[..limit]).into_owned();if limit<file.bytes.len(){ui.label("Text preview truncated at128KiB; original bytes remain retained.");}egui::ScrollArea::both().max_height(300.).show(ui,|ui|{ui.add(egui::TextEdit::multiline(&mut text).code_editor().interactive(false).desired_rows(16).desired_width(600.));});}});
    });
   }else{ui.label("Enter an absolute local package directory containing .project.json and .meta.json in the source/action path, then choose Browse library folder.");}
   ui.label("This window retains a frozen preview even if disk changes. No source copying, package export, network access or active-model changes occur.");
  });
        self.visible = visible;
        if let Some((path, hash)) = reverify {
            self.start(path, Some(hash));
        }
    }
}
