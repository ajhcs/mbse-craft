//! App-owned UI over the bounded, provenance-preserving Stage1 contract.
use craft_ui::{
    buttons::{ButtonVisuals, IconButton, IconButtonStyle},
    theme::WidgetPalette,
};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use mbse_model::{
    stage1::{Command, CreationDraft, RawKey, Record, RecordId, RecordKind, SubjectRef, Validity},
    stage1_calc::{self as calc, CalculationId, Definition, Limits, QuantityType, UnitId},
    stage1_graph::{self, EdgeId, GraphEdge},
    stage1_io::{self, Session},
};
use std::{collections::BTreeMap, path::PathBuf};

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
    path: String,
    save_path: String,
    active_field: Option<RawKey>,
    feedback: String,
    error: bool,
    creation: Option<(RecordKind, RecordId, String)>,
    relation: Option<EdgeId>,
    popout: Option<RecordId>,
    pending: Option<Load>,
    capture: Option<PathBuf>,
    frames: u32,
    screenshot_requested: bool,
    zoom: f32,
    allow_close: Option<String>,
    audit: BTreeMap<String, [f32; 4]>,
    audit_previous: Vec<u8>,
}
impl Workspace {
    pub fn new(session: Session, capture: Option<PathBuf>, ctx: &egui::Context) -> Self {
        let mut style = (*ctx.style()).clone();
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
            Stroke::new(1., LINE),
        );
        style.spacing.item_spacing = Vec2::new(9., 8.);
        ctx.set_style(style);
        let path = session
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self { session, save_path: path.clone(), path, active_field: None, feedback: "Select a model card to edit its fields. Model values and unfinished text remain separate.".into(), error: false,
            creation: None, relation: None, popout: None, pending: None, capture, frames: 0, screenshot_requested: false, zoom: 1., allow_close: None, audit: BTreeMap::new(), audit_previous: Vec::new() }
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
    fn command(&mut self, command: Command) {
        let result = self
            .session
            .editor
            .command(&self.session.editor.freshness(), command);
        let message = result.map(|outcome| {
            if let Some(id) = outcome.created {
                self.select(id);
            }
            if outcome.accepted {
                "Model updated. Save includes accepted values and retained drafts.".into()
            } else {
                "Draft retained; last accepted model value is unchanged.".into()
            }
        });
        self.result(message);
    }
    fn select(&mut self, id: RecordId) {
        self.active_field = None;
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
        let key = RawKey {
            record: record.id,
            field: field.into(),
            item: None,
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
                .id_salt((record.id.0, field))
                .desired_width(f32::INFINITY),
        );
        self.bound(format!("field:{}:{}", record.id.0, field), &response);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label));
        if response.has_focus() {
            self.active_field = Some(key.clone());
        }
        if response.changed() {
            self.command(Command::EditText {
                key: key.clone(),
                text,
            });
        }
        if response.lost_focus() {
            self.session.editor.finish_field();
            self.active_field = None;
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
    fn save(&mut self) {
        let path = PathBuf::from(self.save_path.trim());
        if self.save_path.trim().is_empty() {
            self.result(Err("Enter a destination path before saving.".into()));
            return;
        }
        let result = self
            .session
            .begin_save(&path)
            .map(|job| {
                let result = stage1_io::execute_write(job);
                let error = result.error().map(|e| format!("{e:?}"));
                let consumed = self.session.complete(result);
                if let Some(error) = error {
                    return Err(error);
                }
                if !consumed
                    || self.session.is_dirty()
                    || !matches!(self.session.status(), stage1_io::SaveStatus::Saved)
                {
                    return Err(format!(
                        "Save receipt did not confirm current bytes: {:?}. Current work retained.",
                        self.session.status()
                    ));
                }
                Ok("Saved accepted values and raw drafts; current bytes verified.".into())
            })
            .and_then(|r| r);
        self.result(result);
    }
    fn recovery(&mut self) -> Result<String, String> {
        let job = self.session.begin_recovery()?;
        let result = stage1_io::execute_write(job);
        let error = result.error().map(|e| format!("{e:?}"));
        let consumed = self.session.complete(result);
        if let Some(error) = error {
            return Err(error);
        }
        if !consumed || !self.session.recovery_current() {
            return Err(
                "Recovery receipt did not confirm current snapshot; current work retained.".into(),
            );
        }
        Ok(format!(
            "Recovery copy: {}",
            self.session
                .last_recovery()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        ))
    }
    fn request_load(&mut self, action: Load) {
        if self.session.is_dirty() {
            self.pending = Some(action);
        } else {
            self.load(action);
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
                self.session = session;
                self.creation = None;
                self.relation = None;
                self.popout = None;
                self.save_path = self
                    .session
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                self.active_field = None;
                self.result(Ok(
                    "Model opened. Imported and recovered documents are independent drafts.".into(),
                ));
            }
            Err(error) => self.result(Err(error)),
        }
    }
    fn toolbar(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("workspace_toolbar").show(root, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("MBSE-Craft"); ui.label("Model workspace"); ui.separator();
                for (label, action) in [("New", Load::New), ("Open", Load::Open(PathBuf::from(self.path.trim()))), ("Recover", Load::Recover(PathBuf::from(self.path.trim()))), ("Import SysML", Load::Import(PathBuf::from(self.path.trim())))] {
                    let response = ui.button(label); self.bound(format!("action:{label}"),&response);
                    if response.clicked() { self.request_load(action); }
                }
                let save = ui.button("Save"); self.bound("action:Save",&save); if save.clicked() { self.save(); }
                let recovery = ui.button("Recovery copy"); self.bound("action:Recovery copy",&recovery); if recovery.clicked() { let result = self.recovery(); self.result(result); }
                if ui.button("Export SysML copy").clicked() {
                    let result = self.session.begin_export_sysml(&PathBuf::from(self.path.trim())).and_then(|job| self.session.complete_export(stage1_io::execute_export(job))).and_then(|accepted| if accepted { Ok("Exported restricted SysML copy.".into()) } else { Err("Export receipt was not accepted.".into()) });
                    self.result(result);
                }
                ui.separator();
                // Real ArtCraft IconButton controls with accessible action labels.
                let visual = ButtonVisuals { fill: Color32::from_rgb(231, 239, 238), stroke: Stroke::new(1., LINE), icon: ACCENT };
                let style = IconButtonStyle { normal: visual, hovered: visual, pressed: Some(visual), selected: visual,
                    corner_radius: egui::CornerRadius::same(4), focus_stroke: Stroke::new(2., ACCENT) };
                for (label, undo, enabled) in [("Undo", true, self.session.editor.can_undo() || self.active_field.is_some()), ("Redo", false, self.session.editor.can_redo() || self.active_field.is_some())] {
                    ui.add_enabled_ui(enabled, |ui| {
                        let response = IconButton::new(label, Vec2::splat(30.), &style).show(ui, |ui, rect, tint| {
                            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, if undo { "↶" } else { "↷" }, FontId::proportional(20.), tint);
                        });
                        self.bound(format!("action:{label}"),&response);
                        if response.clicked() { self.history(undo); }
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
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{:?} · model r{} · raw v{}", self.session.status(), self.session.editor.snapshot.model_revision, self.session.editor.snapshot.raw_version));
                if self.session.recovery_current() { ui.colored_label(ACCENT, "Current recovery copy"); }
                ui.label(RichText::new("App-owned model · restricted SysML subset").small());
            });
        });
    }
    fn history(&mut self, undo: bool) {
        let freshness = self.session.editor.freshness();
        let result = if let Some(key) = self.active_field.as_ref() {
            if undo {
                self.session.editor.undo_text(&freshness, key)
            } else {
                self.session.editor.redo_text(&freshness, key)
            }
        } else if undo {
            self.session.editor.undo_model(&freshness)
        } else {
            self.session.editor.redo_model(&freshness)
        };
        self.result(result.map(|changed| {
            if changed {
                "Model history applied. Raw drafts remain separately retained.".into()
            } else {
                "No model history available.".into()
            }
        }));
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
                    if ui
                        .checkbox(&mut linked, format!("{} #{}", mission.name, mission.id.0))
                        .changed()
                    {
                        self.command(Command::ToggleMission {
                            capability: record.id,
                            mission: mission.id,
                        });
                    }
                }
            }
            RecordKind::Measure => {
                ui.label(RichText::new("Quantity and unit").strong());
                egui::ComboBox::from_id_salt(("measure-unit", record.id.0))
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
                            if ui
                                .selectable_label(record.text("unit") == unit, label)
                                .clicked()
                            {
                                self.command(Command::SetQuantity {
                                    measure: record.id,
                                    quantity: quantity.into(),
                                    unit: unit.into(),
                                });
                            }
                        }
                    });
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
            RecordKind::Stakeholder => self.field(ui, record, "interest", "Interest", true),
            RecordKind::Expectation => self.field(ui, record, "text", "Expectation", true),
            _ => {
                ui.label("Typed record; additional source properties remain preserved in the native file.");
            }
        }
    }
    fn calculation(&mut self, ui: &mut egui::Ui, record: &Record) {
        ui.separator();
        ui.label(RichText::new("Local calculation").strong());
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
                        if ui
                            .selectable_label(
                                unit == definition.output.unit,
                                calc::unit_label(unit),
                            )
                            .clicked()
                        {
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
    }
    fn canvas(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().show(root, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("System canvas");
                ui.label("Select to edit · Details opens a popout");
                ui.add(egui::Slider::new(&mut self.zoom, 0.65..=1.4).text("Scale"));
            });
            let records: Vec<_> = self
                .session
                .editor
                .snapshot
                .records
                .values()
                .cloned()
                .collect();
            let graph = match stage1_graph::derive_graph(&self.session.editor.snapshot) {
                Ok(graph) => graph,
                Err(error) => {
                    ui.colored_label(ERROR, error);
                    return;
                }
            };
            let canvas_height = (ui.available_height() * 0.60).max(260.);
            egui::ScrollArea::both()
                .id_salt("primary-canvas")
                .max_height(canvas_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let scale = self.zoom;
                    let size = Vec2::new(
                        1170. * scale,
                        (records.len().div_ceil(3) as f32 * 155. + 50.).max(350.) * scale,
                    );
                    let (area, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    let rects: BTreeMap<_, _> = records
                        .iter()
                        .enumerate()
                        .map(|(index, r)| {
                            let min = area.min
                                + Vec2::new(
                                    25. + (index % 3) as f32 * 385.,
                                    24. + (index / 3) as f32 * 155.,
                                ) * scale;
                            (
                                r.id,
                                Rect::from_min_size(min, Vec2::new(330., 112.) * scale),
                            )
                        })
                        .collect();
                    for (index, edge) in graph.edges.iter().enumerate() {
                        let (Some(source), Some(target)) =
                            (rects.get(&edge.id.source), rects.get(&edge.id.target))
                        else {
                            continue;
                        };
                        let a = source.center_bottom();
                        let b = target.center_top();
                        let bend = Pos2::new(a.x + (index % 5) as f32 * 8., a.y + 14. * scale);
                        ui.painter()
                            .line_segment([a, bend], Stroke::new(1., ACCENT));
                        ui.painter().arrow(bend, b - bend, Stroke::new(1., ACCENT));
                    }
                    for record in &records {
                        let rect = rects[&record.id];
                        let selected = self.session.editor.selected == Some(record.id);
                        ui.painter().rect(
                            rect,
                            7.,
                            PAPER,
                            Stroke::new(
                                if selected { 2.5 } else { 1. },
                                if selected { ACCENT } else { LINE },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(14., 13.) * scale,
                            egui::Align2::LEFT_TOP,
                            format!("{}  #{}", record.kind.label(), record.id.0),
                            FontId::proportional(12. * scale),
                            ACCENT,
                        );
                        let name = if record.name.is_empty() {
                            "Name your system"
                        } else {
                            &record.name
                        };
                        let galley = ui.painter().layout(
                            name.into(),
                            FontId::proportional(17. * scale),
                            INK,
                            rect.width() - 28. * scale,
                        );
                        ui.painter().with_clip_rect(rect.shrink(5.)).galley(
                            rect.min + Vec2::new(14., 36.) * scale,
                            galley,
                            INK,
                        );
                        let select_rect =
                            Rect::from_min_max(rect.min, rect.max - Vec2::new(0., 28. * scale));
                        let response = ui.interact(
                            select_rect,
                            ui.id().with(("record", record.id.0)),
                            egui::Sense::click(),
                        );
                        self.bound(format!("card:{}", record.id.0), &response);
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("{} {} #{}", record.kind.label(), name, record.id.0),
                            )
                        });
                        if response.clicked() {
                            self.select(record.id);
                        }
                        let details = Rect::from_min_size(
                            rect.left_bottom() + Vec2::new(12. * scale, -27. * scale),
                            Vec2::new(90., 22.) * scale,
                        );
                        if ui.put(details, egui::Button::new("Details…")).clicked() {
                            self.popout = Some(record.id);
                        }
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
                        }
                    }
                    if graph.edges.is_empty() {
                        ui.label("Add records and typed links to populate the relation ledger.");
                    }
                });
        });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
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
                ui.label(format!("Source #{} → Target #{}", edge.source.0, edge.target.0));
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
            egui::Window::new("Preserve current work").collapsible(false).resizable(false).show(ctx, |ui| {
                ui.label("The current model has unsaved accepted values or retained drafts.");
                ui.label("Save to the file path, or create a durable recovery copy before continuing.");
                if ui.button("Save and continue").clicked() {
                    self.save();
                    if !self.session.is_dirty() { self.pending = None; if matches!(action,Load::Quit) { self.allow_close = Some(self.session.editor.snapshot.digest()); ctx.send_viewport_cmd(egui::ViewportCommand::Close); } else { self.load(action.clone()); } }
                }
                if ui.button("Keep recovery and continue").clicked() {
                    let result = self.recovery(); let ok = result.is_ok(); self.result(result);
                    if ok { self.pending = None; if matches!(action,Load::Quit) { self.allow_close = Some(self.session.editor.snapshot.digest()); ctx.send_viewport_cmd(egui::ViewportCommand::Close); } else { self.load(action.clone()); } }
                }
                if ui.button("Cancel").clicked() { self.pending = None; }
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
        "{}  — {} →  {}{}",
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
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        self.frames = self.frames.saturating_add(1);
        self.audit.clear();
        if self.allow_close.as_deref() != Some(self.session.editor.snapshot.digest().as_str())
            && ctx.input(|i| i.viewport().close_requested())
            && self.session.is_dirty()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Load::Quit);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save();
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            )
        }) {
            self.history(false);
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
            self.history(true);
        }
        egui::Panel::bottom("feedback").show(root, |ui| {
            ui.colored_label(if self.error { ERROR } else { INK }, &self.feedback);
        });
        self.toolbar(root);
        self.sidebar(root);
        self.canvas(root);
        self.dialogs(&ctx);
        self.capture(&ctx);
        let audit = serde_json::json!({"profile":"mbse-craft.ui-geometry/1", "pixels_per_point":ctx.pixels_per_point(),
            "model_revision":self.session.editor.snapshot.model_revision,"raw_version":self.session.editor.snapshot.raw_version,
            "snapshot_hash":self.session.editor.snapshot.digest(),"selected":self.session.editor.selected.map(|id|id.0),
            "save_status":format!("{:?}",self.session.status()),"recovery_path":self.session.last_recovery().map(|p|p.display().to_string()),"recovery_current":self.session.recovery_current(),"feedback":self.feedback,"error":self.error,"widgets":self.audit});
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
