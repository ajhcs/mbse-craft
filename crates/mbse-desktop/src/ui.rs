//! App-owned UI over the bounded, provenance-preserving Stage1 contract.
use crate::preferences::Preferences;
use craft_ui::{
    buttons::{ButtonVisuals, IconButton, IconButtonStyle},
    theme::WidgetPalette,
};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use mbse_model::{
    stage1::{
        AffiliationKind, Command, ContextKind, CreationDraft, RawKey, Record, RecordId, RecordKind,
        SubjectRef, Validity,
    },
    stage1_calc::{self as calc, CalculationId, Definition, Limits, QuantityType, UnitId},
    stage1_graph::{self, EdgeId, GraphEdge},
    stage1_io::{self, Session, WriteJob, WriteKind, WriteResult},
};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

const INK: Color32 = Color32::from_rgb(36, 48, 63);
const ACCENT: Color32 = Color32::from_rgb(18, 100, 119);
const PAPER: Color32 = Color32::from_rgb(252, 251, 247);
const LINE: Color32 = Color32::from_rgb(197, 207, 209);
const ERROR: Color32 = Color32::from_rgb(160, 42, 55);

#[derive(Clone)]
enum Load {
    New,
    Open(PathBuf),
    Recover(PathBuf),
    Import(PathBuf),
    Quit,
}

