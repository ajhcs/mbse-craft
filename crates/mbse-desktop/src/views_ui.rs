//! Native views controller. File dialogs use explicit local paths only.
use eframe::egui;
use mbse_model::{
    native_views::{
        self as v, Geometry, Layout, OccurrenceId, SemanticKey, SemanticRole, ViewId, Views,
    },
    native_views_io::{self as io, Io, Kind, Receipt},
    stage1::{Editor, RecordId},
    stage1_io::{Session, WriteKind},
};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

pub struct ViewWorkspace {
    pub views: Views,
    pub io: Io,
    pub active: ViewId,
    pub selected: Option<OccurrenceId>,
    pub path: String,
    pub search: String,
    pub note: String,
    title: String,
    sender: Sender<Receipt>,
    receiver: Receiver<Receipt>,
    recovery_serial: u64,
    native_capture: Option<v::Capture>,
    drag: Option<(OccurrenceId, Geometry, v::Stamp, egui::Pos2)>,
    route_source: Option<OccurrenceId>,
    route_target: Option<OccurrenceId>,
}
impl ViewWorkspace {
    pub fn new(session: &Session) -> Self {
        let views = Views::new(&session.editor)
            .expect("validated native snapshot has a bounded default presentation");
        let active = views.document().primary;
        let (sender, receiver) = mpsc::channel();
        let path = session
            .path
            .as_ref()
            .map(|p| p.with_extension("mbse-views.json").display().to_string())
            .unwrap_or_default();
        Self {
            views,
            io: Io::default(),
            active,
            selected: None,
            path,
            search: String::new(),
            note: "Views are unsaved. Native model, text and layout histories are separate.".into(),
            title: "New view".into(),
            sender,
            receiver,
            recovery_serial: 0,
            native_capture: None,
            drag: None,
            route_source: None,
            route_target: None,
        }
    }
    pub fn blocked(&self, session: &Session) -> bool {
        self.io.pending() || !self.io.is_saved(session, &self.views)
    }
    pub fn preserved(&self, session: &Session, kind: &WriteKind) -> bool {
        !self.io.pending()
            && match kind {
                WriteKind::Save => self.io.is_saved(session, &self.views),
                WriteKind::Recovery => self.io.recovery_current(session, &self.views),
            }
    }
    pub fn verify_preserved(&self, session: &Session, kind: &WriteKind) -> Result<(), String> {
        self.io.verify_preserved(
            session,
            &self.views,
            if *kind == WriteKind::Save {
                Kind::Bound
            } else {
                Kind::Recovery
            },
        )
    }
    pub fn poll(&mut self, session: &Session) {
        while let Ok(receipt) = self.receiver.try_recv() {
            let consumed = self.io.complete(session, &self.views, receipt);
            self.note = if !consumed {
                "Stale views acknowledgment refused; current work retained.".into()
            } else if let Some(error) = &self.io.last_error {
                format!("Native and views preservation are independent. Views failed: {error}. Geometry retained.")
            } else if self.io.is_saved(session, &self.views) {
                "Native saved · views saved. Separate acknowledgments verified.".into()
            } else if self.io.recovery_current(session, &self.views) {
                "Unbound views recovery verified; native Save status is separate.".into()
            } else {
                "An older layout was written; newer layout remains unsaved.".into()
            };
        }
    }
    pub fn capture_native(&mut self, session: &Session) {
        self.native_capture = Some(self.views.capture(&session.editor));
    }
    pub fn cancel_native_capture(&mut self) {
        self.native_capture = None;
    }
    /// Called only after the native controller verified a current completion.
    pub fn after_native(&mut self, session: &Session, kind: &WriteKind) -> Result<(), String> {
        if self.io.pending() {
            return Err("Views write pending; repeat preservation after acknowledgment.".into());
        }
        let (path, kind) = match kind {
            WriteKind::Save => {
                io::accepted_native(session)?;
                if self.path.trim().is_empty() {
                    self.path = session
                        .path
                        .as_ref()
                        .ok_or("Native path missing")?
                        .with_extension("mbse-views.json")
                        .display()
                        .to_string();
                }
                (PathBuf::from(self.path.trim()), Kind::Bound)
            }
            WriteKind::Recovery => {
                self.recovery_serial = self
                    .recovery_serial
                    .checked_add(1)
                    .ok_or("Recovery ID exhausted")?;
                (
                    session.state_root.join(format!(
                        "views-recovery-{}-{}.mbse-views.json",
                        session.session_id().0.replace(':', "-"),
                        self.recovery_serial
                    )),
                    Kind::Recovery,
                )
            }
        };
        let capture = self
            .native_capture
            .take()
            .ok_or("Missing frozen native/views request pair")?;
        let job = self.io.begin_captured(session, capture, &path, kind)?;
        self.start_job_with(session, job, |work| {
            std::thread::Builder::new()
                .name("mbse-views-write".into())
                .spawn(work)
                .map(|_| ())
        })?;
        self.note = format!(
            "Native preservation acknowledged. Views write pending: {}",
            path.display()
        );
        Ok(())
    }
    fn start_job_with(
        &mut self,
        session: &Session,
        job: io::Job,
        start: impl FnOnce(Box<dyn FnOnce() + Send>) -> Result<(), std::io::Error>,
    ) -> Result<(), String> {
        let token = job.start_token();
        let sender = self.sender.clone();
        if let Err(error) = start(Box::new(move || {
            let _ = sender.send(io::execute(job));
        })) {
            let message = error.to_string();
            self.io.failed_start(session, token, message.clone());
            self.note = format!("Native preservation succeeded; views worker did not start: {message}. Geometry retained.");
            return Err(self.note.clone());
        }
        Ok(())
    }
    pub fn apply(&mut self, editor: &Editor, action: Layout) {
        let stamp = self.views.stamp(editor);
        self.note = match self.views.apply(editor, &stamp, action) {
            Ok(()) => {
                "Layout updated; semantic identity unchanged. Layout Undo is separate.".into()
            }
            Err(e) => e,
        };
        self.reconcile();
    }
    /// Ephemeral selection/route state never survives an invalidated identity.
    pub fn reconcile(&mut self) {
        let doc = self.views.document();
        if !doc.views.contains_key(&self.active) {
            self.active = doc.primary;
        }
        if self.selected.is_some_and(|id| {
            !doc.occurrences
                .get(&id)
                .is_some_and(|o| o.view == self.active)
        }) {
            self.selected = None;
        }
        let valid = |id: OccurrenceId| {
            doc.occurrences
                .get(&id)
                .is_some_and(|o| o.view == self.active && o.presence.active())
        };
        if self.route_source.is_some_and(|id| !valid(id)) {
            self.route_source = None;
        }
        if self.route_target.is_some_and(|id| !valid(id)) {
            self.route_target = None;
        }
        if self.drag.as_ref().is_some_and(|(id, _, _, _)| !valid(*id)) {
            self.drag = None;
        }
    }
    fn layout_history(&mut self, editor: &Editor, redo: bool) {
        let stamp = self.views.stamp(editor);
        self.note = format!("{:?}", self.views.undo(editor, &stamp, redo));
        self.reconcile();
    }
    /// Returns a selected native ID; selection itself is not a semantic edit.
    pub fn toolbar(&mut self, ui: &mut egui::Ui, session: &Session) -> Option<RecordId> {
        let mut select = None;
        let tabs: Vec<_> = self
            .views
            .document()
            .views
            .values()
            .map(|v| (v.id, v.title.clone()))
            .collect();
        ui.horizontal_wrapped(|ui| {
            for (id, title) in tabs {
                if ui.selectable_label(self.active == id, title).clicked() {
                    self.active = id;
                    self.selected = None;
                }
            }
            if ui.button("Layout Undo").clicked() {
                self.layout_history(&session.editor, false);
            }
            if ui.button("Layout Redo").clicked() {
                self.layout_history(&session.editor, true);
            }
        });
        self.reconcile();
        ui.horizontal_wrapped(|ui| {
            ui.label("View title");
            ui.text_edit_singleline(&mut self.title);
            if ui.button("New view").clicked() {
                self.apply(&session.editor, Layout::AddView(self.title.clone()));
            }
            if ui.button("Rename view").clicked() {
                self.apply(
                    &session.editor,
                    Layout::RenameView(self.active, self.title.clone()),
                );
            }
            if ui
                .add_enabled(
                    self.active != self.views.document().primary,
                    egui::Button::new("Close view"),
                )
                .clicked()
            {
                self.apply(&session.editor, Layout::CloseView(self.active));
            }
            if ui.button("Fit").clicked() {
                let items: Vec<_> = self
                    .views
                    .document()
                    .occurrences
                    .values()
                    .filter(|o| o.view == self.active && o.visible && o.presence.active())
                    .collect();
                let x = items.iter().map(|o| o.geometry.x).fold(0., f64::min);
                let y = items.iter().map(|o| o.geometry.y).fold(0., f64::min);
                let right = items
                    .iter()
                    .map(|o| o.geometry.x + o.geometry.width)
                    .fold(800., f64::max);
                let bottom = items
                    .iter()
                    .map(|o| o.geometry.y + o.geometry.height)
                    .fold(400., f64::max);
                let zoom = ((ui.available_width().max(300.) as f64 / (right - x + 50.))
                    .min(400. / (bottom - y + 50.)))
                .clamp(0.05, 20.);
                self.apply(
                    &session.editor,
                    Layout::Camera(self.active, v::Camera { x, y, zoom }),
                );
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Explicit views path");
            ui.text_edit_singleline(&mut self.path);
            let may_open = !self.io.pending()
                && (self.views.document().revision() == 1
                    || self.io.is_saved(session, &self.views));
            if ui
                .add_enabled(may_open, egui::Button::new("Open views"))
                .clicked()
            {
                self.open(session, false);
            }
            if ui
                .add_enabled(may_open, egui::Button::new("Open unbound views recovery"))
                .clicked()
            {
                self.open(session, true);
            }
        });
        ui.label(&self.note);
        ui.horizontal_wrapped(|ui| {
            ui.label("Find / reuse");
            ui.text_edit_singleline(&mut self.search);
        });
        let matches: Vec<_> = session
            .editor
            .snapshot
            .records
            .values()
            .filter(|r| {
                !self.search.is_empty()
                    && (r.name.to_lowercase().contains(&self.search.to_lowercase())
                        || r.id.0.to_string() == self.search)
            })
            .take(12)
            .map(|r| (r.id, r.name.clone()))
            .collect();
        for (id, name) in matches {
            ui.horizontal(|ui| {
                if ui.button(format!("Select {name} #{}", id.0)).clicked() {
                    select = Some(id);
                }
                if ui.button("Reuse in this view").clicked() {
                    self.reuse(&session.editor, id);
                }
            });
        }
        if let Some(id) = session.editor.selected {
            if ui
                .button(format!("Reuse selected #{} in this view", id.0))
                .clicked()
            {
                self.reuse(&session.editor, id);
            }
        }
        if let Some(o) = self
            .selected
            .and_then(|id| self.views.document().occurrences.get(&id))
            .cloned()
        {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "Occurrence {} → native #{}",
                    o.id.get(),
                    o.record.0
                ));
                if ui
                    .button(if o.visible {
                        "Hide occurrence"
                    } else {
                        "Show occurrence"
                    })
                    .clicked()
                {
                    self.apply(&session.editor, Layout::Visible(o.id, !o.visible));
                }
                if ui.button(if o.pinned { "Unpin" } else { "Pin" }).clicked() {
                    self.apply(&session.editor, Layout::Pin(o.id, !o.pinned));
                }
                if ui.button("Move right").clicked() {
                    let mut geometry = o.geometry.clone();
                    geometry.x += 30.;
                    self.apply(&session.editor, Layout::Move(o.id, geometry));
                }
                if ui.button("Remove occurrence").clicked() {
                    self.apply(&session.editor, Layout::Remove(o.id));
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.selected.is_some(),
                    egui::Button::new("Use occurrence as edge source"),
                )
                .clicked()
            {
                self.route_source = self.selected;
            }
            if ui
                .add_enabled(
                    self.selected.is_some(),
                    egui::Button::new("Use occurrence as edge target"),
                )
                .clicked()
            {
                self.route_target = self.selected;
            }
            ui.label(format!(
                "Explicit endpoints {:?} → {:?}",
                self.route_source.map(|id| id.get()),
                self.route_target.map(|id| id.get())
            ));
        });
        if let (Some(source), Some(target)) = (self.route_source, self.route_target) {
            ui.horizontal_wrapped(|ui| {
                for (label, role) in [
                    (
                        "Attach contributes-to",
                        SemanticRole::CapabilityContributesToMission,
                    ),
                    ("Attach subject", SemanticRole::ObjectiveSubject),
                    ("Attach for-objective", SemanticRole::MeasureForObjective),
                ] {
                    if ui.button(label).clicked() {
                        self.route(&session.editor, source, target, role);
                    }
                }
            });
        }
        egui::CollapsingHeader::new("Occurrences and usages · includes hidden and detached").show(
            ui,
            |ui| {
                let list: Vec<_> = self
                    .views
                    .document()
                    .occurrences
                    .values()
                    .cloned()
                    .collect();
                for o in list {
                    if ui
                        .selectable_label(
                            self.selected == Some(o.id),
                            format!(
                                "View {} · occurrence {} · #{} · {} · {:?}",
                                o.view.get(),
                                o.id.get(),
                                o.record.0,
                                if o.visible { "shown" } else { "hidden" },
                                o.presence
                            ),
                        )
                        .clicked()
                    {
                        self.active = o.view;
                        self.selected = Some(o.id);
                        if o.presence.active() {
                            select = Some(o.record);
                        }
                    }
                }
            },
        );
        self.reconcile();
        select
    }
    fn reuse(&mut self, editor: &Editor, record: RecordId) {
        let n = self
            .views
            .document()
            .occurrences
            .values()
            .filter(|o| o.view == self.active)
            .count();
        self.apply(
            editor,
            Layout::Reuse {
                view: self.active,
                record,
                geometry: Geometry::at(25. + (n % 3) as f64 * 294., 24. + (n / 3) as f64 * 155.),
            },
        );
    }
    fn open(&mut self, session: &Session, recovery: bool) {
        self.note = match self.io.open(
            session,
            &mut self.views,
            &PathBuf::from(self.path.trim()),
            recovery,
        ) {
            Ok(()) => {
                self.active = self.views.document().primary;
                self.selected = None;
                "Views opened with checked identity. Detached runtime history authority was not restored.".into()
            }
            Err(e) => format!("Views input retained read-only; current canvas retained: {e}"),
        };
        self.reconcile();
    }
    pub fn drag(
        &mut self,
        editor: &Editor,
        id: OccurrenceId,
        response: &egui::Response,
        scale: f32,
    ) {
        if response.drag_started() {
            if let Some(o) = self
                .views
                .document()
                .occurrences
                .get(&id)
                .filter(|o| !o.pinned)
            {
                if let Some(pointer) = response.interact_pointer_pos() {
                    self.drag = Some((id, o.geometry.clone(), self.views.stamp(editor), pointer));
                }
            }
        }
        if response.drag_stopped() {
            if let Some((owner, mut geometry, stamp, start)) = self.drag.take() {
                if owner == id {
                    let delta = (response
                        .ctx
                        .input(|i| i.pointer.latest_pos())
                        .unwrap_or(start)
                        - start)
                        / scale;
                    geometry.x += delta.x as f64;
                    geometry.y += delta.y as f64;
                    self.note = format!(
                        "{:?}",
                        self.views.apply(editor, &stamp, Layout::Move(id, geometry))
                    );
                }
            }
        }
    }
    /// Persist an explicit occurrence pair only for one of the three supported
    /// authored keys. Duplicate occurrences never silently change endpoints.
    pub fn route(
        &mut self,
        editor: &Editor,
        source: OccurrenceId,
        target: OccurrenceId,
        role: SemanticRole,
    ) {
        let result: Result<Layout, String> = (|| {
            let a = self
                .views
                .document()
                .occurrences
                .get(&source)
                .ok_or("Source occurrence missing")?;
            let b = self
                .views
                .document()
                .occurrences
                .get(&target)
                .ok_or("Target occurrence missing")?;
            let key = SemanticKey::checked(&editor.snapshot, a.record, role, b.record)?;
            Ok(Layout::Route {
                view: self.active,
                key,
                source,
                target,
            })
        })();
        match result {
            Ok(action) => self.apply(editor, action),
            Err(e) => self.note = e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    fn session() -> (tempfile::TempDir, Session) {
        let root = std::env::var_os("TMPDIR").expect("Controller tests require workspace TMPDIR");
        assert!(Path::new(&root).starts_with("/workspace") || cfg!(windows));
        let area = tempfile::Builder::new()
            .prefix("views-controller-")
            .tempdir_in(root)
            .unwrap();
        let session = Session::new(area.path()).unwrap();
        (area, session)
    }
    #[test]
    fn undo_selected_new_view_reconciles_to_primary_without_panicking() {
        let (_area, session) = session();
        let mut ui = ViewWorkspace::new(&session);
        ui.apply(&session.editor, Layout::AddView("Third".into()));
        ui.active = *ui.views.document().views.keys().last().unwrap();
        ui.reuse(&session.editor, RecordId(1));
        ui.selected = ui.views.document().occurrences.keys().last().copied();
        ui.route_source = ui.selected;
        ui.layout_history(&session.editor, false);
        ui.layout_history(&session.editor, false);
        assert_eq!(ui.active, ui.views.document().primary);
        assert!(ui.selected.is_none());
        assert!(ui.route_source.is_none());
        assert!(ui.views.document().views.get(&ui.active).is_some());
    }
    #[test]
    fn redo_close_selected_view_reconciles_and_keeps_primary() {
        let (_area, session) = session();
        let mut ui = ViewWorkspace::new(&session);
        ui.active = *ui.views.document().views.keys().last().unwrap();
        let closed = ui.active;
        ui.apply(&session.editor, Layout::CloseView(closed));
        ui.layout_history(&session.editor, false);
        ui.active = closed;
        ui.selected = ui
            .views
            .document()
            .occurrences
            .values()
            .find(|o| o.view == closed)
            .map(|o| o.id);
        ui.route_target = ui.selected;
        ui.layout_history(&session.editor, true);
        assert_eq!(ui.active, ui.views.document().primary);
        assert!(ui.selected.is_none());
        assert!(ui.route_target.is_none());
    }
    #[test]
    fn native_success_alone_and_stale_layout_ack_cannot_authorize_continuation() {
        let (area, mut session) = session();
        let mut ui = ViewWorkspace::new(&session);
        let job = session
            .begin_save(&area.path().join("model.kernspan"))
            .unwrap();
        assert!(session.complete(mbse_model::stage1_io::execute_write(job)));
        assert!(!ui.preserved(&session, &WriteKind::Save));
        let job = ui
            .io
            .begin(
                &session,
                &ui.views,
                &area.path().join("layout.mbse-views.json"),
                Kind::Bound,
            )
            .unwrap();
        let receipt = io::execute(job);
        let id = *ui.views.document().occurrences.keys().next().unwrap();
        ui.apply(&session.editor, Layout::Move(id, Geometry::at(150., 160.)));
        ui.sender
            .send(receipt)
            .unwrap_or_else(|_| panic!("controller receiver"));
        ui.poll(&session);
        assert!(ui.blocked(&session));
        assert!(!ui.preserved(&session, &WriteKind::Save));
        assert!(ui.verify_preserved(&session, &WriteKind::Save).is_err());
    }
    #[test]
    fn tampered_recovery_hint_cannot_authorize_navigation() {
        let (area, session) = session();
        let mut ui = ViewWorkspace::new(&session);
        let path = area.path().join("recovery.mbse-views.json");
        let job = ui
            .io
            .begin(&session, &ui.views, &path, Kind::Recovery)
            .unwrap();
        let receipt = io::execute(job);
        ui.sender
            .send(receipt)
            .unwrap_or_else(|_| panic!("controller receiver"));
        ui.poll(&session);
        assert!(ui.preserved(&session, &WriteKind::Recovery));
        ui.verify_preserved(&session, &WriteKind::Recovery).unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(ui.verify_preserved(&session, &WriteKind::Recovery).is_err());
    }
    #[test]
    fn injected_worker_start_failure_clears_only_owned_pending_and_keeps_geometry() {
        let (area, mut session) = session();
        let mut ui = ViewWorkspace::new(&session);
        let native = session
            .begin_save(&area.path().join("model.kernspan"))
            .unwrap();
        assert!(session.complete(mbse_model::stage1_io::execute_write(native)));
        let before = ui.views.document().digest().unwrap();
        let path = area.path().join("layout.mbse-views.json");
        let job = ui
            .io
            .begin(&session, &ui.views, &path, Kind::Bound)
            .unwrap();
        let result = ui.start_job_with(&session, job, |_work| {
            Err(std::io::Error::other("injected start refusal"))
        });
        assert!(result.is_err());
        assert!(!ui.io.pending());
        assert!(!session.is_dirty());
        assert!(!path.exists());
        assert_eq!(ui.views.document().digest().unwrap(), before);
        assert!(ui.blocked(&session));
    }
}
