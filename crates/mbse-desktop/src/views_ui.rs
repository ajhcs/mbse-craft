//! Native views controller. File dialogs use explicit local paths only.
use eframe::egui;
use mbse_model::{
    native_views::{
        self as v, Geometry, Layout, OccurrenceId, SemanticKey, SemanticRole, ViewId, Views,
    },
    native_views_io::{self as io, Io, Kind, Receipt},
    stage1::{Editor, RecordId},
    stage1_io::{IoTicket, Session, WriteKind},
};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

pub struct PreparedPair {
    capture: v::Capture,
    destination: io::Destination,
    kind: WriteKind,
    native_stamp: mbse_model::stage1::Freshness,
    snapshot_hash: String,
}
struct NativePair {
    ticket: IoTicket,
    prepared: PreparedPair,
}
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
    native_capture: Option<NativePair>,
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
    pub fn layout_gesture_active(&self) -> bool {
        self.drag.is_some()
    }
    pub fn pair_pending(&self) -> bool {
        self.native_capture.is_some()
    }
    pub fn prepare_native(
        &mut self,
        session: &Session,
        kind: WriteKind,
        native_path: Option<&std::path::Path>,
    ) -> Result<PreparedPair, String> {
        if self.pair_pending() || self.io.pending() || session.pending() {
            return Err("Preservation already pending; current layout retained".into());
        }
        let (path, view_kind) = match kind {
            WriteKind::Save => {
                let path = if self.path.trim().is_empty() {
                    native_path
                        .ok_or("Native save path required")?
                        .with_extension("mbse-views.json")
                } else {
                    PathBuf::from(self.path.trim())
                };
                (path, Kind::Bound)
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
        let destination = self
            .io
            .freeze_destination(session, &self.views, &path, view_kind)?;
        Ok(PreparedPair {
            capture: self.views.capture(&session.editor),
            destination,
            kind,
            native_stamp: session.editor.freshness(),
            snapshot_hash: session.editor.snapshot.digest(),
        })
    }
    pub fn capture_native(
        &mut self,
        session: &Session,
        ticket: &IoTicket,
        kind: &WriteKind,
        prepared: PreparedPair,
    ) -> Result<(), String> {
        if self.pair_pending()
            || prepared.kind != *kind
            || session.pending_ticket() != Some(ticket)
            || ticket.session_id != *session.session_id()
            || ticket.instance_id != *session.instance_id()
            || ticket.document_id != prepared.native_stamp.document_id
            || ticket.model_revision != prepared.native_stamp.model_revision
            || ticket.raw_version != prepared.native_stamp.raw_version
            || ticket.snapshot_hash != prepared.snapshot_hash
        {
            return Err("Native/views request correlation refused; existing pair retained".into());
        }
        self.native_capture = Some(NativePair {
            ticket: ticket.clone(),
            prepared,
        });
        Ok(())
    }
    pub fn cancel_native_capture(&mut self, ticket: &IoTicket, kind: &WriteKind) -> bool {
        if !self
            .native_capture
            .as_ref()
            .is_some_and(|p| p.ticket == *ticket && p.prepared.kind == *kind)
        {
            return false;
        }
        self.native_capture = None;
        true
    }
    fn view_job_after_native(
        &mut self,
        session: &Session,
        ticket: &IoTicket,
        kind: &WriteKind,
    ) -> Result<io::Job, String> {
        if !self
            .native_capture
            .as_ref()
            .is_some_and(|p| p.ticket == *ticket && p.prepared.kind == *kind)
        {
            return Err("Stale native completion; newer paired capture retained".into());
        }
        if self.io.pending() || session.pending() {
            return Err("Preservation still pending".into());
        }
        match kind {
            WriteKind::Save => {
                let c = io::accepted_native(session)?;
                if c.accepted_request() != Some(&ticket.request_id)
                    || c.session_id() != &ticket.session_id
                    || c.instance_id() != &ticket.instance_id
                    || c.path() != ticket.destination.as_path()
                    || c.snapshot_sha256() != ticket.snapshot_hash
                {
                    return Err("Native accepted checkpoint does not own this pair".into());
                }
            }
            WriteKind::Recovery => {
                if !session.recovery_current()
                    || session.last_recovery() != Some(ticket.destination.as_path())
                    || matches!(
                        session.status(),
                        mbse_model::stage1_io::SaveStatus::Error(_)
                            | mbse_model::stage1_io::SaveStatus::Conflict(_)
                    )
                {
                    return Err("Native recovery does not own this pair".into());
                }
            }
        }
        let pair = self.native_capture.take().expect("matched pair");
        self.io.begin_captured(
            session,
            &self.views,
            pair.prepared.capture,
            pair.prepared.destination,
        )
    }
    /// Only the exact accepted owning native request starts its frozen views job.
    pub fn after_native(
        &mut self,
        session: &Session,
        ticket: &IoTicket,
        kind: &WriteKind,
    ) -> Result<(), String> {
        let job = match self.view_job_after_native(session, ticket, kind) {
            Ok(job) => job,
            Err(error) => {
                self.cancel_native_capture(ticket, kind);
                self.note =
                    format!("Views recovery/save could not start: {error}. Live layout retained.");
                return Err(error);
            }
        };
        self.start_job_with(session, job, |work| {
            std::thread::Builder::new()
                .name("mbse-views-write".into())
                .spawn(work)
                .map(|_| ())
        })?;
        self.note = "Native preservation acknowledged; frozen views destination pending".into();
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
            let path_enabled = !self.pair_pending() && !self.io.pending() && !session.pending();
            ui.add_enabled(path_enabled, egui::TextEdit::singleline(&mut self.path));
            // Admission is checked on click by Io::open; no disk reads in a
            // per-frame status hint, and no wire revision grants permission.
            let may_open = !self.io.pending() && !session.pending() && !self.pair_pending();
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
        if self.pair_pending() || session.pending() {
            self.note =
                "Paired preservation pending; Open canceled and current layout retained".into();
            return;
        }
        self.note = match self.io.open(
            session,
            &mut self.views,
            &PathBuf::from(self.path.trim()),
            recovery,
        ) {
            Ok(()) => {
                self.active = self.views.document().primary;
                self.selected = None;
                self.route_source = None;
                self.route_target = None;
                self.drag = None;
                self.reconcile();
                "Views opened with checked identity. Detached runtime history authority was not restored.".into()
            }
            Err(e) => format!("Views input retained read-only; current canvas retained: {e}"),
        };
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
    #[test]
    fn refused_sidecar_open_preserves_history_active_selection_and_route_endpoints() {
        let (area, session) = session();
        let mut ui = ViewWorkspace::new(&session);
        ui.active = *ui.views.document().views.keys().last().unwrap();
        ui.selected = ui
            .views
            .document()
            .occurrences
            .values()
            .find(|o| o.view == ui.active)
            .map(|o| o.id);
        ui.route_source = ui.selected;
        ui.route_target = ui.selected;
        let target = area.path().join("target.mbse-views.json");
        std::fs::write(
            &target,
            ui.views
                .recovery(&session.editor)
                .unwrap()
                .encoded()
                .unwrap(),
        )
        .unwrap();
        ui.apply(
            &session.editor,
            Layout::Move(ui.selected.unwrap(), Geometry::at(444., 555.)),
        );
        let (active, selected, source, target_id) =
            (ui.active, ui.selected, ui.route_source, ui.route_target);
        let digest = ui.views.document().digest().unwrap();
        let history = ui.views.history_usage();
        ui.path = target.display().to_string();
        ui.open(&session, true);
        assert!(ui.note.contains("PRESERVATION"));
        assert_eq!(ui.views.document().digest().unwrap(), digest);
        assert_eq!(ui.views.history_usage(), history);
        assert_eq!(
            (ui.active, ui.selected, ui.route_source, ui.route_target),
            (active, selected, source, target_id)
        );
    }
    #[test]
    fn deferred_native_ack_keeps_original_destination_and_refuses_open_while_pending() {
        let (area, mut session) = session();
        let mut ui = ViewWorkspace::new(&session);
        let path_a = area.path().join("a.mbse-views.json");
        let path_b = area.path().join("b.mbse-views.json");
        let native_path = area.path().join("model.kernspan");
        ui.path = path_a.display().to_string();
        let prepared = ui
            .prepare_native(&session, WriteKind::Save, Some(&native_path))
            .unwrap();
        let native = session.begin_save(&native_path).unwrap();
        let ticket = native.ticket.clone();
        ui.capture_native(&session, &ticket, &WriteKind::Save, prepared)
            .unwrap();
        let source = ui
            .views
            .recovery(&session.editor)
            .unwrap()
            .encoded()
            .unwrap();
        std::fs::write(&path_b, &source).unwrap();
        ui.path = path_b.display().to_string();
        let before = ui.views.document().digest().unwrap();
        ui.open(&session, true);
        assert!(ui.note.contains("pending"));
        assert_eq!(ui.views.document().digest().unwrap(), before);
        assert!(ui.io.open(&session, &mut ui.views, &path_b, true).is_err());
        assert!(session.complete(mbse_model::stage1_io::execute_write(native)));
        let job = ui
            .view_job_after_native(&session, &ticket, &WriteKind::Save)
            .unwrap();
        let receipt = io::execute(job);
        assert!(ui.io.complete(&session, &ui.views, receipt));
        assert!(path_a.is_file());
        assert_eq!(std::fs::read(&path_b).unwrap(), source);
        ui.verify_preserved(&session, &WriteKind::Save).unwrap();
    }
    #[test]
    fn stale_native_cancel_or_ack_cannot_clear_newer_paired_request() {
        let (area, mut session) = session();
        let mut ui = ViewWorkspace::new(&session);
        let native_path = area.path().join("model.kernspan");
        ui.path = area
            .path()
            .join("views.mbse-views.json")
            .display()
            .to_string();
        let prepared = ui
            .prepare_native(&session, WriteKind::Save, Some(&native_path))
            .unwrap();
        let first = session.begin_save(&native_path).unwrap();
        let old = first.ticket.clone();
        ui.capture_native(&session, &old, &WriteKind::Save, prepared)
            .unwrap();
        assert!(session.complete(mbse_model::stage1_io::execute_write(first)));
        assert!(ui.cancel_native_capture(&old, &WriteKind::Save));
        let prepared = ui
            .prepare_native(&session, WriteKind::Save, Some(&native_path))
            .unwrap();
        let second = session.begin_save(&native_path).unwrap();
        let newer = second.ticket.clone();
        ui.capture_native(&session, &newer, &WriteKind::Save, prepared)
            .unwrap();
        assert!(!ui.cancel_native_capture(&old, &WriteKind::Save));
        assert!(ui
            .view_job_after_native(&session, &old, &WriteKind::Save)
            .is_err());
        assert!(ui.pair_pending());
        assert!(session.complete(mbse_model::stage1_io::execute_write(second)));
        let job = ui
            .view_job_after_native(&session, &newer, &WriteKind::Save)
            .unwrap();
        let receipt = io::execute(job);
        assert!(ui.io.complete(&session, &ui.views, receipt));
        assert!(!ui.pair_pending());
    }

    // Clock-controlled timer/controller regressions. No GUI, sleeping, or
    // background workers: actual frozen jobs execute against isolated files.
    const INTERVAL: std::time::Duration = std::time::Duration::from_secs(15);
    const QUIET: std::time::Duration = std::time::Duration::from_millis(750);
    fn layout_edit(ui: &mut ViewWorkspace, session: &Session, title: &str) {
        ui.apply(&session.editor, Layout::RenameView(ui.active, title.into()));
    }
    fn native_edit(ui: &mut ViewWorkspace, session: &mut Session) {
        let stamp = ui.views.stamp(&session.editor);
        ui.views
            .dispatch(
                &mut session.editor,
                &stamp,
                v::NativeAction::Command(mbse_model::stage1::Command::EditText {
                    key: mbse_model::stage1::RawKey {
                        record: RecordId(1),
                        field: "name".into(),
                        item: None,
                    },
                    text: "Changed native system".into(),
                }),
            )
            .unwrap();
    }
    fn paired_native(
        ui: &mut ViewWorkspace,
        session: &mut Session,
        kind: WriteKind,
        path: &Path,
    ) -> mbse_model::stage1_io::WriteJob {
        let prepared = ui
            .prepare_native(session, kind.clone(), Some(path))
            .unwrap();
        let job = match kind {
            WriteKind::Save => session.begin_save(path).unwrap(),
            WriteKind::Recovery => session.begin_recovery().unwrap(),
        };
        ui.capture_native(session, &job.ticket, &kind, prepared)
            .unwrap();
        job
    }
    fn finish_pair(
        ui: &mut ViewWorkspace,
        session: &mut Session,
        native: mbse_model::stage1_io::WriteJob,
    ) {
        let (ticket, kind) = (native.ticket.clone(), native.kind.clone());
        assert!(session.complete(mbse_model::stage1_io::execute_write(native)));
        let job = ui.view_job_after_native(session, &ticket, &kind).unwrap();
        assert!(ui.io.complete(session, &ui.views, io::execute(job)));
    }
    fn saved_pair(area: &tempfile::TempDir, session: &mut Session) -> ViewWorkspace {
        let mut ui = ViewWorkspace::new(session);
        let native = paired_native(
            &mut ui,
            session,
            WriteKind::Save,
            &area.path().join("model.kernspan"),
        );
        finish_pair(&mut ui, session, native);
        ui.verify_preserved(session, &WriteKind::Save).unwrap();
        ui
    }
    fn tick(
        timer: &mut crate::periodic_recovery::PeriodicRecovery,
        at: std::time::Instant,
        session: &Session,
        ui: &ViewWorkspace,
    ) -> bool {
        timer.due(at, true, INTERVAL, false, session, ui)
    }
    #[test]
    fn periodic_layout_only_clean_native_creates_verified_pair_without_repeating() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        assert!(!session.is_dirty());
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        layout_edit(&mut ui, &session, "Layout only");
        assert!(!tick(&mut timer, now + INTERVAL, &session, &ui));
        assert!(tick(&mut timer, now + INTERVAL + QUIET, &session, &ui));
        let job = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        finish_pair(&mut ui, &mut session, job);
        assert!(!session.is_dirty());
        assert!(session.recovery_current());
        ui.verify_preserved(&session, &WriteKind::Recovery).unwrap();
        assert!(!tick(&mut timer, now + INTERVAL * 3, &session, &ui));
    }
    #[test]
    fn periodic_both_dirty_preserves_native_and_layout_independently() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        native_edit(&mut ui, &mut session);
        layout_edit(&mut ui, &session, "Both dirty");
        assert!(!tick(&mut timer, now + INTERVAL, &session, &ui));
        assert!(tick(&mut timer, now + INTERVAL + QUIET, &session, &ui));
        let job = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        finish_pair(&mut ui, &mut session, job);
        assert!(session.is_dirty()); // Recovery is not native Save.
        assert!(session.recovery_current());
        assert!(!ui.preserved(&session, &WriteKind::Save));
        ui.verify_preserved(&session, &WriteKind::Recovery).unwrap();
    }
    #[test]
    fn periodic_native_edit_during_recovery_cannot_ack_newer_snapshot() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        layout_edit(&mut ui, &session, "Before recovery");
        assert!(!tick(&mut timer, now + INTERVAL, &session, &ui));
        assert!(tick(&mut timer, now + INTERVAL + QUIET, &session, &ui));
        let job = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        let ticket = job.ticket.clone();
        native_edit(&mut ui, &mut session);
        assert!(session.complete(mbse_model::stage1_io::execute_write(job)));
        assert!(!session.recovery_current());
        // Same decision as Workspace::finish_writes for an old native result.
        assert!(ui.cancel_native_capture(&ticket, &WriteKind::Recovery));
        assert!(!ui.io.pending());
        assert!(!tick(&mut timer, now + INTERVAL * 2, &session, &ui));
        assert!(tick(&mut timer, now + INTERVAL * 2 + QUIET, &session, &ui));
    }
    #[test]
    fn periodic_layout_edit_during_write_leaves_new_revision_due() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        layout_edit(&mut ui, &session, "Frozen layout");
        let native = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        let ticket = native.ticket.clone();
        assert!(session.complete(mbse_model::stage1_io::execute_write(native)));
        let views_job = ui
            .view_job_after_native(&session, &ticket, &WriteKind::Recovery)
            .unwrap();
        layout_edit(&mut ui, &session, "Newer live layout");
        let digest = ui.views.document().digest().unwrap();
        assert!(!tick(&mut timer, now + INTERVAL, &session, &ui));
        assert!(ui.io.complete(&session, &ui.views, io::execute(views_job)));
        assert!(!ui.preserved(&session, &WriteKind::Recovery));
        assert!(ui.verify_preserved(&session, &WriteKind::Recovery).is_err());
        assert_eq!(ui.views.document().digest().unwrap(), digest);
        assert!(tick(&mut timer, now + INTERVAL + QUIET, &session, &ui));
    }
    #[test]
    fn periodic_busy_manual_pair_coalesces_without_replacing_ticket_or_destination() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        let job = paired_native(
            &mut ui,
            &mut session,
            WriteKind::Save,
            &area.path().join("model.kernspan"),
        );
        let ticket = job.ticket.clone();
        layout_edit(&mut ui, &session, "During manual save");
        assert!(!tick(&mut timer, now + INTERVAL, &session, &ui));
        assert!(!tick(&mut timer, now + INTERVAL * 2, &session, &ui));
        assert_eq!(session.pending_ticket(), Some(&ticket));
        assert!(ui.pair_pending());
        assert!(session.complete(mbse_model::stage1_io::execute_write(job)));
        let views_job = ui
            .view_job_after_native(&session, &ticket, &WriteKind::Save)
            .unwrap();
        assert!(!tick(&mut timer, now + INTERVAL * 3, &session, &ui));
        assert!(ui.io.complete(&session, &ui.views, io::execute(views_job)));
        // Waiting did not consume a new interval; current edits remain due.
        assert!(tick(&mut timer, now + INTERVAL * 3, &session, &ui));
    }
    #[test]
    fn periodic_recovery_worker_failure_keeps_native_success_and_retries_later() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        layout_edit(&mut ui, &session, "Worker failure");
        assert!(!tick(&mut timer, now + INTERVAL, &session, &ui));
        let attempt = now + INTERVAL + QUIET;
        assert!(tick(&mut timer, attempt, &session, &ui));
        let native = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        let ticket = native.ticket.clone();
        assert!(session.complete(mbse_model::stage1_io::execute_write(native)));
        let job = ui
            .view_job_after_native(&session, &ticket, &WriteKind::Recovery)
            .unwrap();
        let digest = ui.views.document().digest().unwrap();
        assert!(ui
            .start_job_with(&session, job, |_| Err(std::io::Error::other(
                "injected worker failure"
            )))
            .is_err());
        assert!(session.recovery_current());
        assert!(!ui.io.pending());
        assert!(!ui.pair_pending());
        assert_eq!(ui.views.document().digest().unwrap(), digest);
        assert!(!tick(&mut timer, attempt + QUIET, &session, &ui));
        assert!(tick(&mut timer, attempt + INTERVAL, &session, &ui));
    }
    #[test]
    fn periodic_frozen_destination_failure_never_overwrites_foreign_leaf() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        layout_edit(&mut ui, &session, "Destination failure");
        let native = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        let ticket = native.ticket.clone();
        let destination = ui
            .native_capture
            .as_ref()
            .unwrap()
            .prepared
            .destination
            .path()
            .to_owned();
        std::fs::write(&destination, b"foreign leaf").unwrap();
        assert!(session.complete(mbse_model::stage1_io::execute_write(native)));
        assert!(ui
            .view_job_after_native(&session, &ticket, &WriteKind::Recovery)
            .is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"foreign leaf");
        assert!(session.recovery_current());
        assert!(!ui.pair_pending());
        assert!(!ui.io.pending());
        assert!(!ui.preserved(&session, &WriteKind::Recovery));
        // Retry allocates a new leaf rather than claiming the failed one.
        let retry = ui
            .prepare_native(&session, WriteKind::Recovery, None)
            .unwrap();
        assert_ne!(retry.destination.path(), destination);
    }
    #[test]
    fn periodic_disabled_navigation_and_origin_change_reset_quiet_observation() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        layout_edit(&mut ui, &session, "Disabled edit");
        assert!(!timer.due(now + INTERVAL, false, INTERVAL, false, &session, &ui));
        assert!(!timer.due(now + INTERVAL + QUIET, true, INTERVAL, true, &session, &ui));
        assert!(tick(&mut timer, now + INTERVAL + QUIET, &session, &ui));
        // Same native document and default revision, distinct private origin.
        let foreign = ViewWorkspace::new(&session);
        assert!(!tick(&mut timer, now + INTERVAL * 3, &session, &foreign));
        assert!(tick(
            &mut timer,
            now + INTERVAL * 3 + QUIET,
            &session,
            &foreign
        ));
    }
    #[test]
    fn periodic_native_failure_does_not_start_or_ack_views() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        layout_edit(&mut ui, &session, "Native failure");
        let native = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        let ticket = native.ticket.clone();
        std::fs::write(&ticket.destination, b"foreign native leaf").unwrap();
        let result = mbse_model::stage1_io::execute_write(native);
        assert!(result.error().is_some());
        assert!(session.complete(result));
        assert!(ui.cancel_native_capture(&ticket, &WriteKind::Recovery));
        assert!(!session.recovery_current());
        assert!(!ui.io.pending());
        assert!(!ui.preserved(&session, &WriteKind::Recovery));
        assert_eq!(
            std::fs::read(&ticket.destination).unwrap(),
            b"foreign native leaf"
        );
    }

    #[test]
    fn periodic_held_pointer_gesture_and_release_wait_for_persisted_quiet_layout() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        layout_edit(&mut ui, &session, "Prior unsaved edit");
        let o = ui
            .views
            .document()
            .occurrences
            .values()
            .next()
            .unwrap()
            .clone();
        ui.drag = Some((
            o.id,
            o.geometry.clone(),
            ui.views.stamp(&session.editor),
            egui::Pos2::ZERO,
        ));
        assert!(!tick(&mut timer, now + INTERVAL, &session, &ui));
        assert!(!tick(&mut timer, now + INTERVAL * 2, &session, &ui));
        ui.drag = None;
        ui.apply(
            &session.editor,
            Layout::Move(o.id, Geometry::at(400., 300.)),
        );
        let release = now + INTERVAL * 2;
        assert!(!tick(&mut timer, release, &session, &ui));
        assert!(!tick(
            &mut timer,
            release + QUIET - std::time::Duration::from_millis(1),
            &session,
            &ui
        ));
        assert!(tick(&mut timer, release + QUIET, &session, &ui));
    }
    #[test]
    fn periodic_due_time_rechecks_removed_truncated_and_tampered_bound_artifacts() {
        for mutation in 0..3 {
            let (area, mut session) = session();
            let mut ui = saved_pair(&area, &mut session);
            let path = ui.io.saved_path().unwrap().to_owned();
            let original = std::fs::read(&path).unwrap();
            let now = std::time::Instant::now();
            let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
            match mutation {
                0 => std::fs::remove_file(&path).unwrap(),
                1 => std::fs::write(&path, b"{").unwrap(),
                _ => {
                    let mut bytes = original;
                    bytes.push(b'\n');
                    std::fs::write(&path, bytes).unwrap();
                }
            }
            let retained = std::fs::read(&path).ok();
            assert!(ui.io.is_saved(&session, &ui.views)); // Hint deliberately stale.
            assert!(!tick(&mut timer, now + INTERVAL - QUIET, &session, &ui));
            assert_eq!(timer.status(&session, &ui).0, "awaiting timer check");
            assert!(tick(&mut timer, now + INTERVAL, &session, &ui));
            let native = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
            finish_pair(&mut ui, &mut session, native);
            ui.verify_preserved(&session, &WriteKind::Recovery).unwrap();
            assert_eq!(std::fs::read(&path).ok(), retained); // No Save target repair/write.
        }
    }
    #[test]
    fn periodic_due_time_rechecks_layout_recovery_and_preserves_damaged_evidence() {
        for mutation in 0..3 {
            let (area, mut session) = session();
            let mut ui = saved_pair(&area, &mut session);
            layout_edit(&mut ui, &session, "Recovered layout");
            let native = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
            let path = ui
                .native_capture
                .as_ref()
                .unwrap()
                .prepared
                .destination
                .path()
                .to_owned();
            finish_pair(&mut ui, &mut session, native);
            let now = std::time::Instant::now();
            let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
            match mutation {
                0 => std::fs::remove_file(&path).unwrap(),
                1 => std::fs::write(&path, b"{").unwrap(),
                _ => {
                    let mut bytes = std::fs::read(&path).unwrap();
                    bytes.push(b'\n');
                    std::fs::write(&path, bytes).unwrap();
                }
            }
            let retained = std::fs::read(&path).ok();
            assert!(session.recovery_current());
            assert!(ui.io.recovery_current(&session, &ui.views));
            assert!(tick(&mut timer, now + INTERVAL, &session, &ui));
            assert_eq!(
                timer.status(&session, &ui),
                ("verified at timer check", "not verified at timer check")
            );
            let retry = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
            finish_pair(&mut ui, &mut session, retry);
            assert_eq!(std::fs::read(&path).ok(), retained);
            ui.verify_preserved(&session, &WriteKind::Recovery).unwrap();
        }
    }
    #[test]
    fn periodic_verified_noop_checks_are_interval_bounded_and_native_loss_is_due() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        layout_edit(&mut ui, &session, "Current recovery");
        let job = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        finish_pair(&mut ui, &mut session, job);
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        assert!(!tick(&mut timer, now + INTERVAL, &session, &ui));
        assert_eq!(
            timer.status(&session, &ui),
            ("verified at timer check", "verified at timer check")
        );
        std::fs::remove_file(session.last_recovery().unwrap()).unwrap();
        for seconds in 16..30 {
            assert!(!tick(
                &mut timer,
                now + std::time::Duration::from_secs(seconds),
                &session,
                &ui
            ));
        }
        assert!(tick(&mut timer, now + INTERVAL * 2, &session, &ui));
        assert_eq!(
            timer.status(&session, &ui),
            ("not verified at timer check", "verified at timer check")
        );
        let missing = session.last_recovery().unwrap().to_owned();
        let job = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        let ticket = job.ticket.clone();
        let result = mbse_model::stage1_io::execute_write(job);
        assert!(result.error().is_some());
        assert!(session.complete(result));
        assert!(ui.cancel_native_capture(&ticket, &WriteKind::Recovery));
        assert!(!missing.exists()); // No silent repair of removed acknowledged artifact.
        assert!(!ui.io.pending());
        assert!(matches!(
            session.status(),
            mbse_model::stage1_io::SaveStatus::Conflict(_)
                | mbse_model::stage1_io::SaveStatus::Error(_)
        ));
    }
    #[test]
    fn periodic_native_only_edit_is_eligible_without_layout_mutation() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        let now = std::time::Instant::now();
        let mut timer = crate::periodic_recovery::PeriodicRecovery::new(now, &session, &ui);
        native_edit(&mut ui, &mut session);
        assert!(!tick(&mut timer, now + INTERVAL, &session, &ui));
        assert!(tick(&mut timer, now + INTERVAL + QUIET, &session, &ui));
        let job = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        finish_pair(&mut ui, &mut session, job);
        assert!(session.recovery_current());
        ui.verify_preserved(&session, &WriteKind::Recovery).unwrap();
    }
    #[test]
    fn native_validation_failure_clears_exact_pair_and_allows_recovery_retry() {
        let (area, mut session) = session();
        let mut ui = saved_pair(&area, &mut session);
        layout_edit(&mut ui, &session, "Keep geometry on native tamper");
        let native = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        let ticket = native.ticket.clone();
        assert!(session.complete(mbse_model::stage1_io::execute_write(native)));
        let bytes = std::fs::read(&ticket.destination).unwrap();
        let digest = ui.views.document().digest().unwrap();
        std::fs::write(&ticket.destination, b"controlled native tamper").unwrap();
        assert!(ui
            .after_native(&session, &ticket, &WriteKind::Recovery)
            .is_err());
        assert!(!ui.pair_pending());
        assert!(!ui.io.pending());
        assert_eq!(ui.views.document().digest().unwrap(), digest);
        std::fs::write(&ticket.destination, bytes).unwrap(); // Restore owned fixture only.
        let retry = paired_native(&mut ui, &mut session, WriteKind::Recovery, area.path());
        finish_pair(&mut ui, &mut session, retry);
        ui.verify_preserved(&session, &WriteKind::Recovery).unwrap();
    }
}
