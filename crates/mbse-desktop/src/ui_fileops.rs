//! Application-owned chooser and background reads using pinned Craft outcome semantics.
use super::*;
use craft_platform::dialog_outcome::Outcome;
use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Purpose {
    Open,
    Recover,
    Import,
    Save,
}
impl Purpose {
    fn label(self) -> &'static str {
        match self {
            Self::Open => "Open native",
            Self::Recover => "Recover independent draft",
            Self::Import => "Import restricted SysML",
            Self::Save => "Save native",
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
struct Guard {
    native: mbse_model::stage1::Freshness,
    views: String,
    selected: Option<RecordId>,
    save_path: String,
}
struct ReadPending {
    guard: Guard,
    reply: Receiver<Result<Session, String>>,
    cancelled: bool,
}
struct Listing {
    directory: String,
    reply: Receiver<Result<Vec<(PathBuf, bool)>, String>>,
}
pub(super) struct FileOps {
    pub open: bool,
    pub purpose: Purpose,
    pub path: String,
    pub directory: String,
    guard: Option<Guard>,
    rows: Vec<(PathBuf, bool)>,
    listing: Option<Listing>,
    read: Option<ReadPending>,
    pub note: String,
    pub outcome: &'static str,
}
impl Default for FileOps {
    fn default() -> Self {
        Self {
            open: false,
            purpose: Purpose::Open,
            path: String::new(),
            directory: String::new(),
            guard: None,
            rows: vec![],
            listing: None,
            read: None,
            note: String::new(),
            outcome: "idle",
        }
    }
}
impl FileOps {
    pub fn pending(&self) -> bool {
        self.read.is_some()
    }
}
impl Workspace {
    fn file_guard(&self) -> Guard {
        Guard {
            native: self.session.editor.freshness(),
            views: self
                .views
                .views
                .document()
                .digest()
                .unwrap_or_else(|e| format!("invalid:{e}")),
            selected: self.session.editor.selected,
            save_path: self.save_path.clone(),
        }
    }
    pub(super) fn open_file_picker(&mut self) {
        if self.files.pending() || self.dock_blocked() {
            self.result(Err("Finish the pending operation or close the auxiliary editor before choosing a file.".into()));
            return;
        }
        self.files.open = true;
        self.files.purpose = Purpose::Open;
        self.files.path = self.path.clone();
        self.files.guard = Some(self.file_guard());
        self.files.outcome = "choosing";
        self.files.note =
            "Choose a file or enter an explicit destination. Cancel keeps the current work.".into();
        self.files.directory = Path::new(self.path.trim())
            .parent()
            .filter(|p| p.is_absolute())
            .unwrap_or(Path::new("/workspace"))
            .display()
            .to_string();
        self.list_files();
    }
    fn list_files(&mut self) {
        if self.files.listing.is_some() {
            self.files.note = "Directory read pending; retry Browse after it completes.".into();
            return;
        }
        let directory = self.files.directory.clone();
        let path = PathBuf::from(&directory);
        let (tx, rx) = mpsc::channel();
        let ctx = self.context.clone();
        let start=std::thread::Builder::new().name("mbse-file-list".into()).spawn(move||{
   let result=(||{let entries=std::fs::read_dir(&path).map_err(|e|e.to_string())?;let mut rows=vec![];for entry in entries.take(501){let entry=entry.map_err(|e|e.to_string())?;if rows.len()==500{return Err("Directory exceeds500 entries; enter a narrower directory or explicit file path".into())}let p=entry.path();if p.to_str().is_none(){continue}let is_dir=entry.file_type().map_err(|e|e.to_string())?.is_dir();rows.push((p,is_dir));}rows.sort_by(|a,b|b.1.cmp(&a.1).then(a.0.cmp(&b.0)));Ok(rows)})();let _=tx.send(result);ctx.request_repaint();
  });
        match start {
            Ok(_) => {
                self.files.listing = Some(Listing {
                    directory,
                    reply: rx,
                });
                self.files.rows.clear();
                self.files.note = "Reading directory…".into();
            }
            Err(e) => {
                self.files.outcome = "failed";
                self.files.note = format!("Directory worker failed: {e}");
            }
        }
    }
    fn resolve_picker(&mut self, outcome: Outcome<PathBuf>) {
        self.files.open = false;
        match outcome {
            Outcome::Cancelled => {
                self.files.outcome = "cancelled";
                self.files.note =
                    "File selection cancelled explicitly; current work retained.".into();
            }
            Outcome::Indeterminate => {
                self.files.outcome = "indeterminate";
                self.files.note =
                    "No selection and no cancellation evidence; current work retained.".into();
            }
            Outcome::Failed(e) => {
                self.files.outcome = "failed";
                self.files.note = format!("File dialog failed: {e}; current work retained.");
            }
            Outcome::Selected(path) => {
                if self.files.guard.as_ref() != Some(&self.file_guard()) || self.dock_blocked() {
                    self.files.outcome = "stale";
                    self.files.note="File selection is stale; newer values, drafts and views retained. Choose again.".into();
                    self.result(Err(self.files.note.clone()));
                    return;
                }
                if path.as_os_str().is_empty() {
                    self.resolve_picker(Outcome::Indeterminate);
                    return;
                }
                self.files.outcome = "selected";
                match self.files.purpose {
                    Purpose::Open => self.request_load(Load::Open(path)),
                    Purpose::Recover => self.request_load(Load::Recover(path)),
                    Purpose::Import => self.request_load(Load::Import(path)),
                    Purpose::Save => {
                        self.save_path = path.display().to_string();
                        self.save();
                    }
                }
            }
        }
        if matches!(self.files.outcome, "cancelled" | "indeterminate" | "failed") {
            let text = self.files.note.clone();
            self.result(if self.files.outcome == "cancelled" {
                Ok(text)
            } else {
                Err(text)
            });
        }
    }
    pub(super) fn start_background_load(&mut self, action: Load) {
        if self.files.read.is_some() {
            self.result(Err(
                "A model read is already pending; current work retained.".into(),
            ));
            return;
        }
        if matches!(action, Load::Quit) {
            return;
        }
        let guard = self.file_guard();
        let root = self.session.state_root.clone();
        let (tx, rx) = mpsc::channel();
        let ctx = self.context.clone();
        let start = std::thread::Builder::new()
            .name("mbse-model-read".into())
            .spawn(move || {
                let result = match action {
                    Load::New => Session::new(&root),
                    Load::Open(p) => Session::open(&root, &p),
                    Load::Recover(p) => Session::restore(&root, &p),
                    Load::Import(p) => Session::import_sysml(&root, &p),
                    Load::Quit => unreachable!(),
                };
                let _ = tx.send(result);
                ctx.request_repaint();
            });
        match start {
            Ok(_) => {
                self.files.read = Some(ReadPending {
                    guard,
                    reply: rx,
                    cancelled: false,
                });
                self.files.outcome = "pending";
                self.files.note =
                    "Reading model in background… No current work has been replaced.".into();
                self.result(Ok(self.files.note.clone()));
            }
            Err(e) => {
                self.files.outcome = "failed";
                self.result(Err(format!(
                    "Model worker did not start: {e}; work retained."
                )));
            }
        }
    }
    pub(super) fn poll_file_ops(&mut self) {
        if let Some(listing) = self.files.listing.take() {
            match listing.reply.try_recv() {
                Ok(result) => {
                    if self.files.open && self.files.directory == listing.directory {
                        match result {
                            Ok(rows) => {
                                self.files.rows = rows;
                                self.files.note =
                                    "Directory ready; choose a file then Use path.".into()
                            }
                            Err(e) => {
                                self.files.outcome = "failed";
                                self.files.note =
                                    format!("Directory failed: {e}; current work retained.");
                            }
                        }
                    }
                }
                Err(TryRecvError::Empty) => self.files.listing = Some(listing),
                Err(TryRecvError::Disconnected) => {
                    self.files.outcome = "failed";
                    self.files.note =
                        "Directory worker stopped without a reply; current work retained.".into();
                }
            }
        }
        // eframe calls update before ui: let queued editing input reach the current
        // editor before deciding that an already-ready candidate is still current.
        if self.files.pending()
            && self.context.input(|i| {
                i.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Text(_)
                            | egui::Event::Paste(_)
                            | egui::Event::Cut
                            | egui::Event::Key { pressed: true, .. }
                            | egui::Event::PointerButton { pressed: true, .. }
                            | egui::Event::MouseWheel { .. }
                    )
                })
            })
        {
            return;
        }
        let Some(pending) = self.files.read.take() else {
            return;
        };
        match pending.reply.try_recv() {
            Err(TryRecvError::Empty) => self.files.read = Some(pending),
            Err(TryRecvError::Disconnected) => {
                self.files.outcome = "failed";
                self.result(Err(
                    "Model worker stopped without a reply; current work retained.".into(),
                ));
            }
            Ok(result) => {
                if pending.cancelled {
                    self.files.outcome = "cancelled";
                    self.files.note="Model read cancelled explicitly; completed candidate discarded, current work retained.".into();
                    self.result(Ok(self.files.note.clone()));
                } else if pending.guard != self.file_guard()
                    || self.pending.is_some()
                    || self.views.occurrence_navigation_blocked(&self.session)
                {
                    self.files.outcome = "stale";
                    self.files.note="Completed read is stale; newer model, text or views retained. Open again explicitly.".into();
                    self.result(Err(self.files.note.clone()));
                } else {
                    match result {
                        Ok(session) => {
                            self.install_loaded(session);
                            self.files.outcome = "completed";
                            self.files.note =
                                "Model read completed and current result accepted.".into();
                        }
                        Err(e) => {
                            self.files.outcome = "failed";
                            self.files.note =
                                format!("Model read failed: {e}; current work retained.");
                            self.result(Err(self.files.note.clone()));
                        }
                    }
                }
            }
        }
    }
    pub(super) fn file_operations_ui(&mut self, ctx: &egui::Context) {
        if self.files.open {
            let mut open = true;
            let mut browse = false;
            let mut up = false;
            let mut chosen = None;
            let mut selected = None;
            let mut cancel = false;
            let mut accept = false;
            egui::Window::new("Choose model file")
                .id(egui::Id::new("mbse-file-picker"))
                .open(&mut open)
                .default_width(730.)
                .show(ctx, |ui| {
                    ui.label("Application file chooser");
                    ui.horizontal_wrapped(|ui| {
                        for purpose in [
                            Purpose::Open,
                            Purpose::Recover,
                            Purpose::Import,
                            Purpose::Save,
                        ] {
                            let r = ui.selectable_value(
                                &mut self.files.purpose,
                                purpose,
                                purpose.label(),
                            );
                            self.bound(format!("files:purpose:{purpose:?}"), &r);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Directory");
                        let r = ui.text_edit_singleline(&mut self.files.directory);
                        self.bound("files:directory", &r);
                        let r = ui.button("Browse");
                        self.bound("files:browse", &r);
                        browse = r.clicked();
                        let r = ui.button("Parent");
                        self.bound("files:parent", &r);
                        up = r.clicked();
                    });
                    if self.files.listing.is_some() {
                        ui.spinner();
                        ui.label("Reading directory…");
                    }
                    let rows = self.files.rows.clone();
                    egui::ScrollArea::vertical()
                        .id_salt("file-picker-rows")
                        .max_height(330.)
                        .show(ui, |ui| {
                            for (index, (path, dir)) in rows.into_iter().enumerate() {
                                let label = format!(
                                    "{} {}",
                                    if dir { "Folder" } else { "File" },
                                    path.file_name().unwrap_or_default().to_string_lossy()
                                );
                                let r = ui.selectable_label(false, label);
                                self.bound(format!("files:entry:{index}"), &r);
                                if r.clicked() {
                                    if dir {
                                        chosen = Some(path)
                                    } else {
                                        selected = Some(path)
                                    }
                                }
                            }
                        });
                    ui.label(self.files.purpose.label());
                    let r = ui.text_edit_singleline(&mut self.files.path);
                    self.bound("files:path", &r);
                    ui.label(&self.files.note);
                    ui.horizontal(|ui| {
                        let r = ui.button("Use path");
                        self.bound("files:accept", &r);
                        accept = r.clicked();
                        let r = ui.button("Cancel");
                        self.bound("files:cancel", &r);
                        cancel = r.clicked();
                    });
                });
            if !open || cancel {
                self.resolve_picker(Outcome::Cancelled);
            } else if let Some(path) = chosen {
                self.files.directory = path.display().to_string();
                self.list_files();
            } else {
                if up {
                    self.files.directory = Path::new(&self.files.directory)
                        .parent()
                        .unwrap_or(Path::new("/workspace"))
                        .display()
                        .to_string();
                    browse = true;
                }
                if browse {
                    self.list_files();
                }
                if let Some(path) = selected {
                    self.files.path = path.display().to_string();
                }
                if accept {
                    self.resolve_picker(Outcome::Selected(PathBuf::from(self.files.path.trim())));
                }
            }
        }
        if self.files.pending() {
            let mut cancel = false;
            egui::Window::new("Reading model").id(egui::Id::new("mbse-read-progress")).collapsible(false).show(ctx,|ui|{ui.spinner();ui.label(&self.files.note);let r=ui.button("Cancel read");self.bound("files:cancel-read",&r);cancel=r.clicked();ui.label("Cancellation prevents publication; the bounded read may finish before its worker returns.");});
            if cancel {
                self.cancel_model_read();
            }
        }
    }
    pub(super) fn cancel_model_read(&mut self) {
        if let Some(read) = self.files.read.as_mut() {
            read.cancelled = true;
            self.files.note =
                "Cancellation requested; waiting for reader to return. Current work retained."
                    .into();
        }
    }
    pub(super) fn file_audit(&self) -> serde_json::Value {
        serde_json::json!({"schema":"mbse-craft.file-operations/1","open":self.files.open,"pending":self.files.pending(),"listing_pending":self.files.listing.is_some(),"outcome":self.files.outcome,"note":self.files.note,"backend":"application-owned chooser; no OS-picker claim"})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> (tempfile::TempDir, Workspace) {
        let area = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        let mut session = Session::new(area.path()).unwrap();
        let snapshot = mbse_model::stage1::Snapshot::from_fixture(
            session.editor.snapshot.document_id.clone(),
            &serde_json::from_str(include_str!(
                "../../mbse-model/fixtures/stage1/inspection.json"
            ))
            .unwrap(),
        )
        .unwrap();
        session.editor =
            mbse_model::stage1::Editor::new(snapshot, session.editor.freshness().instance_id)
                .unwrap();
        session.editor.select(RecordId(301)).unwrap();
        let ctx = egui::Context::default();
        let a = Workspace::new(
            session,
            None,
            None,
            craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),
            &ctx,
        );
        (area, a)
    }
    fn state(a: &Workspace) -> (String, String, Option<RecordId>, String) {
        (
            a.session.editor.snapshot.digest(),
            a.views.views.document().digest().unwrap(),
            a.session.editor.selected,
            a.save_path.clone(),
        )
    }
    fn pending(a: &mut Workspace) -> mpsc::Sender<Result<Session, String>> {
        let (tx, rx) = mpsc::channel();
        a.files.read = Some(ReadPending {
            guard: a.file_guard(),
            reply: rx,
            cancelled: false,
        });
        tx
    }
    fn wait(a: &mut Workspace) {
        let end = Instant::now() + Duration::from_secs(8);
        while a.files.pending() && Instant::now() < end {
            a.poll_file_ops();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!a.files.pending());
    }
    #[test]
    fn explicit_cancel_backend_failure_and_unknown_empty_results_are_distinct_and_preserving() {
        let (_area, mut a) = app();
        let before = state(&a);
        for (outcome, label) in [
            (Outcome::Cancelled, "cancelled"),
            (Outcome::failed("backend diagnostic"), "failed"),
            (Outcome::Indeterminate, "indeterminate"),
        ] {
            a.files.open = true;
            a.resolve_picker(outcome);
            assert_eq!(a.files.outcome, label);
            assert!(!a.files.open);
            assert_eq!(state(&a), before);
            assert!(a.feedback.contains("retained"));
        }
        a.files.guard = Some(a.file_guard());
        a.resolve_picker(Outcome::Selected(PathBuf::new()));
        assert_eq!(a.files.outcome, "indeterminate");
        assert_eq!(state(&a), before);
    }
    #[test]
    fn stale_selection_cannot_open_or_save_over_new_native_drafts() {
        let (_area, mut a) = app();
        a.files.guard = Some(a.file_guard());
        a.files.purpose = Purpose::Save;
        a.command(Command::EditText {
            key: RawKey {
                record: RecordId(301),
                field: "criterion".into(),
                item: None,
            },
            text: "9e".into(),
        });
        let before = state(&a);
        a.resolve_picker(Outcome::Selected(PathBuf::from(
            "/workspace/must-not-be-written.kernspan",
        )));
        assert_eq!(a.files.outcome, "stale");
        assert_eq!(state(&a), before);
        assert!(!a.session.pending());
    }
    #[test]
    fn controlled_background_completion_rejects_newer_native_draft_and_views_without_discard() {
        for native in [true, false] {
            let (area, mut a) = app();
            let tx = pending(&mut a);
            if native {
                a.command(Command::EditText {
                    key: RawKey {
                        record: RecordId(301),
                        field: "criterion".into(),
                        item: None,
                    },
                    text: "9e".into(),
                });
            } else {
                a.views.apply(
                    &a.session.editor,
                    mbse_model::native_views::Layout::AddView("Newer view".into()),
                );
            }
            let before = state(&a);
            tx.send(Ok(Session::new(area.path()).unwrap()))
                .ok()
                .unwrap();
            a.poll_file_ops();
            assert_eq!(a.files.outcome, "stale");
            assert_eq!(state(&a), before);
            if native {
                assert_eq!(
                    a.session.editor.display_text(&RawKey {
                        record: RecordId(301),
                        field: "criterion".into(),
                        item: None
                    }),
                    "9e"
                );
            }
        }
    }
    #[test]
    fn cancel_pending_read_does_not_release_lane_early_or_publish_late_candidate() {
        let (area, mut a) = app();
        let tx = pending(&mut a);
        let before = state(&a);
        a.cancel_model_read();
        assert!(a.files.pending());
        a.start_background_load(Load::New);
        assert!(a.files.pending());
        tx.send(Ok(Session::new(area.path()).unwrap()))
            .ok()
            .unwrap();
        a.poll_file_ops();
        assert_eq!(a.files.outcome, "cancelled");
        assert_eq!(state(&a), before);
    }
    #[test]
    fn worker_failure_disconnect_and_current_success_have_owned_completion_paths() {
        let (area, mut a) = app();
        let before = state(&a);
        let tx = pending(&mut a);
        tx.send(Err("owned read failure".into())).ok().unwrap();
        a.poll_file_ops();
        assert_eq!(a.files.outcome, "failed");
        assert_eq!(state(&a), before);
        let tx = pending(&mut a);
        drop(tx);
        a.poll_file_ops();
        assert_eq!(a.files.outcome, "failed");
        assert_eq!(state(&a), before);
        let tx = pending(&mut a);
        let candidate = Session::new(area.path()).unwrap();
        let new = candidate.editor.snapshot.digest();
        tx.send(Ok(candidate)).ok().unwrap();
        a.poll_file_ops();
        assert_eq!(a.files.outcome, "completed");
        assert_eq!(a.session.editor.snapshot.digest(), new);
    }
    #[test]
    fn actual_background_read_failure_and_new_session_success_do_not_block_dispatch() {
        let (area, mut a) = app();
        let before = state(&a);
        a.start_background_load(Load::Open(area.path().join("missing.kernspan")));
        assert!(a.files.pending());
        wait(&mut a);
        assert_eq!(a.files.outcome, "failed");
        assert_eq!(state(&a), before);
        a.start_background_load(Load::New);
        wait(&mut a);
        assert_eq!(a.files.outcome, "completed");
        assert_ne!(state(&a).0, before.0);
    }
    #[test]
    fn stale_directory_reply_cannot_reopen_picker_or_replace_current_rows() {
        let (_area, mut a) = app();
        let (tx, rx) = mpsc::channel();
        a.files.listing = Some(Listing {
            directory: "/workspace/old".into(),
            reply: rx,
        });
        a.files.directory = "/workspace/new".into();
        a.files.open = true;
        tx.send(Ok(vec![(PathBuf::from("/workspace/old/file"), false)]))
            .unwrap();
        a.poll_file_ops();
        assert!(a.files.rows.is_empty());
        a.files.open = false;
        assert!(!a.files.pending());
    }
    #[test]
    fn queued_frame_edit_is_processed_before_a_ready_background_result_can_publish() {
        let (area, mut a) = app();
        let tx = pending(&mut a);
        tx.send(Ok(Session::new(area.path()).unwrap()))
            .ok()
            .unwrap();
        let old = a.session.editor.snapshot.document_id.clone();
        let ctx = a.context.clone();
        ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Text("9e".into())],
                ..Default::default()
            },
            |_| {
                a.poll_file_ops();
                assert!(a.files.pending());
                assert_eq!(a.session.editor.snapshot.document_id, old);
                a.command(Command::EditText {
                    key: RawKey {
                        record: RecordId(301),
                        field: "criterion".into(),
                        item: None,
                    },
                    text: "9e".into(),
                });
            },
        )
        .drop_without_applying_deltas();
        let edited = state(&a);
        ctx.run_ui(egui::RawInput::default(), |_| a.poll_file_ops())
            .drop_without_applying_deltas();
        assert_eq!(a.files.outcome, "stale");
        assert_eq!(state(&a), edited);
        assert_eq!(a.session.editor.snapshot.document_id, old);
    }
}