pub struct Workspace {
    session: Session,
    views: crate::views_ui::ViewWorkspace,
    path: String,
    save_path: String,
    model_fields: HashMap<egui::Id, RawKey>,
    context: egui::Context,
    catalog: crate::catalog_ui::CatalogBrowser,
    exchange: crate::semantic_ui::SemanticBrowser,
    assurance: crate::assurance_ui::AssuranceWorkspace,
    frame_choice: craft_window::Choice,
    close_intent: bool,
    writes: Sender<WriteResult>,
    completed_writes: Receiver<WriteResult>,
    pending_continue: Option<(Load, WriteKind)>,
    preferences: Preferences,
    settings_window: bool,
    periodic_recovery: crate::periodic_recovery::PeriodicRecovery,
    feedback: String,
    error: bool,
    creation: Option<(RecordKind, RecordId, String)>,
    relation: Option<EdgeId>,
    popout: Option<RecordId>,
    usages: Option<mbse_model::native_navigation::UsageReport>,
    pending: Option<Load>,
    context_creation: Option<(RecordId, String, ContextKind)>,
    recovery_window: bool,
    recovery_inventory: Option<mbse_model::recovery::RecoveryInventory>,
    recovery_selected: Option<PathBuf>,
    capture: Option<PathBuf>,
    frames: u32,
    screenshot_requested: bool,
    zoom: f32,
    show_ownership: bool,
    allow_close: Option<String>,
    audit: BTreeMap<String, [f32; 4]>,
    audit_previous: Vec<u8>,
}
impl Workspace {
    pub fn new(
        session: Session,
        capture: Option<PathBuf>,
        config: Option<PathBuf>,
        frame_choice: craft_window::Choice,
        ctx: &egui::Context,
    ) -> Self {
        let mut style = (*ctx.style_of(egui::Theme::Light)).clone();
        style.visuals = egui::Visuals::light();
        style.visuals.panel_fill = PAPER;
        style.visuals.window_fill = PAPER;
        style.visuals.override_text_color = Some(INK);
        // Real ArtCraft shared palette API, pinned and licensed in vendor/.
        WidgetPalette {
            noninteractive: PAPER,
            inactive: Color32::from_rgb(236, 241, 240),
            hovered: Color32::from_rgb(220, 234, 232),
            active: Color32::from_rgb(202, 226, 224),
            open: Color32::from_rgb(220, 234, 232),
        }
        .apply_to(
            &mut style.visuals.widgets,
            egui::CornerRadius::same(5),
            Stroke::new(1., INK),
        );
        style.spacing.item_spacing = Vec2::new(9., 8.);
        ctx.set_style_of(egui::Theme::Light, style);
        ctx.set_theme(egui::Theme::Light);
        let path = session
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (writes, completed_writes) = mpsc::channel();
        let preferences = Preferences::open(config, ctx.clone());
        let views = crate::views_ui::ViewWorkspace::new(&session);
        let periodic_recovery = crate::periodic_recovery::PeriodicRecovery::new(Instant::now(), &session, &views);
        let mut app=Self { views, session, save_path: path.clone(), path, model_fields:HashMap::new(), context:ctx.clone(),catalog:crate::catalog_ui::CatalogBrowser::default(),exchange:crate::semantic_ui::SemanticBrowser::default(),assurance:crate::assurance_ui::AssuranceWorkspace::default(),frame_choice,close_intent:false,writes,completed_writes,pending_continue:None,preferences,settings_window:false,periodic_recovery, feedback: "Select a model card to edit its fields. Model values and unfinished text remain separate.".into(), error: false,
            creation: None, relation: None, popout: None, usages: None, pending: None, context_creation: None, recovery_window: false, recovery_inventory: None, recovery_selected: None, capture, frames: 0, screenshot_requested: false, zoom: 1., show_ownership: false, allow_close: None, audit: BTreeMap::new(), audit_previous: Vec::new() };
        app.refresh_recoveries();
        app
    }
    fn refresh_recoveries(&mut self) {
        match mbse_model::recovery::discover_recoveries(&self.session.state_root) {
            Ok(inventory) => self.recovery_inventory = Some(inventory),
            Err(error) => self.result(Err(format!("Recovery discovery: {error}"))),
        }
    }
    fn bound(&mut self, key: impl Into<String>, response: &egui::Response) {
        let rect = response.rect.intersect(response.interact_rect);
        self.audit
            .insert(key.into(), [rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
    }
    fn result(&mut self, result: Result<String, String>) {
        match result {
            Ok(message) => {
                self.feedback = message;
                self.error = false;
            }
            Err(error) => {
                self.feedback = error;
                self.error = true;
            }
        }
    }
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            if self.views.inspection_visible() {
                self.result(Ok("Read-only source inspection: close it to use the model Save shortcut.".into()));
            } else if self.assurance.is_visible() {
                self.assurance.request_save();
            } else if !self.exchange.is_visible() {
                self.save();
            }
        }
        let transition = ctx.input(|i| {
            i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::PointerButton { pressed: true, .. }
                        | egui::Event::Key {
                            key: egui::Key::Tab,
                            pressed: true,
                            ..
                        }
                )
            })
        });
        let target = if self.views.inspection_visible() || self.assurance.is_visible() || self.exchange.is_visible() {
            ShortcutTarget::Widget
        } else {
            shortcut_policy(
                self.focused_raw(),
                ctx.text_edit_focused()
                    || egui::Popup::is_any_open(&ctx)
                    || self.creation.is_some()
                    || self.context_creation.is_some()
                    || self.settings_window
                    || self.recovery_window
                    || self.pending.is_some(),
                transition,
            )
        };
        if !matches!(target, ShortcutTarget::Widget) {
            let redo = ctx.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                )
            });
            let undo =
                !redo && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
            if redo || undo {
                let field = match target {
                    ShortcutTarget::Field(key) => Some(key),
                    _ => None,
                };
                self.history(undo, field);
            }
        }
    }
    fn command(&mut self, command: Command) {
        let stamp = self.views.views.stamp(&self.session.editor);
        let result = self.views.views.dispatch(&mut self.session.editor, &stamp,
            mbse_model::native_views::NativeAction::Command(command)).and_then(|outcome| match outcome {
                mbse_model::native_views::NativeOutcome::Edit(outcome) => Ok(outcome),
                _ => Err("Unexpected native history outcome".into()),
            });
        self.views.reconcile();
        let message = result.map(|outcome| {
            if let Some(id) = outcome.created {
                if self.session.editor.snapshot.records.contains_key(&id) {
                    self.select(id);
                }
            }
            if outcome.accepted {
                "Model updated. Save includes accepted values and retained drafts.".into()
            } else {
                "Draft retained; last accepted model value is unchanged.".into()
            }
        });
        self.result(message);
    }
    fn open_usages(&mut self, id: RecordId) {
        match mbse_model::native_navigation::find_usages(&self.session.editor,&self.views.views,id) {
            Ok(report)=>self.usages=Some(report),Err(error)=>self.result(Err(error)),
        }
    }
    fn navigate_usage(&mut self, index: usize) {
        let result=self.usages.as_ref().ok_or_else(||"No native usages report".to_string()).and_then(|report|report.referencing_record(&self.session.editor,&self.views.views,index));
        match result {Ok(id)=>self.select(id),Err(error)=>self.result(Err(error))}
    }
    fn show_usages(&mut self,ctx:&egui::Context) {
        let Some(report)=self.usages.take() else {return;};
        let target=report.target();let stale=report.check(&self.session.editor,&self.views.views).err();
        let mut open=true;let mut refresh=false;let mut navigate=None;
        egui::Window::new("Native Find usages").id(egui::Id::new("native-find-usages")).open(&mut open).default_width(760.).show(ctx,|ui|{
            ui.heading(format!("Authored graph usages of native record #{}",target.0));
            ui.label("Known scope, contributesTo, subject, forObjective and resolved calculation-output references only. Opaque fields, drafts and unresolved references are not interpreted here.");
            ui.label("This report selects existing native records. It does not edit references or change diagram occurrences.");
            let response=ui.button("Refresh usages");self.bound("usages:refresh",&response);refresh=response.clicked();
            if let Some(error)=&stale {ui.label(error);}
            ui.label(format!("{} referencing record/role rows (named calculation inputs are grouped).",report.usages().len()));
            egui::ScrollArea::vertical().id_salt("native-usage-rows").max_height(500.).show(ui,|ui|{
                for (index,usage) in report.usages().iter().enumerate() {
                    let label=self.session.editor.snapshot.records.get(&usage.referencing_record).map(|record|format!("{} #{} · {}",record.kind.label(),record.id.0,record.name)).unwrap_or_else(||format!("Unavailable #{}",usage.referencing_record.0));
                    ui.group(|ui|{
                        ui.label(format!("{label} references #{} via {:?}",target.0,usage.role));
                        if !usage.binding_names.is_empty(){ui.label(format!("Input bindings: {}",usage.binding_names.join(", ")));}
                        let response=ui.add_enabled(stale.is_none(),egui::Button::new("Select referencing record"));self.bound(format!("usages:select:{index}"),&response);if response.clicked(){navigate=Some(index);}
                    });
                }
                if report.usages().is_empty(){ui.label("No usages found among the supported authored graph roles at this snapshot.");}
            });
        });
        if open {self.usages=Some(report);if refresh {self.open_usages(target);} else if let Some(index)=navigate {self.navigate_usage(index);}}
    }
    fn select(&mut self, id: RecordId) {
        if let Some(focused) = self.context.memory(|m| m.focused()) {
            if self.model_fields.contains_key(&focused) {
                self.context.memory_mut(|m| m.surrender_focus(focused));
            }
        }
        let result = self.session.editor.select(id);
        if let Err(error) = result {
            self.result(Err(error));
        }
    }
    fn field(
        &mut self,
        ui: &mut egui::Ui,
        record: &Record,
        field: &str,
        label: &str,
        multiline: bool,
    ) {
        self.raw_field(ui, record, None, field, label, multiline);
    }
    fn raw_field(
        &mut self,
        ui: &mut egui::Ui,
        record: &Record,
        item: Option<RecordId>,
        field: &str,
        label: &str,
        multiline: bool,
    ) {
        let key = RawKey {
            record: record.id,
            field: field.into(),
            item,
        };
        if !self.session.editor.field_available(&key) {
            return;
        }
        ui.label(RichText::new(label).strong());
        let mut text = self.session.editor.display_text(&key);
        let input = if multiline {
            egui::TextEdit::multiline(&mut text).desired_rows(2)
        } else {
            egui::TextEdit::singleline(&mut text)
        };
        let response = ui.add(
            input
                .id_salt((record.id.0, item.map(|id| id.0), field))
                .desired_width(f32::INFINITY),
        );
        self.bound(
            if let Some(item) = item {
                format!("field:{}:{}:{}", record.id.0, field, item.0)
            } else {
                format!("field:{}:{}", record.id.0, field)
            },
            &response,
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label));
        self.model_fields.insert(response.id, key.clone());
        if response.changed() {
            self.command(Command::EditText {
                key: key.clone(),
                text,
            });
        }
        if response.lost_focus() {
            self.session.editor.finish_field();
        }
        if let Some(raw) = self.session.editor.raw(&key) {
            if raw.validity != Validity::Valid {
                ui.colored_label(ERROR, format!("{:?} draft retained", raw.validity));
                ui.label(
                    RichText::new(format!(
                        "Last accepted: {}",
                        if field == "criterion" {
                            record
                                .number(field)
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "absent".into())
                        } else {
                            record.text(field).into()
                        }
                    ))
                    .small(),
                );
            }
        }
    }
    fn spawn_write(&mut self, job: WriteJob, prepared: crate::views_ui::PreparedPair) -> bool {
        self.spawn_write_with(job, prepared, |work| {
            std::thread::Builder::new().name("mbse-native-write".into()).spawn(work).map(|_| ())
        })
    }
    fn spawn_write_with(&mut self, job: WriteJob, prepared: crate::views_ui::PreparedPair,
        start: impl FnOnce(Box<dyn FnOnce() + Send>) -> Result<(), std::io::Error>) -> bool {
        let token = job.start_token();
        let ticket = job.ticket.clone();
        let kind = job.kind.clone();
        if let Err(error) = self.views.capture_native(&self.session, &ticket, &kind, prepared) {
            self.session.failed_start(token, error.clone());
            self.result(Err(error));
            return false;
        }
        let tx = self.writes.clone();
        let ctx = self.context.clone();
        if let Err(error) = start(Box::new(move || {
            let result = stage1_io::execute_write(job);
            let _ = tx.send(result);
            ctx.request_repaint();
        })) {
            self.session.failed_start(token, error.to_string());
            self.views.cancel_native_capture(&ticket, &kind);
            self.views.note = "Native worker did not start; paired layout retained for retry.".into();
            self.result(Err(format!("{kind:?} worker did not start: {error}. Current work retained.")));
            return false;
        }
        self.result(Ok(format!("{kind:?} write pending; the frozen snapshot is being written.")));
        true
    }
    fn save(&mut self) -> bool {
        if self.views.io.pending() { self.result(Err("Views write pending; current work retained.".into())); return false; }
        if self.save_path.trim().is_empty() {
            self.result(Err("Enter a native save destination before saving.".into()));
            return false;
        }
        let native_path = PathBuf::from(self.save_path.trim());
        let prepared = match self.views.prepare_native(&self.session, WriteKind::Save, Some(&native_path)) {
            Ok(prepared) => prepared, Err(error) => { self.result(Err(error)); return false; }
        };
        match self.session.begin_save(&native_path) {
            Ok(job) => {
                self.spawn_write(job, prepared)
            }
            Err(error) => {
                self.result(Err(error));
                false
            }
        }
    }
    fn recovery(&mut self) -> bool {
        if self.views.io.pending() { self.result(Err("Views write pending; current work retained.".into())); return false; }
        let prepared = match self.views.prepare_native(&self.session, WriteKind::Recovery, None) {
            Ok(prepared) => prepared, Err(error) => { self.result(Err(error)); return false; }
        };
        match self.session.begin_recovery() {
            Ok(job) => {
                self.spawn_write(job, prepared)
            }
            Err(error) => {
                self.result(Err(error));
                false
            }
        }
    }
    fn finish_writes(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.completed_writes.try_recv() {
            let kind = result.kind.clone();
            let ticket = result.ticket.clone();
            let error = result.error().map(ToString::to_string);
            let consumed = self.session.complete(result);
            let current = consumed
                && error.is_none()
                && match kind {
                    WriteKind::Save => {
                        !self.session.is_dirty()
                            && matches!(self.session.status(), stage1_io::SaveStatus::Saved)
                    }
                    WriteKind::Recovery => self.session.recovery_current() && !matches!(self.session.status(), stage1_io::SaveStatus::Error(_) | stage1_io::SaveStatus::Conflict(_)),
                };
            if !consumed {
                self.result(Err("Stale native receipt ignored; active request and paired capture retained.".into()));
                continue;
            }
            if let Some(error) = error {
                self.views.cancel_native_capture(&ticket, &kind);
                self.result(Err(format!("{kind:?} failed: {error}. Current work retained.")));
                self.pending_continue = None;
            } else if !current {
                self.views.cancel_native_capture(&ticket, &kind);
                self.result(Ok(format!("Earlier {kind:?} snapshot written. Newer accepted values or drafts remain unsaved; repeat preservation to continue.")));
                self.pending_continue = None;
            } else {
                self.result(Ok(match kind {
                    WriteKind::Save => {
                        "Saved accepted values and drafts; current bytes verified.".into()
                    }
                    WriteKind::Recovery => format!(
                        "Current recovery copy verified: {}",
                        self.session
                            .last_recovery()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    ),
                }));
                self.refresh_recoveries();
                if let Err(error) = self.views.after_native(&self.session, &ticket, &kind) {
                    self.result(Err(format!("Native preservation succeeded; views remain unsaved: {error}")));
                    self.pending_continue = None;
                }

            }
        }
    }
    fn focused_raw(&self) -> Option<RawKey> {
        self.context
            .memory(|m| m.focused())
            .and_then(|id| self.model_fields.get(&id).cloned())
            .filter(|key| {
                self.session.editor.selected == Some(key.record)
                    && self.session.editor.field_available(key)
            })
    }
    fn request_load(&mut self, action: Load) {
        if self.session.is_dirty() || self.session.pending() || self.views.blocked(&self.session) {
            self.pending = Some(action);
            self.pending_continue = None;
        } else if self.views.verify_preserved(&self.session, &WriteKind::Save).is_ok() {
            self.load(action);
        } else {
            self.pending = Some(action);
            self.result(Err("Acknowledged view file changed or is unavailable; live layout retained.".into()));
        }
    }
    fn load(&mut self, action: Load) {
        if matches!(action, Load::Quit) {
            return;
        }
        let root = self.session.state_root.clone();
        let result = match action {
            Load::New => Session::new(&root),
            Load::Open(path) => Session::open(&root, &path),
            Load::Recover(path) => Session::restore(&root, &path),
            Load::Import(path) => Session::import_sysml(&root, &path),
            Load::Quit => unreachable!(),
        };
        match result {
            Ok(session) => {
                self.views = crate::views_ui::ViewWorkspace::new(&session);
                self.session = session;
                self.creation = None;
                self.context_creation = None;
                self.relation = None;
                self.popout = None;
                self.usages = None;
                self.save_path = self
                    .session
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                self.model_fields.clear();
                self.allow_close = None;
                self.periodic_recovery = crate::periodic_recovery::PeriodicRecovery::new(Instant::now(), &self.session, &self.views);
                self.refresh_recoveries();
                self.result(Ok(
                    "Model opened. Imported and recovered documents are independent drafts.".into(),
                ));
            }
            Err(error) => self.result(Err(error)),
        }
    }
    fn frame_button(&mut self, ui: &mut egui::Ui, button: craft_window::Button) {
        let (glyph, label) = match button {
            craft_window::Button::Minimize => ("_", "Minimize window"),
            craft_window::Button::Maximize => ("[]", "Maximize or restore window"),
            craft_window::Button::Close => ("X", "Close window"),
        };
        let response = ui.add_sized([34., 26.], egui::Button::new(glyph));
        self.bound(format!("frame:{label}"), &response);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
        if response.on_hover_text(label).clicked() {
            match button {
                craft_window::Button::Minimize => ui
                    .ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                craft_window::Button::Maximize => {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                        !ui.input(|i| i.viewport().maximized.unwrap_or(false)),
                    ))
                }
                craft_window::Button::Close => {
                    self.close_intent = true;
                    ui.ctx().request_repaint();
                }
            }
        }
    }
    fn frame_header(&mut self, root: &mut egui::Ui) {
        let Some(layout) = self.frame_choice.frame().layout().cloned() else {
            return;
        };
        egui::Panel::top("native-app-frame").show(root, |ui| {
            ui.horizontal(|ui| {
                for button in &layout.left {
                    self.frame_button(ui, *button);
                }
                let width = (ui.available_width() - layout.right.len() as f32 * 43. - 8.).max(60.);
                let response = ui.add_sized(
                    [width, 28.],
                    egui::Label::new("MBSE-Craft · Model workspace")
                        .sense(egui::Sense::click_and_drag()),
                );
                if response.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if response.double_clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                        !ui.input(|i| i.viewport().maximized.unwrap_or(false)),
                    ));
                }
                for button in &layout.right {
                    self.frame_button(ui, *button);
                }
            });
        });
    }
    fn toolbar(&mut self, root: &mut egui::Ui) {
        let history_field = self.focused_raw();
        egui::Panel::top("workspace_toolbar").show(root, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("MBSE-Craft"); ui.label("Model workspace"); ui.separator();
                for (label, action) in [("New", Load::New), ("Open", Load::Open(PathBuf::from(self.path.trim()))), ("Recover", Load::Recover(PathBuf::from(self.path.trim()))), ("Import SysML", Load::Import(PathBuf::from(self.path.trim())))] {
                    let response = ui.button(label); self.bound(format!("action:{label}"),&response);
                    if response.clicked() { self.request_load(action); }
                }
                let save = ui.button("Save"); self.bound("action:Save",&save); if save.clicked() { self.save(); }
                let recovery = ui.button("Recovery copy"); self.bound("action:Recovery copy",&recovery); if recovery.clicked() { self.recovery(); }
                let recovery_count=self.recovery_inventory.as_ref().map_or(0,|i|i.entries.len());
                let response=ui.button(format!("Review recovery copies ({recovery_count})"));self.bound("recovery:Review copies",&response);if response.clicked(){self.refresh_recoveries();self.recovery_window=true;}
                let response=ui.button("Open external projection");self.bound("action:Open external projection",&response);if response.clicked(){self.exchange.open(&self.path);}
                let response=ui.button("Requirements and evidence");self.bound("action:Requirements and evidence",&response);if response.clicked(){self.assurance.open_window();}
                let response=ui.button("Browse library folder");self.bound("action:Browse library folder",&response);if response.clicked(){self.catalog.open(&self.path);}
                let response=ui.button("Recovery settings");self.bound("settings:Open",&response);if response.clicked(){self.settings_window=true;}
                if ui.button("Export SysML copy").clicked() {
                    let result = self.session.begin_export_sysml(&PathBuf::from(self.path.trim())).and_then(|job| self.session.complete_export(stage1_io::execute_export(job))).and_then(|accepted| if accepted { Ok("Exported restricted SysML copy.".into()) } else { Err("Export receipt was not accepted.".into()) });
                    self.result(result);
                }
                ui.separator();
                // Real ArtCraft IconButton controls with accessible action labels.
                let visual = ButtonVisuals { fill: Color32::from_rgb(231, 239, 238), stroke: Stroke::new(1., LINE), icon: ACCENT };
                let style = IconButtonStyle { normal: visual, hovered: visual, pressed: Some(visual), selected: visual,
                    corner_radius: egui::CornerRadius::same(4), focus_stroke: Stroke::new(2., ACCENT) };
                for (label, undo, enabled) in [("Undo", true, self.session.editor.can_undo() || self.focused_raw().is_some()), ("Redo", false, self.session.editor.can_redo() || self.focused_raw().is_some())] {
                    ui.add_enabled_ui(enabled, |ui| {
                        let response = IconButton::new(label, Vec2::new(44.,28.), &style).show(ui, |ui, rect, tint| {
                            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, if undo { "Undo" } else { "Redo" }, FontId::proportional(12.), tint);
                        });
                        self.bound(format!("action:{label}"),&response);
                        if response.clicked() { self.history(undo,history_field.clone()); }
                    });
                }
            });
            ui.horizontal(|ui| {
                ui.label("Open / import / recover / export");
                let response = ui.add(egui::TextEdit::singleline(&mut self.path).id_salt("document-path").desired_width(ui.available_width()).hint_text("/workspace/…/model.kernspan (or source/recovery path for Open / Recover)"));
                self.bound("path",&response);
            });
            ui.horizontal(|ui| {
                ui.label("Save native model to"); let response=ui.add(egui::TextEdit::singleline(&mut self.save_path).id_salt("save-path").desired_width(ui.available_width()).hint_text("/workspace/…/model.kernspan")); self.bound("save-path",&response);
            });
            if let Some(inventory)=self.recovery_inventory.as_ref(){if !inventory.entries.is_empty(){let label=format!("{} recognized recovery copies available in this app data directory",inventory.entries.len());ui.label(RichText::new(label).color(ACCENT));}}
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{:?} · model r{} · raw v{}", self.session.status(), self.session.editor.snapshot.model_revision, self.session.editor.snapshot.raw_version));
                let (native_recovery, layout_recovery) = self.periodic_recovery.status(&self.session, &self.views);
                ui.label(format!("Native recovery: {native_recovery} · layout recovery: {layout_recovery}"));
                ui.label(if self.views.pair_pending() { "Layout waiting for native receipt".into() }
                    else if self.views.io.pending() { "Layout write pending".into() }
                    else if let Some(error) = &self.views.io.last_error { format!("Layout write failed: {error}; live layout retained") }
                    else if self.views.io.is_saved(&self.session, &self.views.views) { "Layout saved receipt (file rechecked before navigation)".into() }
                    else { "Layout not saved to bound sidecar".into() });
                ui.label(if self.preferences.value.periodic_recovery {format!("Periodic recovery on · {}s · {}",self.preferences.value.recovery_interval_seconds,if self.session.pending()||self.views.pair_pending()||self.views.io.pending(){"write pending"}else{"waiting for quiet interval"})}else{"Periodic recovery off".into()});
                if self.preferences.pending() || self.preferences.read_only {ui.colored_label(if self.preferences.status.contains("failed")||self.preferences.read_only{ERROR}else{INK},&self.preferences.status);}
                if self.preferences.load_warning{ui.colored_label(ERROR,"Settings load needs attention; open Recovery settings for the retained report.");}
                ui.label(RichText::new("App-owned model · restricted SysML subset").small());
            });
        });
    }
    fn history(&mut self, undo: bool, field: Option<RawKey>) {
        use mbse_model::native_views::{NativeAction, NativeOutcome};
        let action = match (undo, field.clone()) {
            (true, Some(key)) => NativeAction::TextUndo(key), (false, Some(key)) => NativeAction::TextRedo(key),
            (true, None) => NativeAction::ModelUndo, (false, None) => NativeAction::ModelRedo,
        };
        let stamp = self.views.views.stamp(&self.session.editor);
        let result = self.views.views.dispatch(&mut self.session.editor, &stamp, action).and_then(|outcome| match outcome {
            NativeOutcome::History(changed) => Ok(changed), _ => Err("Unexpected command outcome".into()),
        });
        self.views.reconcile();
        let text_history = field.is_some();
        self.result(result.map(|changed| {if text_history {if changed {"Field text history applied; accepted values and raw drafts retain their separate state.".into()}else{"No field text history available.".into()}}else if changed {"Model history applied. Raw drafts remain separately retained.".into()}else{"No model history available.".into()}}));
    }
    fn sidebar(&mut self, root: &mut egui::Ui) {
        egui::Panel::left("model_navigation").default_size(338.).size_range(290. ..=420.).resizable(true).show(root, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Model");
                let records: Vec<_> = self.session.editor.snapshot.records.values().cloned().collect();
                let selected = self.session.editor.selected;
                egui::CollapsingHeader::new(format!("Records · {}", records.len())).default_open(true).show(ui, |ui| {
                    for record in &records {
                        let response = ui.selectable_label(selected == Some(record.id), format!("{} · {}", record.kind.label(), if record.name.is_empty() { "Unnamed" } else { &record.name }));
                        self.bound(format!("tree:{}",record.id.0),&response);
                        if response.clicked() { self.select(record.id); }
                    }
                });
                ui.separator();
                if let Some(record) = selected.and_then(|id| records.iter().find(|r| r.id == id)).cloned() {
                    ui.label(RichText::new(format!("{} #{}", record.kind.label(), record.id.0)).color(ACCENT));
                    let response=ui.button("Find usages");self.bound("action:Find usages",&response);if response.clicked(){self.open_usages(record.id);}
                    self.inspector(ui, &record, &records);
                }
                ui.separator(); ui.label(RichText::new("Add a typed record").strong());
                ui.horizontal_wrapped(|ui| {
                    for kind in [RecordKind::Mission, RecordKind::Capability, RecordKind::Objective, RecordKind::Measure, RecordKind::Actor, RecordKind::Stakeholder, RecordKind::Environment, RecordKind::Organization, RecordKind::Expectation] {
                        let owner = if kind == RecordKind::Measure { selected.filter(|id| records.iter().any(|r| r.id == *id && r.kind == RecordKind::Objective)) }
                            else if kind == RecordKind::Expectation { selected.filter(|id| records.iter().any(|r| r.id == *id && r.kind == RecordKind::Stakeholder)) }
                            else { selected.and_then(|id| records.iter().find(|r| r.id == id).map(|r| r.scope)) };
                        let label = format!("Add {}", kind.label());
                        let response = ui.add_enabled(owner.is_some(), egui::Button::new(&label));
                        self.bound(format!("action:{label}"),&response);
                        if response.on_hover_text(if kind == RecordKind::Measure { "Select the Objective this Measure assesses first." } else if kind == RecordKind::Expectation { "Select the Stakeholder expressing this expectation first." } else { "Create a record in this System." }).clicked() {
                            let owner = owner.expect("enabled owner");
                            let draft_id = format!("mbse-ui-add-{}-{}", kind.label(), owner.0);
                            let text = self.session.editor.snapshot.creation_drafts.get(&draft_id).and_then(|d| d.fields.get("name")).cloned().unwrap_or_default();
                            self.creation = Some((kind, owner, text));
                        }
                    }
                });
                ui.label(RichText::new("Select an Objective to Add Measure; select a Stakeholder to Add Expectation.").small());
            });
        });
    }
    fn inspector(&mut self, ui: &mut egui::Ui, record: &Record, records: &[Record]) {
        self.field(ui, record, "name", "Name", false);
        if record.id != record.scope {
            let response = ui.button(format!("System scope #{}", record.scope.0));
            self.bound(format!("scope:{}", record.id.0), &response);
            if response.clicked() {
                self.select(record.scope);
            }
        }
        match record.kind {
            RecordKind::System => self.field(ui, record, "purpose", "Purpose", true),
            RecordKind::Mission => {
                ui.label("Mission: intended accomplishment. Naming does not infer a goal, owner or classifier.");
            }
            RecordKind::Objective => {
                self.field(ui, record, "outcome", "Desired outcome", true);
                ui.label(RichText::new("Subject").strong());
                let subject_combo =
                    egui::ComboBox::from_id_salt(("objective-subject", record.id.0))
                        .selected_text(subject_caption(record.subject(), records))
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(record.subject().is_none(), "No subject")
                                .clicked()
                            {
                                self.command(Command::SetSubject {
                                    objective: record.id,
                                    subject: None,
                                });
                            }
                            for target in records.iter().filter(|r| r.scope == record.scope) {
                                let subject = match target.kind {
                                    RecordKind::System => Some(SubjectRef::System(target.id)),
                                    RecordKind::Mission => Some(SubjectRef::Mission(target.id)),
                                    RecordKind::Capability => {
                                        Some(SubjectRef::Capability(target.id))
                                    }
                                    _ => None,
                                };
                                if let Some(subject) = subject {
                                    let response = ui.selectable_label(
                                        record.subject() == Some(subject),
                                        format!(
                                            "{} · {} #{}",
                                            target.kind.label(),
                                            target.name,
                                            target.id.0
                                        ),
                                    );
                                    self.bound(
                                        format!("subject-option:{}:{}", record.id.0, target.id.0),
                                        &response,
                                    );
                                    if response.clicked() {
                                        self.command(Command::SetSubject {
                                            objective: record.id,
                                            subject: Some(subject),
                                        });
                                    }
                                }
                            }
                        });
                self.bound(format!("subject:{}", record.id.0), &subject_combo.response);
            }
            RecordKind::Capability => {
                ui.label(RichText::new("Contributes to missions").strong());
                for mission in records
                    .iter()
                    .filter(|r| r.kind == RecordKind::Mission && r.scope == record.scope)
                {
                    let mut linked = record.links().contains(&mission.id);
                    let response =
                        ui.checkbox(&mut linked, format!("{} #{}", mission.name, mission.id.0));
                    self.bound(
                        format!("mission-link:{}:{}", record.id.0, mission.id.0),
                        &response,
                    );
                    if response.changed() {
                        self.command(Command::ToggleMission {
                            capability: record.id,
                            mission: mission.id,
                        });
                    }
                }
            }
            RecordKind::Measure => {
                ui.label(RichText::new("Quantity and unit").strong());
                let measure_unit = egui::ComboBox::from_id_salt(("measure-unit", record.id.0))
                    .selected_text(format!(
                        "{} · {}",
                        record.text("quantity"),
                        record.text("unit")
                    ))
                    .show_ui(ui, |ui| {
                        for (quantity, unit, label) in [
                            ("dimensionless", "percent", "Ratio · %"),
                            ("dimensionless", "one", "Scalar · 1"),
                            ("length", "metre", "Length · m"),
                        ] {
                            let response = ui.selectable_label(record.text("unit") == unit, label);
                            self.bound(
                                format!("measure-unit-option:{}:{}", record.id.0, unit),
                                &response,
                            );
                            if response.clicked() {
                                self.command(Command::SetQuantity {
                                    measure: record.id,
                                    quantity: quantity.into(),
                                    unit: unit.into(),
                                });
                            }
                        }
                    });
                self.bound(
                    format!("measure-unit:{}", record.id.0),
                    &measure_unit.response,
                );
                self.field(ui, record, "criterion", "Target criterion", false);
                ui.label(format!(
                    "Accepted criterion: {} {}",
                    record
                        .number("criterion")
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "absent".into()),
                    record.text("unit")
                ));
                ui.collapsing("For Objective", |ui| {
                    for objective in records
                        .iter()
                        .filter(|r| r.kind == RecordKind::Objective && r.scope == record.scope)
                    {
                        if ui
                            .selectable_label(
                                record
                                    .fields
                                    .get("forObjective")
                                    .and_then(serde_json::Value::as_u64)
                                    == Some(objective.id.0),
                                format!("{} #{}", objective.name, objective.id.0),
                            )
                            .clicked()
                        {
                            self.command(Command::SetForObjective {
                                measure: record.id,
                                objective: objective.id,
                            });
                        }
                    }
                });
                ui.collapsing("Evidence and context", |ui| {
                    for (field, label) in [
                        ("assessmentMethod", "Assessment method"),
                        ("conditions", "Conditions"),
                        ("uncertainty", "Uncertainty"),
                    ] {
                        self.field(ui, record, field, label, true);
                    }
                });
                self.calculation(ui, record);
                ui.label(RichText::new("Custom Measure. MOE/MOP metadata does not imply SysML classifier equivalence.").small());
            }
            RecordKind::Stakeholder => {
                self.field(ui, record, "interest", "Interest in this System", true);
                self.affiliations(ui, record, records);
                if record
                    .fields
                    .get("actors")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|a| !a.is_empty())
                {
                    ui.label("Existing actor-role data is preserved. Its editing semantics are outside this profile.");
                }
            }
            RecordKind::Environment => self.contexts(ui, record),
            RecordKind::Organization => {
                ui.label("Affiliated stakeholders");
                let key = record.id.0.to_string();
                let mut count = 0;
                for stakeholder in records
                    .iter()
                    .filter(|r| r.kind == RecordKind::Stakeholder && r.scope == record.scope)
                {
                    if let Some(kind) = stakeholder
                        .fields
                        .get("affiliations")
                        .and_then(|v| v.get(&key))
                        .and_then(serde_json::Value::as_str)
                    {
                        count += 1;
                        let response = ui.button(format!(
                            "{} #{} · {}",
                            stakeholder.name, stakeholder.id.0, kind
                        ));
                        self.bound(
                            format!("affiliation-source:{}:{}", record.id.0, stakeholder.id.0),
                            &response,
                        );
                        if response.clicked() {
                            self.select(stakeholder.id);
                        }
                    }
                }
                if count == 0 {
                    ui.label(
                        "No authored affiliations. Select a Stakeholder to declare an affiliation.",
                    );
                }
                ui.label("This Organization retains its own identity and System scope. Additional source properties remain preserved.");
            }
            RecordKind::Actor => {
                ui.label("Named actor in this System's scope. No role or interaction is inferred from its name.");
            }
            RecordKind::Expectation => self.field(ui, record, "text", "Expectation", true),
        }
    }
    fn affiliations(&mut self, ui: &mut egui::Ui, record: &Record, records: &[Record]) {
        let organizations: Vec<_> = records
            .iter()
            .filter(|r| r.kind == RecordKind::Organization && r.scope == record.scope)
            .collect();
        ui.label(
            RichText::new(format!(
                "Affiliations · {} eligible organizations",
                organizations.len()
            ))
            .strong(),
        );
        if record
            .fields
            .get("affiliations")
            .is_some_and(|v| !v.is_object())
        {
            ui.label("Existing affiliation data is preserved but cannot be edited as this profile's affiliation map.");
            return;
        }
        for org in organizations {
            let current = record
                .fields
                .get("affiliations")
                .and_then(|v| v.get(org.id.0.to_string()))
                .and_then(serde_json::Value::as_str);
            ui.label(format!("{} #{}", org.name, org.id.0));
            let combo = egui::ComboBox::from_id_salt(("affiliation", record.id.0, org.id.0))
                .selected_text(current.unwrap_or("No affiliation"))
                .show_ui(ui, |ui| {
                    for choice in [
                        None,
                        Some(AffiliationKind::Unknown),
                        Some(AffiliationKind::Internal),
                        Some(AffiliationKind::External),
                    ] {
                        let label = choice.map(|c| c.label()).unwrap_or("No affiliation");
                        let response =
                            ui.selectable_label(current == choice.map(|c| c.label()), label);
                        self.bound(
                            format!("affiliation-option:{}:{}:{}", record.id.0, org.id.0, label),
                            &response,
                        );
                        if response.clicked() {
                            self.command(Command::SetAffiliation {
                                stakeholder: record.id,
                                organization: org.id,
                                relationship: choice,
                            });
                        }
                    }
                });
            self.bound(
                format!("affiliation:{}:{}", record.id.0, org.id.0),
                &combo.response,
            );
        }
        ui.label(
            RichText::new(
                "No affiliation is absent; Unknown is an explicitly authored relationship.",
            )
            .small(),
        );
    }
    fn contexts(&mut self, ui: &mut egui::Ui, record: &Record) {
        ui.label(RichText::new("Environment context").strong());
        let items = record
            .fields
            .get("context")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        for item in items {
            let Some(id) = item
                .get("id")
                .and_then(serde_json::Value::as_u64)
                .map(RecordId)
            else {
                ui.label("Unrecognized context item preserved.");
                continue;
            };
            let title = format!(
                "{} #{}",
                item.get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Context"),
                id.0
            );
            egui::CollapsingHeader::new(title)
                .id_salt(("context", record.id.0, id.0))
                .default_open(true)
                .show(ui, |ui| {
                    self.raw_field(ui, record, Some(id), "contextName", "Context name", false);
                    let current = item
                        .get("kind")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Unrecognized kind");
                    let combo = egui::ComboBox::from_id_salt(("context-kind", record.id.0, id.0))
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for kind in [
                                ContextKind::Description,
                                ContextKind::Assumption,
                                ContextKind::AcceptedConstraint,
                            ] {
                                let response =
                                    ui.selectable_label(current == kind.label(), kind.label());
                                self.bound(
                                    format!(
                                        "context-kind-option:{}:{}:{}",
                                        record.id.0,
                                        id.0,
                                        kind.label()
                                    ),
                                    &response,
                                );
                                if response.clicked() {
                                    self.command(Command::SetContextKind {
                                        environment: record.id,
                                        item: id,
                                        kind,
                                    });
                                }
                            }
                        });
                    self.bound(
                        format!("context-kind:{}:{}", record.id.0, id.0),
                        &combo.response,
                    );
                    self.raw_field(ui, record, Some(id), "contextText", "Context text", true);
                });
        }
        let response = ui.button("Add context");
        self.bound(format!("context:add:{}", record.id.0), &response);
        if response.clicked() {
            let draft = self
                .session
                .editor
                .snapshot
                .creation_drafts
                .get(&format!("mbse-ui-context-{}", record.id.0));
            let name = draft
                .and_then(|d| d.fields.get("name"))
                .cloned()
                .unwrap_or_default();
            let kind = match draft.and_then(|d| d.fields.get("kind")).map(String::as_str) {
                Some("Assumption") => ContextKind::Assumption,
                Some("AcceptedConstraint") => ContextKind::AcceptedConstraint,
                _ => ContextKind::Description,
            };
            self.context_creation = Some((record.id, name, kind));
        }
        ui.label(RichText::new("Context classifications are authored statements. Choosing AcceptedConstraint does not record an independent engineering approval.").small());
        if record
            .fields
            .get("inputs")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|a| !a.is_empty())
        {
            ui.label(
                "Existing external inputs remain preserved; no external connection is inferred.",
            );
        }
    }
    fn calculation(&mut self, ui: &mut egui::Ui, record: &Record) {
        ui.separator();
        ui.label(RichText::new("Local calculation").strong());
        match self.session.editor.evaluate_calculations() {
            Ok(receipt) => {
                if let Some(outcome) = receipt.outcomes.get(&CalculationId(record.id.0)) {
                    match outcome {
                        calc::Outcome::Value(value) => {
                            ui.colored_label(
                                ACCENT,
                                format!(
                                    "Derived result: {} {}",
                                    value.value,
                                    calc::unit_label(value.quantity.unit)
                                ),
                            );
                        }
                        calc::Outcome::Failure(error) => {
                            ui.colored_label(ERROR, format!("{:?}: {}", error.code, error.message));
                        }
                    }
                    ui.label(
                        RichText::new(
                            "Derived result is separate from criterion and observations.",
                        )
                        .small(),
                    );
                }
            }
            Err(error) => {
                ui.colored_label(ERROR, error);
            }
        }
        let definitions = self
            .session
            .editor
            .snapshot
            .calculations()
            .unwrap_or_default();
        if let Some(definition) = definitions.values().find(|d| d.owner == record.id) {
            self.field(ui, record, "calc.expression", "Expression", false);
            ui.label(format!(
                "Accepted expression: {}",
                calc::format_expression(&definition.expression)
            ));
            for (name, input) in &definition.bindings {
                match input {
                    calc::Input::Literal(literal) => {
                        self.field(
                            ui,
                            record,
                            &format!("calc.input.{name}"),
                            &format!("Input {name}"),
                            false,
                        );
                        let combo = egui::ComboBox::from_id_salt(("input-unit", record.id.0, name))
                            .selected_text(calc::unit_label(literal.quantity.unit))
                            .show_ui(ui, |ui| {
                                for unit in all_units()
                                    .into_iter()
                                    .filter(|u| calc::unit_kind(*u) == literal.quantity.kind)
                                {
                                    let response = ui.selectable_label(
                                        unit == literal.quantity.unit,
                                        calc::unit_label(unit),
                                    );
                                    self.bound(
                                        format!(
                                            "input-unit-option:{}:{}:{}",
                                            record.id.0,
                                            name,
                                            calc::unit_key(unit)
                                        ),
                                        &response,
                                    );
                                    if response.clicked() {
                                        self.command(Command::SetCalculationInputUnit {
                                            measure: record.id,
                                            binding: name.clone(),
                                            unit,
                                        });
                                    }
                                }
                            });
                        self.bound(
                            format!("input-unit:{}:{}", record.id.0, name),
                            &combo.response,
                        );
                    }
                    calc::Input::Output(id) => {
                        ui.label(format!("Input {name}: derived output #{}", id.0));
                    }
                }
            }
            let output = egui::ComboBox::from_id_salt(("output-unit", record.id.0))
                .selected_text(format!(
                    "Output: {}",
                    calc::unit_label(definition.output.unit)
                ))
                .show_ui(ui, |ui| {
                    for unit in all_units() {
                        let response = ui.selectable_label(
                            unit == definition.output.unit,
                            calc::unit_label(unit),
                        );
                        self.bound(
                            format!(
                                "output-unit-option:{}:{}",
                                record.id.0,
                                calc::unit_key(unit)
                            ),
                            &response,
                        );
                        if response.clicked() {
                            let mut updated = definition.clone();
                            updated.output = QuantityType {
                                kind: calc::unit_kind(unit),
                                unit,
                            };
                            self.command(Command::PutCalculation {
                                measure: record.id,
                                definition: Some(updated),
                            });
                        }
                    }
                });
            self.bound(format!("output-unit:{}", record.id.0), &output.response);
            ui.horizontal_wrapped(|ui| {
                for (label, unit) in [
                    ("Add scalar input", UnitId::ScalarOne),
                    ("Add length input", UnitId::Metre),
                    ("Add duration input", UnitId::Second),
                    ("Add count input", UnitId::Count),
                ] {
                    let response = ui.button(label);
                    self.bound(format!("calc:{label}"), &response);
                    if response.clicked() {
                        let mut updated = definition.clone();
                        let name = next_binding(&updated.bindings);
                        updated.bindings.insert(
                            name,
                            calc::Input::Literal(calc::Literal {
                                decimal: "0".into(),
                                quantity: QuantityType {
                                    kind: calc::unit_kind(unit),
                                    unit,
                                },
                            }),
                        );
                        self.command(Command::PutCalculation {
                            measure: record.id,
                            definition: Some(updated),
                        });
                    }
                }
            });
            ui.collapsing("Use another calculation output", |ui| {
                for target in definitions.values().filter(|d| d.owner != record.id) {
                    if ui
                        .button(format!("Link output #{} as input", target.id.0))
                        .clicked()
                    {
                        let mut updated = definition.clone();
                        let name = next_binding(&updated.bindings);
                        updated
                            .bindings
                            .insert(name, calc::Input::Output(target.id));
                        self.command(Command::PutCalculation {
                            measure: record.id,
                            definition: Some(updated),
                        });
                    }
                }
            });
            if ui.button("Remove calculation").clicked() {
                self.command(Command::PutCalculation {
                    measure: record.id,
                    definition: None,
                });
            }
        } else {
            ui.label("Create a bounded expression. Bind named inputs to explicit units; bare numbers are dimensionless. Input x starts in this Measure’s unit.");
            let response = ui.button("Add calculation");
            self.bound("calc:Add calculation", &response);
            if response.clicked() {
                let unit = match record.text("unit") {
                    "percent" => UnitId::Percent,
                    "metre" => UnitId::Metre,
                    _ => UnitId::ScalarOne,
                };
                let expression = calc::parse("x", &Limits::default()).expect("binding expression");
                self.command(Command::PutCalculation {
                    measure: record.id,
                    definition: Some(Definition {
                        id: CalculationId(record.id.0),
                        owner: record.id,
                        bindings: BTreeMap::from([(
                            "x".into(),
                            calc::Input::Literal(calc::Literal {
                                decimal: "0".into(),
                                quantity: QuantityType {
                                    kind: calc::unit_kind(unit),
                                    unit,
                                },
                            }),
                        )]),
                        expression,
                        output: QuantityType {
                            kind: calc::unit_kind(unit),
                            unit,
                        },
                    }),
                });
            }
        }
    }
    fn canvas(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().show(root, |ui| {
            ui.heading("Native views");
            if let Some(record) = self.views.toolbar(ui, &self.session) { self.select(record); }
            let records: Vec<_> = self.session.editor.snapshot.records.values().cloned().collect();
            let graph = match stage1_graph::derive_graph(&self.session.editor.snapshot) {
                Ok(graph) => graph, Err(error) => { ui.colored_label(ERROR, error); return; }
            };
            ui.checkbox(&mut self.show_ownership, "Show derived ownership links");
            ui.label("Drag or move an occurrence; hide/remove affects this presentation. Model fields remain in the inspector. Semantic Delete is unavailable.");
            ui.label("Persisted edges keep exact occurrence endpoints. Other arrows are derived only when both visible endpoints are unique; the complete ledger remains authoritative. Bend-point routing is unavailable.");
            self.views.reconcile();
            let Some(view) = self.views.views.document().views.get(&self.views.active) else {
                ui.colored_label(ERROR, "No valid view available; model retained."); return;
            };
            let camera = view.camera.clone();
            let occurrences: Vec<_> = self.views.views.document().occurrences.values().filter(|o| o.view == self.views.active && o.visible && o.presence.active()).cloned().collect();
            let attached: Vec<_> = self.views.views.document().edges.values().filter(|e| e.view == self.views.active && e.presence.active()).cloned().collect();
            let scale = camera.zoom as f32;
            let canvas_height = (ui.available_height() * 0.60).max(260.);
            egui::ScrollArea::both().id_salt(("native-view", self.views.active.get())).max_height(canvas_height).auto_shrink([false, false]).show(ui, |ui| {
                let width = occurrences.iter().map(|o| (o.geometry.x - camera.x + o.geometry.width + 50.) as f32 * scale).fold(ui.available_width().max(400.), f32::max).clamp(400., 32768.);
                let height = occurrences.iter().map(|o| (o.geometry.y - camera.y + o.geometry.height + 50.) as f32 * scale).fold(350., f32::max).clamp(350., 32768.);
                let (area, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
                let rects: BTreeMap<_, _> = occurrences.iter().map(|o| (o.id, Rect::from_min_size(area.min + Vec2::new((o.geometry.x - camera.x) as f32, (o.geometry.y - camera.y) as f32) * scale, Vec2::new(o.geometry.width as f32, o.geometry.height as f32) * scale))).collect();
                let mut by_record = BTreeMap::<RecordId, Vec<mbse_model::native_views::OccurrenceId>>::new();
                for o in &occurrences { by_record.entry(o.record).or_default().push(o.id); }
                let mut arrows = Vec::new();
                for e in &attached { if let (Some(a), Some(b)) = (rects.get(&e.source), rects.get(&e.target)) { arrows.push((*a, *b, self.relation.is_some_and(|r| r.source == e.key.owner && r.target == e.key.target))); } }
                for edge in &graph.edges {
                    if edge.id.role == stage1_graph::RelationRole::Scope && !self.show_ownership { continue; }
                    if attached.iter().any(|e| e.key.owner == edge.id.source && e.key.target == edge.id.target) { continue; }
                    if let (Some(a), Some(b)) = (by_record.get(&edge.id.source), by_record.get(&edge.id.target)) {
                        if a.len() == 1 && b.len() == 1 { arrows.push((rects[&a[0]], rects[&b[0]], self.relation == Some(edge.id))); }
                    }
                }
                for (a, b, selected) in arrows { let start = a.center_bottom(); let end = b.center_top(); ui.painter().arrow(start, end - start, Stroke::new(if selected { 3. } else { 1. }, ACCENT)); }
                for occurrence in &occurrences {
                    let Some(record) = self.session.editor.snapshot.records.get(&occurrence.record).cloned() else { continue; };
                    let rect = rects[&occurrence.id];
                    let selected = self.views.selected == Some(occurrence.id);
                    ui.painter().rect(rect, 7., PAPER, Stroke::new(if selected { 2.5 } else { 1. }, if selected { ACCENT } else { LINE }), egui::StrokeKind::Inside);
                    ui.painter().text(rect.min + Vec2::new(14., 13.) * scale, egui::Align2::LEFT_TOP, format!("{} #{} · occurrence {}", record.kind.label(), record.id.0, occurrence.id.get()), FontId::proportional(12. * scale), ACCENT);
                    let name = if record.name.is_empty() { "Unnamed" } else { &record.name };
                    let galley = ui.painter().layout(name.into(), FontId::proportional(17. * scale), INK, (rect.width() - 28. * scale).max(1.));
                    ui.painter().with_clip_rect(rect.shrink(5.)).galley(rect.min + Vec2::new(14., 36.) * scale, galley, INK);
                    let response = ui.interact(Rect::from_min_max(rect.min, rect.max - Vec2::new(0., 28. * scale)), ui.id().with(("occurrence", occurrence.id.get())), egui::Sense::click_and_drag());
                    self.bound(format!("occurrence:{}", occurrence.id.get()), &response);
                    self.views.drag(&self.session.editor, occurrence.id, &response, scale);
                    if response.clicked() { self.views.selected = Some(occurrence.id); self.select(record.id); }
                    let details = Rect::from_min_size(rect.left_bottom() + Vec2::new(12. * scale, -27. * scale), Vec2::new(90., 22.) * scale);
                    if ui.put(details, egui::Button::new("Details…")).clicked() { self.popout = Some(record.id); }
                }
            });
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "Typed relations · {} · all links remain listed regardless of canvas density",
                    graph.edges.len()
                ))
                .strong(),
            );
            egui::ScrollArea::vertical()
                .id_salt("relation-ledger")
                .show(ui, |ui| {
                    for edge in &graph.edges {
                        let response = ui.selectable_label(
                            self.relation == Some(edge.id),
                            relation_label(edge, &records),
                        );
                        self.bound(
                            format!(
                                "relation:{:?}:{}:{}",
                                edge.id.role, edge.id.source.0, edge.id.target.0
                            ),
                            &response,
                        );
                        if response.clicked() {
                            self.relation = Some(edge.id);
                            if edge.id.role==stage1_graph::RelationRole::Scope { self.show_ownership=true; }
                        }
                    }
                    if graph.edges.is_empty() {
                        ui.label("Add records and typed links to populate the relation ledger.");
                    }
                });
        });
    }
    fn settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.settings_window {
            return;
        }
        let mut open = true;
        let before = self.preferences.value.clone();
        egui::Window::new("Recovery settings").id(egui::Id::new("recovery-settings")).open(&mut open).default_width(480.).show(ctx,|ui|{
            ui.label("Application-local recovery copies; never an automatic save to your source file.");let response=ui.checkbox(&mut self.preferences.value.periodic_recovery,"Enable periodic recovery");self.bound("settings:periodic",&response);
            let combo=egui::ComboBox::from_id_salt("recovery-interval").selected_text(format!("Every {} seconds",self.preferences.value.recovery_interval_seconds)).show_ui(ui,|ui|{for seconds in [15,30,60,120,300,600,3600]{let response=ui.selectable_value(&mut self.preferences.value.recovery_interval_seconds,seconds,format!("{seconds} seconds"));self.bound(format!("settings:interval:{seconds}"),&response);}});self.bound("settings:interval",&combo.response);
            ui.label("Native or layout changes are copied after a quiet period and the interval. Pending writes finish independently; newer edits wait for another recovery copy.");
            ui.label(&self.preferences.report);ui.label(&self.preferences.status);if self.preferences.pending(){ui.label("Preference write pending; the GUI remains responsive.");}
            ui.label(format!("Recovery data: {}",self.session.state_root.display()));
        });
        self.settings_window = open;
        if self.preferences.value != before {
            self.periodic_recovery.reset_interval(Instant::now());
            self.preferences.changed();
        }
    }
    fn recovery_dialog(&mut self, ctx: &egui::Context) {
        if !self.recovery_window {
            return;
        }
        let mut open = true;
        let mut restore = None;
        egui::Window::new("Recovery copies").id(egui::Id::new("recovery-inventory")).open(&mut open).default_width(650.).show(ctx,|ui| {
            ui.label("Recognized recovery envelopes in this app data directory. Choose explicitly; no copy is restored automatically.");
            let response=ui.button("Refresh recovery inventory");self.bound("recovery:Refresh",&response);if response.clicked(){self.refresh_recoveries();}
            let entries:Vec<_>=self.recovery_inventory.as_ref().map(|i|i.entries.iter().map(|e|(e.path.clone(),e.system_names.join(", "),e.model_revision,e.raw_version,e.record_count,e.snapshot_hash.clone())).collect()).unwrap_or_default();
            egui::ScrollArea::vertical().max_height(300.).show(ui,|ui| {
                for (index,(path,names,revision,raw,count,_)) in entries.iter().enumerate(){
                    let response=ui.selectable_label(self.recovery_selected.as_ref()==Some(path),format!("{} · model r{} / raw v{} · {} records\n{}",if names.is_empty(){"Unnamed System"}else{names},revision,raw,count,path.display()));self.bound(format!("recovery:entry:{index}"),&response);if response.clicked(){self.recovery_selected=Some(path.clone());}
                }
                if entries.is_empty(){ui.label("No recognized recovery copies found.");}
            });
            if let Some((path,_,revision,raw,_,hash))=entries.iter().find(|e|Some(&e.0)==self.recovery_selected.as_ref()){
                ui.label(format!("Selected snapshot r{revision} / raw v{raw} · hash {hash}"));
                let response=ui.button("Restore independent copy");self.bound("recovery:Restore independent copy",&response);if response.clicked(){restore=Some(path.clone());}
            }
            if let Some(inventory)=self.recovery_inventory.as_ref(){
                if inventory.truncated{ui.colored_label(ERROR,"Inventory scan reached its bound. Some copies may be omitted; no files were deleted.");}
                ui.collapsing(format!("Rejected entries ({})",inventory.rejected.len()),|ui|{for rejected in &inventory.rejected{ui.label(format!("{}: {}",rejected.path.display(),rejected.reason));}});
            }
            ui.label("Restore preserves the source, opens a dirty independent document and requires a new save destination.");
        });
        self.recovery_window = open;
        if let Some(path) = restore {
            self.recovery_window = false;
            self.request_load(Load::Recover(path));
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        self.recovery_dialog(ctx);
        self.settings_dialog(ctx);
        if let Some((environment, mut name, mut kind)) = self.context_creation.clone() {
            let mut open = true;
            let mut create = false;
            let before = (name.clone(), kind);
            egui::Window::new("Add environment context")
                .id(egui::Id::new("context-creation"))
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Environment #{} · System scope retained",
                        environment.0
                    ));
                    ui.label("Context name");
                    let response = ui.text_edit_singleline(&mut name);
                    self.bound("context-creation:name", &response);
                    egui::ComboBox::from_id_salt("new-context-kind")
                        .selected_text(kind.label())
                        .show_ui(ui, |ui| {
                            for choice in [
                                ContextKind::Description,
                                ContextKind::Assumption,
                                ContextKind::AcceptedConstraint,
                            ] {
                                ui.selectable_value(&mut kind, choice, choice.label());
                            }
                        });
                    ui.label("The draft is retained when this panel is closed.");
                    let response = ui
                        .add_enabled(!name.trim().is_empty(), egui::Button::new("Create context"));
                    self.bound("context-creation:commit", &response);
                    if response.clicked() {
                        create = true;
                    }
                });
            if before != (name.clone(), kind) {
                self.command(Command::PutCreationDraft {
                    id: format!("mbse-ui-context-{}", environment.0),
                    draft: CreationDraft {
                        owner: environment,
                        kind: RecordKind::Environment,
                        fields: BTreeMap::from([
                            ("name".into(), name.clone()),
                            ("kind".into(), kind.label().into()),
                        ]),
                    },
                });
            }
            self.context_creation = if open && !create {
                Some((environment, name.clone(), kind))
            } else {
                None
            };
            if create {
                self.command(Command::AddContext {
                    environment,
                    name,
                    kind,
                });
            }
        }
        if let Some((kind, owner, mut name)) = self.creation.clone() {
            let mut open = true;
            let mut create = false;
            egui::Window::new(format!("Add {}", kind.label())).id(egui::Id::new("create-record")).open(&mut open).collapsible(false).show(ctx, |ui| {
                ui.label(format!("Owner #{} · explicit typed scope", owner.0));
                ui.label("Name");
                let response = ui.add(egui::TextEdit::singleline(&mut name).id_salt("creation-name").desired_width(350.));
                self.bound("creation:name",&response);
                if response.changed() {
                    self.command(Command::PutCreationDraft { id: format!("mbse-ui-add-{}-{}",kind.label(),owner.0), draft: CreationDraft { kind, owner, fields: BTreeMap::from([("name".into(), name.clone())]) } });
                }
                ui.label("Unfinished text is retained when you close this panel. Reopen Add to continue.");
                let response = ui.add_enabled(!name.trim().is_empty(), egui::Button::new(format!("Create {}",kind.label())));
                self.bound("creation:commit",&response);
                if response.clicked() { create = true; }
            });
            self.creation = if open && !create {
                Some((kind, owner, name.clone()))
            } else {
                None
            };
            if create {
                self.command(Command::Add { kind, owner, name });
            }
        }
        if let Some(edge) = self.relation {
            let mut open = true;
            egui::Window::new("Typed relation").open(&mut open).show(ctx, |ui| {
                ui.label(format!("{:?}", edge.role));
                ui.label(format!("Source #{} -> Target #{}", edge.source.0, edge.target.0));
                for (label,id) in [("Select source",edge.source),("Select target",edge.target)] { if ui.button(label).clicked() { self.select(id); } }
                ui.label("This view displays authored references. Canvas proximity creates no semantic relation.");
            });
            if !open {
                self.relation = None;
            }
        }
        if let Some(id) = self.popout {
            let mut open = true;
            if let Some(record) = self.session.editor.snapshot.records.get(&id).cloned() {
                egui::Window::new(format!("{} #{}", record.kind.label(), id.0))
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.heading(&record.name);
                        ui.label(format!("System scope #{}", record.scope.0));
                        if ui.button("Edit in left controls").clicked() {
                            self.select(id);
                        }
                        ui.collapsing("Preserved record properties", |ui| {
                            ui.monospace(
                                serde_json::to_string_pretty(&record.fields).unwrap_or_default(),
                            );
                        });
                    });
            } else {
                open = false;
            }
            if !open {
                self.popout = None;
            }
        }
        if let Some(action) = self.pending.clone() {
            egui::Window::new("Preserve current work").collapsible(false).resizable(false).show(ctx,|ui|{
                ui.label("Current model, raw drafts and views remain in this document.");
                ui.label(&self.views.note);ui.label("Continue only after a current save or recovery is verified. Cancel keeps this document open.");
                if self.session.pending(){ui.label("A frozen snapshot write is pending. New edits stay separate from its acknowledgment.");}
                let response=ui.add_enabled(!self.session.pending() && !self.views.io.pending(),egui::Button::new("Save and continue"));self.bound("preserve:Save and continue",&response);if response.clicked()&&self.save(){self.pending_continue=Some((action.clone(),WriteKind::Save));}
                let response=ui.add_enabled(!self.session.pending() && !self.views.io.pending(),egui::Button::new("Keep recovery and continue"));self.bound("preserve:Keep recovery and continue",&response);if response.clicked()&&self.recovery(){self.pending_continue=Some((action.clone(),WriteKind::Recovery));}
                let response=ui.button("Cancel");self.bound("preserve:Cancel",&response);if response.clicked(){self.pending=None;self.pending_continue=None;self.allow_close=None;self.assurance.cancel_close();self.result(Ok("Navigation canceled; current work retained. A snapshot write already in progress may still finish.".into()));}
            });
        }
    }

    fn capture(&mut self, ctx: &egui::Context) {
        if self.capture.is_none() {
            return;
        }
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                if !self.screenshot_requested {
                    continue;
                }
                let path = self.capture.take().expect("capture configured");
                let result = (|| -> Result<String, String> {
                    let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    let buffer = image::RgbaImage::from_raw(
                        image.width() as u32,
                        image.height() as u32,
                        pixels,
                    )
                    .ok_or("Invalid image dimensions")?;
                    let file = std::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&path)
                        .map_err(|e| e.to_string())?;
                    image::DynamicImage::ImageRgba8(buffer)
                        .write_to(&mut std::io::BufWriter::new(file), image::ImageFormat::Png)
                        .map_err(|e| e.to_string())?;
                    Ok(format!("Native frame captured: {}", path.display()))
                })();
                self.result(result);
                eprintln!("{}", self.feedback);
            }
        }
        if self.frames > 4 && !self.screenshot_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.screenshot_requested = true;
        }
        ctx.request_repaint();
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum ShortcutTarget {
    Field(RawKey),
    Model,
    Widget,
}
fn shortcut_policy(
    field: Option<RawKey>,
    text_focus: bool,
    focus_transition: bool,
) -> ShortcutTarget {
    if focus_transition {
        ShortcutTarget::Widget
    } else if let Some(key) = field {
        ShortcutTarget::Field(key)
    } else if text_focus {
        ShortcutTarget::Widget
    } else {
        ShortcutTarget::Model
    }
}
fn block_close(
    requested: bool,
    dirty: bool,
    pending: bool,
    current: &str,
    permit: Option<&str>,
) -> bool {
    requested && (pending || (dirty && permit != Some(current)))
}
#[cfg(test)]
mod policies {
    use super::*;
    #[test]
    fn native_usages_navigation_preserves_dirty_target_and_refuses_stale_rows() {
        let area=tempfile::Builder::new().prefix("native-usages-ui-").tempdir_in(std::env::var_os("TMPDIR").expect("workspace TMPDIR")).unwrap();
        let session=Session::new(area.path()).unwrap();let ctx=egui::Context::default();let mut app=Workspace::new(session,None,None,craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),&ctx);
        app.command(Command::Add{kind:RecordKind::Mission,owner:RecordId(1),name:"Mission".into()});let mission=app.session.editor.snapshot.records.values().find(|r|r.kind==RecordKind::Mission).unwrap().id;
        app.select(RecordId(1));assert_eq!(app.session.editor.selected,Some(RecordId(1)));
        app.open_usages(RecordId(1));let index=app.usages.as_ref().unwrap().usages().iter().position(|u|u.referencing_record==mission).unwrap();
        let before=(app.session.editor.snapshot.digest(),app.views.views.document().digest().unwrap(),app.views.views.history_usage(),app.save_path.clone());app.navigate_usage(index);assert_eq!(app.session.editor.selected,Some(mission));assert_eq!((app.session.editor.snapshot.digest(),app.views.views.document().digest().unwrap(),app.views.views.history_usage(),app.save_path.clone()),before);
        app.command(Command::EditText{key:RawKey{record:mission,field:"name".into(),item:None},text:"Edited after report".into()});let selected=app.session.editor.selected;app.navigate_usage(index);assert!(app.error);assert!(app.feedback.contains("USAGES_STALE"));assert_eq!(app.session.editor.selected,selected);assert!(!app.session.pending());assert!(!app.views.pair_pending());
    }
    #[test]
    fn source_inspection_shortcuts_never_undo_or_save_the_live_target() {
        let tmp=std::env::var_os("TMPDIR").expect("workspace TMPDIR");let area=tempfile::Builder::new().prefix("inspection-shortcut-").tempdir_in(tmp).unwrap();
        let session=Session::new(area.path()).unwrap();let ctx=egui::Context::default();
        let mut app=Workspace::new(session,None,None,craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),&ctx);
        app.command(Command::EditText{key:RawKey{record:RecordId(1),field:"name".into(),item:None},text:"Retained target edit".into()});
        let dest=area.path().join("must-not-write.kernspan");app.save_path=dest.display().to_string();app.views.open_inspection();
        let before=(app.session.editor.snapshot.digest(),app.views.views.document().digest().unwrap());
        let modifiers=egui::Modifiers::COMMAND;
        let input=egui::RawInput{events:vec![egui::Event::ModifiersChanged(modifiers),egui::Event::Key{key:egui::Key::Z,physical_key:None,pressed:true,repeat:false,modifiers},egui::Event::Key{key:egui::Key::S,physical_key:None,pressed:true,repeat:false,modifiers}],..Default::default()};
        ctx.run_ui(input,|ui|app.handle_shortcuts(ui.ctx())).drop_without_applying_deltas();
        assert_eq!((app.session.editor.snapshot.digest(),app.views.views.document().digest().unwrap()),before);
        assert!(!app.session.pending());assert!(!app.views.pair_pending());assert!(!dest.exists());assert!(app.feedback.contains("Read-only source inspection"));
    }
    #[test]
    fn native_worker_start_failure_clears_only_own_pair_and_retains_live_state() {
        let root=std::env::var_os("TMPDIR").expect("workspace TMPDIR");
        let area=tempfile::Builder::new().prefix("native-start-").tempdir_in(root).unwrap();
        let session=Session::new(area.path()).unwrap();let ctx=egui::Context::default();
        let mut app=Workspace::new(session,None,None,craft_window::Choice::probe(craft_window::Prefix::new("MBSE_CRAFT")),&ctx);
        let before=app.views.views.document().digest().unwrap();
        let prepared=app.views.prepare_native(&app.session,WriteKind::Recovery,None).unwrap();
        let job=app.session.begin_recovery().unwrap();let path=job.ticket.destination.clone();
        assert!(!app.spawn_write_with(job,prepared,|_| Err(std::io::Error::other("injected native start refusal"))));
        assert!(!app.session.pending());assert!(!app.views.pair_pending());assert!(!app.views.io.pending());
        assert!(!path.exists());assert_eq!(app.views.views.document().digest().unwrap(),before);
        assert!(app.error);assert!(app.feedback.contains("did not start"));
        assert!(app.views.prepare_native(&app.session,WriteKind::Recovery,None).is_ok());
    }
    #[test]
    fn close_requires_current_preservation_even_while_hidden() {
        assert!(block_close(true, true, false, "new", Some("old")));
        assert!(block_close(true, false, true, "same", Some("same")));
        assert!(!block_close(true, true, false, "same", Some("same")));
        assert!(!block_close(false, true, true, "new", None));
    }
    #[test]
    fn ordinary_text_and_focus_transitions_do_not_undo_model() {
        let key = RawKey {
            record: RecordId(1),
            field: "purpose".into(),
            item: None,
        };
        assert_eq!(shortcut_policy(None, true, false), ShortcutTarget::Widget);
        assert_eq!(
            shortcut_policy(Some(key.clone()), true, true),
            ShortcutTarget::Widget
        );
        assert_eq!(
            shortcut_policy(Some(key.clone()), true, false),
            ShortcutTarget::Field(key)
        );
        assert_eq!(shortcut_policy(None, false, false), ShortcutTarget::Model);
    }
}
fn subject_caption(subject: Option<SubjectRef>, records: &[Record]) -> String {
    let id = match subject {
        Some(SubjectRef::System(id) | SubjectRef::Mission(id) | SubjectRef::Capability(id)) => id,
        None => return "Choose typed subject…".into(),
    };
    records
        .iter()
        .find(|r| r.id == id)
        .map(|r| format!("{} · {} #{}", r.kind.label(), r.name, id.0))
        .unwrap_or_else(|| format!("Unavailable subject #{}", id.0))
}
fn all_units() -> [UnitId; 10] {
    [
        UnitId::ScalarOne,
        UnitId::RatioOne,
        UnitId::Percent,
        UnitId::Count,
        UnitId::Metre,
        UnitId::Kilometre,
        UnitId::Second,
        UnitId::Minute,
        UnitId::MetrePerSecond,
        UnitId::KilometrePerHour,
    ]
}
fn next_binding(bindings: &BTreeMap<String, calc::Input>) -> String {
    (1..)
        .map(|n| format!("x{n}"))
        .find(|name| !bindings.contains_key(name))
        .expect("finite bindings")
}
fn relation_label(edge: &GraphEdge, records: &[Record]) -> String {
    let name = |id| {
        records
            .iter()
            .find(|r| r.id == id)
            .map(|r| format!("{} · {} #{}", r.kind.label(), r.name, id.0))
            .unwrap_or_else(|| format!("Unavailable #{}", id.0))
    };
    format!(
        "{}  — {} ->  {}{}",
        name(edge.id.source),
        edge.label,
        name(edge.id.target),
        if edge.binding_names.is_empty() {
            String::new()
        } else {
            format!(" [{}]", edge.binding_names.join(", "))
        }
    )
}
impl eframe::App for Workspace {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.preferences.poll();
        self.finish_writes(ctx);
        self.views.poll(&self.session);
        if let Some((action, kind)) = self.pending_continue.clone() {
            let native_current = match kind { WriteKind::Save => !self.session.is_dirty() && matches!(self.session.status(), stage1_io::SaveStatus::Saved), WriteKind::Recovery => self.session.recovery_current() };
            let preserved = self.views.preserved(&self.session, &kind);
            let verification = if native_current && preserved { Some(self.views.verify_preserved(&self.session, &kind)) } else { None };
            if verification.as_ref().is_some_and(|r| r.is_ok()) {
                self.pending_continue = None; self.pending = None;
                match action { Load::Quit => { self.allow_close = Some(self.session.editor.snapshot.digest()); self.close_intent = true; ctx.request_repaint(); }, action => self.load(action) }
            } else if let Some(Err(error)) = verification { self.pending_continue = None; self.result(Err(format!("Preserved artifact unavailable; live work retained: {error}"))); }
            else if !self.views.io.pending() && self.views.io.last_error.is_some() { self.pending_continue = None; }
        }
        let hash = self.session.editor.snapshot.digest();
        if self.allow_close.is_some() && !self.views.io.is_saved(&self.session, &self.views.views)
            && !self.views.io.recovery_current(&self.session, &self.views.views) { self.allow_close = None; }
        if self.allow_close.is_some() && (self.close_intent || ctx.input(|i| i.viewport().close_requested())) {
            let saved = self.views.verify_preserved(&self.session, &WriteKind::Save).is_ok();
            let recovered = self.session.recovery_current() && self.views.verify_preserved(&self.session, &WriteKind::Recovery).is_ok();
            if !saved && !recovered { self.allow_close = None; }
        }
        let requested = self.close_intent || ctx.input(|i| i.viewport().close_requested());
        let app_close = self.close_intent;
        self.close_intent = false;
        if requested && self.exchange.close_blocked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            self.exchange.keep_open_for_copy();
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        if requested && !self.assurance.request_close() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.request_repaint();
            return;
        }
        if requested && self.allow_close.is_none() && !self.views.blocked(&self.session)
            && self.views.verify_preserved(&self.session, &WriteKind::Save).is_err() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Load::Quit);
            self.result(Err("Saved view artifact could not be reverified; live layout retained.".into()));
            return;
        }
        if block_close(
            requested,
            self.session.is_dirty() || self.views.blocked(&self.session),
            self.session.pending() || self.views.io.pending(),
            &hash,
            self.allow_close.as_deref(),
        ) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            if !matches!(self.pending_continue, Some((Load::Quit, _))) {
                self.pending_continue = None;
            }
            self.pending = Some(Load::Quit);
            ctx.request_repaint();
        } else if app_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let now = Instant::now();
        let interval = Duration::from_secs(self.preferences.value.recovery_interval_seconds);
        if self.periodic_recovery.due(
            now,
            self.preferences.value.periodic_recovery,
            interval,
            self.pending.is_some(),
            &self.session,
            &self.views,
        ) {
            self.recovery();
        }
        if self.preferences.value.periodic_recovery || self.session.pending() || self.views.io.pending() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        craft_quit::on_close_request(ctx);
    }
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        self.frames = self.frames.saturating_add(1);
        self.audit.clear();
        self.handle_shortcuts(&ctx);
        egui::Panel::bottom("feedback").show(root, |ui| {
            ui.colored_label(if self.error { ERROR } else { INK }, &self.feedback);
        });
        self.frame_header(root);
        self.toolbar(root);
        self.sidebar(root);
        self.canvas(root);
        self.dialogs(&ctx);
        self.show_usages(&ctx);
        self.views.show_inspection(&ctx, &self.session.editor);
        self.capture(&ctx);
        self.catalog.show(&ctx);
        self.assurance.show(&ctx, &self.session.editor);
        self.audit.extend(self.assurance.audit());
        self.audit.extend(self.exchange.show(&ctx));
        let audit = serde_json::json!({"profile":"mbse-craft.ui-geometry/1","frame_policy":format!("{:?}",self.frame_choice.frame()),"frame_override":self.frame_choice.overridden(), "pixels_per_point":ctx.pixels_per_point(),
            "views_mapping_rehearsal":self.views.mapping_audit(&self.session.editor),"views_inspection_visible":self.views.inspection_visible(),"views_inspection_sha256":self.views.inspection_source_sha256(),"views_inspection_note":self.views.inspection_note(),"views_revision":self.views.views.document().revision().to_string(),"views_active":self.views.active.get().to_string(),"views_pending":self.views.io.pending(),"views_saved_hint":self.views.io.is_saved(&self.session,&self.views.views),"views_note":self.views.note,
            "model_revision":self.session.editor.snapshot.model_revision,"raw_version":self.session.editor.snapshot.raw_version,
            "native_usages":self.usages.as_ref().map(|report|serde_json::json!({"target":report.target().0,"rows":report.usages().len(),"current":report.check(&self.session.editor,&self.views.views).is_ok()})),"snapshot_hash":self.session.editor.snapshot.digest(),"selected":self.session.editor.selected.map(|id|id.0),
            "save_status":format!("{:?}",self.session.status()),"periodic_recovery":self.preferences.value.periodic_recovery,"recovery_interval_seconds":self.preferences.value.recovery_interval_seconds,"preferences_status":self.preferences.status,"write_pending":self.session.pending(),"external_copy_pending":self.exchange.copy_pending(),"source_edit_dirty":self.exchange.source_edit_dirty(),"source_edit_pending":self.exchange.source_edit_pending(),"recovery_path":self.session.last_recovery().map(|p|p.display().to_string()),"native_recovery_status":self.periodic_recovery.status(&self.session,&self.views).0,"layout_recovery_status":self.periodic_recovery.status(&self.session,&self.views).1,"views_recovery_current_hint":self.views.io.recovery_current(&self.session,&self.views.views),"views_pair_pending":self.views.pair_pending(),"views_gesture_active":self.views.layout_gesture_active(),"feedback":self.feedback,"error":self.error,"widgets":self.audit});
        if let Ok(bytes) = serde_json::to_vec_pretty(&audit) {
            if bytes != self.audit_previous {
                let temp = self.session.state_root.join("ui-geometry.pending");
                if std::fs::write(&temp, &bytes)
                    .and_then(|_| {
                        std::fs::rename(&temp, self.session.state_root.join("ui-geometry.json"))
                    })
                    .is_ok()
                {
                    self.audit_previous = bytes;
                }
            }
        }
    }
}
